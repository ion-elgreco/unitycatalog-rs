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
use uc_client::models::tables::v1::*;

/// Builder for listing tables
pub struct ListTablesBuilder {
    client: TableServiceClient,
    request: ListTablesRequest,
}

impl ListTablesBuilder {
    /// Create a new builder instance.
    /// Obtain via the corresponding method on `TableServiceClient`.
    pub(crate) fn new(
        client: TableServiceClient,
        catalog_name: impl Into<String>,
        schema_name: impl Into<String>,
    ) -> Self {
        let request = ListTablesRequest {
            catalog_name: catalog_name.into(),
            schema_name: schema_name.into(),
            ..Default::default()
        };
        Self { client, request }
    }

    /// The maximum number of tables to return.
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
    pub fn into_stream(self) -> BoxStr<'static, Result<TableInfo>> {
        let remaining = self.request.max_results;
        let stream = stream_paginated(
            (self, remaining),
            move |(mut builder, mut remaining), page_token| async move {
                builder.request.page_token = page_token;
                if remaining.is_some() {
                    builder.request.max_results = remaining;
                }
                let mut res = builder.client.list_tables(&builder.request).await?;
                if let Some(ref mut rem) = remaining {
                    res.tables.truncate((*rem).max(0) as usize);
                    *rem -= res.tables.len() as i32;
                }
                let next_page_token = if remaining.is_some_and(|r| r <= 0) {
                    None
                } else {
                    res.next_page_token.clone()
                };
                Ok((res, (builder, remaining), next_page_token))
            },
        )
        .map_ok(|resp| futures::stream::iter(resp.tables.into_iter().map(Ok)))
        .try_flatten();
        #[cfg(not(target_arch = "wasm32"))]
        let stream = stream.boxed();
        #[cfg(target_arch = "wasm32")]
        let stream = stream.boxed_local();
        stream
    }
}

impl IntoFuture for ListTablesBuilder {
    type Output = Result<ListTablesResponse>;
    type IntoFuture = BoxFut<'static, Self::Output>;
    fn into_future(self) -> Self::IntoFuture {
        let client = self.client;
        let request = self.request;
        Box::pin(async move { client.list_tables(&request).await })
    }
}

/// Builder for creating a table
pub struct CreateTableBuilder {
    client: TableServiceClient,
    request: CreateTable,
}

impl CreateTableBuilder {
    /// Create a new builder instance.
    /// Obtain via the corresponding method on `TableServiceClient`.
    pub(crate) fn new(
        client: TableServiceClient,
        name: impl Into<String>,
        catalog_name: impl Into<String>,
        schema_name: impl Into<String>,
        table_type: TableType,
    ) -> Self {
        let request = CreateTable {
            name: name.into(),
            catalog_name: catalog_name.into(),
            schema_name: schema_name.into(),
            table_type: buffa::EnumValue::Known(table_type),
            ..Default::default()
        };
        Self { client, request }
    }

    /// The columns of the table. Views and metric views need at least one.
    pub fn with_columns<I>(mut self, columns: I) -> Self
    where
        I: IntoIterator<Item = ColumnInfo>,
    {
        self.request.columns = columns.into_iter().collect();
        self
    }

    /// File format of the data of the table. MANAGED tables are DELTA; ICEBERG tables come only
    /// from the Iceberg REST catalog.
    pub fn with_data_source_format(
        mut self,
        data_source_format: impl Into<Option<DataSourceFormat>>,
    ) -> Self {
        self.request.data_source_format = data_source_format.into().map(buffa::EnumValue::Known);
        self
    }

    /// Storage location URL of the table. Required for EXTERNAL tables; for MANAGED tables, the
    /// `staging_location` of a staging table that the caller created.
    pub fn with_storage_location(mut self, storage_location: impl Into<Option<String>>) -> Self {
        self.request.storage_location = storage_location.into();
        self
    }

    /// User-provided free-form text description.
    pub fn with_comment(mut self, comment: impl Into<Option<String>>) -> Self {
        self.request.comment = comment.into();
        self
    }

    /// Key-value properties of the table.
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

    /// Definition of a view-like table: SQL for views, YAML for metric views.
    pub fn with_view_definition(mut self, view_definition: impl Into<Option<String>>) -> Self {
        self.request.view_definition = view_definition.into();
        self
    }

    /// The tables and functions that a view-like table reads. Required for metric views.
    pub fn with_view_dependencies(
        mut self,
        view_dependencies: impl Into<Option<DependencyList>>,
    ) -> Self {
        self.request.view_dependencies = {
            let view_dependencies: ::core::option::Option<_> = view_dependencies.into();
            buffa::MessageField::from(view_dependencies)
        };
        self
    }
}

impl IntoFuture for CreateTableBuilder {
    type Output = Result<TableInfo>;
    type IntoFuture = BoxFut<'static, Self::Output>;
    fn into_future(self) -> Self::IntoFuture {
        let client = self.client;
        let request = self.request;
        Box::pin(async move { client.create_table(&request).await })
    }
}

/// Builder for getting a table
pub struct GetTableBuilder {
    client: TableServiceClient,
    request: GetTableRequest,
}

impl GetTableBuilder {
    /// Create a new builder instance.
    /// Obtain via the corresponding method on `TableServiceClient`.
    pub(crate) fn new(client: TableServiceClient, full_name: impl Into<String>) -> Self {
        let request = GetTableRequest {
            full_name: full_name.into(),
            ..Default::default()
        };
        Self { client, request }
    }

    /// Show a streaming table as a MANAGED table, as upstream's server can. This server does not
    /// read it.
    pub fn with_read_streaming_table_as_managed(
        mut self,
        read_streaming_table_as_managed: impl Into<Option<bool>>,
    ) -> Self {
        self.request.read_streaming_table_as_managed = read_streaming_table_as_managed.into();
        self
    }

    /// Show a materialized view as a MANAGED table, as upstream's server can. This server does not
    /// read it.
    pub fn with_read_materialized_view_as_managed(
        mut self,
        read_materialized_view_as_managed: impl Into<Option<bool>>,
    ) -> Self {
        self.request.read_materialized_view_as_managed = read_materialized_view_as_managed.into();
        self
    }
}

impl IntoFuture for GetTableBuilder {
    type Output = Result<TableInfo>;
    type IntoFuture = BoxFut<'static, Self::Output>;
    fn into_future(self) -> Self::IntoFuture {
        let client = self.client;
        let request = self.request;
        Box::pin(async move { client.get_table(&request).await })
    }
}

/// Builder for updating a table
pub struct UpdateTableBuilder {
    client: TableServiceClient,
    request: UpdateTable,
}

impl UpdateTableBuilder {
    /// Create a new builder instance.
    /// Obtain via the corresponding method on `TableServiceClient`.
    pub(crate) fn new(client: TableServiceClient, full_name: impl Into<String>) -> Self {
        let request = UpdateTable {
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

    /// Columns that get a new comment; the others keep theirs.
    pub fn with_columns<I>(mut self, columns: I) -> Self
    where
        I: IntoIterator<Item = UpdateColumn>,
    {
        self.request.columns = columns.into_iter().collect();
        self
    }
}

impl IntoFuture for UpdateTableBuilder {
    type Output = Result<TableInfo>;
    type IntoFuture = BoxFut<'static, Self::Output>;
    fn into_future(self) -> Self::IntoFuture {
        let client = self.client;
        let request = self.request;
        Box::pin(async move { client.update_table(&request).await })
    }
}

/// Builder for deleting a table
pub struct DeleteTableBuilder {
    client: TableServiceClient,
    request: DeleteTableRequest,
}

impl DeleteTableBuilder {
    /// Create a new builder instance.
    /// Obtain via the corresponding method on `TableServiceClient`.
    pub(crate) fn new(client: TableServiceClient, full_name: impl Into<String>) -> Self {
        let request = DeleteTableRequest {
            full_name: full_name.into(),
            ..Default::default()
        };
        Self { client, request }
    }
}

impl IntoFuture for DeleteTableBuilder {
    type Output = Result<()>;
    type IntoFuture = BoxFut<'static, Self::Output>;
    fn into_future(self) -> Self::IntoFuture {
        let client = self.client;
        let request = self.request;
        Box::pin(async move { client.delete_table(&request).await })
    }
}
