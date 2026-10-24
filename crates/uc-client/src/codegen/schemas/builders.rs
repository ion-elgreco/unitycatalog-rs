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
use uc_client::models::schemas::v1::*;

/// Builder for listing schemas
pub struct ListSchemasBuilder {
    client: SchemaServiceClient,
    request: ListSchemasRequest,
}

impl ListSchemasBuilder {
    /// Create a new builder instance.
    /// Obtain via the corresponding method on `SchemaServiceClient`.
    pub(crate) fn new(client: SchemaServiceClient, catalog_name: impl Into<String>) -> Self {
        let request = ListSchemasRequest {
            catalog_name: catalog_name.into(),
            ..Default::default()
        };
        Self { client, request }
    }

    /// The maximum number of schemas to return.
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
    pub fn into_stream(self) -> BoxStr<'static, Result<SchemaInfo>> {
        let remaining = self.request.max_results;
        let stream = stream_paginated(
            (self, remaining),
            move |(mut builder, mut remaining), page_token| async move {
                builder.request.page_token = page_token;
                if remaining.is_some() {
                    builder.request.max_results = remaining;
                }
                let mut res = builder.client.list_schemas(&builder.request).await?;
                if let Some(ref mut rem) = remaining {
                    res.schemas.truncate((*rem).max(0) as usize);
                    *rem -= res.schemas.len() as i32;
                }
                let next_page_token = if remaining.is_some_and(|r| r <= 0) {
                    None
                } else {
                    res.next_page_token.clone()
                };
                Ok((res, (builder, remaining), next_page_token))
            },
        )
        .map_ok(|resp| futures::stream::iter(resp.schemas.into_iter().map(Ok)))
        .try_flatten();
        #[cfg(not(target_arch = "wasm32"))]
        let stream = stream.boxed();
        #[cfg(target_arch = "wasm32")]
        let stream = stream.boxed_local();
        stream
    }
}

impl IntoFuture for ListSchemasBuilder {
    type Output = Result<ListSchemasResponse>;
    type IntoFuture = BoxFut<'static, Self::Output>;
    fn into_future(self) -> Self::IntoFuture {
        let client = self.client;
        let request = self.request;
        Box::pin(async move { client.list_schemas(&request).await })
    }
}

/// Builder for creating a schema
pub struct CreateSchemaBuilder {
    client: SchemaServiceClient,
    request: CreateSchema,
}

impl CreateSchemaBuilder {
    /// Create a new builder instance.
    /// Obtain via the corresponding method on `SchemaServiceClient`.
    pub(crate) fn new(
        client: SchemaServiceClient,
        name: impl Into<String>,
        catalog_name: impl Into<String>,
    ) -> Self {
        let request = CreateSchema {
            name: name.into(),
            catalog_name: catalog_name.into(),
            ..Default::default()
        };
        Self { client, request }
    }

    /// User-provided free-form text description.
    pub fn with_comment(mut self, comment: impl Into<Option<String>>) -> Self {
        self.request.comment = comment.into();
        self
    }

    /// Key-value properties of the schema.
    pub fn with_properties<I, K, V>(mut self, properties: I) -> Self
    where
        I: IntoIterator<Item = (K, V)>,
        K: Into<String>,
        V: Into<String>,
    {
        self.request.properties = properties
            .into_iter()
            .map(|(k, v)| (k.into(), v.into()))
            .collect();
        self
    }

    /// Storage root URL for the managed storage of the schema.
    pub fn with_storage_root(mut self, storage_root: impl Into<Option<String>>) -> Self {
        self.request.storage_root = storage_root.into();
        self
    }
}

impl IntoFuture for CreateSchemaBuilder {
    type Output = Result<SchemaInfo>;
    type IntoFuture = BoxFut<'static, Self::Output>;
    fn into_future(self) -> Self::IntoFuture {
        let client = self.client;
        let request = self.request;
        Box::pin(async move { client.create_schema(&request).await })
    }
}

/// Builder for getting a schema
pub struct GetSchemaBuilder {
    client: SchemaServiceClient,
    request: GetSchemaRequest,
}

impl GetSchemaBuilder {
    /// Create a new builder instance.
    /// Obtain via the corresponding method on `SchemaServiceClient`.
    pub(crate) fn new(client: SchemaServiceClient, full_name: impl Into<String>) -> Self {
        let request = GetSchemaRequest {
            full_name: full_name.into(),
            ..Default::default()
        };
        Self { client, request }
    }
}

impl IntoFuture for GetSchemaBuilder {
    type Output = Result<SchemaInfo>;
    type IntoFuture = BoxFut<'static, Self::Output>;
    fn into_future(self) -> Self::IntoFuture {
        let client = self.client;
        let request = self.request;
        Box::pin(async move { client.get_schema(&request).await })
    }
}

/// Builder for updating a schema
pub struct UpdateSchemaBuilder {
    client: SchemaServiceClient,
    request: UpdateSchema,
}

impl UpdateSchemaBuilder {
    /// Create a new builder instance.
    /// Obtain via the corresponding method on `SchemaServiceClient`.
    pub(crate) fn new(client: SchemaServiceClient, full_name: impl Into<String>) -> Self {
        let request = UpdateSchema {
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

    /// Key-value properties of the schema. They replace the properties of the schema.
    pub fn with_properties<I, K, V>(mut self, properties: I) -> Self
    where
        I: IntoIterator<Item = (K, V)>,
        K: Into<String>,
        V: Into<String>,
    {
        self.request.properties = properties
            .into_iter()
            .map(|(k, v)| (k.into(), v.into()))
            .collect();
        self
    }

    /// New name of the schema, relative to the parent catalog.
    pub fn with_new_name(mut self, new_name: impl Into<Option<String>>) -> Self {
        self.request.new_name = new_name.into();
        self
    }
}

impl IntoFuture for UpdateSchemaBuilder {
    type Output = Result<SchemaInfo>;
    type IntoFuture = BoxFut<'static, Self::Output>;
    fn into_future(self) -> Self::IntoFuture {
        let client = self.client;
        let request = self.request;
        Box::pin(async move { client.update_schema(&request).await })
    }
}

/// Builder for deleting a schema
pub struct DeleteSchemaBuilder {
    client: SchemaServiceClient,
    request: DeleteSchemaRequest,
}

impl DeleteSchemaBuilder {
    /// Create a new builder instance.
    /// Obtain via the corresponding method on `SchemaServiceClient`.
    pub(crate) fn new(client: SchemaServiceClient, full_name: impl Into<String>) -> Self {
        let request = DeleteSchemaRequest {
            full_name: full_name.into(),
            ..Default::default()
        };
        Self { client, request }
    }

    /// Delete the tables, volumes, functions and models of the schema too.
    pub fn with_force(mut self, force: impl Into<Option<bool>>) -> Self {
        self.request.force = force.into();
        self
    }
}

impl IntoFuture for DeleteSchemaBuilder {
    type Output = Result<()>;
    type IntoFuture = BoxFut<'static, Self::Output>;
    fn into_future(self) -> Self::IntoFuture {
        let client = self.client;
        let request = self.request;
        Box::pin(async move { client.delete_schema(&request).await })
    }
}
