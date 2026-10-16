//! `unitycatalog._core`: the extension module of the Python client.
//!
//! [`client::Client`] wraps `uc_client::Client`; each method comes twice,
//! `name` and `name_async`. The classes that it takes and returns are in a
//! submodule per resource, as in `uc_api`: `unitycatalog._core.catalogs`,
//! `unitycatalog._core.tables`, … pyo3-stub-gen writes their stubs from
//! this code: `cargo run -p uc-py --bin stub_gen`.
#![allow(clippy::too_many_arguments)]

use pyo3::prelude::*;

#[macro_use]
mod utils;

mod admin;
mod audit;
mod catalogs;
mod client;
mod credentials;
mod errors;
mod external_locations;
mod functions;
mod iceberg_rest;
mod metastore;
mod models;
mod permissions;
mod py_future;
mod runtime;
mod schemas;
mod scim;
mod str_enum;
mod tables;
mod tokens;
mod volumes;

#[pymodule]
fn _core(m: &Bound<'_, PyModule>) -> PyResult<()> {
    runtime::init_async_runtime();

    m.add_class::<client::Client>()?;
    catalogs::register_module(m)?;
    schemas::register_module(m)?;
    tables::register_module(m)?;
    volumes::register_module(m)?;
    functions::register_module(m)?;
    models::register_module(m)?;
    credentials::register_module(m)?;
    external_locations::register_module(m)?;
    permissions::register_module(m)?;
    metastore::register_module(m)?;
    tokens::register_module(m)?;
    scim::register_module(m)?;
    admin::register_module(m)?;
    audit::register_module(m)?;
    iceberg_rest::register_module(m)?;

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
