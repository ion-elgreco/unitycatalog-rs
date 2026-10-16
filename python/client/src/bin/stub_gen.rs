//! Write the stubs of `unitycatalog._core` from the Rust code, into
//! `python/client/unitycatalog/_core/`. Run it with `just generate-stubs`: it
//! builds this program without `extension-module`, as it runs Python.

fn main() -> pyo3_stub_gen::Result<()> {
    _core::stub_info()?.generate()
}
