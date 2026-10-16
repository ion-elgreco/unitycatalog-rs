//! The Tokio runtime of the client's requests.
//!
//! One multi-thread runtime, made on import: the sync methods block on it and
//! `pyo3-async-runtimes` spawns the async ones on it. It panics in a forked
//! child, where Tokio's thread pool does not work.

use std::sync::OnceLock;

use tokio::runtime::{Builder, Runtime};

pub fn rt() -> &'static Runtime {
    static TOKIO_RT: OnceLock<Runtime> = OnceLock::new();
    static PID: OnceLock<u32> = OnceLock::new();
    let pid = std::process::id();
    let runtime_pid = *PID.get_or_init(|| pid);
    if pid != runtime_pid {
        panic!(
            "Forked process detected — current PID is {pid} but the tokio runtime was created by \
             {runtime_pid}. The tokio runtime does not support forked processes \
             (https://github.com/tokio-rs/tokio/issues/4301). When using Python multiprocessing, \
             set the start method to 'spawn' or 'forkserver'."
        );
    }
    TOKIO_RT.get_or_init(|| {
        Builder::new_multi_thread()
            .enable_all()
            .thread_name("uc-py")
            .build()
            .expect("uc-py: failed to build tokio runtime")
    })
}

/// Make `rt()` the runtime of `pyo3_async_runtimes`, so that `future_into_py` uses it.
pub fn init_async_runtime() {
    pyo3_async_runtimes::tokio::init_with_runtime(rt())
        .expect("uc-py: failed to install tokio runtime into pyo3-async-runtimes");
}
