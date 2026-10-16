//! The web UI's files, as uc-server and `uc web` serve them.
//!
//! The UI's build puts the wasm and its JS loader in `assets/`. Other files,
//! such as `index.html`, `styles.css` and `favicon.ico`, are at the root of
//! the build.
//!
//! - Browsers check each file with the server before they use a kept copy
//!   (`Cache-Control: no-cache`). The server answers 304 when the file has
//!   not changed, so a new build takes effect at once.
//! - The wasm is compressed once: `just wasm` writes `uc_ui_bg.wasm.br`
//!   (brotli at quality 11) and `uc_ui_bg.wasm.gz` next to it, and they go
//!   out as they are, to the browsers that accept them.
//!   The wasm itself is never compressed on a request: quality 11 takes
//!   seconds, and a dev build's wasm, with no such files, is too large.
//! - Other responses are compressed on each request, with brotli or gzip,
//!   as the browser accepts: they are small. Images other than SVG are not:
//!   they are compressed already.
//! - A path that is not a file gets `index.html`, for the UI's client-side
//!   routes. A missing file of `assets/` is not found.

use std::path::Path;

use axum::{
    Router,
    extract::Request,
    http::{HeaderValue, header},
    middleware::{self, Next},
    response::Response,
};
use tower_http::{
    compression::{
        CompressionLayer, CompressionLevel,
        predicate::{DefaultPredicate, NotForContentType, Predicate},
    },
    services::{ServeDir, ServeFile},
};

/// Brotli and gzip at quality 5, for the small files that are compressed on
/// each request: about as small as quality 7, at half its time. Brotli's
/// own default, 11, is for the wasm, once, in the build.
const QUALITY: CompressionLevel = CompressionLevel::Precise(5);

/// The web UI in the folder `dir`.
pub fn files(dir: &Path) -> Router {
    let index = ServeFile::new(dir.join("index.html"));
    Router::new()
        // A missing asset is not found: it does not get `index.html`.
        .nest_service(
            "/assets",
            ServeDir::new(dir.join("assets"))
                .precompressed_br()
                .precompressed_gzip(),
        )
        // Client-side routes load the UI, with 200.
        .fallback_service(ServeDir::new(dir).fallback(index))
        .layer(middleware::from_fn(cache_control))
        .layer(
            CompressionLayer::new().quality(QUALITY).compress_when(
                DefaultPredicate::new().and(NotForContentType::const_new("application/wasm")),
            ),
        )
}

/// Set `Cache-Control: no-cache`. A file of `assets/` also varies by
/// `Accept-Encoding`: a browser that accepts brotli gets the `.br` file.
async fn cache_control(request: Request, next: Next) -> Response {
    let asset = request.uri().path().starts_with("/assets/");
    let mut response = next.run(request).await;
    let headers = response.headers_mut();
    headers.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-cache"));
    if asset {
        headers.insert(header::VARY, HeaderValue::from_static("accept-encoding"));
    }
    response
}
