use axum::Router;
use axum::extract::{Request, State};
use axum::middleware::{Next, from_fn_with_state};
use axum::response::Response;

use crate::{AuthError, Authenticator};

/// Runs `authenticator` in front of `router`: a request it finds the caller
/// of reaches the routes with the [`Identity`](crate::Identity) in its
/// extensions; any other gets [`Authenticator::reject`].
///
/// The layer covers the routes added to `router` before the call. Merge
/// public routes, such as a health check, into the app separately. A host
/// with routes open to anonymous callers calls
/// [`Authenticator::authenticate`] from its own middleware instead.
pub fn auth_layer<S>(router: Router<S>, authenticator: Authenticator<S>, state: S) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    router.layer(from_fn_with_state((authenticator, state), authenticate))
}

async fn authenticate<S>(
    State((authenticator, state)): State<(Authenticator<S>, S)>,
    request: Request,
    next: Next,
) -> Response
where
    S: Clone + Send + Sync + 'static,
{
    let (mut parts, body) = request.into_parts();
    match authenticator.authenticate(&parts, &state).await {
        Ok(identity) => {
            parts.extensions.insert(identity);
            next.run(Request::from_parts(parts, body)).await
        }
        Err(error) => {
            match &error {
                AuthError::Unavailable(reason) => {
                    tracing::warn!(%reason, "authentication backend unavailable");
                }
                _ => tracing::debug!(%error, "request not authenticated"),
            }
            authenticator.reject(&error)
        }
    }
}

#[cfg(test)]
mod tests {
    use std::net::SocketAddr;

    use axum::body::Body;
    use axum::extract::ConnectInfo;
    use axum::http::{Request, StatusCode};
    use axum::routing::get;
    use tower::ServiceExt;

    use crate::{Identity, TrustedHeaders};

    use super::*;

    #[tokio::test]
    async fn handlers_get_the_identity() {
        let proxy = TrustedHeaders::new("proxy", ["10.0.0.0/8".parse().unwrap()], ["Remote-User"]);
        let auth = Authenticator::builder().resolver(proxy).build().unwrap();
        let app = auth_layer(
            Router::new().route("/me", get(|me: Identity| async move { me.subject })),
            auth,
            (),
        );
        let request = |peer: &str| {
            let mut request = Request::builder()
                .uri("/me")
                .header("Remote-User", "alice")
                .body(Body::empty())
                .unwrap();
            let peer: SocketAddr = peer.parse().unwrap();
            request.extensions_mut().insert(ConnectInfo(peer));
            request
        };
        let response = app.clone().oneshot(request("10.0.0.1:1")).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), 100)
            .await
            .unwrap();
        assert_eq!(&body[..], b"alice");
        let response = app.oneshot(request("192.0.2.1:1")).await.unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }
}
