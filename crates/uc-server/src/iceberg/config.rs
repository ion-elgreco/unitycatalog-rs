//! The FileIO properties an Iceberg client needs to reach a table: vended
//! credentials under Iceberg's property names. Upstream
//! `FileOperations.getFileIOConfig`. With `aws.endpoint`, S3 clients also
//! get the store's endpoint and region.

use std::collections::BTreeMap;

use uc_api::TemporaryCredentials;
use uc_common::{Error, NormalizedUrl, Result, UriScheme};

use crate::{
    server::state::AppState,
    storage::vendor::{VendOp, aws::S3Store, azure, vend_credential},
};

/// Credentials for `location` that allow `op`, as FileIO properties, with
/// the endpoint (relative to the catalog) where a client renews them. Local
/// tables need none.
pub async fn table_config(
    state: &AppState,
    location: &NormalizedUrl,
    op: VendOp,
    refresh_endpoint: &str,
) -> Result<BTreeMap<String, String>> {
    if location.scheme() == UriScheme::File {
        return Ok(BTreeMap::new());
    }
    let credentials =
        vend_credential(&state.metastore, &state.vendor, location, op.privileges()).await?;
    let store = state.vendor.s3_store();
    // A store's buckets are not AWS's: no region from AWS.
    let region = match (&credentials.aws_temp_credentials, store) {
        (Some(_), Some(store)) => Some(store.region.clone()),
        (Some(_), None) => Some(state.s3_regions.region(location).await?),
        (None, _) => None,
    };
    properties(
        location,
        &credentials,
        region.as_deref(),
        Some(refresh_endpoint),
        store,
    )
}

/// The FileIO properties of vended `credentials` for `location`, in the
/// S3-compatible `store` if any.
pub fn properties(
    location: &NormalizedUrl,
    credentials: &TemporaryCredentials,
    region: Option<&str>,
    refresh_endpoint: Option<&str>,
    store: Option<&S3Store>,
) -> Result<BTreeMap<String, String>> {
    let mut config = BTreeMap::new();
    let mut put = |key: String, value: Option<String>| {
        if let Some(value) = value {
            config.insert(key, value);
        }
    };
    let expires = credentials.expiration_time.map(|t| t.to_string());
    let endpoint = refresh_endpoint.map(str::to_string);
    if let Some(sas) = &credentials.azure_user_delegation_sas {
        let account = azure::parse_location(location)?.account;
        put(format!("adls.sas-token.{account}"), sas.sas_token.clone());
        put(format!("adls.sas-token-expires-at-ms.{account}"), expires);
        put("adls.refresh-credentials-endpoint".into(), endpoint);
    } else if let Some(token) = &credentials.gcp_oauth_token {
        put("gcs.oauth2.token".into(), token.oauth_token.clone());
        put("gcs.oauth2.token-expires-at".into(), expires);
        put("gcs.oauth2.refresh-credentials-endpoint".into(), endpoint);
    } else if let Some(aws) = &credentials.aws_temp_credentials {
        put("s3.access-key-id".into(), aws.access_key_id.clone());
        put("s3.secret-access-key".into(), aws.secret_access_key.clone());
        put("s3.session-token".into(), aws.session_token.clone());
        put("client.region".into(), region.map(str::to_string));
        put("s3.session-token-expires-at-ms".into(), expires);
        put("client.refresh-credentials-endpoint".into(), endpoint);
        if let Some(store) = store {
            put("s3.endpoint".into(), Some(store.endpoint.clone()));
            put("s3.path-style-access".into(), Some("true".into()));
        }
    } else {
        return Err(Error::internal(format!(
            "No recognized storage credential was vended for location: {location}"
        )));
    }
    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::*;
    use uc_api::{AwsCredentials, AzureUserDelegationSAS, GcpOauthToken};

    fn url(s: &str) -> NormalizedUrl {
        NormalizedUrl::parse(s).unwrap()
    }

    #[test]
    fn properties_per_cloud() {
        let aws = TemporaryCredentials {
            aws_temp_credentials: Some(AwsCredentials {
                access_key_id: Some("ak".into()),
                secret_access_key: Some("sk".into()),
                session_token: Some("st".into()),
            }),
            expiration_time: Some(42),
            ..Default::default()
        };
        let config = properties(
            &url("s3://b/t"),
            &aws,
            Some("eu-west-1"),
            Some("v1/x/credentials"),
            None,
        )
        .unwrap();
        assert_eq!(
            config,
            BTreeMap::from([
                ("s3.access-key-id".to_string(), "ak".to_string()),
                ("s3.secret-access-key".into(), "sk".into()),
                ("s3.session-token".into(), "st".into()),
                ("client.region".into(), "eu-west-1".into()),
                ("s3.session-token-expires-at-ms".into(), "42".into()),
                (
                    "client.refresh-credentials-endpoint".into(),
                    "v1/x/credentials".into()
                ),
            ])
        );

        // An S3-compatible store: its endpoint, with path-style access.
        let store = S3Store {
            endpoint: "http://localhost:9000".into(),
            region: "us-east-1".into(),
        };
        let config = properties(
            &url("s3://b/t"),
            &aws,
            Some("us-east-1"),
            None,
            Some(&store),
        )
        .unwrap();
        assert_eq!(config["s3.endpoint"], "http://localhost:9000");
        assert_eq!(config["s3.path-style-access"], "true");
        assert_eq!(config["client.region"], "us-east-1");

        let azure = TemporaryCredentials {
            azure_user_delegation_sas: Some(AzureUserDelegationSAS {
                sas_token: Some("sas".into()),
            }),
            expiration_time: Some(42),
            ..Default::default()
        };
        let config = properties(
            &url("abfss://c@acct.dfs.core.windows.net/t"),
            &azure,
            None,
            None,
            None,
        )
        .unwrap();
        assert_eq!(
            config,
            BTreeMap::from([
                (
                    "adls.sas-token.acct.dfs.core.windows.net".to_string(),
                    "sas".to_string()
                ),
                (
                    "adls.sas-token-expires-at-ms.acct.dfs.core.windows.net".into(),
                    "42".into()
                ),
            ])
        );

        let gcp = TemporaryCredentials {
            gcp_oauth_token: Some(GcpOauthToken {
                oauth_token: Some("tok".into()),
            }),
            ..Default::default()
        };
        let config = properties(&url("gs://b/t"), &gcp, None, Some("e"), None).unwrap();
        assert_eq!(
            config,
            BTreeMap::from([
                ("gcs.oauth2.token".to_string(), "tok".to_string()),
                ("gcs.oauth2.refresh-credentials-endpoint".into(), "e".into()),
            ])
        );

        let err = properties(
            &url("s3://b/t"),
            &TemporaryCredentials::default(),
            None,
            None,
            None,
        )
        .unwrap_err();
        assert_eq!(
            err.message(),
            "No recognized storage credential was vended for location: s3://b/t"
        );
    }
}
