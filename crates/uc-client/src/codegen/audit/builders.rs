// @generated — do not edit by hand.
#![allow(unused_mut)]
#![allow(unused_imports)]

#[cfg(not(target_arch = "wasm32"))]
type BoxFut<'a, T> = ::futures::future::BoxFuture<'a, T>;
#[cfg(target_arch = "wasm32")]
type BoxFut<'a, T> = ::futures::future::LocalBoxFuture<'a, T>;
use super::client::*;
use crate::codegen::error::Result;
use std::future::IntoFuture;
use uc_client::models::audit::v1::*;

/// Builder for audit events
pub struct ListAuditEventsBuilder {
    client: AuditClient,
    request: ListAuditEventsRequest,
}

impl ListAuditEventsBuilder {
    /// Create a new builder instance.
    /// Obtain via the corresponding method on `AuditClient`.
    pub(crate) fn new(client: AuditClient) -> Self {
        let request = ListAuditEventsRequest {
            ..Default::default()
        };
        Self { client, request }
    }

    /// Return only the events of this principal.
    pub fn with_principal(mut self, principal: impl Into<Option<String>>) -> Self {
        self.request.principal = principal.into();
        self
    }

    /// Return only the events of this HTTP method, such as `DELETE`.
    pub fn with_method(mut self, method: impl Into<Option<String>>) -> Self {
        self.request.method = method.into();
        self
    }

    /// Return only the events whose path starts with this prefix.
    pub fn with_path_prefix(mut self, path_prefix: impl Into<Option<String>>) -> Self {
        self.request.path_prefix = path_prefix.into();
        self
    }

    /// Return only the events at or after this time, in epoch milliseconds.
    pub fn with_since(mut self, since: impl Into<Option<i64>>) -> Self {
        self.request.since = since.into();
        self
    }

    /// Return only the events at or before this time, in epoch milliseconds.
    pub fn with_until(mut self, until: impl Into<Option<i64>>) -> Self {
        self.request.until = until.into();
        self
    }

    /// The maximum number of events to return.
    pub fn with_max_results(mut self, max_results: impl Into<Option<i32>>) -> Self {
        self.request.max_results = max_results.into();
        self
    }

    /// Opaque token of the page to return: the `next_page_token` of the page before.
    pub fn with_page_token(mut self, page_token: impl Into<Option<String>>) -> Self {
        self.request.page_token = page_token.into();
        self
    }
}

impl IntoFuture for ListAuditEventsBuilder {
    type Output = Result<ListAuditEventsResponse>;
    type IntoFuture = BoxFut<'static, Self::Output>;
    fn into_future(self) -> Self::IntoFuture {
        let client = self.client;
        let request = self.request;
        Box::pin(async move { client.list_audit_events(&request).await })
    }
}
