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
use uc_client::models::functions::v1::*;

/// Builder for listing functions
pub struct ListFunctionsBuilder {
    client: FunctionServiceClient,
    request: ListFunctionsRequest,
}

impl ListFunctionsBuilder {
    /// Create a new builder instance.
    /// Obtain via the corresponding method on `FunctionServiceClient`.
    pub(crate) fn new(
        client: FunctionServiceClient,
        catalog_name: impl Into<String>,
        schema_name: impl Into<String>,
    ) -> Self {
        let request = ListFunctionsRequest {
            catalog_name: catalog_name.into(),
            schema_name: schema_name.into(),
            ..Default::default()
        };
        Self { client, request }
    }

    /// The maximum number of functions to return.
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
    pub fn into_stream(self) -> BoxStr<'static, Result<FunctionInfo>> {
        let remaining = self.request.max_results;
        let stream = stream_paginated(
            (self, remaining),
            move |(mut builder, mut remaining), page_token| async move {
                builder.request.page_token = page_token;
                if remaining.is_some() {
                    builder.request.max_results = remaining;
                }
                let mut res = builder.client.list_functions(&builder.request).await?;
                if let Some(ref mut rem) = remaining {
                    res.functions.truncate((*rem).max(0) as usize);
                    *rem -= res.functions.len() as i32;
                }
                let next_page_token = if remaining.is_some_and(|r| r <= 0) {
                    None
                } else {
                    res.next_page_token.clone()
                };
                Ok((res, (builder, remaining), next_page_token))
            },
        )
        .map_ok(|resp| futures::stream::iter(resp.functions.into_iter().map(Ok)))
        .try_flatten();
        #[cfg(not(target_arch = "wasm32"))]
        let stream = stream.boxed();
        #[cfg(target_arch = "wasm32")]
        let stream = stream.boxed_local();
        stream
    }
}

impl IntoFuture for ListFunctionsBuilder {
    type Output = Result<ListFunctionsResponse>;
    type IntoFuture = BoxFut<'static, Self::Output>;
    fn into_future(self) -> Self::IntoFuture {
        let client = self.client;
        let request = self.request;
        Box::pin(async move { client.list_functions(&request).await })
    }
}

/// Builder for creating a function
pub struct CreateFunctionBuilder {
    client: FunctionServiceClient,
    request: CreateFunctionRequest,
}

impl CreateFunctionBuilder {
    /// Create a new builder instance.
    /// Obtain via the corresponding method on `FunctionServiceClient`.
    pub(crate) fn new(client: FunctionServiceClient, function_info: CreateFunction) -> Self {
        let request = CreateFunctionRequest {
            function_info: buffa::MessageField::some(function_info),
            ..Default::default()
        };
        Self { client, request }
    }
}

impl IntoFuture for CreateFunctionBuilder {
    type Output = Result<FunctionInfo>;
    type IntoFuture = BoxFut<'static, Self::Output>;
    fn into_future(self) -> Self::IntoFuture {
        let client = self.client;
        let request = self.request;
        Box::pin(async move { client.create_function(&request).await })
    }
}

/// Builder for getting a function
pub struct GetFunctionBuilder {
    client: FunctionServiceClient,
    request: GetFunctionRequest,
}

impl GetFunctionBuilder {
    /// Create a new builder instance.
    /// Obtain via the corresponding method on `FunctionServiceClient`.
    pub(crate) fn new(client: FunctionServiceClient, full_name: impl Into<String>) -> Self {
        let request = GetFunctionRequest {
            full_name: full_name.into(),
            ..Default::default()
        };
        Self { client, request }
    }
}

impl IntoFuture for GetFunctionBuilder {
    type Output = Result<FunctionInfo>;
    type IntoFuture = BoxFut<'static, Self::Output>;
    fn into_future(self) -> Self::IntoFuture {
        let client = self.client;
        let request = self.request;
        Box::pin(async move { client.get_function(&request).await })
    }
}

/// Builder for deleting a function
pub struct DeleteFunctionBuilder {
    client: FunctionServiceClient,
    request: DeleteFunctionRequest,
}

impl DeleteFunctionBuilder {
    /// Create a new builder instance.
    /// Obtain via the corresponding method on `FunctionServiceClient`.
    pub(crate) fn new(client: FunctionServiceClient, full_name: impl Into<String>) -> Self {
        let request = DeleteFunctionRequest {
            full_name: full_name.into(),
            ..Default::default()
        };
        Self { client, request }
    }
}

impl IntoFuture for DeleteFunctionBuilder {
    type Output = Result<()>;
    type IntoFuture = BoxFut<'static, Self::Output>;
    fn into_future(self) -> Self::IntoFuture {
        let client = self.client;
        let request = self.request;
        Box::pin(async move { client.delete_function(&request).await })
    }
}
