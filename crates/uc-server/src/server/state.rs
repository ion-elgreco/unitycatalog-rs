use std::collections::HashSet;
use std::ops::Deref;
use std::path::PathBuf;
use std::sync::Arc;

use uc_metastore::DynMetastore;

use crate::{
    authn::idp::AuthSettings,
    authn::security::SecurityContext,
    authz::{PermissionEnforcer, PolicyEngine},
    observability::audit::sink::{self as audit_sink, AuditHandle},
    storage::s3_region::S3Regions,
    storage::vendor::{CredentialVendor, DynVendor, PlaceholderVendor},
};

/// The state of every route: [`AppStateInner`] behind one `Arc`, so that a
/// clone, which axum makes several times per request, is one count. Each
/// worker gets a copy with a count of its own ([`Self::for_worker`]): the
/// workers do not share a counter.
#[derive(Clone)]
pub struct AppState(Arc<AppStateInner>);

impl Deref for AppState {
    type Target = AppStateInner;

    fn deref(&self) -> &AppStateInner {
        &self.0
    }
}

#[derive(Clone)]
pub struct AppStateInner {
    pub metastore: DynMetastore,
    /// The server's signing keys. `None`: no UC tokens are issued or
    /// accepted (tests that do not sign in).
    pub security: Option<Arc<SecurityContext>>,
    /// Token exchange and sign-in settings.
    pub auth: Arc<AuthSettings>,
    /// `auth.enabled`: privileges are checked and every
    /// API request must be signed in.
    pub authorization_enabled: bool,
    /// Where the grants of a caller come from when authorization is on:
    /// the `permission` table ([`PermissionEnforcer`]), or the Cedar engine
    /// ([`Self::with_cedar`]).
    pub policy: Arc<dyn PolicyEngine>,
    /// Mints temporary cloud credentials for the `/temporary-*-credentials`
    /// endpoints: [`PlaceholderVendor`] by default, `CloudVendor` (AWS STS,
    /// Azure SAS, GCP OAuth) in the server.
    pub vendor: DynVendor,
    /// `auth.admin_principals`: metastore admins besides the metastore
    /// owner.
    pub admin_principals: Arc<HashSet<String>>,
    /// `tables.delta_api_only`: MANAGED tables are
    /// written through the Delta API only, so the UC endpoints that would
    /// write them are refused.
    pub delta_api_only: bool,
    /// `tables.iceberg_writes`: the Iceberg REST catalog
    /// accepts namespace and table writes.
    pub iceberg_writes_enabled: bool,
    /// `storage.external_local_roots`: local directories under which
    /// external Iceberg tables may be created without an external location.
    pub external_local_roots: Arc<Vec<PathBuf>>,
    /// The regions of S3 buckets, configured or discovered.
    pub s3_regions: Arc<S3Regions>,
    /// The server's AWS role (upstream `aws.masterRoleArn`), shown as
    /// `unity_catalog_iam_arn` on AWS IAM role credentials.
    pub aws_identity_arn: Option<Arc<str>>,
    /// Producer handle for the audit-event drainer task.
    pub audit: AuditHandle,
    /// The last result of the readiness check, for `/readyz`.
    pub readiness: crate::routes::health::Readiness,
    /// The API port, for the `hostname_pattern` of request metrics.
    pub api_port: u16,
    /// What the API listener serves, and its limits.
    pub http: Arc<crate::server::app::HttpSettings>,
}

impl AppState {
    pub fn new(metastore: DynMetastore) -> Self {
        let audit = audit_sink::spawn_drainer(metastore.clone(), audit_sink::DEFAULT_CAPACITY);
        let policy = Arc::new(PermissionEnforcer::new(metastore.clone()));
        Self(Arc::new(AppStateInner {
            metastore,
            security: None,
            auth: Arc::new(AuthSettings::default()),
            authorization_enabled: false,
            policy,
            vendor: Arc::new(PlaceholderVendor),
            admin_principals: Arc::new(HashSet::new()),
            aws_identity_arn: None,
            delta_api_only: false,
            iceberg_writes_enabled: true,
            s3_regions: Arc::new(S3Regions::default()),
            external_local_roots: Arc::new(Vec::new()),
            audit,
            readiness: Default::default(),
            api_port: 8080,
            http: Default::default(),
        }))
    }

    /// A copy for one worker: the same values, with a reference count of
    /// its own.
    pub fn for_worker(&self) -> Self {
        Self(Arc::new(AppStateInner::clone(&self.0)))
    }

    fn inner(&mut self) -> &mut AppStateInner {
        Arc::make_mut(&mut self.0)
    }

    pub fn with_api_port(mut self, port: u16) -> Self {
        self.inner().api_port = port;
        self
    }

    pub fn with_http(mut self, http: crate::server::app::HttpSettings) -> Self {
        self.inner().http = Arc::new(http);
        self
    }

    pub fn with_delta_api_only(mut self, on: bool) -> Self {
        self.inner().delta_api_only = on;
        self
    }

    /// Configure the region of the S3 bucket of `location`.
    pub fn with_s3_region(self, location: &uc_common::NormalizedUrl, region: &str) -> Self {
        self.s3_regions.configure(location, region);
        self
    }

    pub fn with_external_local_roots(mut self, roots: Vec<PathBuf>) -> Self {
        self.inner().external_local_roots = Arc::new(roots);
        self
    }

    pub fn with_iceberg_writes_enabled(mut self, on: bool) -> Self {
        self.inner().iceberg_writes_enabled = on;
        self
    }

    /// Upstream `checkDeltaApiOnlyEnabled`: refuse a UC endpoint whose Delta
    /// API equivalent must be used.
    pub fn check_delta_api_only(&self, delta_endpoint: &str) -> uc_common::Result<()> {
        if self.delta_api_only {
            return Err(uc_common::Error::invalid_argument(format!(
                "This Unity Catalog endpoint is disabled when \
                 tables.delta_api_only = true (UC_TABLES_DELTA_API_ONLY). Use the Delta endpoint \
                 {delta_endpoint} instead."
            )));
        }
        Ok(())
    }

    /// Upstream `checkDeltaApiOnlyForManagedTable`.
    pub fn check_delta_api_only_for_managed(
        &self,
        table_type: Option<uc_api::TableType>,
        delta_endpoint: &str,
    ) -> uc_common::Result<()> {
        if self.delta_api_only && table_type == Some(uc_api::TableType::Managed) {
            return Err(uc_common::Error::invalid_argument(format!(
                "This Unity Catalog endpoint is disabled for MANAGED Delta tables when \
                 tables.delta_api_only = true (UC_TABLES_DELTA_API_ONLY). Use the Delta endpoint \
                 {delta_endpoint} instead."
            )));
        }
        Ok(())
    }

    pub fn with_aws_identity_arn(mut self, arn: impl Into<Arc<str>>) -> Self {
        self.inner().aws_identity_arn = Some(arn.into());
        self
    }

    pub fn with_security(mut self, security: Arc<SecurityContext>) -> Self {
        self.inner().security = Some(security);
        self
    }

    pub fn with_auth_settings(mut self, settings: AuthSettings) -> Self {
        self.inner().auth = Arc::new(settings);
        self
    }

    pub fn with_authz(mut self, on: bool) -> Self {
        self.inner().authorization_enabled = on;
        self
    }

    /// The Cedar policy engine, with the policies of the `.cedar` files
    /// `paths` on top of its defaults.
    pub fn with_cedar(mut self, paths: &[PathBuf]) -> Result<Self, uc_common::Error> {
        let engine =
            crate::authz::cedar::CedarEngine::with_policy_files(self.metastore.clone(), paths)?;
        self.inner().policy = Arc::new(engine);
        Ok(self)
    }

    pub fn with_vendor<V: CredentialVendor>(mut self, vendor: V) -> Self {
        self.inner().vendor = Arc::new(vendor);
        self
    }

    pub fn with_admins<I, S>(mut self, admins: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.inner().admin_principals = Arc::new(admins.into_iter().map(Into::into).collect());
        self
    }
}
