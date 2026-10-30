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
use uc_client::models::sharing::v1::*;

/// Builder for list shares
pub struct SharingListSharesBuilder {
    client: SharingClient,
    request: SharingListSharesRequest,
}

impl SharingListSharesBuilder {
    /// Create a new builder instance.
    /// Obtain via the corresponding method on `SharingClient`.
    pub(crate) fn new(client: SharingClient) -> Self {
        let request = SharingListSharesRequest {
            ..Default::default()
        };
        Self { client, request }
    }
}

impl IntoFuture for SharingListSharesBuilder {
    type Output = Result<SharingListSharesResponse>;
    type IntoFuture = BoxFut<'static, Self::Output>;
    fn into_future(self) -> Self::IntoFuture {
        let client = self.client;
        let request = self.request;
        Box::pin(async move { client.sharing_list_shares(&request).await })
    }
}

/// Builder for list schemas
pub struct SharingListSchemasBuilder {
    client: SharingClient,
    request: SharingListSchemasRequest,
}

impl SharingListSchemasBuilder {
    /// Create a new builder instance.
    /// Obtain via the corresponding method on `SharingClient`.
    pub(crate) fn new(client: SharingClient, share: impl Into<String>) -> Self {
        let request = SharingListSchemasRequest {
            share: share.into(),
            ..Default::default()
        };
        Self { client, request }
    }
}

impl IntoFuture for SharingListSchemasBuilder {
    type Output = Result<SharingListSchemasResponse>;
    type IntoFuture = BoxFut<'static, Self::Output>;
    fn into_future(self) -> Self::IntoFuture {
        let client = self.client;
        let request = self.request;
        Box::pin(async move { client.sharing_list_schemas(&request).await })
    }
}

/// Builder for list tables
pub struct SharingListTablesBuilder {
    client: SharingClient,
    request: SharingListTablesRequest,
}

impl SharingListTablesBuilder {
    /// Create a new builder instance.
    /// Obtain via the corresponding method on `SharingClient`.
    pub(crate) fn new(
        client: SharingClient,
        share: impl Into<String>,
        schema: impl Into<String>,
    ) -> Self {
        let request = SharingListTablesRequest {
            share: share.into(),
            schema: schema.into(),
            ..Default::default()
        };
        Self { client, request }
    }
}

impl IntoFuture for SharingListTablesBuilder {
    type Output = Result<SharingListTablesResponse>;
    type IntoFuture = BoxFut<'static, Self::Output>;
    fn into_future(self) -> Self::IntoFuture {
        let client = self.client;
        let request = self.request;
        Box::pin(async move { client.sharing_list_tables(&request).await })
    }
}

/// Builder for list all tables
pub struct SharingListAllTablesBuilder {
    client: SharingClient,
    request: SharingListAllTablesRequest,
}

impl SharingListAllTablesBuilder {
    /// Create a new builder instance.
    /// Obtain via the corresponding method on `SharingClient`.
    pub(crate) fn new(client: SharingClient, share: impl Into<String>) -> Self {
        let request = SharingListAllTablesRequest {
            share: share.into(),
            ..Default::default()
        };
        Self { client, request }
    }
}

impl IntoFuture for SharingListAllTablesBuilder {
    type Output = Result<SharingListTablesResponse>;
    type IntoFuture = BoxFut<'static, Self::Output>;
    fn into_future(self) -> Self::IntoFuture {
        let client = self.client;
        let request = self.request;
        Box::pin(async move { client.sharing_list_all_tables(&request).await })
    }
}

/// Builder for get table version
pub struct SharingGetTableVersionBuilder {
    client: SharingClient,
    request: SharingGetTableVersionRequest,
}

impl SharingGetTableVersionBuilder {
    /// Create a new builder instance.
    /// Obtain via the corresponding method on `SharingClient`.
    pub(crate) fn new(
        client: SharingClient,
        share: impl Into<String>,
        schema: impl Into<String>,
        table: impl Into<String>,
    ) -> Self {
        let request = SharingGetTableVersionRequest {
            share: share.into(),
            schema: schema.into(),
            table: table.into(),
            ..Default::default()
        };
        Self { client, request }
    }
}

impl IntoFuture for SharingGetTableVersionBuilder {
    type Output = Result<SharingGetTableVersionResponse>;
    type IntoFuture = BoxFut<'static, Self::Output>;
    fn into_future(self) -> Self::IntoFuture {
        let client = self.client;
        let request = self.request;
        Box::pin(async move { client.sharing_get_table_version(&request).await })
    }
}

/// Builder for get table metadata
pub struct SharingGetTableMetadataBuilder {
    client: SharingClient,
    request: SharingGetTableMetadataRequest,
}

impl SharingGetTableMetadataBuilder {
    /// Create a new builder instance.
    /// Obtain via the corresponding method on `SharingClient`.
    pub(crate) fn new(
        client: SharingClient,
        share: impl Into<String>,
        schema: impl Into<String>,
        table: impl Into<String>,
    ) -> Self {
        let request = SharingGetTableMetadataRequest {
            share: share.into(),
            schema: schema.into(),
            table: table.into(),
            ..Default::default()
        };
        Self { client, request }
    }
}

impl IntoFuture for SharingGetTableMetadataBuilder {
    type Output = Result<SharingTableLine>;
    type IntoFuture = BoxFut<'static, Self::Output>;
    fn into_future(self) -> Self::IntoFuture {
        let client = self.client;
        let request = self.request;
        Box::pin(async move { client.sharing_get_table_metadata(&request).await })
    }
}

/// Builder for query table
pub struct SharingQueryTableBuilder {
    client: SharingClient,
    request: SharingQueryTableRequest,
}

impl SharingQueryTableBuilder {
    /// Create a new builder instance.
    /// Obtain via the corresponding method on `SharingClient`.
    pub(crate) fn new(
        client: SharingClient,
        share: impl Into<String>,
        schema: impl Into<String>,
        table: impl Into<String>,
    ) -> Self {
        let request = SharingQueryTableRequest {
            share: share.into(),
            schema: schema.into(),
            table: table.into(),
            ..Default::default()
        };
        Self { client, request }
    }
}

impl IntoFuture for SharingQueryTableBuilder {
    type Output = Result<SharingTableLine>;
    type IntoFuture = BoxFut<'static, Self::Output>;
    fn into_future(self) -> Self::IntoFuture {
        let client = self.client;
        let request = self.request;
        Box::pin(async move { client.sharing_query_table(&request).await })
    }
}
