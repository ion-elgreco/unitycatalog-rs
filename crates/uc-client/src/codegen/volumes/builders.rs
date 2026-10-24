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
use uc_client::models::volumes::v1::*;

/// Builder for listing volumes
pub struct ListVolumesBuilder {
    client: VolumeServiceClient,
    request: ListVolumesRequest,
}

impl ListVolumesBuilder {
    /// Create a new builder instance.
    /// Obtain via the corresponding method on `VolumeServiceClient`.
    pub(crate) fn new(
        client: VolumeServiceClient,
        catalog_name: impl Into<String>,
        schema_name: impl Into<String>,
    ) -> Self {
        let request = ListVolumesRequest {
            catalog_name: catalog_name.into(),
            schema_name: schema_name.into(),
            ..Default::default()
        };
        Self { client, request }
    }

    /// The maximum number of volumes to return.
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
    pub fn into_stream(self) -> BoxStr<'static, Result<VolumeInfo>> {
        let remaining = self.request.max_results;
        let stream = stream_paginated(
            (self, remaining),
            move |(mut builder, mut remaining), page_token| async move {
                builder.request.page_token = page_token;
                if remaining.is_some() {
                    builder.request.max_results = remaining;
                }
                let mut res = builder.client.list_volumes(&builder.request).await?;
                if let Some(ref mut rem) = remaining {
                    res.volumes.truncate((*rem).max(0) as usize);
                    *rem -= res.volumes.len() as i32;
                }
                let next_page_token = if remaining.is_some_and(|r| r <= 0) {
                    None
                } else {
                    res.next_page_token.clone()
                };
                Ok((res, (builder, remaining), next_page_token))
            },
        )
        .map_ok(|resp| futures::stream::iter(resp.volumes.into_iter().map(Ok)))
        .try_flatten();
        #[cfg(not(target_arch = "wasm32"))]
        let stream = stream.boxed();
        #[cfg(target_arch = "wasm32")]
        let stream = stream.boxed_local();
        stream
    }
}

impl IntoFuture for ListVolumesBuilder {
    type Output = Result<ListVolumesResponseContent>;
    type IntoFuture = BoxFut<'static, Self::Output>;
    fn into_future(self) -> Self::IntoFuture {
        let client = self.client;
        let request = self.request;
        Box::pin(async move { client.list_volumes(&request).await })
    }
}

/// Builder for creating a volume
pub struct CreateVolumeBuilder {
    client: VolumeServiceClient,
    request: CreateVolumeRequestContent,
}

impl CreateVolumeBuilder {
    /// Create a new builder instance.
    /// Obtain via the corresponding method on `VolumeServiceClient`.
    pub(crate) fn new(
        client: VolumeServiceClient,
        catalog_name: impl Into<String>,
        schema_name: impl Into<String>,
        name: impl Into<String>,
        volume_type: VolumeType,
    ) -> Self {
        let request = CreateVolumeRequestContent {
            catalog_name: catalog_name.into(),
            schema_name: schema_name.into(),
            name: name.into(),
            volume_type: buffa::EnumValue::Known(volume_type),
            ..Default::default()
        };
        Self { client, request }
    }

    /// User-provided free-form text description.
    pub fn with_comment(mut self, comment: impl Into<Option<String>>) -> Self {
        self.request.comment = comment.into();
        self
    }

    /// Storage location URL of the volume. Required for EXTERNAL volumes; not allowed for MANAGED
    /// volumes.
    pub fn with_storage_location(mut self, storage_location: impl Into<Option<String>>) -> Self {
        self.request.storage_location = storage_location.into();
        self
    }
}

impl IntoFuture for CreateVolumeBuilder {
    type Output = Result<VolumeInfo>;
    type IntoFuture = BoxFut<'static, Self::Output>;
    fn into_future(self) -> Self::IntoFuture {
        let client = self.client;
        let request = self.request;
        Box::pin(async move { client.create_volume(&request).await })
    }
}

/// Builder for getting a volume
pub struct GetVolumeBuilder {
    client: VolumeServiceClient,
    request: GetVolumeRequest,
}

impl GetVolumeBuilder {
    /// Create a new builder instance.
    /// Obtain via the corresponding method on `VolumeServiceClient`.
    pub(crate) fn new(client: VolumeServiceClient, full_name: impl Into<String>) -> Self {
        let request = GetVolumeRequest {
            full_name: full_name.into(),
            ..Default::default()
        };
        Self { client, request }
    }
}

impl IntoFuture for GetVolumeBuilder {
    type Output = Result<VolumeInfo>;
    type IntoFuture = BoxFut<'static, Self::Output>;
    fn into_future(self) -> Self::IntoFuture {
        let client = self.client;
        let request = self.request;
        Box::pin(async move { client.get_volume(&request).await })
    }
}

/// Builder for updating a volume
pub struct UpdateVolumeBuilder {
    client: VolumeServiceClient,
    request: UpdateVolumeRequestContent,
}

impl UpdateVolumeBuilder {
    /// Create a new builder instance.
    /// Obtain via the corresponding method on `VolumeServiceClient`.
    pub(crate) fn new(client: VolumeServiceClient, full_name: impl Into<String>) -> Self {
        let request = UpdateVolumeRequestContent {
            full_name: full_name.into(),
            ..Default::default()
        };
        Self { client, request }
    }

    /// User-provided free-form text description.
    pub fn with_comment(mut self, comment: impl Into<Option<String>>) -> Self {
        self.request.comment = comment.into();
        self
    }

    /// New name of the volume, relative to the parent schema.
    pub fn with_new_name(mut self, new_name: impl Into<Option<String>>) -> Self {
        self.request.new_name = new_name.into();
        self
    }
}

impl IntoFuture for UpdateVolumeBuilder {
    type Output = Result<VolumeInfo>;
    type IntoFuture = BoxFut<'static, Self::Output>;
    fn into_future(self) -> Self::IntoFuture {
        let client = self.client;
        let request = self.request;
        Box::pin(async move { client.update_volume(&request).await })
    }
}

/// Builder for deleting a volume
pub struct DeleteVolumeBuilder {
    client: VolumeServiceClient,
    request: DeleteVolumeRequest,
}

impl DeleteVolumeBuilder {
    /// Create a new builder instance.
    /// Obtain via the corresponding method on `VolumeServiceClient`.
    pub(crate) fn new(client: VolumeServiceClient, full_name: impl Into<String>) -> Self {
        let request = DeleteVolumeRequest {
            full_name: full_name.into(),
            ..Default::default()
        };
        Self { client, request }
    }
}

impl IntoFuture for DeleteVolumeBuilder {
    type Output = Result<()>;
    type IntoFuture = BoxFut<'static, Self::Output>;
    fn into_future(self) -> Self::IntoFuture {
        let client = self.client;
        let request = self.request;
        Box::pin(async move { client.delete_volume(&request).await })
    }
}
