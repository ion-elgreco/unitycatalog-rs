//! `unitycatalog._core`: the extension module of the Python client.
//!
//! The module is `codegen::extension_module`, which trestle generates from the protos in
//! `proto/` (`just generate`): `Client` wraps `uc_client::Client`, and each method comes twice,
//! `name` and `name_async`. The models are in a submodule per proto package, such as
//! `unitycatalog._core.catalogs`. The hand-written submodule `identity` has the token sources of
//! cloud identities. PyO3's introspection writes the stubs from the built module:
//! `just generate-stubs`.

mod codegen;
mod identity;
