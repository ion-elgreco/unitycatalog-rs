//! The API's listener and its workers. One loop accepts the connections and
//! hands them to the workers in turn. A worker is a thread with a runtime of
//! its own (`uc_storage::runtime::worker`): its connections, their requests
//! and its PostgreSQL connections stay on that thread, so no task moves
//! between threads. The connections are served as axum's `serve` does.

use std::io;
use std::net::SocketAddr;
use std::time::Duration;

use axum::{Router, body::Body, extract::ConnectInfo, http::Request};
use hyper::body::Incoming;
use hyper_util::rt::TokioIo;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{mpsc, oneshot, watch};
use tower::ServiceExt;
use tracing::{debug, error};

/// A connection that the listener accepted, and its client.
type Accepted = (std::net::TcpStream, SocketAddr);

/// Serve `app` on `listener` with `workers` workers until `shutdown`
/// completes; then finish the requests in progress. The current thread is
/// worker 0; the others get threads of their own.
pub async fn serve(
    listener: TcpListener,
    app: Router,
    workers: usize,
    shutdown: impl Future<Output = ()>,
) -> io::Result<()> {
    let workers = workers.max(1);
    let mut senders = Vec::with_capacity(workers);
    let (sender, receiver) = mpsc::unbounded_channel();
    senders.push(sender);
    let first = tokio::spawn(serve_connections(receiver, app.clone()));
    let mut threads = Vec::with_capacity(workers - 1);
    for index in 1..workers {
        let (sender, receiver) = mpsc::unbounded_channel();
        senders.push(sender);
        let app = app.clone();
        let (finished, finished_rx) = oneshot::channel::<()>();
        let thread = std::thread::Builder::new()
            .name(format!("uc-worker-{index}"))
            .spawn(move || {
                uc_storage::runtime::enter_worker(index);
                let runtime = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .expect("cannot start a worker runtime");
                runtime.block_on(serve_connections(receiver, app));
                drop(runtime);
                let _ = finished.send(());
            })?;
        threads.push((thread, finished_rx));
    }

    let mut shutdown = std::pin::pin!(shutdown);
    let mut next = 0;
    loop {
        let (stream, peer) = tokio::select! {
            accepted = accept(&listener) => match accepted {
                Some(accepted) => accepted,
                None => continue,
            },
            () = &mut shutdown => break,
        };
        let stream = match stream.into_std() {
            Ok(stream) => stream,
            Err(e) => {
                debug!(%peer, "cannot hand over the connection: {e}");
                continue;
            }
        };
        if senders[next].send((stream, peer)).is_err() {
            error!(
                worker = next,
                "the worker stopped; its connection is closed"
            );
        }
        next = (next + 1) % senders.len();
    }

    // Each worker finishes its connections once its channel closes.
    drop(listener);
    drop(senders);
    let _ = first.await;
    for (thread, finished) in threads {
        let _ = finished.await;
        if thread.join().is_err() {
            error!("a worker thread panicked");
        }
    }
    Ok(())
}

/// Serve the connections that come from `incoming` until it closes; then
/// finish them.
async fn serve_connections(mut incoming: mpsc::UnboundedReceiver<Accepted>, app: Router) {
    // Dropping `signal` tells the connections to finish; `closed` counts them.
    let (signal, _) = watch::channel(());
    let (close, closed) = watch::channel(());
    while let Some((stream, peer)) = incoming.recv().await {
        let stream = match TcpStream::from_std(stream) {
            Ok(stream) => stream,
            Err(e) => {
                debug!(%peer, "cannot take over the connection: {e}");
                continue;
            }
        };
        let app = app.clone();
        let mut stopping = signal.subscribe();
        let closed = closed.clone();
        tokio::spawn(async move {
            let service = hyper::service::service_fn(move |request: Request<Incoming>| {
                let mut request = request.map(Body::new);
                request.extensions_mut().insert(ConnectInfo(peer));
                app.clone().oneshot(request)
            });
            let connection = hyper::server::conn::http1::Builder::new()
                .serve_connection(TokioIo::new(stream), service)
                .with_upgrades();
            let mut connection = std::pin::pin!(connection);
            let mut finishing = false;
            loop {
                tokio::select! {
                    result = connection.as_mut() => {
                        if let Err(e) = result {
                            debug!(%peer, "connection error: {e}");
                        }
                        break;
                    }
                    _ = stopping.changed(), if !finishing => {
                        finishing = true;
                        connection.as_mut().graceful_shutdown();
                    }
                }
            }
            drop(closed);
        });
    }
    drop(signal);
    drop(closed);
    close.closed().await;
}

/// The next connection, as axum's `serve` accepts: none after the error of
/// one connection, or after any other error (such as too many open files)
/// and a second's wait.
async fn accept(listener: &TcpListener) -> Option<(TcpStream, SocketAddr)> {
    match listener.accept().await {
        Ok(accepted) => Some(accepted),
        Err(e)
            if matches!(
                e.kind(),
                io::ErrorKind::ConnectionRefused
                    | io::ErrorKind::ConnectionAborted
                    | io::ErrorKind::ConnectionReset
            ) =>
        {
            None
        }
        Err(e) => {
            error!("accept error: {e}");
            tokio::time::sleep(Duration::from_secs(1)).await;
            None
        }
    }
}
