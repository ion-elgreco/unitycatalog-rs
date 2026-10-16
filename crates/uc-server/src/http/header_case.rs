//! Response header names in the spelling clients look up.
//!
//! hyper writes HTTP/1 header names in lowercase, which HTTP allows, but
//! released Iceberg clients keep response headers in a case-sensitive map
//! and look up `ETag`. hyper keeps a header's original spelling only through
//! a private map it attaches to the messages it parses. So hyper parses one
//! request that spells `ETag`, and the responses that carry the header get
//! the extensions it attached.

use std::convert::Infallible;
use std::sync::Mutex;

use axum::http::{Extensions, Request, Response};
use hyper::body::Incoming;
use hyper_util::rt::TokioIo;
use tokio::{
    io::AsyncWriteExt,
    sync::{OnceCell, oneshot},
};

static SPELLINGS: OnceCell<Option<Extensions>> = OnceCell::const_new();

/// The extensions hyper attaches to a request that spells `ETag`, if any.
async fn spellings() -> Option<Extensions> {
    let (mut client, server) = tokio::io::duplex(1024);
    let (sender, receiver) = oneshot::channel();
    let sender = Mutex::new(Some(sender));
    let service = hyper::service::service_fn(move |request: Request<Incoming>| {
        if let Some(sender) = sender.lock().ok().and_then(|mut s| s.take()) {
            let _ = sender.send(request.extensions().clone());
        }
        async { Ok::<_, Infallible>(Response::new(String::new())) }
    });
    let connection = hyper::server::conn::http1::Builder::new()
        .preserve_header_case(true)
        .serve_connection(TokioIo::new(server), service);
    tokio::spawn(connection);
    client
        .write_all(b"GET / HTTP/1.1\r\nETag: x\r\n\r\n")
        .await
        .ok()?;
    let extensions = receiver.await.ok()?;
    (!extensions.is_empty()).then_some(extensions)
}

/// Have HTTP/1 write the `ETag` header of `response` as `ETag`.
pub async fn spell_etag<B>(response: &mut Response<B>) {
    if let Some(extensions) = SPELLINGS.get_or_init(spellings).await {
        response.extensions_mut().extend(extensions.clone());
    }
}
