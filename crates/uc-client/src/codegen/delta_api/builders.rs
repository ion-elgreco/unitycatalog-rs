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
use uc_client::models::delta::v1::*;

/// Builder for delta config
pub struct GetDeltaConfigBuilder {
    client: DeltaApiClient,
    request: GetDeltaConfigRequest,
}

impl GetDeltaConfigBuilder {
    /// Create a new builder instance.
    /// Obtain via the corresponding method on `DeltaApiClient`.
    pub(crate) fn new(
        client: DeltaApiClient,
        catalog: impl Into<String>,
        protocol_versions: impl Into<String>,
    ) -> Self {
        let request = GetDeltaConfigRequest {
            catalog: catalog.into(),
            protocol_versions: protocol_versions.into(),
            ..Default::default()
        };
        Self { client, request }
    }
}

impl IntoFuture for GetDeltaConfigBuilder {
    type Output = Result<DeltaCatalogConfig>;
    type IntoFuture = BoxFut<'static, Self::Output>;
    fn into_future(self) -> Self::IntoFuture {
        let client = self.client;
        let request = self.request;
        Box::pin(async move { client.get_delta_config(&request).await })
    }
}

/// Builder for delta staging table
pub struct CreateDeltaStagingTableBuilder {
    client: DeltaApiClient,
    request: DeltaCreateStagingTableRequest,
}

impl CreateDeltaStagingTableBuilder {
    /// Create a new builder instance.
    /// Obtain via the corresponding method on `DeltaApiClient`.
    pub(crate) fn new(
        client: DeltaApiClient,
        catalog: impl Into<String>,
        schema: impl Into<String>,
        name: impl Into<String>,
    ) -> Self {
        let request = DeltaCreateStagingTableRequest {
            catalog: catalog.into(),
            schema: schema.into(),
            name: name.into(),
            ..Default::default()
        };
        Self { client, request }
    }
}

impl IntoFuture for CreateDeltaStagingTableBuilder {
    type Output = Result<DeltaStagingTableResponse>;
    type IntoFuture = BoxFut<'static, Self::Output>;
    fn into_future(self) -> Self::IntoFuture {
        let client = self.client;
        let request = self.request;
        Box::pin(async move { client.create_delta_staging_table(&request).await })
    }
}

/// Builder for delta table
pub struct CreateDeltaTableBuilder {
    client: DeltaApiClient,
    request: DeltaCreateTableRequest,
}

impl CreateDeltaTableBuilder {
    /// Create a new builder instance.
    /// Obtain via the corresponding method on `DeltaApiClient`.
    pub(crate) fn new(
        client: DeltaApiClient,
        catalog: impl Into<String>,
        schema: impl Into<String>,
        name: impl Into<String>,
        location: impl Into<String>,
        table_type: DeltaTableType,
        columns: DeltaStructType,
        protocol: DeltaProtocol,
        last_commit_timestamp_ms: i64,
    ) -> Self {
        let request = DeltaCreateTableRequest {
            catalog: catalog.into(),
            schema: schema.into(),
            name: name.into(),
            location: location.into(),
            table_type: buffa::EnumValue::Known(table_type),
            columns: buffa::MessageField::some(columns),
            protocol: buffa::MessageField::some(protocol),
            last_commit_timestamp_ms,
            ..Default::default()
        };
        Self { client, request }
    }

    /// User-provided free-form text description.
    pub fn with_comment(mut self, comment: impl Into<Option<String>>) -> Self {
        self.request.comment = comment.into();
        self
    }

    /// Names of the partition columns.
    pub fn with_partition_columns<I>(mut self, partition_columns: I) -> Self
    where
        I: IntoIterator<Item = String>,
    {
        self.request.partition_columns = partition_columns.into_iter().collect();
        self
    }

    /// The Delta table properties.
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

    /// The domain metadata of the table.
    pub fn with_domain_metadata(
        mut self,
        domain_metadata: impl Into<Option<DeltaDomainMetadataUpdates>>,
    ) -> Self {
        self.request.domain_metadata = {
            let domain_metadata: ::core::option::Option<_> = domain_metadata.into();
            buffa::MessageField::from(domain_metadata)
        };
        self
    }

    /// UniForm conversion metadata. With it, the table is UniForm-enabled; the client writes the
    /// Iceberg metadata file before the call.
    pub fn with_uniform(mut self, uniform: impl Into<Option<DeltaUniformMetadata>>) -> Self {
        self.request.uniform = {
            let uniform: ::core::option::Option<_> = uniform.into();
            buffa::MessageField::from(uniform)
        };
        self
    }
}

impl IntoFuture for CreateDeltaTableBuilder {
    type Output = Result<DeltaLoadTableResponse>;
    type IntoFuture = BoxFut<'static, Self::Output>;
    fn into_future(self) -> Self::IntoFuture {
        let client = self.client;
        let request = self.request;
        Box::pin(async move { client.create_delta_table(&request).await })
    }
}

/// Builder for delta table
pub struct LoadDeltaTableBuilder {
    client: DeltaApiClient,
    request: LoadDeltaTableRequest,
}

impl LoadDeltaTableBuilder {
    /// Create a new builder instance.
    /// Obtain via the corresponding method on `DeltaApiClient`.
    pub(crate) fn new(
        client: DeltaApiClient,
        catalog: impl Into<String>,
        schema: impl Into<String>,
        table: impl Into<String>,
    ) -> Self {
        let request = LoadDeltaTableRequest {
            catalog: catalog.into(),
            schema: schema.into(),
            table: table.into(),
            ..Default::default()
        };
        Self { client, request }
    }
}

impl IntoFuture for LoadDeltaTableBuilder {
    type Output = Result<DeltaLoadTableResponse>;
    type IntoFuture = BoxFut<'static, Self::Output>;
    fn into_future(self) -> Self::IntoFuture {
        let client = self.client;
        let request = self.request;
        Box::pin(async move { client.load_delta_table(&request).await })
    }
}

/// Builder for delta table
pub struct UpdateDeltaTableBuilder {
    client: DeltaApiClient,
    request: DeltaUpdateTableRequest,
}

impl UpdateDeltaTableBuilder {
    /// Create a new builder instance.
    /// Obtain via the corresponding method on `DeltaApiClient`.
    pub(crate) fn new(
        client: DeltaApiClient,
        catalog: impl Into<String>,
        schema: impl Into<String>,
        table: impl Into<String>,
        requirements: Vec<DeltaTableRequirement>,
        updates: Vec<DeltaTableUpdate>,
    ) -> Self {
        let request = DeltaUpdateTableRequest {
            catalog: catalog.into(),
            schema: schema.into(),
            table: table.into(),
            requirements,
            updates,
            ..Default::default()
        };
        Self { client, request }
    }
}

impl IntoFuture for UpdateDeltaTableBuilder {
    type Output = Result<DeltaLoadTableResponse>;
    type IntoFuture = BoxFut<'static, Self::Output>;
    fn into_future(self) -> Self::IntoFuture {
        let client = self.client;
        let request = self.request;
        Box::pin(async move { client.update_delta_table(&request).await })
    }
}

/// Builder for table exists
pub struct DeltaTableExistsBuilder {
    client: DeltaApiClient,
    request: DeltaTableExistsRequest,
}

impl DeltaTableExistsBuilder {
    /// Create a new builder instance.
    /// Obtain via the corresponding method on `DeltaApiClient`.
    pub(crate) fn new(
        client: DeltaApiClient,
        catalog: impl Into<String>,
        schema: impl Into<String>,
        table: impl Into<String>,
    ) -> Self {
        let request = DeltaTableExistsRequest {
            catalog: catalog.into(),
            schema: schema.into(),
            table: table.into(),
            ..Default::default()
        };
        Self { client, request }
    }
}

impl IntoFuture for DeltaTableExistsBuilder {
    type Output = Result<bool>;
    type IntoFuture = BoxFut<'static, Self::Output>;
    fn into_future(self) -> Self::IntoFuture {
        let client = self.client;
        let request = self.request;
        Box::pin(async move { client.delta_table_exists(&request).await })
    }
}

/// Builder for delta table
pub struct DeleteDeltaTableBuilder {
    client: DeltaApiClient,
    request: DeleteDeltaTableRequest,
}

impl DeleteDeltaTableBuilder {
    /// Create a new builder instance.
    /// Obtain via the corresponding method on `DeltaApiClient`.
    pub(crate) fn new(
        client: DeltaApiClient,
        catalog: impl Into<String>,
        schema: impl Into<String>,
        table: impl Into<String>,
    ) -> Self {
        let request = DeleteDeltaTableRequest {
            catalog: catalog.into(),
            schema: schema.into(),
            table: table.into(),
            ..Default::default()
        };
        Self { client, request }
    }
}

impl IntoFuture for DeleteDeltaTableBuilder {
    type Output = Result<()>;
    type IntoFuture = BoxFut<'static, Self::Output>;
    fn into_future(self) -> Self::IntoFuture {
        let client = self.client;
        let request = self.request;
        Box::pin(async move { client.delete_delta_table(&request).await })
    }
}

/// Builder for delta table
pub struct RenameDeltaTableBuilder {
    client: DeltaApiClient,
    request: DeltaRenameTableRequest,
}

impl RenameDeltaTableBuilder {
    /// Create a new builder instance.
    /// Obtain via the corresponding method on `DeltaApiClient`.
    pub(crate) fn new(
        client: DeltaApiClient,
        catalog: impl Into<String>,
        schema: impl Into<String>,
        table: impl Into<String>,
        new_name: impl Into<String>,
    ) -> Self {
        let request = DeltaRenameTableRequest {
            catalog: catalog.into(),
            schema: schema.into(),
            table: table.into(),
            new_name: new_name.into(),
            ..Default::default()
        };
        Self { client, request }
    }
}

impl IntoFuture for RenameDeltaTableBuilder {
    type Output = Result<()>;
    type IntoFuture = BoxFut<'static, Self::Output>;
    fn into_future(self) -> Self::IntoFuture {
        let client = self.client;
        let request = self.request;
        Box::pin(async move { client.rename_delta_table(&request).await })
    }
}

/// Builder for delta table metrics
pub struct ReportDeltaTableMetricsBuilder {
    client: DeltaApiClient,
    request: DeltaReportMetricsRequest,
}

impl ReportDeltaTableMetricsBuilder {
    /// Create a new builder instance.
    /// Obtain via the corresponding method on `DeltaApiClient`.
    pub(crate) fn new(
        client: DeltaApiClient,
        catalog: impl Into<String>,
        schema: impl Into<String>,
        table: impl Into<String>,
        table_id: impl Into<String>,
    ) -> Self {
        let request = DeltaReportMetricsRequest {
            catalog: catalog.into(),
            schema: schema.into(),
            table: table.into(),
            table_id: table_id.into(),
            ..Default::default()
        };
        Self { client, request }
    }

    /// The metrics. The server accepts them and does not store them.
    pub fn with_report(mut self, report: impl Into<Option<DeltaMetricsReport>>) -> Self {
        self.request.report = {
            let report: ::core::option::Option<_> = report.into();
            buffa::MessageField::from(report)
        };
        self
    }
}

impl IntoFuture for ReportDeltaTableMetricsBuilder {
    type Output = Result<()>;
    type IntoFuture = BoxFut<'static, Self::Output>;
    fn into_future(self) -> Self::IntoFuture {
        let client = self.client;
        let request = self.request;
        Box::pin(async move { client.report_delta_table_metrics(&request).await })
    }
}

/// Builder for delta table credentials
pub struct GetDeltaTableCredentialsBuilder {
    client: DeltaApiClient,
    request: GetDeltaTableCredentialsRequest,
}

impl GetDeltaTableCredentialsBuilder {
    /// Create a new builder instance.
    /// Obtain via the corresponding method on `DeltaApiClient`.
    pub(crate) fn new(
        client: DeltaApiClient,
        catalog: impl Into<String>,
        schema: impl Into<String>,
        table: impl Into<String>,
        operation: DeltaCredentialOperation,
    ) -> Self {
        let request = GetDeltaTableCredentialsRequest {
            catalog: catalog.into(),
            schema: schema.into(),
            table: table.into(),
            operation: buffa::EnumValue::Known(operation),
            ..Default::default()
        };
        Self { client, request }
    }
}

impl IntoFuture for GetDeltaTableCredentialsBuilder {
    type Output = Result<DeltaCredentialsResponse>;
    type IntoFuture = BoxFut<'static, Self::Output>;
    fn into_future(self) -> Self::IntoFuture {
        let client = self.client;
        let request = self.request;
        Box::pin(async move { client.get_delta_table_credentials(&request).await })
    }
}

/// Builder for delta staging table credentials
pub struct GetDeltaStagingTableCredentialsBuilder {
    client: DeltaApiClient,
    request: GetDeltaStagingTableCredentialsRequest,
}

impl GetDeltaStagingTableCredentialsBuilder {
    /// Create a new builder instance.
    /// Obtain via the corresponding method on `DeltaApiClient`.
    pub(crate) fn new(client: DeltaApiClient, table_id: impl Into<String>) -> Self {
        let request = GetDeltaStagingTableCredentialsRequest {
            table_id: table_id.into(),
            ..Default::default()
        };
        Self { client, request }
    }
}

impl IntoFuture for GetDeltaStagingTableCredentialsBuilder {
    type Output = Result<DeltaCredentialsResponse>;
    type IntoFuture = BoxFut<'static, Self::Output>;
    fn into_future(self) -> Self::IntoFuture {
        let client = self.client;
        let request = self.request;
        Box::pin(async move { client.get_delta_staging_table_credentials(&request).await })
    }
}
