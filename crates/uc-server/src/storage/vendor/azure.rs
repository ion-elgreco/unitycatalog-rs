//! Azure credentials, after upstream `AzureCredentialVendor`.
//!
//! Each storage account is configured by `adls.*.N`. The service principal
//! (tenant, client id and secret) signs in to Microsoft Entra ID through
//! `object_store`, gets a user delegation key for the hour, and signs a
//! directory SAS for the path: read and list, plus write and delete for
//! writes. In test mode the SAS is `tenant/client/secret`. External
//! locations are not supported for Azure yet, as upstream.

use std::collections::HashMap;
use std::sync::Mutex;

use base64::{Engine, engine::general_purpose::STANDARD as B64};
use chrono::{DateTime, Duration, SecondsFormat, Utc};
use hmac::{Hmac, Mac};
use object_store::{
    azure::{AzureCredential, AzureCredentialProvider, MicrosoftAzureBuilder},
    client::SpawnedReqwestConnector,
};
use percent_encoding::{AsciiSet, NON_ALPHANUMERIC, utf8_percent_encode};
use sha2::Sha256;

use uc_api::{AzureUserDelegationSAS, TemporaryCredentials};
use uc_common::{Error, NormalizedUrl, Result};
use uc_storage::runtime::io_handle;

use super::{CredentialContext, StoragePrivilege, xml_text};
use crate::http::outbound::SendOnIo;

/// The SAS version signed: its string-to-sign is the one of
/// [`user_delegation_sas`].
const SAS_VERSION: &str = "2021-08-06";

/// Upstream's expiration of a test-mode SAS.
const TEST_MODE_EXPIRATION: i64 = 253_370_790_000_000;

/// Upstream `ADLSStorageConfig`.
#[derive(Debug, Clone)]
pub struct AdlsConfig {
    pub tenant_id: String,
    pub client_id: String,
    pub client_secret: String,
    pub test_mode: bool,
}

pub struct AzureVendor {
    accounts: HashMap<String, AdlsConfig>,
    http: reqwest::Client,
    /// Entra ID, when not Microsoft's: a test's.
    login: Option<String>,
    blob: Option<String>,
    /// The service principals' sign-ins, by account, made on first use.
    sign_ins: Mutex<HashMap<String, AzureCredentialProvider>>,
}

/// Upstream `ADLSLocationUtils.ADLSLocationParts`.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct LocationParts {
    container: Option<String>,
    /// `account.dfs.core.windows.net`.
    pub(crate) account: String,
    /// `account`.
    account_name: String,
}

pub(crate) fn parse_location(base: &NormalizedUrl) -> Result<LocationParts> {
    let authority = base
        .as_str()
        .split_once("://")
        .map(|(_, rest)| rest.split('/').next().unwrap_or_default())
        .filter(|a| !a.is_empty())
        .ok_or_else(|| {
            Error::invalid_argument(format!("Invalid ADLS location, missing authority: {base}"))
        })?;
    let (container, account) = match authority.split_once('@') {
        Some((container, account)) => (Some(container.to_string()), account.to_string()),
        None => (None, authority.to_string()),
    };
    let account_name = account.split('.').next().unwrap_or_default().to_string();
    Ok(LocationParts {
        container,
        account,
        account_name,
    })
}

/// A user delegation key, from `Get User Delegation Key`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DelegationKey {
    pub signed_oid: String,
    pub signed_tid: String,
    pub signed_start: String,
    pub signed_expiry: String,
    pub signed_service: String,
    pub signed_version: String,
    /// Base64.
    pub value: String,
}

fn rfc3339(t: DateTime<Utc>) -> String {
    t.to_rfc3339_opts(SecondsFormat::Secs, true)
}

/// What Python's `quote` encodes: all but unreserved characters and `/`.
const QUOTED: &AsciiSet = &NON_ALPHANUMERIC
    .remove(b'-')
    .remove(b'_')
    .remove(b'.')
    .remove(b'~')
    .remove(b'/');

/// Percent-encode as Python's `quote`.
fn quote(s: &str) -> String {
    utf8_percent_encode(s, QUOTED).to_string()
}

/// A directory user delegation SAS (`sr=d`) for `path` in `container`, in
/// the Azure SDKs' format for version 2021-08-06.
pub fn user_delegation_sas(
    key: &DelegationKey,
    account_name: &str,
    container: &str,
    path: &str,
    permissions: &str,
    start: &str,
    expiry: &str,
) -> Result<String> {
    let depth = path
        .split('/')
        .filter(|s| !s.is_empty())
        .count()
        .to_string();
    let resource = format!("/blob/{account_name}/{container}/{path}");
    let string_to_sign = [
        permissions,
        start,
        expiry,
        &resource,
        &key.signed_oid,
        &key.signed_tid,
        &key.signed_start,
        &key.signed_expiry,
        &key.signed_service,
        &key.signed_version,
        "", // signedAuthorizedUserObjectId
        "", // signedUnauthorizedUserObjectId
        "", // signedCorrelationId
        "", // signedIP
        "", // signedProtocol
        SAS_VERSION,
        "d",
        "", // signedSnapshotTime
        "", // signedEncryptionScope
        "", // rscc
        "", // rscd
        "", // rsce
        "", // rscl
        "", // rsct
    ]
    .join("\n");
    let secret = B64
        .decode(&key.value)
        .map_err(|e| Error::internal(format!("user delegation key is not base64: {e}")))?;
    let mut mac =
        Hmac::<Sha256>::new_from_slice(&secret).map_err(|e| Error::internal(e.to_string()))?;
    mac.update(string_to_sign.as_bytes());
    let signature = B64.encode(mac.finalize().into_bytes());
    let query = [
        ("st", start),
        ("se", expiry),
        ("sp", permissions),
        ("sv", SAS_VERSION),
        ("sr", "d"),
        ("sdd", &depth),
        ("skoid", &key.signed_oid),
        ("sktid", &key.signed_tid),
        ("skt", &key.signed_start),
        ("ske", &key.signed_expiry),
        ("sks", &key.signed_service),
        ("skv", &key.signed_version),
        ("sig", &signature),
    ];
    Ok(query
        .iter()
        .map(|(k, v)| format!("{k}={}", quote(v)))
        .collect::<Vec<_>>()
        .join("&"))
}

/// Upstream `resolvePrivileges`, in `PathSasPermission` order.
fn permissions(privileges: &[StoragePrivilege]) -> String {
    let read = privileges.contains(&StoragePrivilege::Select);
    let write = privileges.contains(&StoragePrivilege::Update);
    [(read, 'r'), (write, 'w'), (write, 'd'), (read, 'l')]
        .iter()
        .filter(|(on, _)| *on)
        .map(|(_, c)| c)
        .collect()
}

impl AzureVendor {
    pub fn new(accounts: HashMap<String, AdlsConfig>) -> Self {
        Self {
            accounts,
            http: reqwest::Client::new(),
            login: None,
            blob: None,
            sign_ins: Mutex::new(HashMap::new()),
        }
    }

    /// Entra ID and blob endpoints, for tests.
    pub fn with_endpoints(mut self, login: Option<String>, blob: Option<String>) -> Self {
        self.login = login.or(self.login);
        self.blob = blob.or(self.blob);
        self
    }

    pub async fn vend(&self, ctx: &CredentialContext) -> Result<TemporaryCredentials> {
        if ctx.credential.is_some() {
            return Err(Error::unimplemented(
                "Storage credential/external location for Azure is not supported yet.",
            ));
        }
        let parts = parse_location(&ctx.storage_base)?;
        let config = self.accounts.get(&parts.account_name).ok_or_else(|| {
            Error::failed_precondition("Azure storage account configuration not found.")
        })?;
        let (sas_token, expiration) = match config.test_mode {
            true => {
                let token = format!(
                    "{}/{}/{}",
                    config.tenant_id, config.client_id, config.client_secret
                );
                (token, TEST_MODE_EXPIRATION)
            }
            false => self.delegation_sas(ctx, &parts, config).await?,
        };
        Ok(TemporaryCredentials {
            azure_user_delegation_sas: Some(AzureUserDelegationSAS {
                sas_token: Some(sas_token),
            }),
            expiration_time: Some(expiration),
            ..Default::default()
        })
    }

    /// Upstream `DatalakeCredentialGenerator`: an hour from now.
    async fn delegation_sas(
        &self,
        ctx: &CredentialContext,
        parts: &LocationParts,
        config: &AdlsConfig,
    ) -> Result<(String, i64)> {
        let container = parts.container.as_deref().ok_or_else(|| {
            Error::invalid_argument(format!(
                "Invalid ADLS location, missing container: {}",
                ctx.storage_base
            ))
        })?;
        // Whole seconds, as the SAS states them.
        let start = DateTime::from_timestamp(Utc::now().timestamp(), 0).unwrap_or_else(Utc::now);
        let expiry = start + Duration::hours(1);
        let (start_text, expiry_text) = (rfc3339(start), rfc3339(expiry));
        let token = self.access_token(&parts.account_name, config).await?;
        let key = self
            .delegation_key(parts, &token, &start_text, &expiry_text)
            .await?;
        // ABFS drops trailing slashes, so the SAS names the path without
        // leading or trailing ones.
        let path = ctx.locations.first().map(|l| l.path()).unwrap_or_default();
        let path = path.trim_matches('/');
        let sas = user_delegation_sas(
            &key,
            &parts.account_name,
            container,
            path,
            &permissions(&ctx.privileges),
            &start_text,
            &expiry_text,
        )?;
        Ok((sas, expiry.timestamp_millis()))
    }

    /// The service principal's token for Azure Storage, from `object_store`,
    /// which keeps it until it is about to expire.
    async fn access_token(&self, account: &str, config: &AdlsConfig) -> Result<String> {
        let failed = |e: String| Error::internal(format!("Azure sign-in failed: {e}"));
        let credential = self
            .sign_in(account, config)
            .map_err(|e| failed(e.to_string()))?
            .get_credential()
            .await
            .map_err(|e| failed(e.to_string()))?;
        match credential.as_ref() {
            AzureCredential::BearerToken(token) => Ok(token.clone()),
            _ => Err(failed("no bearer token".into())),
        }
    }

    fn sign_in(
        &self,
        account: &str,
        config: &AdlsConfig,
    ) -> object_store::Result<AzureCredentialProvider> {
        let mut sign_ins = self.sign_ins.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(found) = sign_ins.get(account) {
            return Ok(found.clone());
        }
        // A store only for its credentials: it sends no requests.
        let mut builder = MicrosoftAzureBuilder::new()
            .with_account(account)
            .with_container_name("uc-credentials")
            .with_tenant_id(&config.tenant_id)
            .with_client_id(&config.client_id)
            .with_client_secret(&config.client_secret)
            .with_http_connector(SpawnedReqwestConnector::new(io_handle()));
        if let Some(login) = &self.login {
            builder = builder.with_authority_host(login).with_allow_http(true);
        }
        let found = builder.build()?.credentials().clone();
        sign_ins.insert(account.to_string(), found.clone());
        Ok(found)
    }

    /// `Get User Delegation Key` from the account's blob service.
    async fn delegation_key(
        &self,
        parts: &LocationParts,
        token: &str,
        start: &str,
        expiry: &str,
    ) -> Result<DelegationKey> {
        let endpoint = match &self.blob {
            Some(blob) => blob.clone(),
            None => format!("https://{}", parts.account.replacen(".dfs.", ".blob.", 1)),
        };
        let body = format!(
            "<?xml version=\"1.0\" encoding=\"utf-8\"?><KeyInfo><Start>{start}</Start><Expiry>{expiry}</Expiry></KeyInfo>"
        );
        let response = self
            .http
            .post(format!(
                "{endpoint}/?restype=service&comp=userdelegationkey"
            ))
            .bearer_auth(token)
            .header("x-ms-version", SAS_VERSION)
            .header("content-type", "application/xml")
            .body(body)
            .send_on_io()
            .await
            .map_err(|e| Error::internal(format!("Azure user delegation key failed: {e}")))?;
        let status = response.status();
        let xml = response.text().await.unwrap_or_default();
        if !status.is_success() {
            return Err(Error::internal(format!(
                "Azure user delegation key failed ({status}): {xml}"
            )));
        }
        let field = |tag: &str| {
            xml_text(&xml, tag)
                .map(str::to_string)
                .ok_or_else(|| Error::internal(format!("Azure user delegation key without {tag}")))
        };
        Ok(DelegationKey {
            signed_oid: field("SignedOid")?,
            signed_tid: field("SignedTid")?,
            signed_start: field("SignedStart")?,
            signed_expiry: field("SignedExpiry")?,
            signed_service: field("SignedService")?,
            signed_version: field("SignedVersion")?,
            value: field("Value")?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key() -> DelegationKey {
        DelegationKey {
            signed_oid: "11111111-1111-1111-1111-111111111111".into(),
            signed_tid: "22222222-2222-2222-2222-222222222222".into(),
            signed_start: "2026-01-01T00:00:00Z".into(),
            signed_expiry: "2026-01-01T01:00:00Z".into(),
            signed_service: "b".into(),
            signed_version: "2021-08-06".into(),
            value: "c2VjcmV0LWtleS1ieXRlcy0xMjM0NTY3ODkwMTIzNDU2Nzg5MA==".into(),
        }
    }

    /// The same SAS as the Azure SDK for Python (`generate_directory_sas`,
    /// azure-storage-blob 12.13.1, version 2021-08-06) for these inputs.
    #[test]
    fn signs_like_the_azure_sdk() {
        let common = "st=2026-01-01T00%3A00%3A00Z&se=2026-01-01T01%3A00%3A00Z";
        let key_params = "skoid=11111111-1111-1111-1111-111111111111&sktid=22222222-2222-2222-2222-222222222222\
            &skt=2026-01-01T00%3A00%3A00Z&ske=2026-01-01T01%3A00%3A00Z&sks=b&skv=2021-08-06";
        for (privileges, sp, sig) in [
            (
                vec![StoragePrivilege::Select],
                "rl",
                "QMgFhd5Ofj%2BoK4/arJbulCxTVqwhPONzt6sErsO34ZI%3D",
            ),
            (
                vec![StoragePrivilege::Select, StoragePrivilege::Update],
                "rwdl",
                "abzNlgcapRkb4mc%2BL8Jk5lt%2Bh2p3gJHm%2BqxV44iSovY%3D",
            ),
        ] {
            let sas = user_delegation_sas(
                &key(),
                "myaccount",
                "mycontainer",
                "tables/t1",
                &permissions(&privileges),
                "2026-01-01T00:00:00Z",
                "2026-01-01T01:00:00Z",
            )
            .unwrap();
            assert_eq!(
                sas,
                format!("{common}&sp={sp}&sv=2021-08-06&sr=d&sdd=2&{key_params}&sig={sig}")
            );
        }
    }

    #[test]
    fn parses_adls_locations() {
        let parts =
            parse_location(&NormalizedUrl::parse("abfss://c@acct.dfs.core.windows.net").unwrap())
                .unwrap();
        assert_eq!(
            parts,
            LocationParts {
                container: Some("c".into()),
                account: "acct.dfs.core.windows.net".into(),
                account_name: "acct".into(),
            }
        );
        let parts =
            parse_location(&NormalizedUrl::parse("abfs://acct.dfs.core.windows.net").unwrap())
                .unwrap();
        assert_eq!(parts.container, None);
        assert_eq!(parts.account_name, "acct");
    }

    #[test]
    fn reads_the_delegation_key_response() {
        let xml = "<?xml version=\"1.0\"?><UserDelegationKey><SignedOid>o</SignedOid><Value>v==</Value></UserDelegationKey>";
        assert_eq!(xml_text(xml, "SignedOid"), Some("o"));
        assert_eq!(xml_text(xml, "Value"), Some("v=="));
        assert_eq!(xml_text(xml, "SignedTid"), None);
    }
}
