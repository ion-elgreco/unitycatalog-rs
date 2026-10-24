//! Each route of the client has a call in the tests of this folder: each
//! method of `Client` that sends a request (`src/codegen/client.rs`), and
//! each such method of a resource handle (`src/codegen/*/resource.rs`),
//! called on its handle, such as `.catalog(name).get()`.

use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// The methods of the source `code`: name and return type.
fn methods(code: &str) -> Vec<(String, String)> {
    code.split("pub fn ")
        .skip(1)
        .filter_map(|item| {
            let name = item.split('(').next()?.trim();
            let signature = item.split('{').next()?;
            let returns = signature.rsplit("->").next()?.trim();
            Some((name.to_string(), returns.to_string()))
        })
        .collect()
}

/// The code of the tests of this folder, without whitespace.
fn tests() -> String {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/client");
    fs::read_dir(dir)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.file_name().is_some_and(|name| name != "coverage.rs"))
        .map(|path| fs::read_to_string(path).unwrap())
        .collect::<String>()
        .split_whitespace()
        .collect()
}

/// Whether `code` calls `.method()` on the result of a call `.accessor(..)`.
fn calls_on(code: &str, accessor: &str, method: &str) -> bool {
    let start = format!(".{accessor}(");
    let call = format!(".{method}()");
    code.match_indices(&start).any(|(at, _)| {
        let mut depth = 0;
        let rest = &code[at + start.len() - 1..];
        let end = rest.char_indices().find_map(|(i, c)| {
            match c {
                '(' => depth += 1,
                ')' => depth -= 1,
                _ => {}
            }
            (depth == 0).then_some(i)
        });
        end.is_some_and(|end| rest[end + 1..].starts_with(&call))
    })
}

#[test]
fn every_route_has_a_test() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/codegen");
    let client = methods(&fs::read_to_string(root.join("client.rs")).unwrap());
    let code = tests();
    let mut missing = Vec::new();
    for (name, returns) in &client {
        if returns.ends_with("Builder") && !code.contains(&format!(".{name}(")) {
            missing.push(name.clone());
        }
    }
    // The handles: the accessor of each one, such as `catalog` for
    // `CatalogClient`.
    let accessors: HashMap<&str, &str> = client
        .iter()
        .map(|(name, returns)| (returns.as_str(), name.as_str()))
        .collect();
    let mut handles = 0;
    for entry in fs::read_dir(&root).unwrap() {
        let resource = entry.unwrap().path().join("resource.rs");
        let Ok(source) = fs::read_to_string(&resource) else {
            continue;
        };
        let handle = source
            .split("pub struct ")
            .nth(1)
            .and_then(|rest| rest.split_whitespace().next())
            .unwrap();
        let accessor = accessors[handle];
        handles += 1;
        for (method, returns) in methods(&source) {
            if returns.ends_with("Builder") && !calls_on(&code, accessor, &method) {
                missing.push(format!("{accessor}(..).{method}"));
            }
        }
    }
    assert!(client.len() > 100 && handles > 10, "{client:?}");
    assert!(missing.is_empty(), "no test calls {missing:?}");
}
