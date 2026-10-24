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
use uc_client::models::shares::v1::*;

/// Builder for listing shares
pub struct ListSharesBuilder {
    client: ShareServiceClient,
    request: ListSharesRequest,
}

impl ListSharesBuilder {
    /// Create a new builder instance.
    /// Obtain via the corresponding method on `ShareServiceClient`.
    pub(crate) fn new(client: ShareServiceClient) -> Self {
        let request = ListSharesRequest {
            ..Default::default()
        };
        Self { client, request }
    }

    /// The maximum number of shares to return.
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
    pub fn into_stream(self) -> BoxStr<'static, Result<ShareInfo>> {
        let remaining = self.request.max_results;
        let stream = stream_paginated(
            (self, remaining),
            move |(mut builder, mut remaining), page_token| async move {
                builder.request.page_token = page_token;
                if remaining.is_some() {
                    builder.request.max_results = remaining;
                }
                let mut res = builder.client.list_shares(&builder.request).await?;
                if let Some(ref mut rem) = remaining {
                    res.items.truncate((*rem).max(0) as usize);
                    *rem -= res.items.len() as i32;
                }
                let next_page_token = if remaining.is_some_and(|r| r <= 0) {
                    None
                } else {
                    res.next_page_token.clone()
                };
                Ok((res, (builder, remaining), next_page_token))
            },
        )
        .map_ok(|resp| futures::stream::iter(resp.items.into_iter().map(Ok)))
        .try_flatten();
        #[cfg(not(target_arch = "wasm32"))]
        let stream = stream.boxed();
        #[cfg(target_arch = "wasm32")]
        let stream = stream.boxed_local();
        stream
    }
}

impl IntoFuture for ListSharesBuilder {
    type Output = Result<ListSharesResponse>;
    type IntoFuture = BoxFut<'static, Self::Output>;
    fn into_future(self) -> Self::IntoFuture {
        let client = self.client;
        let request = self.request;
        Box::pin(async move { client.list_shares(&request).await })
    }
}

/// Builder for creating a share
pub struct CreateShareBuilder {
    client: ShareServiceClient,
    request: CreateShare,
}

impl CreateShareBuilder {
    /// Create a new builder instance.
    /// Obtain via the corresponding method on `ShareServiceClient`.
    pub(crate) fn new(client: ShareServiceClient, name: impl Into<String>) -> Self {
        let request = CreateShare {
            name: name.into(),
            ..Default::default()
        };
        Self { client, request }
    }

    /// User-provided free-form text description.
    pub fn with_comment(mut self, comment: impl Into<Option<String>>) -> Self {
        self.request.comment = comment.into();
        self
    }
}

impl IntoFuture for CreateShareBuilder {
    type Output = Result<ShareInfo>;
    type IntoFuture = BoxFut<'static, Self::Output>;
    fn into_future(self) -> Self::IntoFuture {
        let client = self.client;
        let request = self.request;
        Box::pin(async move { client.create_share(&request).await })
    }
}

/// Builder for getting a share
pub struct GetShareBuilder {
    client: ShareServiceClient,
    request: GetShareRequest,
}

impl GetShareBuilder {
    /// Create a new builder instance.
    /// Obtain via the corresponding method on `ShareServiceClient`.
    pub(crate) fn new(client: ShareServiceClient, name: impl Into<String>) -> Self {
        let request = GetShareRequest {
            name: name.into(),
            ..Default::default()
        };
        Self { client, request }
    }
}

impl IntoFuture for GetShareBuilder {
    type Output = Result<ShareInfo>;
    type IntoFuture = BoxFut<'static, Self::Output>;
    fn into_future(self) -> Self::IntoFuture {
        let client = self.client;
        let request = self.request;
        Box::pin(async move { client.get_share(&request).await })
    }
}

/// Builder for updating a share
pub struct UpdateShareBuilder {
    client: ShareServiceClient,
    request: UpdateShare,
}

impl UpdateShareBuilder {
    /// Create a new builder instance.
    /// Obtain via the corresponding method on `ShareServiceClient`.
    pub(crate) fn new(client: ShareServiceClient, name: impl Into<String>) -> Self {
        let request = UpdateShare {
            name: name.into(),
            ..Default::default()
        };
        Self { client, request }
    }

    /// User-provided free-form text description.
    pub fn with_comment(mut self, comment: impl Into<Option<String>>) -> Self {
        self.request.comment = comment.into();
        self
    }

    /// New name of the share.
    pub fn with_new_name(mut self, new_name: impl Into<Option<String>>) -> Self {
        self.request.new_name = new_name.into();
        self
    }

    /// Tables to add to the share. A table that the share holds already is skipped.
    pub fn with_add_objects<I>(mut self, add_objects: I) -> Self
    where
        I: IntoIterator<Item = SharedDataObject>,
    {
        self.request.add_objects = add_objects.into_iter().collect();
        self
    }

    /// Tables to remove from the share, matched by `source_table_full_name`.
    pub fn with_remove_objects<I>(mut self, remove_objects: I) -> Self
    where
        I: IntoIterator<Item = SharedDataObject>,
    {
        self.request.remove_objects = remove_objects.into_iter().collect();
        self
    }
}

impl IntoFuture for UpdateShareBuilder {
    type Output = Result<ShareInfo>;
    type IntoFuture = BoxFut<'static, Self::Output>;
    fn into_future(self) -> Self::IntoFuture {
        let client = self.client;
        let request = self.request;
        Box::pin(async move { client.update_share(&request).await })
    }
}

/// Builder for deleting a share
pub struct DeleteShareBuilder {
    client: ShareServiceClient,
    request: DeleteShareRequest,
}

impl DeleteShareBuilder {
    /// Create a new builder instance.
    /// Obtain via the corresponding method on `ShareServiceClient`.
    pub(crate) fn new(client: ShareServiceClient, name: impl Into<String>) -> Self {
        let request = DeleteShareRequest {
            name: name.into(),
            ..Default::default()
        };
        Self { client, request }
    }
}

impl IntoFuture for DeleteShareBuilder {
    type Output = Result<()>;
    type IntoFuture = BoxFut<'static, Self::Output>;
    fn into_future(self) -> Self::IntoFuture {
        let client = self.client;
        let request = self.request;
        Box::pin(async move { client.delete_share(&request).await })
    }
}
