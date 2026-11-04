//! A uc-server in this process, and what the tests of the areas share.

use std::collections::HashMap;
use std::fmt::Debug;
use std::net::SocketAddr;
use std::sync::{Arc, OnceLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Form, Json, Router};
use jsonwebtoken::{Algorithm, EncodingKey, Header};
use serde_json::{Value, json};
use tempfile::TempDir;
use url::Url;

use uc_client::Client;
use uc_client::error::{ApiError, Error, Result};
use uc_client::models::scim::v1::ScimEmail;
use uc_client::models::tables::v1::{
    ColumnInfo, ColumnTypeName, DataSourceFormat, TableInfo, TableType,
};
use uc_metastore::{DynMetastore, MetastoreConfig, SqlMetastore};
use uc_server::authn::security::SecurityContext;
use uc_server::{AppState, build_app};

/// Serve `app` on a free port of this machine; its root URL.
pub async fn serve(app: Router) -> Url {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(
            listener,
            app.into_make_service_with_connect_info::<SocketAddr>(),
        )
        .await
        .unwrap();
    });
    format!("http://{addr}/").parse().unwrap()
}

/// A new test database.
pub async fn metastore() -> DynMetastore {
    SqlMetastore::connect(MetastoreConfig::for_tests())
        .await
        .expect("open the test database")
}

/// The signing keys of the servers of the tests.
pub fn security() -> Arc<SecurityContext> {
    static SECURITY: OnceLock<Arc<SecurityContext>> = OnceLock::new();
    SECURITY
        .get_or_init(|| Arc::new(SecurityContext::generate().unwrap()))
        .clone()
}

/// A UC access token of `user`, which the servers with [`security`] take.
pub fn token(user: &str) -> String {
    security()
        .create_access_token(user, Duration::from_secs(600))
        .unwrap()
}

/// A uc-server with a new database and no sign-in; its root URL.
pub async fn server() -> Url {
    serve(build_app(AppState::new(metastore().await))).await
}

/// A client of a new [`server`].
pub async fn client() -> Client {
    Client::new_unauthenticated(server().await)
}

/// A uc-server as [`server`] that also takes the tokens of [`token`]: a
/// token names its caller. Its root URL.
pub async fn server_with_keys() -> Url {
    let state = AppState::new(metastore().await).with_security(security());
    serve(build_app(state)).await
}

/// The catalog `c`, with its managed storage in the returned directory,
/// and its schema `c.s`.
pub async fn catalog_and_schema(uc: &Client) -> TempDir {
    let storage = tempfile::tempdir().unwrap();
    uc.create_catalog("c")
        .with_storage_root(file_url(&storage))
        .await
        .unwrap();
    uc.create_schema("s", "c").await.unwrap();
    storage
}

/// The `file://` URL of `dir`.
pub fn file_url(dir: &TempDir) -> String {
    format!("file://{}", dir.path().display())
}

/// A nullable column `name` at `position`: of the Spark type `spark`, such
/// as `long`, which is `sql` in SQL, such as `bigint`.
pub fn column(
    name: &str,
    sql: &str,
    spark: &str,
    type_name: ColumnTypeName,
    position: i32,
) -> ColumnInfo {
    ColumnInfo {
        name: name.into(),
        type_text: Some(sql.into()),
        type_json: format!(
            r#"{{"name":"{name}","type":"{spark}","nullable":true,"metadata":{{}}}}"#
        ),
        type_name: Some(type_name.into()),
        position: Some(position),
        nullable: Some(true),
        ..Default::default()
    }
}

/// The EXTERNAL Delta table `c.s.<name>`, with the column `id`, in the
/// folder `name` of `storage`.
pub async fn external_table(uc: &Client, storage: &TempDir, name: &str) -> TableInfo {
    uc.create_table(name, "c", "s", TableType::External)
        .with_columns([column("id", "bigint", "long", ColumnTypeName::Long, 0)])
        .with_data_source_format(DataSourceFormat::Delta)
        .with_storage_location(format!("{}/{name}", file_url(storage)))
        .await
        .unwrap()
}

/// The primary email `address` of a SCIM user.
pub fn email(address: &str) -> ScimEmail {
    ScimEmail {
        value: address.into(),
        primary: Some(true),
        ..Default::default()
    }
}

/// The error answer of a call that fails.
#[track_caller]
pub fn api_error<T: Debug>(result: Result<T>) -> ApiError {
    match result {
        Err(Error::Api(error)) => error,
        other => panic!("not an error answer: {other:?}"),
    }
}

/// The error answer of a call that fails with HTTP `status` and the error
/// code `code`.
#[track_caller]
pub fn rejected<T: Debug>(result: Result<T>, status: u16, code: &str) -> ApiError {
    let error = api_error(result);
    assert_eq!(
        (error.status, error.code.as_deref()),
        (status, Some(code)),
        "{error:?}"
    );
    error
}

/// The P-256 key of [`Idp`], a key for these tests only, in PKCS #8.
const IDP_KEY: &str = "-----BEGIN PRIVATE KEY-----
MIGHAgEAMBMGByqGSM49AgEGCCqGSM49AwEHBG0wawIBAQQg9bV2OxfT2NY4FhNp
tB1eTLqKzvnAKPsbWpjnyDSUIL2hRANCAASTdscsBJf35ZbWbx/g//Vy1ff3ouV+
+6X+vzSzOilBukWUMgs5eB7pEhpv4eu45k1jCFMdVSMjBNVRdkK0afBc
-----END PRIVATE KEY-----";

/// The OAuth client of [`Idp`]'s token endpoint, and its secret.
pub const IDP_CLIENT: &str = "etl";
pub const IDP_SECRET: &str = "s3cret";

/// An identity provider in this process: OIDC discovery and the public
/// key of [`IDP_KEY`] (its `x` and `y`), which signs its tokens; and a
/// token endpoint that gives [`IDP_CLIENT`] an access token for its
/// secret, with the client-credentials grant.
pub struct Idp {
    pub issuer: String,
    key: EncodingKey,
}

impl Idp {
    pub async fn start() -> Self {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let issuer = format!("http://{}", listener.local_addr().unwrap());
        let key = EncodingKey::from_ec_pem(IDP_KEY.as_bytes()).unwrap();
        let config = json!({
            "issuer": issuer, "jwks_uri": format!("{issuer}/jwks"),
            "token_endpoint": format!("{issuer}/token"),
        });
        let jwks = json!({ "keys": [{
            "kty": "EC", "kid": "p256", "use": "sig", "alg": "ES256", "crv": "P-256",
            "x": "k3bHLASX9-WW1m8f4P_1ctX396Llfvul_r80szopQbo",
            "y": "RZQyCzl4HukSGm_h67jmTWMIUx1VIyME1VF2QrRp8Fw",
        }] });
        let app = Router::new()
            .route(
                "/.well-known/openid-configuration",
                get(move || async move { Json(config) }),
            )
            .route("/jwks", get(move || async move { Json(jwks) }))
            .route("/token", post(token_endpoint))
            .with_state((issuer.clone(), key.clone()));
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        Self { issuer, key }
    }

    /// The token endpoint.
    pub fn token_url(&self) -> String {
        format!("{}/token", self.issuer)
    }

    /// An ID token of the user `email`, for the audience `uc`.
    pub fn id_token(&self, email: &str) -> String {
        sign(
            &self.key,
            &self.issuer,
            json!({ "aud": "uc", "sub": email, "email": email }),
        )
    }
}

/// A token of `issuer` with `claims`, valid for ten minutes.
fn sign(key: &EncodingKey, issuer: &str, claims: Value) -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let mut claims = claims;
    let object = claims.as_object_mut().unwrap();
    object.insert("iss".into(), json!(issuer));
    object.insert("iat".into(), json!(now));
    object.insert("exp".into(), json!(now + 600));
    let mut header = Header::new(Algorithm::ES256);
    header.kid = Some("p256".into());
    jsonwebtoken::encode(&header, &claims, key).unwrap()
}

/// The token endpoint: the client-credentials grant of [`IDP_CLIENT`], with
/// the requested `audience` (else `uc`); any other client or secret is
/// `invalid_client`.
async fn token_endpoint(
    State((issuer, key)): State<(String, EncodingKey)>,
    Form(form): Form<HashMap<String, String>>,
) -> Response {
    let field = |name: &str| form.get(name).map(String::as_str).unwrap_or_default();
    if field("grant_type") != "client_credentials"
        || field("client_id") != IDP_CLIENT
        || field("client_secret") != IDP_SECRET
    {
        return (
            StatusCode::UNAUTHORIZED,
            Json(json!({ "error": "invalid_client" })),
        )
            .into_response();
    }
    let audience = Some(field("audience"))
        .filter(|a| !a.is_empty())
        .unwrap_or("uc");
    let token = sign(
        &key,
        &issuer,
        json!({
            "aud": audience, "sub": IDP_CLIENT, "azp": IDP_CLIENT, "client_id": IDP_CLIENT,
        }),
    );
    Json(json!({ "access_token": token, "token_type": "Bearer", "expires_in": 600 }))
        .into_response()
}

/// A fake AWS STS that checks Signature Version 4 as STS does: it answers
/// `GetCallerIdentity` with the ARN of the access key when the signature is
/// that key's secret over the request it got.
pub struct Sts {
    /// Where it listens, such as `http://127.0.0.1:4566`.
    pub endpoint: String,
}

/// The access keys of [`Sts`]: the id, the secret and the ARN.
pub const STS_KEYS: [(&str, &str, &str); 1] = [(
    "ASIAETL",
    "etl-secret",
    "arn:aws:sts::123456789012:assumed-role/etl/i-0a1",
)];

impl Sts {
    pub async fn start() -> Self {
        let url = serve(Router::new().route("/", post(sts_answer))).await;
        Self {
            endpoint: url.as_str().trim_end_matches('/').to_string(),
        }
    }
}

async fn sts_answer(headers: axum::http::HeaderMap, body: String) -> (StatusCode, String) {
    let refused = |code: &str| {
        (
            StatusCode::FORBIDDEN,
            format!("<ErrorResponse><Error><Code>{code}</Code></Error></ErrorResponse>"),
        )
    };
    let header = |name: &str| headers.get(name).and_then(|v| v.to_str().ok());
    let Some(authorization) = header("authorization") else {
        return refused("MissingAuthenticationToken");
    };
    let field = |name: &str| {
        authorization
            .split([' ', ','])
            .find_map(|part| part.strip_prefix(name))
            .unwrap_or_default()
    };
    let credential = field("Credential=");
    let signed_headers = field("SignedHeaders=");
    let signature = field("Signature=");
    let mut scope = credential.splitn(2, '/');
    let (key_id, scope) = (scope.next().unwrap_or(""), scope.next().unwrap_or(""));
    let Some((_, secret, arn)) = STS_KEYS.iter().find(|(id, _, _)| *id == key_id) else {
        return refused("InvalidClientTokenId");
    };
    let mut canonical_headers = String::new();
    for name in signed_headers.split(';') {
        let Some(value) = header(name) else {
            return refused("SignatureDoesNotMatch");
        };
        canonical_headers.push_str(&format!("{name}:{}\n", value.trim()));
    }
    let sha = |data: &[u8]| hex::encode(<sha2::Sha256 as sha2::Digest>::digest(data));
    let canonical_request = format!(
        "POST\n/\n\n{canonical_headers}\n{signed_headers}\n{}",
        sha(body.as_bytes())
    );
    let amz_date = header("x-amz-date").unwrap_or_default();
    let string_to_sign = format!(
        "AWS4-HMAC-SHA256\n{amz_date}\n{scope}\n{}",
        sha(canonical_request.as_bytes())
    );
    let hmac = |key: &[u8], data: &str| {
        use hmac::Mac;
        let mut mac = hmac::Hmac::<sha2::Sha256>::new_from_slice(key).unwrap();
        mac.update(data.as_bytes());
        mac.finalize().into_bytes().to_vec()
    };
    let key = scope
        .split('/')
        .fold(format!("AWS4{secret}").into_bytes(), |key, part| {
            hmac(&key, part)
        });
    if hex::encode(hmac(&key, &string_to_sign)) != signature
        || !scope.ends_with("/sts/aws4_request")
        || body != "Action=GetCallerIdentity&Version=2011-06-15"
    {
        return refused("SignatureDoesNotMatch");
    }
    let account = arn.split(':').nth(4).unwrap_or_default();
    (
        StatusCode::OK,
        format!(
            "<GetCallerIdentityResponse><GetCallerIdentityResult><Arn>{arn}</Arn>\
             <UserId>AROAEXAMPLE:i-0a1</UserId><Account>{account}</Account>\
             </GetCallerIdentityResult></GetCallerIdentityResponse>"
        ),
    )
}
