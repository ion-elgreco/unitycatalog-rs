//! `unitycatalog._core`: the extension module of the Python client.
//!
//! The classes come from `codegen`, which trestle generates from the protos in `proto/`
//! (`just generate`): `Client` wraps `uc_client::Client`, and each method comes twice,
//! `name` and `name_async`. The models are in a submodule per proto package, such as
//! `unitycatalog._core.catalogs`. pyo3-stub-gen writes their stubs from this code:
//! `just generate-stubs`.

use pyo3::prelude::*;

mod codegen;

#[pymodule]
fn _core(m: &Bound<'_, PyModule>) -> PyResult<()> {
    codegen::register(m)?;
    // Not `m.add`: that would also put it in `__all__`, unlike the stub.
    m.setattr("__version__", env!("CARGO_PKG_VERSION"))?;
    Ok(())
}

pyo3_stub_gen::module_doc!(
    "unitycatalog._core",
    "The extension module of the client: `Client`, written in Rust."
);
pyo3_stub_gen::module_variable!("unitycatalog._core", "__version__", String);

pyo3_stub_gen::define_stub_info_gatherer!(stub_info);
