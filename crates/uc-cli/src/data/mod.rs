//! The operations on data, which run on this machine as upstream's do: the
//! server gives the metadata and vends credentials, and `uc` reads and
//! writes the storage itself.
//!
//! - [`delta`]: `table create`, `read` and `write` with the Delta kernel.
//! - [`files`]: `volume read` and `write`.
//! - [`python`]: `function call` with `python3`.
//! - [`storage`]: the vended credentials and the object stores.

pub mod delta;
pub mod files;
pub mod python;
mod storage;

pub use storage::S3Store;
