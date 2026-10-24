// @generated — do not edit by hand.
#![allow(unused_imports)]

use crate::codegen::admin::*;
use crate::codegen::audit::*;
use crate::codegen::auth::*;
use crate::codegen::catalogs::*;
use crate::codegen::credentials::*;
use crate::codegen::delta_api::*;
use crate::codegen::delta_commits::*;
use crate::codegen::external_locations::*;
use crate::codegen::functions::*;
use crate::codegen::health::*;
use crate::codegen::iceberg::*;
use crate::codegen::metastore::*;
use crate::codegen::model_versions::*;
use crate::codegen::permissions::*;
use crate::codegen::recipients::*;
use crate::codegen::registered_models::*;
use crate::codegen::schemas::*;
use crate::codegen::scim::*;
use crate::codegen::shares::*;
use crate::codegen::sharing::*;
use crate::codegen::staging_tables::*;
use crate::codegen::tables::*;
use crate::codegen::temporary_credentials::*;
use crate::codegen::tokens::*;
use crate::codegen::transport::HttpClient;
use crate::codegen::volumes::*;
use uc_client::models::admin::v1::*;
use uc_client::models::audit::v1::*;
use uc_client::models::auth::v1::*;
use uc_client::models::catalogs::v1::*;
use uc_client::models::credentials::v1::*;
use uc_client::models::delta::v1::*;
use uc_client::models::external_locations::v1::*;
use uc_client::models::functions::v1::*;
use uc_client::models::health::v1::*;
use uc_client::models::iceberg::v1::*;
use uc_client::models::metastore::v1::*;
use uc_client::models::models::v1::*;
use uc_client::models::permissions::v1::*;
use uc_client::models::recipients::v1::*;
use uc_client::models::schemas::v1::*;
use uc_client::models::scim::v1::*;
use uc_client::models::shares::v1::*;
use uc_client::models::sharing::v1::*;
use uc_client::models::tables::v1::*;
use uc_client::models::tokens::v1::*;
use uc_client::models::volumes::v1::*;
use url::Url;

#[derive(Clone)]
pub struct Client {
    pub(crate) client: HttpClient,
    pub(crate) base_url: Url,
}

impl Client {
    /// Create a new aggregate client from a cloud client and base URL.
    ///
    /// Per-service clients are constructed on demand (they only hold a cheaply-cloneable
    /// `CloudClient` + `Url`), so nothing is allocated per service here.
    pub fn new(client: HttpClient, mut base_url: Url) -> Self {
        if !base_url.path().ends_with('/') {
            base_url.set_path(&format!("{}/", base_url.path()));
        }
        Self { client, base_url }
    }

    /// Create a new aggregate client with no authentication.
    pub fn new_unauthenticated(base_url: Url) -> Self {
        Self::new(HttpClient::default(), base_url)
    }

    /// Create a new aggregate client authenticating with a bearer token.
    pub fn new_with_token(base_url: Url, token: impl ToString) -> Self {
        Self::new(
            HttpClient::default().with_token(token.to_string()),
            base_url,
        )
    }

    ///Low-level `admin` client exposing request/response passthrough methods.
    pub fn admin_client(&self) -> crate::codegen::admin::AdminClient {
        crate::codegen::admin::AdminClient::new(self.client.clone(), self.base_url.clone())
    }

    ///Low-level `audit` client exposing request/response passthrough methods.
    pub fn audit_client(&self) -> crate::codegen::audit::AuditClient {
        crate::codegen::audit::AuditClient::new(self.client.clone(), self.base_url.clone())
    }

    ///Low-level `auth` client exposing request/response passthrough methods.
    pub fn auth_client(&self) -> crate::codegen::auth::AuthClient {
        crate::codegen::auth::AuthClient::new(self.client.clone(), self.base_url.clone())
    }

    ///Low-level `catalogs` client exposing request/response passthrough methods.
    pub fn catalogs_client(&self) -> crate::codegen::catalogs::CatalogServiceClient {
        crate::codegen::catalogs::CatalogServiceClient::new(
            self.client.clone(),
            self.base_url.clone(),
        )
    }

    ///Low-level `credentials` client exposing request/response passthrough methods.
    pub fn credentials_client(&self) -> crate::codegen::credentials::CredentialServiceClient {
        crate::codegen::credentials::CredentialServiceClient::new(
            self.client.clone(),
            self.base_url.clone(),
        )
    }

    ///Low-level `delta_api` client exposing request/response passthrough methods.
    pub fn delta_api_client(&self) -> crate::codegen::delta_api::DeltaApiClient {
        crate::codegen::delta_api::DeltaApiClient::new(self.client.clone(), self.base_url.clone())
    }

    ///Low-level `delta_commits` client exposing request/response passthrough methods.
    pub fn delta_commits_client(&self) -> crate::codegen::delta_commits::DeltaCommitClient {
        crate::codegen::delta_commits::DeltaCommitClient::new(
            self.client.clone(),
            self.base_url.clone(),
        )
    }

    ///Low-level `external_locations` client exposing request/response passthrough methods.
    pub fn external_locations_client(
        &self,
    ) -> crate::codegen::external_locations::ExternalLocationServiceClient {
        crate::codegen::external_locations::ExternalLocationServiceClient::new(
            self.client.clone(),
            self.base_url.clone(),
        )
    }

    ///Low-level `functions` client exposing request/response passthrough methods.
    pub fn functions_client(&self) -> crate::codegen::functions::FunctionServiceClient {
        crate::codegen::functions::FunctionServiceClient::new(
            self.client.clone(),
            self.base_url.clone(),
        )
    }

    ///Low-level `health` client exposing request/response passthrough methods.
    pub fn health_client(&self) -> crate::codegen::health::HealthClient {
        crate::codegen::health::HealthClient::new(self.client.clone(), self.base_url.clone())
    }

    ///Low-level `iceberg` client exposing request/response passthrough methods.
    pub fn iceberg_client(&self) -> crate::codegen::iceberg::IcebergClient {
        crate::codegen::iceberg::IcebergClient::new(self.client.clone(), self.base_url.clone())
    }

    ///Low-level `metastore` client exposing request/response passthrough methods.
    pub fn metastore_client(&self) -> crate::codegen::metastore::MetastoreClient {
        crate::codegen::metastore::MetastoreClient::new(self.client.clone(), self.base_url.clone())
    }

    ///Low-level `model_versions` client exposing request/response passthrough methods.
    pub fn model_versions_client(
        &self,
    ) -> crate::codegen::model_versions::ModelVersionServiceClient {
        crate::codegen::model_versions::ModelVersionServiceClient::new(
            self.client.clone(),
            self.base_url.clone(),
        )
    }

    ///Low-level `permissions` client exposing request/response passthrough methods.
    pub fn permissions_client(&self) -> crate::codegen::permissions::PermissionClient {
        crate::codegen::permissions::PermissionClient::new(
            self.client.clone(),
            self.base_url.clone(),
        )
    }

    ///Low-level `recipients` client exposing request/response passthrough methods.
    pub fn recipients_client(&self) -> crate::codegen::recipients::RecipientServiceClient {
        crate::codegen::recipients::RecipientServiceClient::new(
            self.client.clone(),
            self.base_url.clone(),
        )
    }

    ///Low-level `registered_models` client exposing request/response passthrough methods.
    pub fn registered_models_client(
        &self,
    ) -> crate::codegen::registered_models::RegisteredModelServiceClient {
        crate::codegen::registered_models::RegisteredModelServiceClient::new(
            self.client.clone(),
            self.base_url.clone(),
        )
    }

    ///Low-level `schemas` client exposing request/response passthrough methods.
    pub fn schemas_client(&self) -> crate::codegen::schemas::SchemaServiceClient {
        crate::codegen::schemas::SchemaServiceClient::new(
            self.client.clone(),
            self.base_url.clone(),
        )
    }

    ///Low-level `scim` client exposing request/response passthrough methods.
    pub fn scim_client(&self) -> crate::codegen::scim::ScimClient {
        crate::codegen::scim::ScimClient::new(self.client.clone(), self.base_url.clone())
    }

    ///Low-level `shares` client exposing request/response passthrough methods.
    pub fn shares_client(&self) -> crate::codegen::shares::ShareServiceClient {
        crate::codegen::shares::ShareServiceClient::new(self.client.clone(), self.base_url.clone())
    }

    ///Low-level `sharing` client exposing request/response passthrough methods.
    pub fn sharing_client(&self) -> crate::codegen::sharing::SharingClient {
        crate::codegen::sharing::SharingClient::new(self.client.clone(), self.base_url.clone())
    }

    ///Low-level `staging_tables` client exposing request/response passthrough methods.
    pub fn staging_tables_client(&self) -> crate::codegen::staging_tables::StagingTableClient {
        crate::codegen::staging_tables::StagingTableClient::new(
            self.client.clone(),
            self.base_url.clone(),
        )
    }

    ///Low-level `tables` client exposing request/response passthrough methods.
    pub fn tables_client(&self) -> crate::codegen::tables::TableServiceClient {
        crate::codegen::tables::TableServiceClient::new(self.client.clone(), self.base_url.clone())
    }

    ///Low-level `temporary_credentials` client exposing request/response passthrough methods.
    pub fn temporary_credentials_client(
        &self,
    ) -> crate::codegen::temporary_credentials::TemporaryCredentialClient {
        crate::codegen::temporary_credentials::TemporaryCredentialClient::new(
            self.client.clone(),
            self.base_url.clone(),
        )
    }

    ///Low-level `tokens` client exposing request/response passthrough methods.
    pub fn tokens_client(&self) -> crate::codegen::tokens::TokenServiceClient {
        crate::codegen::tokens::TokenServiceClient::new(self.client.clone(), self.base_url.clone())
    }

    ///Low-level `volumes` client exposing request/response passthrough methods.
    pub fn volumes_client(&self) -> crate::codegen::volumes::VolumeServiceClient {
        crate::codegen::volumes::VolumeServiceClient::new(
            self.client.clone(),
            self.base_url.clone(),
        )
    }

    /// Get a snapshot of the whole catalog, in one answer.
    pub fn get_backup(&self) -> GetBackupBuilder {
        GetBackupBuilder::new(crate::codegen::admin::AdminClient::new(
            self.client.clone(),
            self.base_url.clone(),
        ))
    }

    /// Replace the whole catalog with a snapshot, in one transaction: every table of the store is
    /// emptied, then filled with the rows of the snapshot.
    ///
    /// # Arguments
    ///
    /// * `version` - Version of the snapshot format. A restore takes only version 1.
    /// * `tables` - The rows of each table, by table name. A row is a JSON object of its columns, as the table
    /// of the store has them.
    pub fn restore_backup(
        &self,
        version: i64,
        tables: impl IntoIterator<
            Item = (
                impl Into<String>,
                ::buffa_types::google::protobuf::ListValue,
            ),
        >,
    ) -> RestoreBackupBuilder {
        RestoreBackupBuilder::new(
            crate::codegen::admin::AdminClient::new(self.client.clone(), self.base_url.clone()),
            version,
            tables,
        )
    }

    /// List the audit events, newest first.
    pub fn list_audit_events(&self) -> ListAuditEventsBuilder {
        ListAuditEventsBuilder::new(crate::codegen::audit::AuditClient::new(
            self.client.clone(),
            self.base_url.clone(),
        ))
    }

    /// Exchange a token of an identity provider for a Unity Catalog access token (OAuth 2.0 token
    /// exchange, RFC 8693). It needs no sign-in.
    ///
    /// # Arguments
    ///
    /// * `grant_type` - The grant type: `urn:ietf:params:oauth:grant-type:token-exchange`.
    /// * `requested_token_type` - The type of the token to issue: `urn:ietf:params:oauth:token-type:access_token`.
    /// * `subject_token` - The token of the identity provider that names the user, such as an ID token.
    /// * `subject_token_type` - The type of `subject_token`, such as `urn:ietf:params:oauth:token-type:id_token`.
    pub fn exchange_token(
        &self,
        grant_type: impl Into<String>,
        requested_token_type: impl Into<String>,
        subject_token: impl Into<String>,
        subject_token_type: impl Into<String>,
    ) -> ExchangeTokenBuilder {
        ExchangeTokenBuilder::new(
            crate::codegen::auth::AuthClient::new(self.client.clone(), self.base_url.clone()),
            grant_type,
            requested_token_type,
            subject_token,
            subject_token_type,
        )
    }

    /// Get an access token for a personal access token: the OAuth 2.0 `client_credentials` grant
    /// (RFC 6749, section 4.4). It needs no sign-in.
    ///
    /// # Arguments
    ///
    /// * `grant_type` - The grant type: `client_credentials`.
    /// * `client_secret` - The personal access token.
    pub fn request_access_token(
        &self,
        grant_type: impl Into<String>,
        client_secret: impl Into<String>,
    ) -> RequestAccessTokenBuilder {
        RequestAccessTokenBuilder::new(
            crate::codegen::auth::AuthClient::new(self.client.clone(), self.base_url.clone()),
            grant_type,
            client_secret,
        )
    }

    /// Sign out: expire the `UC_TOKEN` cookie of the request, if any.
    pub fn logout(&self) -> LogoutBuilder {
        LogoutBuilder::new(crate::codegen::auth::AuthClient::new(
            self.client.clone(),
            self.base_url.clone(),
        ))
    }

    /// Get how the web UI signs in. It needs no sign-in.
    pub fn get_auth_mode(&self) -> GetAuthModeBuilder {
        GetAuthModeBuilder::new(crate::codegen::auth::AuthClient::new(
            self.client.clone(),
            self.base_url.clone(),
        ))
    }

    /// List the catalogs that the caller can see.
    pub fn list_catalogs(&self) -> ListCatalogsBuilder {
        ListCatalogsBuilder::new(crate::codegen::catalogs::CatalogServiceClient::new(
            self.client.clone(),
            self.base_url.clone(),
        ))
    }

    /// Create a catalog.
    ///
    /// # Arguments
    ///
    /// * `name` - Name of the catalog.
    pub fn create_catalog(&self, name: impl Into<String>) -> CreateCatalogBuilder {
        CreateCatalogBuilder::new(
            crate::codegen::catalogs::CatalogServiceClient::new(
                self.client.clone(),
                self.base_url.clone(),
            ),
            name,
        )
    }

    /// Access the `catalog` resource scoped to the given name.
    pub fn catalog(&self, catalog_name: impl Into<String>) -> CatalogClient {
        CatalogClient::new(
            catalog_name,
            crate::codegen::catalogs::CatalogServiceClient::new(
                self.client.clone(),
                self.base_url.clone(),
            ),
        )
    }

    /// List the credentials that the caller can see.
    pub fn list_credentials(&self) -> ListCredentialsBuilder {
        ListCredentialsBuilder::new(crate::codegen::credentials::CredentialServiceClient::new(
            self.client.clone(),
            self.base_url.clone(),
        ))
    }

    /// Create a credential.
    ///
    /// # Arguments
    ///
    /// * `name` - Name of the credential.
    /// * `aws_iam_role` - The AWS IAM role of the credential: an AWS IAM role is the only kind of credential.
    pub fn create_credential(
        &self,
        name: impl Into<String>,
        aws_iam_role: AwsIamRoleRequest,
    ) -> CreateCredentialBuilder {
        CreateCredentialBuilder::new(
            crate::codegen::credentials::CredentialServiceClient::new(
                self.client.clone(),
                self.base_url.clone(),
            ),
            name,
            aws_iam_role,
        )
    }

    /// Access the `credential` resource scoped to the given name.
    pub fn credential(&self, credential_name: impl Into<String>) -> CredentialClient {
        CredentialClient::new(
            credential_name,
            crate::codegen::credentials::CredentialServiceClient::new(
                self.client.clone(),
                self.base_url.clone(),
            ),
        )
    }

    /// Get the configuration of the Delta API for a catalog: the endpoints that the server supports
    /// and the protocol version.
    ///
    /// # Arguments
    ///
    /// * `catalog` - Name of the catalog.
    pub fn get_delta_config(&self, catalog: impl Into<String>) -> GetDeltaConfigBuilder {
        GetDeltaConfigBuilder::new(
            crate::codegen::delta_api::DeltaApiClient::new(
                self.client.clone(),
                self.base_url.clone(),
            ),
            catalog,
        )
    }

    /// Create a staging table: reserve a table id and a storage location for a new MANAGED table,
    /// with credentials to write its first commit.
    ///
    /// # Arguments
    ///
    /// * `catalog` - Name of the parent catalog.
    /// * `schema` - Name of the parent schema.
    /// * `name` - Name of the table.
    pub fn create_delta_staging_table(
        &self,
        catalog: impl Into<String>,
        schema: impl Into<String>,
        name: impl Into<String>,
    ) -> CreateDeltaStagingTableBuilder {
        CreateDeltaStagingTableBuilder::new(
            crate::codegen::delta_api::DeltaApiClient::new(
                self.client.clone(),
                self.base_url.clone(),
            ),
            catalog,
            schema,
            name,
        )
    }

    /// Create a table. A MANAGED table takes over the staging table at its location.
    ///
    /// # Arguments
    ///
    /// * `catalog` - Name of the parent catalog.
    /// * `schema` - Name of the parent schema.
    /// * `name` - Name of the table.
    /// * `location` - Storage location of the table. For a MANAGED table, the location of its staging table.
    /// * `table_type` - Type of the table.
    /// * `columns` - The columns of the table. They must hold at least one field.
    /// * `protocol` - The Delta protocol of the table.
    /// * `last_commit_timestamp_ms` - Timestamp of version 0, the commit that the client wrote before the call, in epoch
    /// milliseconds.
    pub fn create_delta_table(
        &self,
        catalog: impl Into<String>,
        schema: impl Into<String>,
        name: impl Into<String>,
        location: impl Into<String>,
        table_type: DeltaTableType,
        columns: DeltaStructType,
        protocol: DeltaProtocol,
        last_commit_timestamp_ms: i64,
    ) -> CreateDeltaTableBuilder {
        CreateDeltaTableBuilder::new(
            crate::codegen::delta_api::DeltaApiClient::new(
                self.client.clone(),
                self.base_url.clone(),
            ),
            catalog,
            schema,
            name,
            location,
            table_type,
            columns,
            protocol,
            last_commit_timestamp_ms,
        )
    }

    /// Load a table: its metadata and, for a MANAGED table, its commits that are not backfilled yet.
    /// It holds no credentials.
    ///
    /// # Arguments
    ///
    /// * `catalog` - Name of the parent catalog.
    /// * `schema` - Name of the parent schema.
    /// * `table` - Name of the table.
    pub fn load_delta_table(
        &self,
        catalog: impl Into<String>,
        schema: impl Into<String>,
        table: impl Into<String>,
    ) -> LoadDeltaTableBuilder {
        LoadDeltaTableBuilder::new(
            crate::codegen::delta_api::DeltaApiClient::new(
                self.client.clone(),
                self.base_url.clone(),
            ),
            catalog,
            schema,
            table,
        )
    }

    /// Update a table or commit to it, then load it. A replay of an accepted `add-commit` is a no-op.
    /// A version or requirement conflict is status 409; too many commits that are not backfilled
    /// is status 429.
    ///
    /// # Arguments
    ///
    /// * `catalog` - Name of the parent catalog.
    /// * `schema` - Name of the parent schema.
    /// * `table` - Name of the table.
    /// * `requirements` - Preconditions of the update. An `assert-table-uuid` requirement is always required; at most
    /// one of each type.
    /// * `updates` - The updates to apply. At least one; at most one of each action.
    pub fn update_delta_table(
        &self,
        catalog: impl Into<String>,
        schema: impl Into<String>,
        table: impl Into<String>,
        requirements: Vec<DeltaTableRequirement>,
        updates: Vec<DeltaTableUpdate>,
    ) -> UpdateDeltaTableBuilder {
        UpdateDeltaTableBuilder::new(
            crate::codegen::delta_api::DeltaApiClient::new(
                self.client.clone(),
                self.base_url.clone(),
            ),
            catalog,
            schema,
            table,
            requirements,
            updates,
        )
    }

    /// Check that a table exists. A missing table is status 404.
    ///
    /// # Arguments
    ///
    /// * `catalog` - Name of the parent catalog.
    /// * `schema` - Name of the parent schema.
    /// * `table` - Name of the table.
    pub fn delta_table_exists(
        &self,
        catalog: impl Into<String>,
        schema: impl Into<String>,
        table: impl Into<String>,
    ) -> DeltaTableExistsBuilder {
        DeltaTableExistsBuilder::new(
            crate::codegen::delta_api::DeltaApiClient::new(
                self.client.clone(),
                self.base_url.clone(),
            ),
            catalog,
            schema,
            table,
        )
    }

    /// Delete a table.
    ///
    /// # Arguments
    ///
    /// * `catalog` - Name of the parent catalog.
    /// * `schema` - Name of the parent schema.
    /// * `table` - Name of the table.
    pub fn delete_delta_table(
        &self,
        catalog: impl Into<String>,
        schema: impl Into<String>,
        table: impl Into<String>,
    ) -> DeleteDeltaTableBuilder {
        DeleteDeltaTableBuilder::new(
            crate::codegen::delta_api::DeltaApiClient::new(
                self.client.clone(),
                self.base_url.clone(),
            ),
            catalog,
            schema,
            table,
        )
    }

    /// Rename a table in its schema.
    ///
    /// # Arguments
    ///
    /// * `catalog` - Name of the parent catalog.
    /// * `schema` - Name of the parent schema.
    /// * `table` - Name of the table.
    /// * `new_name` - New name of the table, in the same schema.
    pub fn rename_delta_table(
        &self,
        catalog: impl Into<String>,
        schema: impl Into<String>,
        table: impl Into<String>,
        new_name: impl Into<String>,
    ) -> RenameDeltaTableBuilder {
        RenameDeltaTableBuilder::new(
            crate::codegen::delta_api::DeltaApiClient::new(
                self.client.clone(),
                self.base_url.clone(),
            ),
            catalog,
            schema,
            table,
            new_name,
        )
    }

    /// Report the metrics of a commit. The server checks the table id and does not store the
    /// metrics.
    ///
    /// # Arguments
    ///
    /// * `catalog` - Name of the parent catalog.
    /// * `schema` - Name of the parent schema.
    /// * `table` - Name of the table.
    /// * `table_id` - Unique identifier of the table. It must be the id of the table in the path.
    pub fn report_delta_table_metrics(
        &self,
        catalog: impl Into<String>,
        schema: impl Into<String>,
        table: impl Into<String>,
        table_id: impl Into<String>,
    ) -> ReportDeltaTableMetricsBuilder {
        ReportDeltaTableMetricsBuilder::new(
            crate::codegen::delta_api::DeltaApiClient::new(
                self.client.clone(),
                self.base_url.clone(),
            ),
            catalog,
            schema,
            table,
            table_id,
        )
    }

    /// Get temporary credentials for the data of a table.
    ///
    /// # Arguments
    ///
    /// * `catalog` - Name of the parent catalog.
    /// * `schema` - Name of the parent schema.
    /// * `table` - Name of the table.
    /// * `operation` - The access level of the credentials.
    pub fn get_delta_table_credentials(
        &self,
        catalog: impl Into<String>,
        schema: impl Into<String>,
        table: impl Into<String>,
        operation: DeltaCredentialOperation,
    ) -> GetDeltaTableCredentialsBuilder {
        GetDeltaTableCredentialsBuilder::new(
            crate::codegen::delta_api::DeltaApiClient::new(
                self.client.clone(),
                self.base_url.clone(),
            ),
            catalog,
            schema,
            table,
            operation,
        )
    }

    /// Get temporary credentials for the data of a staging table. They are always `READ_WRITE`, for
    /// the first commit. Only the owner of the staging table gets them.
    ///
    /// # Arguments
    ///
    /// * `table_id` - Unique identifier of the staging table.
    pub fn get_delta_staging_table_credentials(
        &self,
        table_id: impl Into<String>,
    ) -> GetDeltaStagingTableCredentialsBuilder {
        GetDeltaStagingTableCredentialsBuilder::new(
            crate::codegen::delta_api::DeltaApiClient::new(
                self.client.clone(),
                self.base_url.clone(),
            ),
            table_id,
        )
    }

    /// Ratify a staged commit, report backfilled commits, or both. Another commit at the same
    /// version is a conflict (status 409).
    ///
    /// # Arguments
    ///
    /// * `table_id` - Unique identifier of the table to commit to.
    /// * `table_uri` - Storage location of the table. It must be the location of the table.
    pub fn commit_delta(
        &self,
        table_id: impl Into<String>,
        table_uri: impl Into<String>,
    ) -> CommitDeltaBuilder {
        CommitDeltaBuilder::new(
            crate::codegen::delta_commits::DeltaCommitClient::new(
                self.client.clone(),
                self.base_url.clone(),
            ),
            table_id,
            table_uri,
        )
    }

    /// Get the commits of a table that are not backfilled yet. The server takes the request as the
    /// JSON body of a `GET`.
    ///
    /// # Arguments
    ///
    /// * `table_id` - Unique identifier of the table.
    /// * `start_version` - First version to return (inclusive). It must be 0 or more.
    pub fn get_delta_commits(
        &self,
        table_id: impl Into<String>,
        start_version: i64,
    ) -> GetDeltaCommitsBuilder {
        GetDeltaCommitsBuilder::new(
            crate::codegen::delta_commits::DeltaCommitClient::new(
                self.client.clone(),
                self.base_url.clone(),
            ),
            table_id,
            start_version,
        )
    }

    /// List the external locations that the caller can see.
    pub fn list_external_locations(&self) -> ListExternalLocationsBuilder {
        ListExternalLocationsBuilder::new(
            crate::codegen::external_locations::ExternalLocationServiceClient::new(
                self.client.clone(),
                self.base_url.clone(),
            ),
        )
    }

    /// Create an external location.
    ///
    /// # Arguments
    ///
    /// * `name` - Name of the external location.
    /// * `url` - URL of the storage path, such as `s3://bucket/path`.
    /// * `credential_name` - Name of the credential that gives access to the path.
    pub fn create_external_location(
        &self,
        name: impl Into<String>,
        url: impl Into<String>,
        credential_name: impl Into<String>,
    ) -> CreateExternalLocationBuilder {
        CreateExternalLocationBuilder::new(
            crate::codegen::external_locations::ExternalLocationServiceClient::new(
                self.client.clone(),
                self.base_url.clone(),
            ),
            name,
            url,
            credential_name,
        )
    }

    /// Access the `external_location` resource scoped to the given name.
    pub fn external_location(
        &self,
        external_location_name: impl Into<String>,
    ) -> ExternalLocationClient {
        ExternalLocationClient::new(
            external_location_name,
            crate::codegen::external_locations::ExternalLocationServiceClient::new(
                self.client.clone(),
                self.base_url.clone(),
            ),
        )
    }

    /// List the functions of a schema that the caller can see.
    ///
    /// # Arguments
    ///
    /// * `catalog_name` - Name of the parent catalog.
    /// * `schema_name` - Name of the parent schema.
    pub fn list_functions(
        &self,
        catalog_name: impl Into<String>,
        schema_name: impl Into<String>,
    ) -> ListFunctionsBuilder {
        ListFunctionsBuilder::new(
            crate::codegen::functions::FunctionServiceClient::new(
                self.client.clone(),
                self.base_url.clone(),
            ),
            catalog_name,
            schema_name,
        )
    }

    /// Create a function.
    ///
    /// # Arguments
    ///
    /// * `function_info` - The function to create.
    pub fn create_function(&self, function_info: CreateFunction) -> CreateFunctionBuilder {
        CreateFunctionBuilder::new(
            crate::codegen::functions::FunctionServiceClient::new(
                self.client.clone(),
                self.base_url.clone(),
            ),
            function_info,
        )
    }

    /// Access the `function` resource scoped to the given name.
    pub fn function(&self, function_name: impl Into<String>) -> FunctionClient {
        FunctionClient::new(
            function_name,
            crate::codegen::functions::FunctionServiceClient::new(
                self.client.clone(),
                self.base_url.clone(),
            ),
        )
    }

    /// Check that the server answers. The database is not asked.
    pub fn get_health(&self) -> GetHealthBuilder {
        GetHealthBuilder::new(crate::codegen::health::HealthClient::new(
            self.client.clone(),
            self.base_url.clone(),
        ))
    }

    /// Get the version of the server.
    pub fn get_version(&self) -> GetVersionBuilder {
        GetVersionBuilder::new(crate::codegen::health::HealthClient::new(
            self.client.clone(),
            self.base_url.clone(),
        ))
    }

    /// Get the client configuration of a catalog.
    ///
    /// # Arguments
    ///
    /// * `warehouse` - Name of the catalog: the Iceberg warehouse.
    pub fn get_iceberg_config(&self, warehouse: impl Into<String>) -> GetIcebergConfigBuilder {
        GetIcebergConfigBuilder::new(
            crate::codegen::iceberg::IcebergClient::new(self.client.clone(), self.base_url.clone()),
            warehouse,
        )
    }

    /// List the namespaces of a catalog: its schemas that the caller can read.
    ///
    /// # Arguments
    ///
    /// * `catalog` - Name of the catalog.
    pub fn list_iceberg_namespaces(
        &self,
        catalog: impl Into<String>,
    ) -> ListIcebergNamespacesBuilder {
        ListIcebergNamespacesBuilder::new(
            crate::codegen::iceberg::IcebergClient::new(self.client.clone(), self.base_url.clone()),
            catalog,
        )
    }

    /// Create a namespace: a schema of the catalog.
    ///
    /// # Arguments
    ///
    /// * `catalog` - Name of the catalog.
    /// * `namespace` - The levels of the namespace. Exactly one level: the name of the schema to create.
    pub fn create_iceberg_namespace(
        &self,
        catalog: impl Into<String>,
        namespace: Vec<String>,
    ) -> CreateIcebergNamespaceBuilder {
        CreateIcebergNamespaceBuilder::new(
            crate::codegen::iceberg::IcebergClient::new(self.client.clone(), self.base_url.clone()),
            catalog,
            namespace,
        )
    }

    /// Get a namespace and its properties.
    ///
    /// # Arguments
    ///
    /// * `catalog` - Name of the catalog.
    /// * `namespace` - Name of the namespace: a schema of the catalog. Namespaces have one level; the server does
    /// not split the value at the unit separator (`%1F`) that Iceberg puts between levels.
    pub fn load_iceberg_namespace(
        &self,
        catalog: impl Into<String>,
        namespace: impl Into<String>,
    ) -> LoadIcebergNamespaceBuilder {
        LoadIcebergNamespaceBuilder::new(
            crate::codegen::iceberg::IcebergClient::new(self.client.clone(), self.base_url.clone()),
            catalog,
            namespace,
        )
    }

    /// Check that a namespace exists. A namespace that does not exist is a not-found error.
    ///
    /// # Arguments
    ///
    /// * `catalog` - Name of the catalog.
    /// * `namespace` - Name of the namespace: a schema of the catalog. Namespaces have one level; the server does
    /// not split the value at the unit separator (`%1F`) that Iceberg puts between levels.
    pub fn iceberg_namespace_exists(
        &self,
        catalog: impl Into<String>,
        namespace: impl Into<String>,
    ) -> IcebergNamespaceExistsBuilder {
        IcebergNamespaceExistsBuilder::new(
            crate::codegen::iceberg::IcebergClient::new(self.client.clone(), self.base_url.clone()),
            catalog,
            namespace,
        )
    }

    /// Drop a namespace. It must be empty.
    ///
    /// # Arguments
    ///
    /// * `catalog` - Name of the catalog.
    /// * `namespace` - Name of the namespace: a schema of the catalog. Namespaces have one level; the server does
    /// not split the value at the unit separator (`%1F`) that Iceberg puts between levels.
    pub fn drop_iceberg_namespace(
        &self,
        catalog: impl Into<String>,
        namespace: impl Into<String>,
    ) -> DropIcebergNamespaceBuilder {
        DropIcebergNamespaceBuilder::new(
            crate::codegen::iceberg::IcebergClient::new(self.client.clone(), self.base_url.clone()),
            catalog,
            namespace,
        )
    }

    /// Set and remove properties of a namespace.
    ///
    /// # Arguments
    ///
    /// * `catalog` - Name of the catalog.
    /// * `namespace` - Name of the namespace: a schema of the catalog. Namespaces have one level; the server does
    /// not split the value at the unit separator (`%1F`) that Iceberg puts between levels.
    pub fn update_iceberg_namespace_properties(
        &self,
        catalog: impl Into<String>,
        namespace: impl Into<String>,
    ) -> UpdateIcebergNamespacePropertiesBuilder {
        UpdateIcebergNamespacePropertiesBuilder::new(
            crate::codegen::iceberg::IcebergClient::new(self.client.clone(), self.base_url.clone()),
            catalog,
            namespace,
        )
    }

    /// List the tables of a namespace that have Iceberg metadata and that the caller can read.
    ///
    /// # Arguments
    ///
    /// * `catalog` - Name of the catalog.
    /// * `namespace` - Name of the namespace: a schema of the catalog. Namespaces have one level; the server does
    /// not split the value at the unit separator (`%1F`) that Iceberg puts between levels.
    pub fn list_iceberg_tables(
        &self,
        catalog: impl Into<String>,
        namespace: impl Into<String>,
    ) -> ListIcebergTablesBuilder {
        ListIcebergTablesBuilder::new(
            crate::codegen::iceberg::IcebergClient::new(self.client.clone(), self.base_url.clone()),
            catalog,
            namespace,
        )
    }

    /// Create a native Iceberg table, or stage its creation.
    ///
    /// # Arguments
    ///
    /// * `catalog` - Name of the catalog.
    /// * `namespace` - Name of the namespace: a schema of the catalog. Namespaces have one level; the server does
    /// not split the value at the unit separator (`%1F`) that Iceberg puts between levels.
    /// * `name` - Name of the table.
    /// * `schema` - Schema of the table. The server assigns fresh ids.
    pub fn create_iceberg_table(
        &self,
        catalog: impl Into<String>,
        namespace: impl Into<String>,
        name: impl Into<String>,
        schema: IcebergSchema,
    ) -> CreateIcebergTableBuilder {
        CreateIcebergTableBuilder::new(
            crate::codegen::iceberg::IcebergClient::new(self.client.clone(), self.base_url.clone()),
            catalog,
            namespace,
            name,
            schema,
        )
    }

    /// Get the metadata of a table, with credentials for its files.
    ///
    /// # Arguments
    ///
    /// * `catalog` - Name of the catalog.
    /// * `namespace` - Name of the namespace: a schema of the catalog. Namespaces have one level; the server does
    /// not split the value at the unit separator (`%1F`) that Iceberg puts between levels.
    /// * `table` - Name of the table.
    pub fn load_iceberg_table(
        &self,
        catalog: impl Into<String>,
        namespace: impl Into<String>,
        table: impl Into<String>,
    ) -> LoadIcebergTableBuilder {
        LoadIcebergTableBuilder::new(
            crate::codegen::iceberg::IcebergClient::new(self.client.clone(), self.base_url.clone()),
            catalog,
            namespace,
            table,
        )
    }

    /// Check that a table exists. A table that does not exist is a not-found error.
    ///
    /// # Arguments
    ///
    /// * `catalog` - Name of the catalog.
    /// * `namespace` - Name of the namespace: a schema of the catalog. Namespaces have one level; the server does
    /// not split the value at the unit separator (`%1F`) that Iceberg puts between levels.
    /// * `table` - Name of the table.
    pub fn iceberg_table_exists(
        &self,
        catalog: impl Into<String>,
        namespace: impl Into<String>,
        table: impl Into<String>,
    ) -> IcebergTableExistsBuilder {
        IcebergTableExistsBuilder::new(
            crate::codegen::iceberg::IcebergClient::new(self.client.clone(), self.base_url.clone()),
            catalog,
            namespace,
            table,
        )
    }

    /// Commit changes to a native Iceberg table, or create a staged table.
    ///
    /// # Arguments
    ///
    /// * `catalog` - Name of the catalog.
    /// * `namespace` - Name of the namespace: a schema of the catalog. Namespaces have one level; the server does
    /// not split the value at the unit separator (`%1F`) that Iceberg puts between levels.
    /// * `table` - Name of the table.
    pub fn commit_iceberg_table(
        &self,
        catalog: impl Into<String>,
        namespace: impl Into<String>,
        table: impl Into<String>,
    ) -> CommitIcebergTableBuilder {
        CommitIcebergTableBuilder::new(
            crate::codegen::iceberg::IcebergClient::new(self.client.clone(), self.base_url.clone()),
            catalog,
            namespace,
            table,
        )
    }

    /// Drop a native Iceberg table. UniForm tables are dropped through the Unity Catalog API.
    ///
    /// # Arguments
    ///
    /// * `catalog` - Name of the catalog.
    /// * `namespace` - Name of the namespace: a schema of the catalog. Namespaces have one level; the server does
    /// not split the value at the unit separator (`%1F`) that Iceberg puts between levels.
    /// * `table` - Name of the table.
    pub fn drop_iceberg_table(
        &self,
        catalog: impl Into<String>,
        namespace: impl Into<String>,
        table: impl Into<String>,
    ) -> DropIcebergTableBuilder {
        DropIcebergTableBuilder::new(
            crate::codegen::iceberg::IcebergClient::new(self.client.clone(), self.base_url.clone()),
            catalog,
            namespace,
            table,
        )
    }

    /// Get fresh credentials for the files of a table.
    ///
    /// # Arguments
    ///
    /// * `catalog` - Name of the catalog.
    /// * `namespace` - Name of the namespace: a schema of the catalog. Namespaces have one level; the server does
    /// not split the value at the unit separator (`%1F`) that Iceberg puts between levels.
    /// * `table` - Name of the table.
    pub fn load_iceberg_table_credentials(
        &self,
        catalog: impl Into<String>,
        namespace: impl Into<String>,
        table: impl Into<String>,
    ) -> LoadIcebergTableCredentialsBuilder {
        LoadIcebergTableCredentialsBuilder::new(
            crate::codegen::iceberg::IcebergClient::new(self.client.clone(), self.base_url.clone()),
            catalog,
            namespace,
            table,
        )
    }

    /// Send a scan or commit report of a table. The server checks it and does not keep it.
    ///
    /// # Arguments
    ///
    /// * `catalog` - Name of the catalog.
    /// * `namespace` - Name of the namespace: a schema of the catalog. Namespaces have one level; the server does
    /// not split the value at the unit separator (`%1F`) that Iceberg puts between levels.
    /// * `table` - Name of the table.
    /// * `report_type` - The type of report: `scan-report` or `commit-report`.
    pub fn report_iceberg_metrics(
        &self,
        catalog: impl Into<String>,
        namespace: impl Into<String>,
        table: impl Into<String>,
        report_type: impl Into<String>,
    ) -> ReportIcebergMetricsBuilder {
        ReportIcebergMetricsBuilder::new(
            crate::codegen::iceberg::IcebergClient::new(self.client.clone(), self.base_url.clone()),
            catalog,
            namespace,
            table,
            report_type,
        )
    }

    /// Get a view. Views are not served: the answer is always a not-found error.
    ///
    /// # Arguments
    ///
    /// * `catalog` - Name of the catalog.
    /// * `namespace` - Name of the namespace: a schema of the catalog. Namespaces have one level; the server does
    /// not split the value at the unit separator (`%1F`) that Iceberg puts between levels.
    /// * `view` - Name of the view.
    pub fn load_iceberg_view(
        &self,
        catalog: impl Into<String>,
        namespace: impl Into<String>,
        view: impl Into<String>,
    ) -> LoadIcebergViewBuilder {
        LoadIcebergViewBuilder::new(
            crate::codegen::iceberg::IcebergClient::new(self.client.clone(), self.base_url.clone()),
            catalog,
            namespace,
            view,
        )
    }

    /// Rename a native Iceberg table, in its namespace. A move to another namespace is not
    /// supported.
    ///
    /// # Arguments
    ///
    /// * `catalog` - Name of the catalog.
    /// * `source` - The table to rename.
    /// * `destination` - The new name of the table, in the same namespace.
    pub fn rename_iceberg_table(
        &self,
        catalog: impl Into<String>,
        source: IcebergTableIdentifier,
        destination: IcebergTableIdentifier,
    ) -> RenameIcebergTableBuilder {
        RenameIcebergTableBuilder::new(
            crate::codegen::iceberg::IcebergClient::new(self.client.clone(), self.base_url.clone()),
            catalog,
            source,
            destination,
        )
    }

    /// Get a summary of the metastore.
    pub fn get_metastore_summary(&self) -> GetMetastoreSummaryBuilder {
        GetMetastoreSummaryBuilder::new(crate::codegen::metastore::MetastoreClient::new(
            self.client.clone(),
            self.base_url.clone(),
        ))
    }

    /// List the versions of a registered model.
    ///
    /// # Arguments
    ///
    /// * `full_name` - Three-level name of the registered model: `catalog.schema.model`.
    pub fn list_model_versions(&self, full_name: impl Into<String>) -> ListModelVersionsBuilder {
        ListModelVersionsBuilder::new(
            crate::codegen::model_versions::ModelVersionServiceClient::new(
                self.client.clone(),
                self.base_url.clone(),
            ),
            full_name,
        )
    }

    /// Create a model version with the status PENDING_REGISTRATION. The server gives it the next
    /// version number and a storage location.
    ///
    /// # Arguments
    ///
    /// * `model_name` - Name of the parent registered model, relative to its schema.
    /// * `catalog_name` - Name of the parent catalog.
    /// * `schema_name` - Name of the parent schema.
    /// * `source` - URI of the source files the model version is created from, such as an MLflow run artifact
    /// path.
    pub fn create_model_version(
        &self,
        model_name: impl Into<String>,
        catalog_name: impl Into<String>,
        schema_name: impl Into<String>,
        source: impl Into<String>,
    ) -> CreateModelVersionBuilder {
        CreateModelVersionBuilder::new(
            crate::codegen::model_versions::ModelVersionServiceClient::new(
                self.client.clone(),
                self.base_url.clone(),
            ),
            model_name,
            catalog_name,
            schema_name,
            source,
        )
    }

    /// Get a model version.
    ///
    /// # Arguments
    ///
    /// * `full_name` - Three-level name of the registered model: `catalog.schema.model`.
    /// * `version` - Number of the model version.
    pub fn get_model_version(
        &self,
        full_name: impl Into<String>,
        version: i64,
    ) -> GetModelVersionBuilder {
        GetModelVersionBuilder::new(
            crate::codegen::model_versions::ModelVersionServiceClient::new(
                self.client.clone(),
                self.base_url.clone(),
            ),
            full_name,
            version,
        )
    }

    /// Change a model version.
    ///
    /// # Arguments
    ///
    /// * `full_name` - Three-level name of the registered model: `catalog.schema.model`.
    /// * `version` - Number of the model version.
    /// * `comment` - User-provided free-form text description. The comment is the only field that can change; an
    /// empty one clears it.
    pub fn update_model_version(
        &self,
        full_name: impl Into<String>,
        version: i64,
        comment: impl Into<String>,
    ) -> UpdateModelVersionBuilder {
        UpdateModelVersionBuilder::new(
            crate::codegen::model_versions::ModelVersionServiceClient::new(
                self.client.clone(),
                self.base_url.clone(),
            ),
            full_name,
            version,
            comment,
        )
    }

    /// Delete a model version.
    ///
    /// # Arguments
    ///
    /// * `full_name` - Three-level name of the registered model: `catalog.schema.model`.
    /// * `version` - Number of the model version.
    pub fn delete_model_version(
        &self,
        full_name: impl Into<String>,
        version: i64,
    ) -> DeleteModelVersionBuilder {
        DeleteModelVersionBuilder::new(
            crate::codegen::model_versions::ModelVersionServiceClient::new(
                self.client.clone(),
                self.base_url.clone(),
            ),
            full_name,
            version,
        )
    }

    /// Finalize a model version: its status changes from PENDING_REGISTRATION to READY after the
    /// client wrote all its files.
    ///
    /// # Arguments
    ///
    /// * `full_name` - Three-level name of the registered model: `catalog.schema.model`.
    /// * `version` - Number of the model version.
    pub fn finalize_model_version(
        &self,
        full_name: impl Into<String>,
        version: i64,
    ) -> FinalizeModelVersionBuilder {
        FinalizeModelVersionBuilder::new(
            crate::codegen::model_versions::ModelVersionServiceClient::new(
                self.client.clone(),
                self.base_url.clone(),
            ),
            full_name,
            version,
        )
    }

    /// Get the privileges on a securable.
    ///
    /// # Arguments
    ///
    /// * `securable_type` - Type of the securable. Values: `metastore`, `catalog`, `schema`, `table`, `volume`,
    /// `function`, `registered_model`, `external_location`, `credential`.
    /// * `full_name` - Full name of the securable, such as `catalog.schema.table`.
    pub fn get_permissions(
        &self,
        securable_type: impl Into<String>,
        full_name: impl Into<String>,
    ) -> GetPermissionsBuilder {
        GetPermissionsBuilder::new(
            crate::codegen::permissions::PermissionClient::new(
                self.client.clone(),
                self.base_url.clone(),
            ),
            securable_type,
            full_name,
        )
    }

    /// Grant and revoke privileges on a securable. The response lists the privileges of the
    /// principals of the request.
    ///
    /// # Arguments
    ///
    /// * `securable_type` - Type of the securable. Values: `metastore`, `catalog`, `schema`, `table`, `volume`,
    /// `function`, `registered_model`, `external_location`, `credential`.
    /// * `full_name` - Full name of the securable, such as `catalog.schema.table`.
    /// * `changes` - The changes to make, one per principal.
    pub fn update_permissions(
        &self,
        securable_type: impl Into<String>,
        full_name: impl Into<String>,
        changes: Vec<PermissionsChange>,
    ) -> UpdatePermissionsBuilder {
        UpdatePermissionsBuilder::new(
            crate::codegen::permissions::PermissionClient::new(
                self.client.clone(),
                self.base_url.clone(),
            ),
            securable_type,
            full_name,
            changes,
        )
    }

    /// List the recipients.
    pub fn list_recipients(&self) -> ListRecipientsBuilder {
        ListRecipientsBuilder::new(crate::codegen::recipients::RecipientServiceClient::new(
            self.client.clone(),
            self.base_url.clone(),
        ))
    }

    /// Create a recipient. The response holds its bearer token, which no later response shows.
    ///
    /// # Arguments
    ///
    /// * `name` - Name of the recipient.
    pub fn create_recipient(&self, name: impl Into<String>) -> CreateRecipientBuilder {
        CreateRecipientBuilder::new(
            crate::codegen::recipients::RecipientServiceClient::new(
                self.client.clone(),
                self.base_url.clone(),
            ),
            name,
        )
    }

    /// Access the `recipient` resource scoped to the given name.
    pub fn recipient(&self, recipient_name: impl Into<String>) -> RecipientClient {
        RecipientClient::new(
            recipient_name,
            crate::codegen::recipients::RecipientServiceClient::new(
                self.client.clone(),
                self.base_url.clone(),
            ),
        )
    }

    /// List the registered models that the caller can see, of a catalog or schema when given.
    pub fn list_registered_models(&self) -> ListRegisteredModelsBuilder {
        ListRegisteredModelsBuilder::new(
            crate::codegen::registered_models::RegisteredModelServiceClient::new(
                self.client.clone(),
                self.base_url.clone(),
            ),
        )
    }

    /// Create a registered model.
    ///
    /// # Arguments
    ///
    /// * `name` - Name of the registered model, relative to its schema.
    /// * `catalog_name` - Name of the parent catalog.
    /// * `schema_name` - Name of the parent schema.
    pub fn create_registered_model(
        &self,
        name: impl Into<String>,
        catalog_name: impl Into<String>,
        schema_name: impl Into<String>,
    ) -> CreateRegisteredModelBuilder {
        CreateRegisteredModelBuilder::new(
            crate::codegen::registered_models::RegisteredModelServiceClient::new(
                self.client.clone(),
                self.base_url.clone(),
            ),
            name,
            catalog_name,
            schema_name,
        )
    }

    /// Access the `registered_model` resource scoped to the given name.
    pub fn registered_model(
        &self,
        registered_model_name: impl Into<String>,
    ) -> RegisteredModelClient {
        RegisteredModelClient::new(
            registered_model_name,
            crate::codegen::registered_models::RegisteredModelServiceClient::new(
                self.client.clone(),
                self.base_url.clone(),
            ),
        )
    }

    /// List the schemas of a catalog that the caller can see.
    ///
    /// # Arguments
    ///
    /// * `catalog_name` - Name of the parent catalog.
    pub fn list_schemas(&self, catalog_name: impl Into<String>) -> ListSchemasBuilder {
        ListSchemasBuilder::new(
            crate::codegen::schemas::SchemaServiceClient::new(
                self.client.clone(),
                self.base_url.clone(),
            ),
            catalog_name,
        )
    }

    /// Create a schema.
    ///
    /// # Arguments
    ///
    /// * `name` - Name of the schema, relative to the parent catalog.
    /// * `catalog_name` - Name of the parent catalog.
    pub fn create_schema(
        &self,
        name: impl Into<String>,
        catalog_name: impl Into<String>,
    ) -> CreateSchemaBuilder {
        CreateSchemaBuilder::new(
            crate::codegen::schemas::SchemaServiceClient::new(
                self.client.clone(),
                self.base_url.clone(),
            ),
            name,
            catalog_name,
        )
    }

    /// Access the `schema` resource scoped to the given name.
    pub fn schema(&self, schema_name: impl Into<String>) -> SchemaClient {
        SchemaClient::new(
            schema_name,
            crate::codegen::schemas::SchemaServiceClient::new(
                self.client.clone(),
                self.base_url.clone(),
            ),
        )
    }

    /// List the users. Only the metastore admin can.
    pub fn list_users(&self) -> ListUsersBuilder {
        ListUsersBuilder::new(crate::codegen::scim::ScimClient::new(
            self.client.clone(),
            self.base_url.clone(),
        ))
    }

    /// Create a user, named by its primary email. Only the metastore admin can.
    pub fn create_user(&self) -> CreateUserBuilder {
        CreateUserBuilder::new(crate::codegen::scim::ScimClient::new(
            self.client.clone(),
            self.base_url.clone(),
        ))
    }

    /// Get a user. Only the metastore admin can.
    ///
    /// # Arguments
    ///
    /// * `id` - Unique identifier of the user.
    pub fn get_user(&self, id: impl Into<String>) -> GetUserBuilder {
        GetUserBuilder::new(
            crate::codegen::scim::ScimClient::new(self.client.clone(), self.base_url.clone()),
            id,
        )
    }

    /// Replace a user: the name, state and external id change when given; the email and picture
    /// stay. Only the metastore admin can.
    ///
    /// # Arguments
    ///
    /// * `id` - Unique identifier of the user.
    pub fn replace_user(&self, id: impl Into<String>) -> ReplaceUserBuilder {
        ReplaceUserBuilder::new(
            crate::codegen::scim::ScimClient::new(self.client.clone(), self.base_url.clone()),
            id,
        )
    }

    /// Enable or disable a user. The server takes the first `replace` whose `path` is `active` or
    /// absent; without `path`, the first attribute of `value` is the state (`{"active": false}`).
    /// With no such operation the server answers 501. Only the metastore admin can.
    ///
    /// # Arguments
    ///
    /// * `id` - Unique identifier of the user or group.
    /// * `operations` - The operations, in order.
    pub fn patch_user(
        &self,
        id: impl Into<String>,
        operations: Vec<ScimPatchOperation>,
    ) -> PatchUserBuilder {
        PatchUserBuilder::new(
            crate::codegen::scim::ScimClient::new(self.client.clone(), self.base_url.clone()),
            id,
            operations,
        )
    }

    /// Disable a user and remove its grants. The user stays, disabled. Only the metastore admin can.
    ///
    /// # Arguments
    ///
    /// * `id` - Unique identifier of the user.
    pub fn delete_user(&self, id: impl Into<String>) -> DeleteUserBuilder {
        DeleteUserBuilder::new(
            crate::codegen::scim::ScimClient::new(self.client.clone(), self.base_url.clone()),
            id,
        )
    }

    /// Get the signed-in user. The role `admin` names a caller that can do what the metastore admin
    /// does.
    pub fn get_me(&self) -> GetMeBuilder {
        GetMeBuilder::new(crate::codegen::scim::ScimClient::new(
            self.client.clone(),
            self.base_url.clone(),
        ))
    }

    /// List the groups. Only the metastore admin can.
    pub fn list_groups(&self) -> ListGroupsBuilder {
        ListGroupsBuilder::new(crate::codegen::scim::ScimClient::new(
            self.client.clone(),
            self.base_url.clone(),
        ))
    }

    /// Create a group. Only the metastore admin can.
    ///
    /// # Arguments
    ///
    /// * `display_name` - Name of the group.
    pub fn create_group(&self, display_name: impl Into<String>) -> CreateGroupBuilder {
        CreateGroupBuilder::new(
            crate::codegen::scim::ScimClient::new(self.client.clone(), self.base_url.clone()),
            display_name,
        )
    }

    /// Get a group. Only the metastore admin can.
    ///
    /// # Arguments
    ///
    /// * `id` - Unique identifier of the group.
    pub fn get_group(&self, id: impl Into<String>) -> GetGroupBuilder {
        GetGroupBuilder::new(
            crate::codegen::scim::ScimClient::new(self.client.clone(), self.base_url.clone()),
            id,
        )
    }

    /// Replace a group: its name and members. Only the metastore admin can.
    ///
    /// # Arguments
    ///
    /// * `id` - Unique identifier of the group. The server assigns it.
    /// * `display_name` - Name of the group.
    pub fn replace_group(
        &self,
        id: impl Into<String>,
        display_name: impl Into<String>,
    ) -> ReplaceGroupBuilder {
        ReplaceGroupBuilder::new(
            crate::codegen::scim::ScimClient::new(self.client.clone(), self.base_url.clone()),
            id,
            display_name,
        )
    }

    /// Change a group: `add` to `members`, `remove` of `members` or `members[value eq "<id>"]`,
    /// `replace` of `displayName`, or `replace` without `path` with a whole group. Other operations
    /// are ignored. Only the metastore admin can.
    ///
    /// # Arguments
    ///
    /// * `id` - Unique identifier of the user or group.
    /// * `operations` - The operations, in order.
    pub fn patch_group(
        &self,
        id: impl Into<String>,
        operations: Vec<ScimPatchOperation>,
    ) -> PatchGroupBuilder {
        PatchGroupBuilder::new(
            crate::codegen::scim::ScimClient::new(self.client.clone(), self.base_url.clone()),
            id,
            operations,
        )
    }

    /// Delete a group. Only the metastore admin can.
    ///
    /// # Arguments
    ///
    /// * `id` - Unique identifier of the group.
    pub fn delete_group(&self, id: impl Into<String>) -> DeleteGroupBuilder {
        DeleteGroupBuilder::new(
            crate::codegen::scim::ScimClient::new(self.client.clone(), self.base_url.clone()),
            id,
        )
    }

    /// Run operations on users and groups in one request. Each operation has its own status. Only
    /// the metastore admin can.
    pub fn scim_bulk(&self) -> ScimBulkBuilder {
        ScimBulkBuilder::new(crate::codegen::scim::ScimClient::new(
            self.client.clone(),
            self.base_url.clone(),
        ))
    }

    /// List the SCIM schemas of the server: User and Group. Any signed-in caller can.
    pub fn list_scim_schemas(&self) -> ListScimSchemasBuilder {
        ListScimSchemasBuilder::new(crate::codegen::scim::ScimClient::new(
            self.client.clone(),
            self.base_url.clone(),
        ))
    }

    /// List the SCIM resource types of the server: User and Group. Any signed-in caller can.
    pub fn list_scim_resource_types(&self) -> ListScimResourceTypesBuilder {
        ListScimResourceTypesBuilder::new(crate::codegen::scim::ScimClient::new(
            self.client.clone(),
            self.base_url.clone(),
        ))
    }

    /// Get the SCIM features that the server supports. Any signed-in caller can.
    pub fn get_service_provider_config(&self) -> GetServiceProviderConfigBuilder {
        GetServiceProviderConfigBuilder::new(crate::codegen::scim::ScimClient::new(
            self.client.clone(),
            self.base_url.clone(),
        ))
    }

    /// List the shares.
    pub fn list_shares(&self) -> ListSharesBuilder {
        ListSharesBuilder::new(crate::codegen::shares::ShareServiceClient::new(
            self.client.clone(),
            self.base_url.clone(),
        ))
    }

    /// Create a share. It holds no tables: add them with `UpdateShare`.
    ///
    /// # Arguments
    ///
    /// * `name` - Name of the share.
    pub fn create_share(&self, name: impl Into<String>) -> CreateShareBuilder {
        CreateShareBuilder::new(
            crate::codegen::shares::ShareServiceClient::new(
                self.client.clone(),
                self.base_url.clone(),
            ),
            name,
        )
    }

    /// Access the `share` resource scoped to the given name.
    pub fn share(&self, share_name: impl Into<String>) -> ShareClient {
        ShareClient::new(
            share_name,
            crate::codegen::shares::ShareServiceClient::new(
                self.client.clone(),
                self.base_url.clone(),
            ),
        )
    }

    /// List the shares that the recipient can read.
    pub fn sharing_list_shares(&self) -> SharingListSharesBuilder {
        SharingListSharesBuilder::new(crate::codegen::sharing::SharingClient::new(
            self.client.clone(),
            self.base_url.clone(),
        ))
    }

    /// List the schemas of a share.
    ///
    /// # Arguments
    ///
    /// * `share` - Name of the share.
    pub fn sharing_list_schemas(&self, share: impl Into<String>) -> SharingListSchemasBuilder {
        SharingListSchemasBuilder::new(
            crate::codegen::sharing::SharingClient::new(self.client.clone(), self.base_url.clone()),
            share,
        )
    }

    /// List the tables of a schema of a share.
    ///
    /// # Arguments
    ///
    /// * `share` - Name of the share.
    /// * `schema` - Name of the schema.
    pub fn sharing_list_tables(
        &self,
        share: impl Into<String>,
        schema: impl Into<String>,
    ) -> SharingListTablesBuilder {
        SharingListTablesBuilder::new(
            crate::codegen::sharing::SharingClient::new(self.client.clone(), self.base_url.clone()),
            share,
            schema,
        )
    }

    /// List the tables of all schemas of a share.
    ///
    /// # Arguments
    ///
    /// * `share` - Name of the share.
    pub fn sharing_list_all_tables(&self, share: impl Into<String>) -> SharingListAllTablesBuilder {
        SharingListAllTablesBuilder::new(
            crate::codegen::sharing::SharingClient::new(self.client.clone(), self.base_url.clone()),
            share,
        )
    }

    /// Get the version of a table.
    ///
    /// # Arguments
    ///
    /// * `share` - Name of the share.
    /// * `schema` - Name of the schema.
    /// * `table` - Name of the table.
    pub fn sharing_get_table_version(
        &self,
        share: impl Into<String>,
        schema: impl Into<String>,
        table: impl Into<String>,
    ) -> SharingGetTableVersionBuilder {
        SharingGetTableVersionBuilder::new(
            crate::codegen::sharing::SharingClient::new(self.client.clone(), self.base_url.clone()),
            share,
            schema,
            table,
        )
    }

    /// Get the protocol and the metadata of a table. The response also has the version of the table
    /// in the `Delta-Table-Version` header (always 0).
    ///
    /// # Arguments
    ///
    /// * `share` - Name of the share.
    /// * `schema` - Name of the schema.
    /// * `table` - Name of the table.
    pub fn sharing_get_table_metadata(
        &self,
        share: impl Into<String>,
        schema: impl Into<String>,
        table: impl Into<String>,
    ) -> SharingGetTableMetadataBuilder {
        SharingGetTableMetadataBuilder::new(
            crate::codegen::sharing::SharingClient::new(self.client.clone(), self.base_url.clone()),
            share,
            schema,
            table,
        )
    }

    /// Read a table. The server does not read the Delta log: the answer is that of
    /// `SharingGetTableMetadata`, with no data files. The server ignores the request body.
    ///
    /// # Arguments
    ///
    /// * `share` - Name of the share.
    /// * `schema` - Name of the schema.
    /// * `table` - Name of the table.
    pub fn sharing_query_table(
        &self,
        share: impl Into<String>,
        schema: impl Into<String>,
        table: impl Into<String>,
    ) -> SharingQueryTableBuilder {
        SharingQueryTableBuilder::new(
            crate::codegen::sharing::SharingClient::new(self.client.clone(), self.base_url.clone()),
            share,
            schema,
            table,
        )
    }

    /// Create a staging table: reserve a table id and a managed storage location for a MANAGED
    /// table.
    ///
    /// # Arguments
    ///
    /// * `name` - Name of the table, relative to the parent schema.
    /// * `catalog_name` - Name of the parent catalog.
    /// * `schema_name` - Name of the parent schema.
    pub fn create_staging_table(
        &self,
        name: impl Into<String>,
        catalog_name: impl Into<String>,
        schema_name: impl Into<String>,
    ) -> CreateStagingTableBuilder {
        CreateStagingTableBuilder::new(
            crate::codegen::staging_tables::StagingTableClient::new(
                self.client.clone(),
                self.base_url.clone(),
            ),
            name,
            catalog_name,
            schema_name,
        )
    }

    /// List the tables of a schema that the caller can see.
    ///
    /// # Arguments
    ///
    /// * `catalog_name` - Name of the parent catalog.
    /// * `schema_name` - Name of the parent schema.
    pub fn list_tables(
        &self,
        catalog_name: impl Into<String>,
        schema_name: impl Into<String>,
    ) -> ListTablesBuilder {
        ListTablesBuilder::new(
            crate::codegen::tables::TableServiceClient::new(
                self.client.clone(),
                self.base_url.clone(),
            ),
            catalog_name,
            schema_name,
        )
    }

    /// Create a table.
    ///
    /// # Arguments
    ///
    /// * `name` - Name of the table, relative to the parent schema.
    /// * `catalog_name` - Name of the parent catalog.
    /// * `schema_name` - Name of the parent schema.
    /// * `table_type` - Type of the table.
    pub fn create_table(
        &self,
        name: impl Into<String>,
        catalog_name: impl Into<String>,
        schema_name: impl Into<String>,
        table_type: TableType,
    ) -> CreateTableBuilder {
        CreateTableBuilder::new(
            crate::codegen::tables::TableServiceClient::new(
                self.client.clone(),
                self.base_url.clone(),
            ),
            name,
            catalog_name,
            schema_name,
            table_type,
        )
    }

    /// Access the `table` resource scoped to the given name.
    pub fn table(&self, table_name: impl Into<String>) -> TableClient {
        TableClient::new(
            table_name,
            crate::codegen::tables::TableServiceClient::new(
                self.client.clone(),
                self.base_url.clone(),
            ),
        )
    }

    /// Generate temporary credentials for the storage of a table.
    ///
    /// # Arguments
    ///
    /// * `table_id` - Unique identifier of the table.
    /// * `operation` - The operation on the table.
    pub fn generate_temporary_table_credentials(
        &self,
        table_id: impl Into<String>,
        operation: TableOperation,
    ) -> GenerateTemporaryTableCredentialsBuilder {
        GenerateTemporaryTableCredentialsBuilder::new(
            crate::codegen::temporary_credentials::TemporaryCredentialClient::new(
                self.client.clone(),
                self.base_url.clone(),
            ),
            table_id,
            operation,
        )
    }

    /// Generate temporary credentials for the storage of a volume.
    ///
    /// # Arguments
    ///
    /// * `volume_id` - Unique identifier of the volume.
    /// * `operation` - The operation on the volume.
    pub fn generate_temporary_volume_credentials(
        &self,
        volume_id: impl Into<String>,
        operation: VolumeOperation,
    ) -> GenerateTemporaryVolumeCredentialsBuilder {
        GenerateTemporaryVolumeCredentialsBuilder::new(
            crate::codegen::temporary_credentials::TemporaryCredentialClient::new(
                self.client.clone(),
                self.base_url.clone(),
            ),
            volume_id,
            operation,
        )
    }

    /// Generate temporary credentials for a storage path.
    ///
    /// # Arguments
    ///
    /// * `url` - URL of the storage path.
    /// * `operation` - The operation on the path.
    pub fn generate_temporary_path_credentials(
        &self,
        url: impl Into<String>,
        operation: PathOperation,
    ) -> GenerateTemporaryPathCredentialsBuilder {
        GenerateTemporaryPathCredentialsBuilder::new(
            crate::codegen::temporary_credentials::TemporaryCredentialClient::new(
                self.client.clone(),
                self.base_url.clone(),
            ),
            url,
            operation,
        )
    }

    /// Generate temporary credentials for the storage of a model version.
    ///
    /// # Arguments
    ///
    /// * `catalog_name` - Name of the parent catalog.
    /// * `schema_name` - Name of the parent schema.
    /// * `model_name` - Name of the registered model, relative to its schema.
    /// * `version` - Number of the model version.
    /// * `operation` - The operation on the model version.
    pub fn generate_temporary_model_version_credentials(
        &self,
        catalog_name: impl Into<String>,
        schema_name: impl Into<String>,
        model_name: impl Into<String>,
        version: i64,
        operation: ModelVersionOperation,
    ) -> GenerateTemporaryModelVersionCredentialsBuilder {
        GenerateTemporaryModelVersionCredentialsBuilder::new(
            crate::codegen::temporary_credentials::TemporaryCredentialClient::new(
                self.client.clone(),
                self.base_url.clone(),
            ),
            catalog_name,
            schema_name,
            model_name,
            version,
            operation,
        )
    }

    /// List the personal access tokens of the caller.
    pub fn list_tokens(&self) -> ListTokensBuilder {
        ListTokensBuilder::new(crate::codegen::tokens::TokenServiceClient::new(
            self.client.clone(),
            self.base_url.clone(),
        ))
    }

    /// Create a personal access token for the caller.
    ///
    /// # Arguments
    ///
    /// * `name` - Name of the token.
    pub fn create_token(&self, name: impl Into<String>) -> CreateTokenBuilder {
        CreateTokenBuilder::new(
            crate::codegen::tokens::TokenServiceClient::new(
                self.client.clone(),
                self.base_url.clone(),
            ),
            name,
        )
    }

    /// Access the `token` resource scoped to the given name.
    pub fn token(&self, token_name: impl Into<String>) -> TokenClient {
        TokenClient::new(
            token_name,
            crate::codegen::tokens::TokenServiceClient::new(
                self.client.clone(),
                self.base_url.clone(),
            ),
        )
    }

    /// List the volumes of a schema that the caller can see.
    ///
    /// # Arguments
    ///
    /// * `catalog_name` - Name of the parent catalog.
    /// * `schema_name` - Name of the parent schema.
    pub fn list_volumes(
        &self,
        catalog_name: impl Into<String>,
        schema_name: impl Into<String>,
    ) -> ListVolumesBuilder {
        ListVolumesBuilder::new(
            crate::codegen::volumes::VolumeServiceClient::new(
                self.client.clone(),
                self.base_url.clone(),
            ),
            catalog_name,
            schema_name,
        )
    }

    /// Create a volume.
    ///
    /// # Arguments
    ///
    /// * `catalog_name` - Name of the parent catalog.
    /// * `schema_name` - Name of the parent schema.
    /// * `name` - Name of the volume, relative to the parent schema.
    /// * `volume_type` - Type of the volume.
    pub fn create_volume(
        &self,
        catalog_name: impl Into<String>,
        schema_name: impl Into<String>,
        name: impl Into<String>,
        volume_type: VolumeType,
    ) -> CreateVolumeBuilder {
        CreateVolumeBuilder::new(
            crate::codegen::volumes::VolumeServiceClient::new(
                self.client.clone(),
                self.base_url.clone(),
            ),
            catalog_name,
            schema_name,
            name,
            volume_type,
        )
    }

    /// Access the `volume` resource scoped to the given name.
    pub fn volume(&self, volume_name: impl Into<String>) -> VolumeClient {
        VolumeClient::new(
            volume_name,
            crate::codegen::volumes::VolumeServiceClient::new(
                self.client.clone(),
                self.base_url.clone(),
            ),
        )
    }
}
