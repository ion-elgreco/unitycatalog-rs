//! Generates the clients from the protos in `proto/`: `just generate`.
//!
//! 1. `protoc` compiles the protos into a descriptor set, with their comments.
//! 2. buffa writes the models; trestle's `olai-codegen` writes the Rust client, the PyO3
//!    bindings (with their exception classes and Tokio runtime) and the enums, `_enums.py`.
//! 3. The JSON rewrites make the models send snake_case field names, and 64-bit integers and
//!    whole numbers of free-form JSON as numbers, as the server does.
//!
//! All of it is written to `target/uc-codegen` first. Then the generated directories of the repo
//! are replaced with it, and rustfmt formats them, with blank lines between items:
//!
//! - `crates/uc-client/src/models/_gen`: the models
//! - `crates/uc-client/src/codegen`: the Rust client
//! - `python/client/src/codegen`: the PyO3 bindings
//! - `python/client/unitycatalog/_enums.py`
//!
//! `--check <dir> <proto>...` generates only from those protos into `dir`, and writes nothing
//! in the repo: it checks a proto without touching the generated code.

use std::collections::BTreeSet;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use olai_codegen::postprocess::{JsonFieldNames, ModelRewrites, field_options, space_items};
use olai_codegen::{
    BindingsConfig, ClientProtocols, CodeGenConfig, CodeGenOutput, ErrorModuleConfig,
    PyConstructors, PyErrorModuleConfig, PyPackageConfig, PythonOptions,
    REQWEST_TRANSPORT_TYPE_PATH, RestClientOptions, Runtime, generate_code,
    parse_file_descriptor_set,
};
use protobuf::Message;
use protobuf::descriptor::FileDescriptorSet;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

/// The proto packages whose JSON field names, and query keys, are lowerCamelCase: SCIM and the
/// Delta Sharing protocol.
const CAMEL_CASE_PACKAGES: &[&str] = &["unitycatalog.scim.v1", "unitycatalog.sharing.v1"];

/// The RPCs whose request body is a form: OAuth's token exchange and token request.
const FORM_METHODS: &[&str] = &[
    "unitycatalog.auth.v1.AuthService.ExchangeToken",
    "unitycatalog.auth.v1.AuthService.RequestAccessToken",
];

/// The RPCs whose request body keeps the path values. Elsewhere the body leaves them out, and
/// the Delta API of the upstream server rejects them. Upstream's finalize checks that the body
/// names the model version of the path, and the body of a SCIM `PUT` is the whole resource,
/// with its `id`.
const PATH_FIELDS_IN_BODY_METHODS: &[&str] = &[
    "unitycatalog.models.v1.ModelVersionsService.FinalizeModelVersion",
    "unitycatalog.scim.v1.ScimService.ReplaceUser",
    "unitycatalog.scim.v1.ScimService.ReplaceGroup",
];

fn main() -> Result<()> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let root = root.canonicalize()?;
    let args: Vec<String> = std::env::args().skip(1).collect();
    let check = args.first().is_some_and(|a| a == "--check");
    let (scratch, protos) = if check {
        let [_, dir, protos @ ..] = args.as_slice() else {
            return Err("usage: uc-codegen --check <dir> <proto>...".into());
        };
        let protos = protos
            .iter()
            .map(|p| Path::new(p).canonicalize())
            .collect::<std::io::Result<_>>()?;
        (PathBuf::from(dir), protos)
    } else {
        (
            root.join("target/uc-codegen"),
            proto_files(&root.join("proto/unitycatalog"))?,
        )
    };
    if scratch.exists() {
        fs::remove_dir_all(&scratch)?;
    }

    let descriptors = compile_descriptors(&root.join("proto"), &protos, &scratch)?;
    let config = config(&scratch);
    let gen_dir = config
        .output
        .models_subdir_path()
        .expect("the config has a models dir");
    for dir in [
        &gen_dir,
        &scratch.join("client"),
        &scratch.join("python"),
        &scratch.join("pkg"),
    ] {
        fs::create_dir_all(dir)?;
    }

    // The models. buffa writes a file per proto file and `<package>.mod.rs`; the models
    // `mod.rs` of trestle includes `<package>.rs`.
    buffa_build::Config::new()
        .files(&protos)
        .includes(&[root.join("proto")])
        .generate_json(true)
        .generate_views(false)
        .out_dir(&gen_dir)
        .include_file("_include.rs")
        .compile()?;
    let fds = FileDescriptorSet::parse_from_bytes(&fs::read(&descriptors)?)?;
    let packages: BTreeSet<&str> = fds
        .file
        .iter()
        .filter(|f| f.name().starts_with("unitycatalog/"))
        .map(|f| f.package())
        .collect();
    for package in &packages {
        fs::write(
            gen_dir.join(format!("{package}.rs")),
            format!("include!(\"{package}.mod.rs\");\n"),
        )?;
    }

    let metadata = parse_file_descriptor_set(&fds)?;
    generate_code(&metadata, &config)?;

    ModelRewrites {
        field_names: JsonFieldNames::Proto,
        packages: CAMEL_CASE_PACKAGES
            .iter()
            .map(|p| (p.to_string(), JsonFieldNames::Json))
            .collect(),
        int64_module: config.json_int64_module_path(),
        json_value_module: config.json_value_module_path(),
        // Explicit `json_name`s and `(olai.trestle.v1.emit_unpopulated)`.
        fields: field_options(&metadata),
    }
    .rewrite_dir(&gen_dir)?;
    if check {
        println!(
            "checked {} protos; the output is in {}",
            protos.len(),
            scratch.display()
        );
        return Ok(());
    }

    let outputs = [
        (gen_dir, root.join("crates/uc-client/src/models/_gen")),
        (
            scratch.join("client"),
            root.join("crates/uc-client/src/codegen"),
        ),
        (
            scratch.join("python"),
            root.join("python/client/src/codegen"),
        ),
    ];
    let mut written = Vec::new();
    for (from, to) in &outputs {
        if to.exists() {
            fs::remove_dir_all(to)?;
        }
        copy_tree(from, to, &mut written)?;
    }
    // No enums, no `_enums.py`.
    let enums = root.join("python/client/unitycatalog/_enums.py");
    match scratch.join("pkg/_enums.py") {
        generated if generated.exists() => {
            fs::copy(generated, &enums)?;
        }
        _ if enums.exists() => fs::remove_file(&enums)?,
        _ => {}
    }

    for file in written
        .iter()
        .filter(|p| p.extension().is_some_and(|e| e == "rs"))
    {
        rustfmt(file, &root.join("rustfmt.toml"))?;
    }
    println!(
        "generated {} files from {} protos",
        written.len(),
        protos.len()
    );
    Ok(())
}

/// The generation config: the Rust client in `uc-client`, the Python bindings in `uc-py`.
fn config(scratch: &Path) -> CodeGenConfig {
    let models = scratch.join("models");
    CodeGenConfig {
        // The server is hand-written, so the handler types are not used.
        context_type_path: "crate::Context".into(),
        result_type_path: "crate::Result".into(),
        // `uc-client` names itself `uc_client` (`extern crate self`), as the Python bindings do.
        models_path_template: "uc_client::models::{service}::{version}".into(),
        models_path_crate_template: "crate::models::{service}::{version}".into(),
        resource_store_crate_name: "olai_store".into(),
        runtime: Runtime::Buffa,
        transport_type_path: REQWEST_TRANSPORT_TYPE_PATH.into(),
        dual_transport: false,
        client_protocols: ClientProtocols::default(),
        connect_client_path: None,
        rest_client: RestClientOptions {
            json_int64_as_number: true,
            ignore_unknown_enum_values: true,
            // Unity Catalog's `{"error_code", "message", "details"}`, and the error bodies of
            // Iceberg REST and Delta REST (`{"error": {"message", "type"}}`), SCIM (`scimType`,
            // `detail`) and OAuth (`error`, `error_description`).
            error_module: Some(ErrorModuleConfig {
                more_code_fields: vec!["type".into(), "scimType".into(), "error".into()],
                more_message_fields: vec!["detail".into(), "error_description".into()],
                envelope_fields: vec!["error".into()],
                ..ErrorModuleConfig::default()
            }),
            form_encoded_methods: FORM_METHODS.iter().map(|m| m.to_string()).collect(),
            path_fields_in_body_methods: PATH_FIELDS_IN_BODY_METHODS
                .iter()
                .map(|m| m.to_string())
                .collect(),
            json_query_packages: CAMEL_CASE_PACKAGES.iter().map(|p| p.to_string()).collect(),
        },
        output: CodeGenOutput {
            common: models.join("_gen"),
            models: Some(models),
            models_subdir: "_gen".into(),
            server: None,
            client: Some(scratch.join("client")),
            generate_resource_clients: true,
            python: Some(scratch.join("python")),
            node: None,
            node_ts: None,
            wasm: None,
            python_typings_filename: "client.pyi".into(),
        },
        generate_resource_enum: false,
        generate_store_integration: false,
        error_type_path: None,
        generate_object_conversions: false,
        bindings: Some(BindingsConfig {
            aggregate_client_name: "Client".into(),
            client_crate_name: "uc_client".into(),
            py_error_type: "PyUnityCatalogError".into(),
            py_result_type: "PyUnityCatalogResult".into(),
            napi_error_ext_trait: "NapiErrorExt".into(),
            typings_package_filter: None,
            ts_error_base_class: "UnityCatalogError".into(),
            ts_error_code_prefix: "UC".into(),
            python: PythonOptions {
                frozen_models: true,
                constructors: PyConstructors::Inputs,
                python_repr: true,
                module: Some("unitycatalog._core".into()),
                async_methods: true,
                runtime: true,
                keyword_only: true,
                str_enums: true,
                hide_unspecified: true,
                stub_gen: true,
                error_module: Some(PyErrorModuleConfig {
                    base_class: "UnityCatalogError".into(),
                }),
                package: Some(PyPackageConfig {
                    dir: scratch.join("pkg"),
                    name: "unitycatalog".into(),
                }),
            },
        }),
    }
}

/// Format `file` with blank lines between its items (buffa's models have none), then with
/// rustfmt and `config`. Through stdin, so that rustfmt formats only this file and not the
/// files of its `mod` declarations.
fn rustfmt(file: &Path, config: &Path) -> Result<()> {
    let mut child = Command::new("rustfmt")
        .args(["--edition", "2024", "--emit", "stdout", "--config-path"])
        .arg(config)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()?;
    child
        .stdin
        .take()
        .expect("stdin is piped")
        .write_all(space_items(&fs::read_to_string(file)?).as_bytes())?;
    let output = child.wait_with_output()?;
    if !output.status.success() {
        return Err(format!("rustfmt failed on {}", file.display()).into());
    }
    fs::write(file, output.stdout)?;
    Ok(())
}

/// The `.proto` files under `dir`, sorted.
fn proto_files(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        for entry in fs::read_dir(&d)? {
            let path = entry?.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().is_some_and(|e| e == "proto") {
                out.push(path);
            }
        }
    }
    out.sort();
    Ok(out)
}

/// Compile `protos` into a descriptor set with their imports and comments; its path.
fn compile_descriptors(include: &Path, protos: &[PathBuf], scratch: &Path) -> Result<PathBuf> {
    fs::create_dir_all(scratch)?;
    let descriptors = scratch.join("api.bin");
    let status = Command::new("protoc")
        .args(["--include_imports", "--include_source_info", "-I"])
        .arg(include)
        .arg(format!("--descriptor_set_out={}", descriptors.display()))
        .args(protos)
        .status()
        .map_err(|e| format!("protoc: {e} (is it installed?)"))?;
    if !status.success() {
        return Err("protoc failed".into());
    }
    Ok(descriptors)
}

/// Copy the tree `from` to `to`, without the server's request extractors and buffa's include
/// file, which the clients do not use. Adds the files it writes to `written`.
fn copy_tree(from: &Path, to: &Path, written: &mut Vec<PathBuf>) -> Result<()> {
    fs::create_dir_all(to)?;
    for entry in fs::read_dir(from)? {
        let path = entry?.path();
        let name = path.file_name().expect("a directory entry has a name");
        if name == "extractors" || name == "_include.rs" || name == "client.pyi" {
            continue;
        }
        let target = to.join(name);
        if path.is_dir() {
            copy_tree(&path, &target, written)?;
        } else if name == "mod.rs" {
            // The declaration of the extractors left out above.
            let source = fs::read_to_string(&path)?;
            fs::write(
                &target,
                source.replace("#[cfg(feature = \"axum\")]\npub mod extractors;\n", ""),
            )?;
            written.push(target);
        } else {
            fs::copy(&path, &target)?;
            written.push(target);
        }
    }
    Ok(())
}
