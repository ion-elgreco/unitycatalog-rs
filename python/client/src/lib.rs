//! `unitycatalog._core`: the extension module of the Python client.
//!
//! The module is `codegen::extension_module`, which trestle generates from the protos in
//! `proto/` (`just generate`): `Client` wraps `uc_client::Client`, and each method comes twice,
//! `name` and `name_async`. The models are in a submodule per proto package, such as
//! `unitycatalog._core.catalogs`. PyO3's introspection writes their stubs from the built
//! module: `just generate-stubs`.

mod codegen;
