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
use uc_client::models::recipients::v1::*;
use uc_client::models::sharing::v1::*;

/// Builder for listing recipients
pub struct ListRecipientsBuilder {
    client: RecipientServiceClient,
    request: ListRecipientsRequest,
}

impl ListRecipientsBuilder {
    /// Create a new builder instance.
    /// Obtain via the corresponding method on `RecipientServiceClient`.
    pub(crate) fn new(client: RecipientServiceClient) -> Self {
        let request = ListRecipientsRequest {
            ..Default::default()
        };
        Self { client, request }
    }

    /// The maximum number of recipients to return.
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
    pub fn into_stream(self) -> BoxStr<'static, Result<RecipientInfo>> {
        let remaining = self.request.max_results;
        let stream = stream_paginated(
            (self, remaining),
            move |(mut builder, mut remaining), page_token| async move {
                builder.request.page_token = page_token;
                if remaining.is_some() {
                    builder.request.max_results = remaining;
                }
                let mut res = builder.client.list_recipients(&builder.request).await?;
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

impl IntoFuture for ListRecipientsBuilder {
    type Output = Result<ListRecipientsResponse>;
    type IntoFuture = BoxFut<'static, Self::Output>;
    fn into_future(self) -> Self::IntoFuture {
        let client = self.client;
        let request = self.request;
        Box::pin(async move { client.list_recipients(&request).await })
    }
}

/// Builder for creating a recipient
pub struct CreateRecipientBuilder {
    client: RecipientServiceClient,
    request: CreateRecipient,
}

impl CreateRecipientBuilder {
    /// Create a new builder instance.
    /// Obtain via the corresponding method on `RecipientServiceClient`.
    pub(crate) fn new(client: RecipientServiceClient, name: impl Into<String>) -> Self {
        let request = CreateRecipient {
            name: name.into(),
            ..Default::default()
        };
        Self { client, request }
    }

    /// How the recipient signs in. Default: `TOKEN`.
    pub fn with_authentication_type(
        mut self,
        authentication_type: impl Into<Option<AuthenticationType>>,
    ) -> Self {
        self.request.authentication_type = authentication_type.into().map(buffa::EnumValue::Known);
        self
    }

    /// User-provided free-form text description.
    pub fn with_comment(mut self, comment: impl Into<Option<String>>) -> Self {
        self.request.comment = comment.into();
        self
    }
}

impl IntoFuture for CreateRecipientBuilder {
    type Output = Result<RecipientInfo>;
    type IntoFuture = BoxFut<'static, Self::Output>;
    fn into_future(self) -> Self::IntoFuture {
        let client = self.client;
        let request = self.request;
        Box::pin(async move { client.create_recipient(&request).await })
    }
}

/// Builder for getting a recipient
pub struct GetRecipientBuilder {
    client: RecipientServiceClient,
    request: GetRecipientRequest,
}

impl GetRecipientBuilder {
    /// Create a new builder instance.
    /// Obtain via the corresponding method on `RecipientServiceClient`.
    pub(crate) fn new(client: RecipientServiceClient, name: impl Into<String>) -> Self {
        let request = GetRecipientRequest {
            name: name.into(),
            ..Default::default()
        };
        Self { client, request }
    }
}

impl IntoFuture for GetRecipientBuilder {
    type Output = Result<RecipientInfo>;
    type IntoFuture = BoxFut<'static, Self::Output>;
    fn into_future(self) -> Self::IntoFuture {
        let client = self.client;
        let request = self.request;
        Box::pin(async move { client.get_recipient(&request).await })
    }
}

/// Builder for updating a recipient
pub struct UpdateRecipientBuilder {
    client: RecipientServiceClient,
    request: UpdateRecipient,
}

impl UpdateRecipientBuilder {
    /// Create a new builder instance.
    /// Obtain via the corresponding method on `RecipientServiceClient`.
    pub(crate) fn new(client: RecipientServiceClient, name: impl Into<String>) -> Self {
        let request = UpdateRecipient {
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

    /// Names of the shares to give the recipient access to.
    pub fn with_add_shares<I>(mut self, add_shares: I) -> Self
    where
        I: IntoIterator<Item = String>,
    {
        self.request.add_shares = add_shares.into_iter().collect();
        self
    }

    /// Names of the shares to take the recipient's access away from.
    pub fn with_remove_shares<I>(mut self, remove_shares: I) -> Self
    where
        I: IntoIterator<Item = String>,
    {
        self.request.remove_shares = remove_shares.into_iter().collect();
        self
    }
}

impl IntoFuture for UpdateRecipientBuilder {
    type Output = Result<RecipientInfo>;
    type IntoFuture = BoxFut<'static, Self::Output>;
    fn into_future(self) -> Self::IntoFuture {
        let client = self.client;
        let request = self.request;
        Box::pin(async move { client.update_recipient(&request).await })
    }
}

/// Builder for deleting a recipient
pub struct DeleteRecipientBuilder {
    client: RecipientServiceClient,
    request: DeleteRecipientRequest,
}

impl DeleteRecipientBuilder {
    /// Create a new builder instance.
    /// Obtain via the corresponding method on `RecipientServiceClient`.
    pub(crate) fn new(client: RecipientServiceClient, name: impl Into<String>) -> Self {
        let request = DeleteRecipientRequest {
            name: name.into(),
            ..Default::default()
        };
        Self { client, request }
    }
}

impl IntoFuture for DeleteRecipientBuilder {
    type Output = Result<()>;
    type IntoFuture = BoxFut<'static, Self::Output>;
    fn into_future(self) -> Self::IntoFuture {
        let client = self.client;
        let request = self.request;
        Box::pin(async move { client.delete_recipient(&request).await })
    }
}

/// Builder for recipient token
pub struct RotateRecipientTokenBuilder {
    client: RecipientServiceClient,
    request: RotateRecipientTokenRequest,
}

impl RotateRecipientTokenBuilder {
    /// Create a new builder instance.
    /// Obtain via the corresponding method on `RecipientServiceClient`.
    pub(crate) fn new(client: RecipientServiceClient, name: impl Into<String>) -> Self {
        let request = RotateRecipientTokenRequest {
            name: name.into(),
            ..Default::default()
        };
        Self { client, request }
    }
}

impl IntoFuture for RotateRecipientTokenBuilder {
    type Output = Result<RecipientInfo>;
    type IntoFuture = BoxFut<'static, Self::Output>;
    fn into_future(self) -> Self::IntoFuture {
        let client = self.client;
        let request = self.request;
        Box::pin(async move { client.rotate_recipient_token(&request).await })
    }
}

/// Builder for recipient profile
pub struct CreateRecipientProfileBuilder {
    client: RecipientServiceClient,
    request: CreateRecipientProfileRequest,
}

impl CreateRecipientProfileBuilder {
    /// Create a new builder instance.
    /// Obtain via the corresponding method on `RecipientServiceClient`.
    pub(crate) fn new(client: RecipientServiceClient, name: impl Into<String>) -> Self {
        let request = CreateRecipientProfileRequest {
            name: name.into(),
            ..Default::default()
        };
        Self { client, request }
    }
}

impl IntoFuture for CreateRecipientProfileBuilder {
    type Output = Result<RecipientProfile>;
    type IntoFuture = BoxFut<'static, Self::Output>;
    fn into_future(self) -> Self::IntoFuture {
        let client = self.client;
        let request = self.request;
        Box::pin(async move { client.create_recipient_profile(&request).await })
    }
}
