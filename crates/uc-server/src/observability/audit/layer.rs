//! Audit-log tower middleware.
//!
//! Records one [`uc_api::AuditEvent`] per mutating request (POST, PATCH,
//! PUT, DELETE) after the handler runs, with the status of its response.
//! Reads are not recorded; they would flood the audit table.

use axum::{
    body::Body,
    extract::State,
    http::{Method, Request},
    middleware::Next,
    response::Response,
};
use uc_api::AuditEvent;
use uc_common::now_epoch_millis;

use crate::{authn::principal::Principal, server::state::AppState};

pub async fn audit_layer(
    State(state): State<AppState>,
    req: Request<Body>,
    next: Next,
) -> Response {
    if !is_mutating(req.method()) {
        return next.run(req).await;
    }

    let method = req.method().clone();
    let path = req
        .uri()
        .path_and_query()
        .map(|pq| pq.as_str().to_string())
        .unwrap_or_else(|| req.uri().path().to_string());
    let principal = req
        .extensions()
        .get::<Principal>()
        .map(|p| p.username.clone())
        .unwrap_or_else(|| "anonymous".to_string());

    let response = next.run(req).await;

    let event = AuditEvent {
        id: None,
        principal,
        method: method.as_str().to_string(),
        path,
        status: response.status().as_u16(),
        timestamp: now_epoch_millis(),
    };
    // Does not wait: the drainer task of `AppState::new` writes the event,
    // and a full channel drops it with a throttled warning.
    state.audit.try_record(event);

    response
}

fn is_mutating(m: &Method) -> bool {
    matches!(
        m,
        &Method::POST | &Method::PATCH | &Method::PUT | &Method::DELETE
    )
}
