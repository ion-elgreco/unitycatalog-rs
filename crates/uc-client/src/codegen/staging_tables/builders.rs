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
use uc_client::models::tables::v1::*;

/// Builder for staging table
pub struct CreateStagingTableBuilder {
    client: StagingTableClient,
    request: CreateStagingTable,
}

impl CreateStagingTableBuilder {
    /// Create a new builder instance.
    /// Obtain via the corresponding method on `StagingTableClient`.
    pub(crate) fn new(
        client: StagingTableClient,
        name: impl Into<String>,
        catalog_name: impl Into<String>,
        schema_name: impl Into<String>,
    ) -> Self {
        let request = CreateStagingTable {
            name: name.into(),
            catalog_name: catalog_name.into(),
            schema_name: schema_name.into(),
            ..Default::default()
        };
        Self { client, request }
    }
}

impl IntoFuture for CreateStagingTableBuilder {
    type Output = Result<StagingTableInfo>;
    type IntoFuture = BoxFut<'static, Self::Output>;
    fn into_future(self) -> Self::IntoFuture {
        let client = self.client;
        let request = self.request;
        Box::pin(async move { client.create_staging_table(&request).await })
    }
}
