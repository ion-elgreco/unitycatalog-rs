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
use uc_client::models::search::v1::*;

/// Builder for objects
pub struct SearchObjectsBuilder {
    client: SearchClient,
    request: SearchObjectsRequest,
}

impl SearchObjectsBuilder {
    /// Create a new builder instance.
    /// Obtain via the corresponding method on `SearchClient`.
    pub(crate) fn new(client: SearchClient, query: impl Into<String>) -> Self {
        let request = SearchObjectsRequest {
            query: query.into(),
            ..Default::default()
        };
        Self { client, request }
    }

    /// Return only the objects of this type: `catalog`, `schema`, `table`, `volume`, `function` or
    /// `registered_model`.
    pub fn with_securable_type(mut self, securable_type: impl Into<Option<String>>) -> Self {
        self.request.securable_type = securable_type.into();
        self
    }

    /// The maximum number of results to return.
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

impl IntoFuture for SearchObjectsBuilder {
    type Output = Result<SearchObjectsResponse>;
    type IntoFuture = BoxFut<'static, Self::Output>;
    fn into_future(self) -> Self::IntoFuture {
        let client = self.client;
        let request = self.request;
        Box::pin(async move { client.search_objects(&request).await })
    }
}
