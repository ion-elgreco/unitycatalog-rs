// @generated — do not edit by hand.
#![allow(unused_mut)]
#![allow(unused_imports)]

#[cfg(not(target_arch = "wasm32"))]
type BoxFut<'a, T> = ::futures::future::BoxFuture<'a, T>;
#[cfg(target_arch = "wasm32")]
type BoxFut<'a, T> = ::futures::future::LocalBoxFuture<'a, T>;
#[cfg(not(target_arch = "wasm32"))]
type BoxStr<'a, T> = ::futures::stream::BoxStream<'a, T>;
#[cfg(target_arch = "wasm32")]
type BoxStr<'a, T> = ::futures::stream::LocalBoxStream<'a, T>;
use super::super::stream_paginated;
use super::client::*;
use crate::codegen::error::Result;
use futures::{StreamExt, TryStreamExt};
use std::future::IntoFuture;
use uc_client::models::tokens::v1::*;

/// Builder for listing tokens
pub struct ListTokensBuilder {
    client: TokenServiceClient,
    request: ListTokensRequest,
}

impl ListTokensBuilder {
    /// Create a new builder instance.
    /// Obtain via the corresponding method on `TokenServiceClient`.
    pub(crate) fn new(client: TokenServiceClient) -> Self {
        let request = ListTokensRequest {
            ..Default::default()
        };
        Self { client, request }
    }

    /// The maximum number of tokens to return.
    pub fn with_max_results(mut self, max_results: impl Into<Option<i32>>) -> Self {
        self.request.max_results = max_results.into();
        self
    }

    /// Opaque token of the page to return: the `next_page_token` of the page before.
    pub fn with_page_token(mut self, page_token: impl Into<Option<String>>) -> Self {
        self.request.page_token = page_token.into();
        self
    }

    /// Convert paginated request into stream of results
    pub fn into_stream(self) -> BoxStr<'static, Result<PatInfo>> {
        let remaining = self.request.max_results;
        let stream = stream_paginated(
            (self, remaining),
            move |(mut builder, mut remaining), page_token| async move {
                builder.request.page_token = page_token;
                if remaining.is_some() {
                    builder.request.max_results = remaining;
                }
                let mut res = builder.client.list_tokens(&builder.request).await?;
                if let Some(ref mut rem) = remaining {
                    res.tokens.truncate((*rem).max(0) as usize);
                    *rem -= res.tokens.len() as i32;
                }
                let next_page_token = if remaining.is_some_and(|r| r <= 0) {
                    None
                } else {
                    res.next_page_token.clone()
                };
                Ok((res, (builder, remaining), next_page_token))
            },
        )
        .map_ok(|resp| futures::stream::iter(resp.tokens.into_iter().map(Ok)))
        .try_flatten();
        #[cfg(not(target_arch = "wasm32"))]
        let stream = stream.boxed();
        #[cfg(target_arch = "wasm32")]
        let stream = stream.boxed_local();
        stream
    }
}

impl IntoFuture for ListTokensBuilder {
    type Output = Result<ListPatsResponse>;
    type IntoFuture = BoxFut<'static, Self::Output>;
    fn into_future(self) -> Self::IntoFuture {
        let client = self.client;
        let request = self.request;
        Box::pin(async move { client.list_tokens(&request).await })
    }
}

/// Builder for creating a token
pub struct CreateTokenBuilder {
    client: TokenServiceClient,
    request: CreatePat,
}

impl CreateTokenBuilder {
    /// Create a new builder instance.
    /// Obtain via the corresponding method on `TokenServiceClient`.
    pub(crate) fn new(client: TokenServiceClient, name: impl Into<String>) -> Self {
        let request = CreatePat {
            name: name.into(),
            ..Default::default()
        };
        Self { client, request }
    }

    /// Time at which the token expires, in epoch milliseconds. Without it the token does not expire.
    pub fn with_expires_at(mut self, expires_at: impl Into<Option<i64>>) -> Self {
        self.request.expires_at = expires_at.into();
        self
    }
}

impl IntoFuture for CreateTokenBuilder {
    type Output = Result<PatInfo>;
    type IntoFuture = BoxFut<'static, Self::Output>;
    fn into_future(self) -> Self::IntoFuture {
        let client = self.client;
        let request = self.request;
        Box::pin(async move { client.create_token(&request).await })
    }
}

/// Builder for deleting a token
pub struct DeleteTokenBuilder {
    client: TokenServiceClient,
    request: DeleteTokenRequest,
}

impl DeleteTokenBuilder {
    /// Create a new builder instance.
    /// Obtain via the corresponding method on `TokenServiceClient`.
    pub(crate) fn new(client: TokenServiceClient, id: impl Into<String>) -> Self {
        let request = DeleteTokenRequest {
            id: id.into(),
            ..Default::default()
        };
        Self { client, request }
    }
}

impl IntoFuture for DeleteTokenBuilder {
    type Output = Result<()>;
    type IntoFuture = BoxFut<'static, Self::Output>;
    fn into_future(self) -> Self::IntoFuture {
        let client = self.client;
        let request = self.request;
        Box::pin(async move { client.delete_token(&request).await })
    }
}
