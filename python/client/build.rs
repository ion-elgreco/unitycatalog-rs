// On macOS, link the extension module as maturin does, with Python's symbols
// left to resolve when Python loads it; else `cargo build` fails to link.
fn main() {
    pyo3_build_config::add_extension_module_link_args();
}
