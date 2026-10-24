//! Errors, names that need escaping in a URL, and the URL of the server:
//! also a server that is not a UC server, or none at all.

use std::error::Error as _;

use axum::Router;
use axum::http::{StatusCode, Uri};

use uc_client::Client;
use uc_client::error::Error;
use uc_server::{AppState, build_app};

use crate::common::{api_error, client, metastore, rejected, serve};

/// The server's errors keep their upstream code, message and details.
#[tokio::test]
async fn errors_keep_the_code_and_the_message_of_the_server() {
    let uc = client().await;
    let error = uc.catalog("missing").get().await.unwrap_err();
    assert!(error.is_not_found());
    assert_eq!(error.status(), Some(404));
    assert_eq!(error.code(), Some("CATALOG_NOT_FOUND"));
    assert_eq!(
        error.to_string(),
        "CATALOG_NOT_FOUND: Catalog not found: missing"
    );
    let error = rejected(
        uc.catalog("missing").delete().await,
        404,
        "CATALOG_NOT_FOUND",
    );
    assert_eq!(error.message, "Catalog not found: missing");
    assert_eq!(error.details[0]["reason"], "CATALOG_NOT_FOUND");
    assert_eq!(
        error.details[0]["@type"],
        "type.googleapis.com/google.rpc.ErrorInfo"
    );
}

/// Names reach the server as they are: in a path segment and in a query
/// value, the characters that would end one or start an escape are
/// escaped.
#[tokio::test]
async fn names_reach_the_server_as_they_are() {
    let uc = client().await;
    for name in ["a/b", "a?b", "a#b", "a%20b", "a b", "a&b=c+d", "été"] {
        let message = format!("Catalog not found: {name}");
        let error = rejected(uc.catalog(name).get().await, 404, "CATALOG_NOT_FOUND");
        assert_eq!(error.message, message);
        let error = rejected(uc.list_schemas(name).await, 404, "CATALOG_NOT_FOUND");
        assert_eq!(error.message, message);
    }
}

/// A server below a path, as behind a proxy: the URL of the client is
/// that path, with a slash at the end or not.
#[tokio::test]
async fn the_server_may_be_below_a_path() {
    let app = Router::new().nest("/uc", build_app(AppState::new(metastore().await)));
    let root = serve(app).await;
    for path in ["uc", "uc/"] {
        let uc = Client::new_unauthenticated(root.join(path).unwrap());
        uc.create_catalog(format!("c-{}", path.len()))
            .await
            .unwrap();
        assert!(!uc.list_catalogs().await.unwrap().catalogs.is_empty());
    }
}

/// A client of a server that is not a UC server, such as a proxy: it
/// answers each request with the status in the last segment of the path,
/// and a body that is not JSON.
async fn stub() -> Client {
    let app = Router::new().fallback(|uri: Uri| async move {
        let status = uri.path().rsplit('/').next().unwrap_or_default();
        (
            StatusCode::from_bytes(status.as_bytes()).unwrap(),
            "<html>proxy</html>",
        )
    });
    Client::new_unauthenticated(serve(app).await)
}

/// An error without the JSON body of a UC server keeps its status, and its
/// body as the message.
#[tokio::test]
async fn errors_without_a_uc_body() {
    let uc = stub().await;
    for status in [400, 401, 403, 404, 409, 502] {
        let error = api_error(uc.catalog(status.to_string()).get().await);
        assert_eq!(error.status, status);
        assert_eq!(error.code, None);
        assert_eq!(error.message, "<html>proxy</html>");
        assert_eq!(
            api_error(uc.table(status.to_string()).delete().await),
            error
        );
    }
}

/// A success whose body is not the answer of the call is an error; calls
/// without an answer take any body.
#[tokio::test]
async fn a_success_with_a_body_of_another_shape() {
    let uc = stub().await;
    let error = uc.catalog("200").get().await.unwrap_err();
    assert!(matches!(error, Error::Json(_)), "{error:?}");
    assert_eq!(error.status(), None);
    uc.table("200").delete().await.unwrap();
}

#[tokio::test]
async fn a_server_that_does_not_answer() {
    let port = std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    let url = format!("http://127.0.0.1:{port}/").parse().unwrap();
    let error = Client::new_unauthenticated(url)
        .list_catalogs()
        .await
        .unwrap_err();
    assert!(matches!(error, Error::Http(_)), "{error:?}");
    assert_eq!((error.status(), error.code()), (None, None));
    // The cause is in the chain of sources.
    let mut causes = Vec::new();
    let mut source = error.source();
    while let Some(cause) = source {
        causes.push(cause.to_string());
        source = cause.source();
    }
    #[cfg(unix)]
    assert!(
        causes.iter().any(|c| c.contains("Connection refused")),
        "{causes:?}"
    );
}
