//! With the UI mounted, browsers get it at the root; other clients get
//! upstream's greeting. The UI's files go compressed, with cache headers;
//! the API's responses do not.

mod common;

use std::io::{Read, Write};
use std::path::Path;

use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode, header},
    response::Response,
};
use tower::ServiceExt;

use uc_metastore::{MetastoreConfig, SqlMetastore};
use uc_server::{AppState, build_app, server::app::HttpSettings};

const INDEX: &str = "<!doctype html><html><head><script type=\"module\" \
    src=\"/assets/uc_ui.js\"></script></head><body></body></html>";

/// An app with the UI of the folder `ui`.
async fn app(ui: &Path) -> Router {
    let metastore = SqlMetastore::connect(MetastoreConfig::for_tests())
        .await
        .unwrap();
    let http = HttpSettings {
        ui_dir: Some(ui.to_path_buf()),
        ..Default::default()
    };
    build_app(AppState::new(metastore).with_http(http))
}

async fn get(app: &Router, path: &str, headers: &[(&str, &str)]) -> Response {
    let mut request = Request::get(path);
    for (name, value) in headers {
        request = request.header(*name, *value);
    }
    let request = request.body(Body::empty()).unwrap();
    app.clone().oneshot(request).await.unwrap()
}

async fn bytes(response: Response) -> Vec<u8> {
    axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap()
        .to_vec()
}

/// A header of `response`; empty when there is none.
fn header_value(response: &Response, name: header::HeaderName) -> &str {
    response
        .headers()
        .get(name)
        .map_or("", |v| v.to_str().unwrap())
}

#[tokio::test]
async fn browsers_get_the_ui_at_the_root() {
    let ui = tempfile::tempdir().unwrap();
    std::fs::write(ui.path().join("index.html"), "<html>ui</html>").unwrap();
    std::fs::write(ui.path().join("uc_ui.js"), "js").unwrap();
    let app = app(ui.path()).await;
    let text = |path: &'static str, accept: &'static str| {
        let app = app.clone();
        async move {
            let response = get(&app, path, &[("accept", accept)]).await;
            let status = response.status();
            (status, String::from_utf8(bytes(response).await).unwrap())
        }
    };
    assert_eq!(
        text("/", "text/html,application/xhtml+xml").await,
        (StatusCode::OK, "<html>ui</html>".into())
    );
    assert_eq!(
        text("/", "*/*").await,
        (StatusCode::OK, "Hello, Unity Catalog!".into())
    );
    assert_eq!(
        text("/uc_ui.js", "*/*").await,
        (StatusCode::OK, "js".into())
    );
    // Client-side routes load the UI.
    assert_eq!(
        text("/data/unity", "text/html").await,
        (StatusCode::OK, "<html>ui</html>".into())
    );
}

#[tokio::test]
async fn the_ui_goes_compressed_and_browsers_check_it() {
    let ui = tempfile::tempdir().unwrap();
    let assets = ui.path().join("assets");
    std::fs::create_dir_all(&assets).unwrap();
    std::fs::write(ui.path().join("index.html"), INDEX).unwrap();
    std::fs::write(ui.path().join("favicon.ico"), [0u8; 64]).unwrap();
    let wasm: Vec<u8> = (0..64 * 1024).map(|i| (i % 251) as u8).collect();
    std::fs::write(assets.join("uc_ui_bg.wasm"), &wasm).unwrap();
    let js = "export function start() {}\n".repeat(1000);
    std::fs::write(assets.join("uc_ui.js"), &js).unwrap();
    let app = app(ui.path()).await;
    let wasm_path = "/assets/uc_ui_bg.wasm";

    // The loader: brotli or gzip as the browser accepts, on each request.
    // Browsers check it with the server before they use a kept copy.
    let br = get(&app, "/assets/uc_ui.js", &[("accept-encoding", "br")]).await;
    assert_eq!(br.status(), StatusCode::OK);
    assert_eq!(header_value(&br, header::CONTENT_ENCODING), "br");
    assert_eq!(header_value(&br, header::CACHE_CONTROL), "no-cache");
    assert_eq!(header_value(&br, header::VARY), "accept-encoding");
    assert!(bytes(br).await.len() < js.len() / 10);
    let gzip = get(&app, "/assets/uc_ui.js", &[("accept-encoding", "gzip")]).await;
    assert_eq!(header_value(&gzip, header::CONTENT_ENCODING), "gzip");
    assert_eq!(gunzip(&bytes(gzip).await), js.as_bytes());

    // The wasm is never compressed on a request: without the build's
    // `.br` and `.gz` files, as a dev build, it goes as it is.
    let raw = get(&app, wasm_path, &[("accept-encoding", "br, gzip")]).await;
    assert_eq!(raw.status(), StatusCode::OK);
    assert_eq!(header_value(&raw, header::CONTENT_ENCODING), "");
    assert_eq!(header_value(&raw, header::CONTENT_TYPE), "application/wasm");
    assert_eq!(header_value(&raw, header::CACHE_CONTROL), "no-cache");
    assert_eq!(header_value(&raw, header::VARY), "accept-encoding");
    assert_eq!(bytes(raw).await, wasm);

    // A release build's `.br` and `.gz` go as they are, to the browsers
    // that accept them.
    let stored_br = b"brotli of the wasm".to_vec();
    std::fs::write(assets.join("uc_ui_bg.wasm.br"), &stored_br).unwrap();
    let mut stored_gz = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::best());
    stored_gz.write_all(&wasm).unwrap();
    std::fs::write(assets.join("uc_ui_bg.wasm.gz"), stored_gz.finish().unwrap()).unwrap();
    let br = get(&app, wasm_path, &[("accept-encoding", "gzip, br")]).await;
    assert_eq!(header_value(&br, header::CONTENT_ENCODING), "br");
    assert_eq!(header_value(&br, header::CONTENT_TYPE), "application/wasm");
    assert_eq!(header_value(&br, header::CACHE_CONTROL), "no-cache");
    assert_eq!(header_value(&br, header::VARY), "accept-encoding");
    assert_eq!(bytes(br).await, stored_br);
    let gzip = get(&app, wasm_path, &[("accept-encoding", "gzip")]).await;
    assert_eq!(header_value(&gzip, header::CONTENT_ENCODING), "gzip");
    assert_eq!(gunzip(&bytes(gzip).await), wasm);
    let plain = get(&app, wasm_path, &[]).await;
    assert_eq!(header_value(&plain, header::CONTENT_ENCODING), "");
    assert_eq!(header_value(&plain, header::CACHE_CONTROL), "no-cache");
    let modified = header_value(&plain, header::LAST_MODIFIED).to_string();
    assert_eq!(bytes(plain).await, wasm);
    // A browser checks its kept copy: the file has not changed.
    let again = get(&app, wasm_path, &[("if-modified-since", modified.as_str())]).await;
    assert_eq!(again.status(), StatusCode::NOT_MODIFIED);
    assert_eq!(header_value(&again, header::CACHE_CONTROL), "no-cache");

    // A missing asset is missing, not the UI.
    let missing = get(&app, "/assets/old.js", &[]).await;
    assert_eq!(missing.status(), StatusCode::NOT_FOUND);
    assert_eq!(header_value(&missing, header::CACHE_CONTROL), "no-cache");

    // `index.html`, at the root and for client-side routes, and the other
    // files: checked with the server each time.
    let html = [("accept", "text/html"), ("accept-encoding", "gzip")];
    for path in ["/", "/data/unity"] {
        let page = get(&app, path, &html).await;
        assert_eq!(page.status(), StatusCode::OK, "{path}");
        assert_eq!(
            header_value(&page, header::CACHE_CONTROL),
            "no-cache",
            "{path}"
        );
        assert_eq!(
            header_value(&page, header::CONTENT_ENCODING),
            "gzip",
            "{path}"
        );
        assert_eq!(gunzip(&bytes(page).await), INDEX.as_bytes(), "{path}");
    }
    let icon = get(&app, "/favicon.ico", &[("accept-encoding", "br")]).await;
    assert_eq!(icon.status(), StatusCode::OK);
    assert_eq!(header_value(&icon, header::CACHE_CONTROL), "no-cache");

    // The API's responses are not compressed.
    let catalogs = get(
        &app,
        &format!("{}/catalogs", common::BASE),
        &[("accept-encoding", "br, gzip")],
    )
    .await;
    assert_eq!(catalogs.status(), StatusCode::OK);
    assert_eq!(header_value(&catalogs, header::CONTENT_ENCODING), "");
    assert_eq!(header_value(&catalogs, header::CACHE_CONTROL), "");
}

fn gunzip(data: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    flate2::read::GzDecoder::new(data)
        .read_to_end(&mut out)
        .unwrap();
    out
}
