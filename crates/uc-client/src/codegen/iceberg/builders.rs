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
use uc_client::models::iceberg::v1::*;

/// Builder for iceberg config
pub struct GetIcebergConfigBuilder {
    client: IcebergClient,
    request: GetIcebergConfigRequest,
}

impl GetIcebergConfigBuilder {
    /// Create a new builder instance.
    /// Obtain via the corresponding method on `IcebergClient`.
    pub(crate) fn new(client: IcebergClient, warehouse: impl Into<String>) -> Self {
        let request = GetIcebergConfigRequest {
            warehouse: warehouse.into(),
            ..Default::default()
        };
        Self { client, request }
    }
}

impl IntoFuture for GetIcebergConfigBuilder {
    type Output = Result<IcebergConfigResponse>;
    type IntoFuture = BoxFut<'static, Self::Output>;
    fn into_future(self) -> Self::IntoFuture {
        let client = self.client;
        let request = self.request;
        Box::pin(async move { client.get_iceberg_config(&request).await })
    }
}

/// Builder for iceberg namespaces
pub struct ListIcebergNamespacesBuilder {
    client: IcebergClient,
    request: ListIcebergNamespacesRequest,
}

impl ListIcebergNamespacesBuilder {
    /// Create a new builder instance.
    /// Obtain via the corresponding method on `IcebergClient`.
    pub(crate) fn new(client: IcebergClient, catalog: impl Into<String>) -> Self {
        let request = ListIcebergNamespacesRequest {
            catalog: catalog.into(),
            ..Default::default()
        };
        Self { client, request }
    }

    /// Parent namespace. Namespaces are not nested: a parent that exists has no children, a parent
    /// that does not exist is not found. An empty value is no parent.
    pub fn with_parent(mut self, parent: impl Into<Option<String>>) -> Self {
        self.request.parent = parent.into();
        self
    }

    /// Opaque token of the page to return: the `next-page-token` of the page before. An empty token
    /// asks for the first page. Without a token the answer is the whole listing.
    pub fn with_page_token(mut self, page_token: impl Into<Option<String>>) -> Self {
        self.request.page_token = page_token.into();
        self
    }

    /// The maximum number of namespaces in a page, at least 1; a larger value than 100 counts as
    /// 100. Read only with a page token.
    pub fn with_page_size(mut self, page_size: impl Into<Option<i32>>) -> Self {
        self.request.page_size = page_size.into();
        self
    }
}

impl IntoFuture for ListIcebergNamespacesBuilder {
    type Output = Result<IcebergListNamespacesResponse>;
    type IntoFuture = BoxFut<'static, Self::Output>;
    fn into_future(self) -> Self::IntoFuture {
        let client = self.client;
        let request = self.request;
        Box::pin(async move { client.list_iceberg_namespaces(&request).await })
    }
}

/// Builder for iceberg namespace
pub struct CreateIcebergNamespaceBuilder {
    client: IcebergClient,
    request: IcebergCreateNamespaceRequest,
}

impl CreateIcebergNamespaceBuilder {
    /// Create a new builder instance.
    /// Obtain via the corresponding method on `IcebergClient`.
    pub(crate) fn new(
        client: IcebergClient,
        catalog: impl Into<String>,
        namespace: Vec<String>,
    ) -> Self {
        let request = IcebergCreateNamespaceRequest {
            catalog: catalog.into(),
            namespace,
            ..Default::default()
        };
        Self { client, request }
    }

    /// Properties of the namespace.
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
}

impl IntoFuture for CreateIcebergNamespaceBuilder {
    type Output = Result<IcebergNamespaceResponse>;
    type IntoFuture = BoxFut<'static, Self::Output>;
    fn into_future(self) -> Self::IntoFuture {
        let client = self.client;
        let request = self.request;
        Box::pin(async move { client.create_iceberg_namespace(&request).await })
    }
}

/// Builder for iceberg namespace
pub struct LoadIcebergNamespaceBuilder {
    client: IcebergClient,
    request: LoadIcebergNamespaceRequest,
}

impl LoadIcebergNamespaceBuilder {
    /// Create a new builder instance.
    /// Obtain via the corresponding method on `IcebergClient`.
    pub(crate) fn new(
        client: IcebergClient,
        catalog: impl Into<String>,
        namespace: impl Into<String>,
    ) -> Self {
        let request = LoadIcebergNamespaceRequest {
            catalog: catalog.into(),
            namespace: namespace.into(),
            ..Default::default()
        };
        Self { client, request }
    }
}

impl IntoFuture for LoadIcebergNamespaceBuilder {
    type Output = Result<IcebergNamespaceResponse>;
    type IntoFuture = BoxFut<'static, Self::Output>;
    fn into_future(self) -> Self::IntoFuture {
        let client = self.client;
        let request = self.request;
        Box::pin(async move { client.load_iceberg_namespace(&request).await })
    }
}

/// Builder for namespace exists
pub struct IcebergNamespaceExistsBuilder {
    client: IcebergClient,
    request: IcebergNamespaceExistsRequest,
}

impl IcebergNamespaceExistsBuilder {
    /// Create a new builder instance.
    /// Obtain via the corresponding method on `IcebergClient`.
    pub(crate) fn new(
        client: IcebergClient,
        catalog: impl Into<String>,
        namespace: impl Into<String>,
    ) -> Self {
        let request = IcebergNamespaceExistsRequest {
            catalog: catalog.into(),
            namespace: namespace.into(),
            ..Default::default()
        };
        Self { client, request }
    }
}

impl IntoFuture for IcebergNamespaceExistsBuilder {
    type Output = Result<bool>;
    type IntoFuture = BoxFut<'static, Self::Output>;
    fn into_future(self) -> Self::IntoFuture {
        let client = self.client;
        let request = self.request;
        Box::pin(async move { client.iceberg_namespace_exists(&request).await })
    }
}

/// Builder for iceberg namespace
pub struct DropIcebergNamespaceBuilder {
    client: IcebergClient,
    request: DropIcebergNamespaceRequest,
}

impl DropIcebergNamespaceBuilder {
    /// Create a new builder instance.
    /// Obtain via the corresponding method on `IcebergClient`.
    pub(crate) fn new(
        client: IcebergClient,
        catalog: impl Into<String>,
        namespace: impl Into<String>,
    ) -> Self {
        let request = DropIcebergNamespaceRequest {
            catalog: catalog.into(),
            namespace: namespace.into(),
            ..Default::default()
        };
        Self { client, request }
    }
}

impl IntoFuture for DropIcebergNamespaceBuilder {
    type Output = Result<()>;
    type IntoFuture = BoxFut<'static, Self::Output>;
    fn into_future(self) -> Self::IntoFuture {
        let client = self.client;
        let request = self.request;
        Box::pin(async move { client.drop_iceberg_namespace(&request).await })
    }
}

/// Builder for iceberg namespace properties
pub struct UpdateIcebergNamespacePropertiesBuilder {
    client: IcebergClient,
    request: IcebergUpdateNamespacePropertiesRequest,
}

impl UpdateIcebergNamespacePropertiesBuilder {
    /// Create a new builder instance.
    /// Obtain via the corresponding method on `IcebergClient`.
    pub(crate) fn new(
        client: IcebergClient,
        catalog: impl Into<String>,
        namespace: impl Into<String>,
    ) -> Self {
        let request = IcebergUpdateNamespacePropertiesRequest {
            catalog: catalog.into(),
            namespace: namespace.into(),
            ..Default::default()
        };
        Self { client, request }
    }

    /// Keys of the properties to remove. A key cannot be both removed and set.
    pub fn with_removals<I>(mut self, removals: I) -> Self
    where
        I: IntoIterator<Item = String>,
    {
        self.request.removals = removals.into_iter().collect();
        self
    }

    /// Properties to set.
    pub fn with_updates<I, K, V>(mut self, updates: I) -> Self
    where
        I: IntoIterator<Item = (K, V)>,
        K: Into<String>,
        V: Into<String>,
    {
        self.request.updates = updates
            .into_iter()
            .map(|(k, v)| (k.into(), v.into()))
            .collect();
        self
    }
}

impl IntoFuture for UpdateIcebergNamespacePropertiesBuilder {
    type Output = Result<IcebergUpdateNamespacePropertiesResponse>;
    type IntoFuture = BoxFut<'static, Self::Output>;
    fn into_future(self) -> Self::IntoFuture {
        let client = self.client;
        let request = self.request;
        Box::pin(async move { client.update_iceberg_namespace_properties(&request).await })
    }
}

/// Builder for iceberg tables
pub struct ListIcebergTablesBuilder {
    client: IcebergClient,
    request: ListIcebergTablesRequest,
}

impl ListIcebergTablesBuilder {
    /// Create a new builder instance.
    /// Obtain via the corresponding method on `IcebergClient`.
    pub(crate) fn new(
        client: IcebergClient,
        catalog: impl Into<String>,
        namespace: impl Into<String>,
    ) -> Self {
        let request = ListIcebergTablesRequest {
            catalog: catalog.into(),
            namespace: namespace.into(),
            ..Default::default()
        };
        Self { client, request }
    }

    /// Opaque token of the page to return: the `next-page-token` of the page before. An empty token
    /// asks for the first page. Without a token the answer is the whole listing.
    pub fn with_page_token(mut self, page_token: impl Into<Option<String>>) -> Self {
        self.request.page_token = page_token.into();
        self
    }

    /// The maximum number of tables in a page, at least 1; a larger value than 100 counts as 100.
    /// Read only with a page token.
    pub fn with_page_size(mut self, page_size: impl Into<Option<i32>>) -> Self {
        self.request.page_size = page_size.into();
        self
    }
}

impl IntoFuture for ListIcebergTablesBuilder {
    type Output = Result<IcebergListTablesResponse>;
    type IntoFuture = BoxFut<'static, Self::Output>;
    fn into_future(self) -> Self::IntoFuture {
        let client = self.client;
        let request = self.request;
        Box::pin(async move { client.list_iceberg_tables(&request).await })
    }
}

/// Builder for iceberg table
pub struct CreateIcebergTableBuilder {
    client: IcebergClient,
    request: IcebergCreateTableRequest,
}

impl CreateIcebergTableBuilder {
    /// Create a new builder instance.
    /// Obtain via the corresponding method on `IcebergClient`.
    pub(crate) fn new(
        client: IcebergClient,
        catalog: impl Into<String>,
        namespace: impl Into<String>,
        name: impl Into<String>,
        schema: IcebergSchema,
    ) -> Self {
        let request = IcebergCreateTableRequest {
            catalog: catalog.into(),
            namespace: namespace.into(),
            name: name.into(),
            schema: buffa::MessageField::some(schema),
            ..Default::default()
        };
        Self { client, request }
    }

    /// Location of the table. With a location the table is EXTERNAL; without one it is MANAGED, at
    /// a location that the server picks.
    pub fn with_location(mut self, location: impl Into<Option<String>>) -> Self {
        self.request.location = location.into();
        self
    }

    /// Partition spec of the table. Unpartitioned when absent.
    pub fn with_partition_spec(
        mut self,
        partition_spec: impl Into<Option<IcebergPartitionSpec>>,
    ) -> Self {
        self.request.partition_spec = {
            let partition_spec: ::core::option::Option<_> = partition_spec.into();
            buffa::MessageField::from(partition_spec)
        };
        self
    }

    /// Sort order of the table. Unsorted when absent.
    pub fn with_write_order(mut self, write_order: impl Into<Option<IcebergSortOrder>>) -> Self {
        self.request.write_order = {
            let write_order: ::core::option::Option<_> = write_order.into();
            buffa::MessageField::from(write_order)
        };
        self
    }

    /// Return the metadata of the table without creating it. A commit with an `assert-create`
    /// requirement creates it.
    pub fn with_stage_create(mut self, stage_create: impl Into<Option<bool>>) -> Self {
        self.request.stage_create = stage_create.into();
        self
    }

    /// Properties of the table. `format-version` picks the format version (2 by default) and is
    /// not kept.
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
}

impl IntoFuture for CreateIcebergTableBuilder {
    type Output = Result<IcebergLoadTableResponse>;
    type IntoFuture = BoxFut<'static, Self::Output>;
    fn into_future(self) -> Self::IntoFuture {
        let client = self.client;
        let request = self.request;
        Box::pin(async move { client.create_iceberg_table(&request).await })
    }
}

/// Builder for iceberg table
pub struct LoadIcebergTableBuilder {
    client: IcebergClient,
    request: LoadIcebergTableRequest,
}

impl LoadIcebergTableBuilder {
    /// Create a new builder instance.
    /// Obtain via the corresponding method on `IcebergClient`.
    pub(crate) fn new(
        client: IcebergClient,
        catalog: impl Into<String>,
        namespace: impl Into<String>,
        table: impl Into<String>,
    ) -> Self {
        let request = LoadIcebergTableRequest {
            catalog: catalog.into(),
            namespace: namespace.into(),
            table: table.into(),
            ..Default::default()
        };
        Self { client, request }
    }

    /// The snapshots to return: `all` (the default) or `refs`, the snapshots that a branch or tag
    /// points at.
    pub fn with_snapshots(mut self, snapshots: impl Into<Option<String>>) -> Self {
        self.request.snapshots = snapshots.into();
        self
    }
}

impl IntoFuture for LoadIcebergTableBuilder {
    type Output = Result<IcebergLoadTableResponse>;
    type IntoFuture = BoxFut<'static, Self::Output>;
    fn into_future(self) -> Self::IntoFuture {
        let client = self.client;
        let request = self.request;
        Box::pin(async move { client.load_iceberg_table(&request).await })
    }
}

/// Builder for table exists
pub struct IcebergTableExistsBuilder {
    client: IcebergClient,
    request: IcebergTableExistsRequest,
}

impl IcebergTableExistsBuilder {
    /// Create a new builder instance.
    /// Obtain via the corresponding method on `IcebergClient`.
    pub(crate) fn new(
        client: IcebergClient,
        catalog: impl Into<String>,
        namespace: impl Into<String>,
        table: impl Into<String>,
    ) -> Self {
        let request = IcebergTableExistsRequest {
            catalog: catalog.into(),
            namespace: namespace.into(),
            table: table.into(),
            ..Default::default()
        };
        Self { client, request }
    }
}

impl IntoFuture for IcebergTableExistsBuilder {
    type Output = Result<bool>;
    type IntoFuture = BoxFut<'static, Self::Output>;
    fn into_future(self) -> Self::IntoFuture {
        let client = self.client;
        let request = self.request;
        Box::pin(async move { client.iceberg_table_exists(&request).await })
    }
}

/// Builder for iceberg table
pub struct CommitIcebergTableBuilder {
    client: IcebergClient,
    request: IcebergCommitTableRequest,
}

impl CommitIcebergTableBuilder {
    /// Create a new builder instance.
    /// Obtain via the corresponding method on `IcebergClient`.
    pub(crate) fn new(
        client: IcebergClient,
        catalog: impl Into<String>,
        namespace: impl Into<String>,
        table: impl Into<String>,
    ) -> Self {
        let request = IcebergCommitTableRequest {
            catalog: catalog.into(),
            namespace: namespace.into(),
            table: table.into(),
            ..Default::default()
        };
        Self { client, request }
    }

    /// The table. The server does not read it: the path names the table.
    pub fn with_identifier(
        mut self,
        identifier: impl Into<Option<IcebergTableIdentifier>>,
    ) -> Self {
        self.request.identifier = {
            let identifier: ::core::option::Option<_> = identifier.into();
            buffa::MessageField::from(identifier)
        };
        self
    }

    /// Conditions that the table must meet for the commit to apply. With `assert-create`, the
    /// commit creates a staged table.
    pub fn with_requirements<I>(mut self, requirements: I) -> Self
    where
        I: IntoIterator<Item = IcebergTableRequirement>,
    {
        self.request.requirements = requirements.into_iter().collect();
        self
    }

    /// Changes to the metadata, applied in order.
    pub fn with_updates<I>(mut self, updates: I) -> Self
    where
        I: IntoIterator<Item = IcebergTableUpdate>,
    {
        self.request.updates = updates.into_iter().collect();
        self
    }
}

impl IntoFuture for CommitIcebergTableBuilder {
    type Output = Result<IcebergLoadTableResponse>;
    type IntoFuture = BoxFut<'static, Self::Output>;
    fn into_future(self) -> Self::IntoFuture {
        let client = self.client;
        let request = self.request;
        Box::pin(async move { client.commit_iceberg_table(&request).await })
    }
}

/// Builder for iceberg table
pub struct DropIcebergTableBuilder {
    client: IcebergClient,
    request: DropIcebergTableRequest,
}

impl DropIcebergTableBuilder {
    /// Create a new builder instance.
    /// Obtain via the corresponding method on `IcebergClient`.
    pub(crate) fn new(
        client: IcebergClient,
        catalog: impl Into<String>,
        namespace: impl Into<String>,
        table: impl Into<String>,
    ) -> Self {
        let request = DropIcebergTableRequest {
            catalog: catalog.into(),
            namespace: namespace.into(),
            table: table.into(),
            ..Default::default()
        };
        Self { client, request }
    }

    /// Whether to delete the data too. Checked, but without effect: the files of a MANAGED table
    /// are always removed, those of an EXTERNAL table never.
    pub fn with_purge_requested(mut self, purge_requested: impl Into<Option<bool>>) -> Self {
        self.request.purge_requested = purge_requested.into();
        self
    }
}

impl IntoFuture for DropIcebergTableBuilder {
    type Output = Result<()>;
    type IntoFuture = BoxFut<'static, Self::Output>;
    fn into_future(self) -> Self::IntoFuture {
        let client = self.client;
        let request = self.request;
        Box::pin(async move { client.drop_iceberg_table(&request).await })
    }
}

/// Builder for iceberg table credentials
pub struct LoadIcebergTableCredentialsBuilder {
    client: IcebergClient,
    request: LoadIcebergTableCredentialsRequest,
}

impl LoadIcebergTableCredentialsBuilder {
    /// Create a new builder instance.
    /// Obtain via the corresponding method on `IcebergClient`.
    pub(crate) fn new(
        client: IcebergClient,
        catalog: impl Into<String>,
        namespace: impl Into<String>,
        table: impl Into<String>,
    ) -> Self {
        let request = LoadIcebergTableCredentialsRequest {
            catalog: catalog.into(),
            namespace: namespace.into(),
            table: table.into(),
            ..Default::default()
        };
        Self { client, request }
    }
}

impl IntoFuture for LoadIcebergTableCredentialsBuilder {
    type Output = Result<IcebergLoadCredentialsResponse>;
    type IntoFuture = BoxFut<'static, Self::Output>;
    fn into_future(self) -> Self::IntoFuture {
        let client = self.client;
        let request = self.request;
        Box::pin(async move { client.load_iceberg_table_credentials(&request).await })
    }
}

/// Builder for iceberg metrics
pub struct ReportIcebergMetricsBuilder {
    client: IcebergClient,
    request: ReportIcebergMetricsRequest,
}

impl ReportIcebergMetricsBuilder {
    /// Create a new builder instance.
    /// Obtain via the corresponding method on `IcebergClient`.
    pub(crate) fn new(
        client: IcebergClient,
        catalog: impl Into<String>,
        namespace: impl Into<String>,
        table: impl Into<String>,
        report_type: impl Into<String>,
    ) -> Self {
        let request = ReportIcebergMetricsRequest {
            catalog: catalog.into(),
            namespace: namespace.into(),
            table: table.into(),
            report_type: report_type.into(),
            ..Default::default()
        };
        Self { client, request }
    }

    /// Full name of the table.
    pub fn with_table_name(mut self, table_name: impl Into<Option<String>>) -> Self {
        self.request.table_name = table_name.into();
        self
    }

    /// Id of the snapshot that was scanned or committed.
    pub fn with_snapshot_id(mut self, snapshot_id: impl Into<Option<i64>>) -> Self {
        self.request.snapshot_id = snapshot_id.into();
        self
    }

    /// `scan-report`: the filter of the scan, an Iceberg expression (`true`, `false`, or an
    /// expression object).
    pub fn with_filter(
        mut self,
        filter: impl Into<Option<::buffa_types::google::protobuf::Value>>,
    ) -> Self {
        self.request.filter = {
            let filter: ::core::option::Option<_> = filter.into();
            buffa::MessageField::from(filter)
        };
        self
    }

    /// `scan-report`: id of the schema of the scan.
    pub fn with_schema_id(mut self, schema_id: impl Into<Option<i32>>) -> Self {
        self.request.schema_id = schema_id.into();
        self
    }

    /// `scan-report`: ids of the projected fields. Always sent.
    pub fn with_projected_field_ids<I>(mut self, projected_field_ids: I) -> Self
    where
        I: IntoIterator<Item = i32>,
    {
        self.request.projected_field_ids = projected_field_ids.into_iter().collect();
        self
    }

    /// `scan-report`: names of the projected fields. Always sent.
    pub fn with_projected_field_names<I>(mut self, projected_field_names: I) -> Self
    where
        I: IntoIterator<Item = String>,
    {
        self.request.projected_field_names = projected_field_names.into_iter().collect();
        self
    }

    /// `commit-report`: sequence number of the commit.
    pub fn with_sequence_number(mut self, sequence_number: impl Into<Option<i64>>) -> Self {
        self.request.sequence_number = sequence_number.into();
        self
    }

    /// `commit-report`: the operation of the commit, such as `append`.
    pub fn with_operation(mut self, operation: impl Into<Option<String>>) -> Self {
        self.request.operation = operation.into();
        self
    }

    /// The metrics, by name: counters (`unit`, `value`) and timers (`time-unit`, `count`,
    /// `total-duration`).
    pub fn with_metrics(
        mut self,
        metrics: impl Into<Option<::buffa_types::google::protobuf::Struct>>,
    ) -> Self {
        self.request.metrics = {
            let metrics: ::core::option::Option<_> = metrics.into();
            buffa::MessageField::from(metrics)
        };
        self
    }

    /// Other properties of the report.
    pub fn with_metadata<I, K, V>(mut self, metadata: I) -> Self
    where
        I: IntoIterator<Item = (K, V)>,
        K: Into<String>,
        V: Into<String>,
    {
        self.request.metadata = metadata
            .into_iter()
            .map(|(k, v)| (k.into(), v.into()))
            .collect();
        self
    }
}

impl IntoFuture for ReportIcebergMetricsBuilder {
    type Output = Result<()>;
    type IntoFuture = BoxFut<'static, Self::Output>;
    fn into_future(self) -> Self::IntoFuture {
        let client = self.client;
        let request = self.request;
        Box::pin(async move { client.report_iceberg_metrics(&request).await })
    }
}

/// Builder for iceberg view
pub struct LoadIcebergViewBuilder {
    client: IcebergClient,
    request: LoadIcebergViewRequest,
}

impl LoadIcebergViewBuilder {
    /// Create a new builder instance.
    /// Obtain via the corresponding method on `IcebergClient`.
    pub(crate) fn new(
        client: IcebergClient,
        catalog: impl Into<String>,
        namespace: impl Into<String>,
        view: impl Into<String>,
    ) -> Self {
        let request = LoadIcebergViewRequest {
            catalog: catalog.into(),
            namespace: namespace.into(),
            view: view.into(),
            ..Default::default()
        };
        Self { client, request }
    }
}

impl IntoFuture for LoadIcebergViewBuilder {
    type Output = Result<()>;
    type IntoFuture = BoxFut<'static, Self::Output>;
    fn into_future(self) -> Self::IntoFuture {
        let client = self.client;
        let request = self.request;
        Box::pin(async move { client.load_iceberg_view(&request).await })
    }
}

/// Builder for iceberg table
pub struct RenameIcebergTableBuilder {
    client: IcebergClient,
    request: IcebergRenameTableRequest,
}

impl RenameIcebergTableBuilder {
    /// Create a new builder instance.
    /// Obtain via the corresponding method on `IcebergClient`.
    pub(crate) fn new(
        client: IcebergClient,
        catalog: impl Into<String>,
        source: IcebergTableIdentifier,
        destination: IcebergTableIdentifier,
    ) -> Self {
        let request = IcebergRenameTableRequest {
            catalog: catalog.into(),
            source: buffa::MessageField::some(source),
            destination: buffa::MessageField::some(destination),
            ..Default::default()
        };
        Self { client, request }
    }
}

impl IntoFuture for RenameIcebergTableBuilder {
    type Output = Result<()>;
    type IntoFuture = BoxFut<'static, Self::Output>;
    fn into_future(self) -> Self::IntoFuture {
        let client = self.client;
        let request = self.request;
        Box::pin(async move { client.rename_iceberg_table(&request).await })
    }
}
