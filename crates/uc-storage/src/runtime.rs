//! The runtime for outbound IO: the connections to object stores and HTTP
//! services. The runtime that serves the API runs the requests; their
//! sockets and connection tasks run here.

use std::future::Future;
use std::pin::Pin;
use std::sync::OnceLock;
use std::task::{Context, Poll};

use tokio::runtime::{Builder, Handle, Runtime};
use tokio::task::JoinHandle;

/// Worker threads of the IO runtime when [`init_io`] does not set them, as
/// the default of `server.io_threads`.
pub const DEFAULT_IO_THREADS: usize = 1;

static IO: OnceLock<Runtime> = OnceLock::new();

/// Start the IO runtime with `threads` worker threads (at least 1), unless
/// it runs already.
pub fn init_io(threads: usize) {
    IO.get_or_init(|| build(threads));
}

/// The IO runtime; its first use starts it with [`DEFAULT_IO_THREADS`].
pub fn io_handle() -> Handle {
    IO.get_or_init(|| build(DEFAULT_IO_THREADS))
        .handle()
        .clone()
}

/// Run `future` on the IO runtime and wait for its output here. A panic in
/// `future` panics here; dropping the returned future stops it.
pub fn spawn_io<F>(future: F) -> IoTask<F::Output>
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
{
    IoTask(io_handle().spawn(future))
}

fn build(threads: usize) -> Runtime {
    Builder::new_multi_thread()
        .worker_threads(threads.max(1))
        .thread_name("uc-io")
        .enable_all()
        .build()
        .expect("cannot start the IO runtime")
}

/// The output of a future that runs on the IO runtime ([`spawn_io`]).
pub struct IoTask<T>(JoinHandle<T>);

impl<T> Future for IoTask<T> {
    type Output = T;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<T> {
        Pin::new(&mut self.get_mut().0)
            .poll(cx)
            .map(|done| match done {
                Ok(output) => output,
                Err(e) => match e.try_into_panic() {
                    Ok(panic) => std::panic::resume_unwind(panic),
                    // Only `drop` aborts the task, and then nothing polls it.
                    Err(e) => unreachable!("IO task stopped: {e}"),
                },
            })
    }
}

impl<T> Drop for IoTask<T> {
    fn drop(&mut self) {
        self.0.abort();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn runs_on_the_io_runtime() {
        let name = spawn_io(async { std::thread::current().name().map(str::to_string) }).await;
        assert_eq!(name.as_deref(), Some("uc-io"));
    }

    #[tokio::test]
    #[should_panic(expected = "boom")]
    async fn panics_reach_the_caller() {
        spawn_io(async { panic!("boom") }).await
    }

    #[tokio::test]
    async fn dropping_the_task_stops_it() {
        let (tx, rx) = tokio::sync::oneshot::channel::<()>();
        let task = spawn_io(async move {
            tokio::time::sleep(std::time::Duration::from_secs(60)).await;
            drop(tx);
        });
        drop(task);
        // The sender is dropped with the aborted task, before its sleep ends.
        assert!(
            tokio::time::timeout(std::time::Duration::from_secs(5), rx)
                .await
                .is_ok()
        );
    }
}
