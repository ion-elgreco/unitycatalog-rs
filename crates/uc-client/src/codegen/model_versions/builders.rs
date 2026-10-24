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
use uc_client::models::models::v1::*;

/// Builder for listing model versions
pub struct ListModelVersionsBuilder {
    client: ModelVersionServiceClient,
    request: ListModelVersionsRequest,
}

impl ListModelVersionsBuilder {
    /// Create a new builder instance.
    /// Obtain via the corresponding method on `ModelVersionServiceClient`.
    pub(crate) fn new(client: ModelVersionServiceClient, full_name: impl Into<String>) -> Self {
        let request = ListModelVersionsRequest {
            full_name: full_name.into(),
            ..Default::default()
        };
        Self { client, request }
    }

    /// The maximum number of model versions to return.
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
    pub fn into_stream(self) -> BoxStr<'static, Result<ModelVersionInfo>> {
        let remaining = self.request.max_results;
        let stream = stream_paginated(
            (self, remaining),
            move |(mut builder, mut remaining), page_token| async move {
                builder.request.page_token = page_token;
                if remaining.is_some() {
                    builder.request.max_results = remaining;
                }
                let mut res = builder.client.list_model_versions(&builder.request).await?;
                if let Some(ref mut rem) = remaining {
                    res.model_versions.truncate((*rem).max(0) as usize);
                    *rem -= res.model_versions.len() as i32;
                }
                let next_page_token = if remaining.is_some_and(|r| r <= 0) {
                    None
                } else {
                    res.next_page_token.clone()
                };
                Ok((res, (builder, remaining), next_page_token))
            },
        )
        .map_ok(|resp| futures::stream::iter(resp.model_versions.into_iter().map(Ok)))
        .try_flatten();
        #[cfg(not(target_arch = "wasm32"))]
        let stream = stream.boxed();
        #[cfg(target_arch = "wasm32")]
        let stream = stream.boxed_local();
        stream
    }
}

impl IntoFuture for ListModelVersionsBuilder {
    type Output = Result<ListModelVersionsResponse>;
    type IntoFuture = BoxFut<'static, Self::Output>;
    fn into_future(self) -> Self::IntoFuture {
        let client = self.client;
        let request = self.request;
        Box::pin(async move { client.list_model_versions(&request).await })
    }
}

/// Builder for creating a model version
pub struct CreateModelVersionBuilder {
    client: ModelVersionServiceClient,
    request: CreateModelVersion,
}

impl CreateModelVersionBuilder {
    /// Create a new builder instance.
    /// Obtain via the corresponding method on `ModelVersionServiceClient`.
    pub(crate) fn new(
        client: ModelVersionServiceClient,
        model_name: impl Into<String>,
        catalog_name: impl Into<String>,
        schema_name: impl Into<String>,
        source: impl Into<String>,
    ) -> Self {
        let request = CreateModelVersion {
            model_name: model_name.into(),
            catalog_name: catalog_name.into(),
            schema_name: schema_name.into(),
            source: source.into(),
            ..Default::default()
        };
        Self { client, request }
    }

    /// ID of the run of the ML package that made the model.
    pub fn with_run_id(mut self, run_id: impl Into<Option<String>>) -> Self {
        self.request.run_id = run_id.into();
        self
    }

    /// User-provided free-form text description.
    pub fn with_comment(mut self, comment: impl Into<Option<String>>) -> Self {
        self.request.comment = comment.into();
        self
    }
}

impl IntoFuture for CreateModelVersionBuilder {
    type Output = Result<ModelVersionInfo>;
    type IntoFuture = BoxFut<'static, Self::Output>;
    fn into_future(self) -> Self::IntoFuture {
        let client = self.client;
        let request = self.request;
        Box::pin(async move { client.create_model_version(&request).await })
    }
}

/// Builder for getting a model version
pub struct GetModelVersionBuilder {
    client: ModelVersionServiceClient,
    request: GetModelVersionRequest,
}

impl GetModelVersionBuilder {
    /// Create a new builder instance.
    /// Obtain via the corresponding method on `ModelVersionServiceClient`.
    pub(crate) fn new(
        client: ModelVersionServiceClient,
        full_name: impl Into<String>,
        version: i64,
    ) -> Self {
        let request = GetModelVersionRequest {
            full_name: full_name.into(),
            version,
            ..Default::default()
        };
        Self { client, request }
    }
}

impl IntoFuture for GetModelVersionBuilder {
    type Output = Result<ModelVersionInfo>;
    type IntoFuture = BoxFut<'static, Self::Output>;
    fn into_future(self) -> Self::IntoFuture {
        let client = self.client;
        let request = self.request;
        Box::pin(async move { client.get_model_version(&request).await })
    }
}

/// Builder for updating a model version
pub struct UpdateModelVersionBuilder {
    client: ModelVersionServiceClient,
    request: UpdateModelVersion,
}

impl UpdateModelVersionBuilder {
    /// Create a new builder instance.
    /// Obtain via the corresponding method on `ModelVersionServiceClient`.
    pub(crate) fn new(
        client: ModelVersionServiceClient,
        full_name: impl Into<String>,
        version: i64,
        comment: impl Into<String>,
    ) -> Self {
        let request = UpdateModelVersion {
            full_name: full_name.into(),
            version,
            comment: comment.into(),
            ..Default::default()
        };
        Self { client, request }
    }
}

impl IntoFuture for UpdateModelVersionBuilder {
    type Output = Result<ModelVersionInfo>;
    type IntoFuture = BoxFut<'static, Self::Output>;
    fn into_future(self) -> Self::IntoFuture {
        let client = self.client;
        let request = self.request;
        Box::pin(async move { client.update_model_version(&request).await })
    }
}

/// Builder for deleting a model version
pub struct DeleteModelVersionBuilder {
    client: ModelVersionServiceClient,
    request: DeleteModelVersionRequest,
}

impl DeleteModelVersionBuilder {
    /// Create a new builder instance.
    /// Obtain via the corresponding method on `ModelVersionServiceClient`.
    pub(crate) fn new(
        client: ModelVersionServiceClient,
        full_name: impl Into<String>,
        version: i64,
    ) -> Self {
        let request = DeleteModelVersionRequest {
            full_name: full_name.into(),
            version,
            ..Default::default()
        };
        Self { client, request }
    }
}

impl IntoFuture for DeleteModelVersionBuilder {
    type Output = Result<()>;
    type IntoFuture = BoxFut<'static, Self::Output>;
    fn into_future(self) -> Self::IntoFuture {
        let client = self.client;
        let request = self.request;
        Box::pin(async move { client.delete_model_version(&request).await })
    }
}

/// Builder for model version
pub struct FinalizeModelVersionBuilder {
    client: ModelVersionServiceClient,
    request: FinalizeModelVersion,
}

impl FinalizeModelVersionBuilder {
    /// Create a new builder instance.
    /// Obtain via the corresponding method on `ModelVersionServiceClient`.
    pub(crate) fn new(
        client: ModelVersionServiceClient,
        full_name: impl Into<String>,
        version: i64,
    ) -> Self {
        let request = FinalizeModelVersion {
            full_name: full_name.into(),
            version,
            ..Default::default()
        };
        Self { client, request }
    }
}

impl IntoFuture for FinalizeModelVersionBuilder {
    type Output = Result<ModelVersionInfo>;
    type IntoFuture = BoxFut<'static, Self::Output>;
    fn into_future(self) -> Self::IntoFuture {
        let client = self.client;
        let request = self.request;
        Box::pin(async move { client.finalize_model_version(&request).await })
    }
}
