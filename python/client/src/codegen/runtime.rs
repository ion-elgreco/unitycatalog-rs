// @generated — do not edit by hand.
//! The Tokio runtime of the bindings: one multi-thread runtime, made on first use. The
//! methods block on it, and the `_async` methods run on it. It panics in a forked child,
//! where Tokio's thread pool does not work.

use std::sync::OnceLock;
use tokio::runtime::{Builder, Runtime};

/// The runtime. It panics in a forked child process.
pub fn rt() -> &'static Runtime {
    static RUNTIME: OnceLock<Runtime> = OnceLock::new();
    static PID: OnceLock<u32> = OnceLock::new();
    let pid = std::process::id();
    let runtime_pid = *PID.get_or_init(|| pid);
    if pid != runtime_pid {
        panic!(
            "Forked process detected: current PID is {pid} but the tokio runtime was \
             created by {runtime_pid}. The tokio runtime does not support forked processes \
             (https://github.com/tokio-rs/tokio/issues/4301). When using Python \
             multiprocessing, set the start method to 'spawn' or 'forkserver'."
        );
    }
    RUNTIME.get_or_init(|| {
        Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("failed to build the tokio runtime")
    })
}

/// Make [`rt`] the runtime of `pyo3-async-runtimes`, so that the `_async` methods run
/// on it. The generated `register` calls it; a second call keeps the runtime already
/// installed.
pub fn init_async_runtime() {
    let _ = ::pyo3_async_runtimes::tokio::init_with_runtime(rt());
}
