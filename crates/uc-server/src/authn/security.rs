//! The server's own tokens, after upstream `SecurityContext`: RS512 JWTs
//! from the `internal` issuer, which always expire.
//!
//! The RSA key comes from `auth.signing_key_file`, or the server makes one
//! in memory at start. The server never writes it. The key id is the key's
//! JWK thumbprint (RFC 7638).

use std::path::Path;
use std::time::Duration;

use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use hmac::{Hmac, Mac};
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation};
use rsa::{
    RsaPrivateKey,
    pkcs1::{DecodeRsaPrivateKey, EncodeRsaPrivateKey},
    pkcs8::{DecodePrivateKey, EncodePrivateKey, LineEnding},
    traits::PublicKeyParts,
};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use uc_common::{Error, ErrorCode, Result, now_epoch_millis};

/// The issuer of the tokens this server signs.
pub const INTERNAL_ISSUER: &str = "internal";
/// The bootstrap user: the first owner of the metastore. Only the tokens of
/// `uc-server token` sign it in.
pub const ADMIN: &str = "admin";
/// The issuer of the signed sign-in state between `/auth/login` and
/// `/auth/callback` (`crate::authn::sign_in`): signed with the server's key, but
/// never a token that [`SecurityContext::verify`] accepts.
pub const LOGIN_ISSUER: &str = "internal-login";

/// Hashed with the RSA key into the key of Delta Sharing's file URLs, so
/// that the key serves no other purpose.
const FILE_URL_KEY_CONTEXT: &[u8] = b"unitycatalog-rs delta sharing file urls\n";

/// The claims of a UC access token.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenClaims {
    pub sub: String,
    pub iss: String,
    pub iat: i64,
    pub exp: i64,
    pub jti: String,
    /// `ACCESS`.
    #[serde(rename = "type")]
    pub token_type: String,
}

pub struct SecurityContext {
    encoding: EncodingKey,
    decoding: DecodingKey,
    key_id: String,
    /// The HMAC key of Delta Sharing's file URLs, from the RSA key: the
    /// servers that share a key file accept each other's URLs.
    file_url_key: [u8; 32],
}

fn internal(context: &str, e: impl std::fmt::Display) -> Error {
    Error::internal(format!("{context}: {e}"))
}

/// A new RSA key, as PKCS#8 PEM: for `auth.signing_key_file`.
pub fn generate_key_pem() -> Result<String> {
    let key = new_key()?;
    let pem = key
        .to_pkcs8_pem(LineEnding::LF)
        .map_err(|e| internal("encode the key", e))?;
    Ok(pem.to_string())
}

fn new_key() -> Result<RsaPrivateKey> {
    RsaPrivateKey::new(&mut rand::rngs::OsRng, 2048).map_err(|e| internal("make an RSA key", e))
}

/// The RFC 7638 thumbprint of the RSA public key with the base64url
/// modulus `n` and exponent `e`: the SHA-256 of its required members, in
/// order and without spaces.
fn thumbprint(n: &str, e: &str) -> String {
    let members = format!(r#"{{"e":"{e}","kty":"RSA","n":"{n}"}}"#);
    URL_SAFE_NO_PAD.encode(Sha256::digest(members.as_bytes()))
}

impl SecurityContext {
    /// A new key in memory: its tokens end with this process.
    pub fn generate() -> Result<Self> {
        Self::new(&new_key()?)
    }

    /// The key of the PEM file `path`: PKCS#8, as `openssl genpkey` and
    /// `uc-server keygen` make it, or PKCS#1, as Helm's `genPrivateKey`.
    pub fn from_pem_file(path: &Path) -> Result<Self> {
        let pem = std::fs::read_to_string(path)
            .map_err(|e| internal(&format!("read {}", path.display()), e))?;
        let key = RsaPrivateKey::from_pkcs8_pem(&pem)
            .or_else(|_| RsaPrivateKey::from_pkcs1_pem(&pem))
            .map_err(|_| {
                Error::internal(format!(
                    "{} is not an RSA private key in PEM",
                    path.display()
                ))
            })?;
        Self::new(&key)
    }

    fn new(key: &RsaPrivateKey) -> Result<Self> {
        let pkcs1 = key
            .to_pkcs1_der()
            .map_err(|e| internal("encode private key", e))?;
        let (n, e) = (
            URL_SAFE_NO_PAD.encode(key.n().to_bytes_be()),
            URL_SAFE_NO_PAD.encode(key.e().to_bytes_be()),
        );
        let decoding =
            DecodingKey::from_rsa_components(&n, &e).map_err(|e| internal("public key", e))?;
        let file_url_key = Sha256::new()
            .chain_update(FILE_URL_KEY_CONTEXT)
            .chain_update(pkcs1.as_bytes())
            .finalize()
            .into();
        Ok(Self {
            encoding: EncodingKey::from_rsa_der(pkcs1.as_bytes()),
            decoding,
            key_id: thumbprint(&n, &e),
            file_url_key,
        })
    }

    /// The signature (an HMAC-SHA256, in base64url) of Delta Sharing's URL
    /// for reading `file` until `expires_at` (epoch seconds).
    pub fn sign_file(&self, file: &str, expires_at: i64) -> String {
        let mac = self.file_mac(file, expires_at).finalize().into_bytes();
        URL_SAFE_NO_PAD.encode(mac)
    }

    /// Whether `signature` is [`Self::sign_file`]'s for `file` and
    /// `expires_at`, and `expires_at` has not passed.
    pub fn verify_file(&self, file: &str, expires_at: i64, signature: &str) -> bool {
        let Ok(mac) = URL_SAFE_NO_PAD.decode(signature) else {
            return false;
        };
        self.file_mac(file, expires_at).verify_slice(&mac).is_ok()
            && now_epoch_millis() / 1000 < expires_at
    }

    fn file_mac(&self, file: &str, expires_at: i64) -> Hmac<Sha256> {
        let mut mac =
            Hmac::<Sha256>::new_from_slice(&self.file_url_key).expect("HMAC takes any key length");
        mac.update(format!("{expires_at}\n{file}").as_bytes());
        mac
    }

    /// Upstream `createAccessToken`: a token for `principal` that expires
    /// after `ttl`.
    pub fn create_access_token(&self, principal: &str, ttl: Duration) -> Result<String> {
        let now = now_epoch_millis() / 1000;
        let claims = TokenClaims {
            sub: principal.to_string(),
            iss: INTERNAL_ISSUER.to_string(),
            iat: now,
            exp: now + ttl.as_secs() as i64,
            jti: Uuid::new_v4().to_string(),
            token_type: "ACCESS".to_string(),
        };
        jsonwebtoken::encode(&self.header(), &claims, &self.encoding)
            .map_err(|e| internal("sign token", e))
    }

    fn header(&self) -> Header {
        let mut header = Header::new(Algorithm::RS512);
        header.kid = Some(self.key_id.clone());
        header
    }

    /// `state`, signed for `ttl` with [`LOGIN_ISSUER`].
    pub fn sign_login_state<T: Serialize>(&self, state: &T, ttl: Duration) -> Result<String> {
        #[derive(Serialize)]
        struct Claims<'a, T> {
            iss: &'static str,
            exp: i64,
            #[serde(flatten)]
            state: &'a T,
        }
        let claims = Claims {
            iss: LOGIN_ISSUER,
            exp: now_epoch_millis() / 1000 + ttl.as_secs() as i64,
            state,
        };
        jsonwebtoken::encode(&self.header(), &claims, &self.encoding)
            .map_err(|e| internal("sign the sign-in state", e))
    }

    /// The state that [`Self::sign_login_state`] signed, if it has not
    /// expired.
    pub fn verify_login_state<T: DeserializeOwned>(&self, token: &str) -> Option<T> {
        let mut validation = Validation::new(Algorithm::RS512);
        validation.set_issuer(&[LOGIN_ISSUER]);
        validation.validate_aud = false;
        validation.leeway = 0;
        jsonwebtoken::decode::<T>(token, &self.decoding, &validation)
            .ok()
            .map(|data| data.claims)
    }

    /// Upstream `AuthDecorator`'s check of a UC token: one this server
    /// signed (`PERMISSION_DENIED` for another issuer), with an expiry that
    /// has not passed.
    pub fn verify(&self, token: &str) -> Result<TokenClaims> {
        let unauthenticated = || Error::new(ErrorCode::Unauthenticated, "Invalid access token.");
        let mut peek = Validation::new(Algorithm::RS512);
        peek.insecure_disable_signature_validation();
        peek.validate_exp = false;
        peek.validate_aud = false;
        peek.required_spec_claims.clear();
        let unverified = jsonwebtoken::decode::<serde_json::Value>(token, &self.decoding, &peek)
            .map_err(|_| unauthenticated())?;
        if unverified.claims.get("iss").and_then(|i| i.as_str()) != Some(INTERNAL_ISSUER) {
            return Err(Error::new(
                ErrorCode::PermissionDenied,
                "Invalid access token.",
            ));
        }
        // `Validation::new` requires `exp`.
        let mut validation = Validation::new(Algorithm::RS512);
        validation.set_issuer(&[INTERNAL_ISSUER]);
        validation.validate_aud = false;
        validation.leeway = 0;
        jsonwebtoken::decode::<TokenClaims>(token, &self.decoding, &validation)
            .map(|data| data.claims)
            .map_err(|_| unauthenticated())
    }
}

/// Upstream `UnityAccessUtil.initializeAdmin`: with authorization on, the
/// user `admin` exists and owns the metastore when it has no owner.
pub async fn initialize_admin(metastore: &uc_metastore::DynMetastore) -> Result<()> {
    use uc_api::scim::{ScimEmail, ScimUser, schema_uri};
    let scim = metastore.scim();
    if scim.find_user_by_email(ADMIN).await?.is_none() {
        let admin = ScimUser {
            schemas: vec![schema_uri::USER.to_string()],
            user_name: ADMIN.to_string(),
            display_name: Some("Admin".to_string()),
            emails: Some(vec![ScimEmail {
                value: ADMIN.to_string(),
                primary: Some(true),
                r#type: None,
            }]),
            active: Some(true),
            ..Default::default()
        };
        // Another replica may create it at the same time.
        match scim.create_user(admin).await {
            Err(e) if e.code().is_already_exists() => {}
            other => {
                other?;
            }
        }
    }
    let permissions = metastore.permissions();
    if permissions.metastore_owner().await?.is_none() {
        permissions.set_metastore_owner(ADMIN).await?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The example of RFC 7638, section 3.1.
    #[test]
    fn the_key_id_is_the_rfc_7638_thumbprint() {
        let n = "0vx7agoebGcQSuuPiLJXZptN9nndrQmbXEps2aiAFbWhM78LhWx4cbbfAAtVT86zwu1RK7aPFFxuhDR1L6tSoc_BJECPebWKRXjBZCiFV4n3oknjhMstn64tZ_2W-5JsGY4Hc5n9yBXArwl93lqt7_RN5w6Cf0h4QyQ5v-65YGjQR0_FDW2QvzqY368QQMicAtaSqzs8KJZgnYb9c7d0zgdAZHzu6qMQvRL5hajrn1n91CbOpbISD08qNLyrdkt-bFTWhAI4vMQFh6WeZu0fM4lFd2NcRwr3XPksINHaQ-G_xBniIqbw0Ls1jF44-csFCur-kEgU8awapJzKnqDKgw";
        assert_eq!(
            thumbprint(n, "AQAB"),
            "NzbLsXh8uDCcd-6MNwXF4W_7noWXFZAfHkxZsRGC9Xs"
        );
    }

    /// Helm's chart mounts a PKCS#1 PEM key read-only; `uc-server keygen`
    /// prints PKCS#8. The server reads either and writes nothing.
    #[test]
    fn loads_a_pem_key_from_a_read_only_folder() {
        let directory = tempfile::tempdir().unwrap();
        let key = new_key().unwrap();
        let pkcs1 = directory.path().join("pkcs1.pem");
        std::fs::write(&pkcs1, key.to_pkcs1_pem(LineEnding::LF).unwrap().as_bytes()).unwrap();
        let pkcs8 = directory.path().join("pkcs8.pem");
        std::fs::write(&pkcs8, key.to_pkcs8_pem(LineEnding::LF).unwrap().as_bytes()).unwrap();
        #[cfg(unix)]
        let mode = |mode: u32| {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(directory.path(), std::fs::Permissions::from_mode(mode))
                .unwrap();
        };
        #[cfg(unix)]
        mode(0o500);
        let (first, second) = (
            SecurityContext::from_pem_file(&pkcs1),
            SecurityContext::from_pem_file(&pkcs8),
        );
        #[cfg(unix)]
        mode(0o700);
        let (first, second) = (first.unwrap(), second.unwrap());
        assert_eq!(first.key_id, second.key_id);
        let token = first
            .create_access_token(ADMIN, Duration::from_secs(60))
            .unwrap();
        assert_eq!(second.verify(&token).unwrap().sub, ADMIN);
        let files: Vec<_> = std::fs::read_dir(directory.path()).unwrap().collect();
        assert_eq!(files.len(), 2);

        let generated = directory.path().join("generated.pem");
        std::fs::write(&generated, generate_key_pem().unwrap()).unwrap();
        assert!(SecurityContext::from_pem_file(&generated).is_ok());
        let error = SecurityContext::from_pem_file(&directory.path().join("missing.pem"))
            .err()
            .unwrap();
        assert!(
            error.message().contains("missing.pem"),
            "{}",
            error.message()
        );
        std::fs::write(&generated, "not a key").unwrap();
        let error = SecurityContext::from_pem_file(&generated).err().unwrap();
        assert!(
            error
                .message()
                .ends_with("is not an RSA private key in PEM"),
            "{}",
            error.message()
        );
    }

    #[test]
    fn tokens_expire_and_name_their_key() {
        let context = SecurityContext::generate().unwrap();
        let token = context
            .create_access_token("alice@example.com", Duration::from_secs(60))
            .unwrap();
        let claims = context.verify(&token).unwrap();
        assert_eq!(
            (claims.sub.as_str(), claims.token_type.as_str()),
            ("alice@example.com", "ACCESS")
        );
        assert_eq!(claims.exp, claims.iat + 60);
        let header = jsonwebtoken::decode_header(&token).unwrap();
        assert_eq!(header.kid.as_deref(), Some(context.key_id.as_str()));

        // Another key, as after a restart without `auth.signing_key_file`.
        let other = SecurityContext::generate().unwrap();
        assert_eq!(
            other.verify(&token).unwrap_err().code(),
            ErrorCode::Unauthenticated
        );
        let expired = context
            .create_access_token("bob", Duration::from_secs(0))
            .unwrap();
        std::thread::sleep(Duration::from_millis(1100));
        assert_eq!(
            context.verify(&expired).unwrap_err().code(),
            ErrorCode::Unauthenticated
        );
        // A token without `exp`, as upstream's service token, never counts.
        let lasting = jsonwebtoken::encode(
            &context.header(),
            &serde_json::json!({
                "sub": ADMIN, "iss": INTERNAL_ISSUER, "iat": 0, "jti": "x", "type": "SERVICE",
            }),
            &context.encoding,
        )
        .unwrap();
        assert_eq!(
            context.verify(&lasting).unwrap_err().code(),
            ErrorCode::Unauthenticated
        );
        let foreign = jsonwebtoken::encode(
            &Header::default(),
            &serde_json::json!({ "iss": "https://idp", "sub": "x" }),
            &EncodingKey::from_secret(b"k"),
        )
        .unwrap();
        assert_eq!(
            context.verify(&foreign).unwrap_err().code(),
            ErrorCode::PermissionDenied
        );
    }

    #[test]
    fn file_signatures_hold_for_their_file_until_they_expire() {
        let context = SecurityContext::generate().unwrap();
        let later = now_epoch_millis() / 1000 + 60;
        let file = "file:///tmp/t/part-0.parquet";
        let signature = context.sign_file(file, later);
        assert!(context.verify_file(file, later, &signature));

        let other = SecurityContext::generate().unwrap();
        assert!(!other.verify_file(file, later, &signature));
        // Another file or expiry with the same signature.
        assert!(!context.verify_file("file:///etc/passwd", later, &signature));
        assert!(!context.verify_file(file, later + 3600, &signature));
        assert!(!context.verify_file(file, later, "not base64!"));
        let earlier = now_epoch_millis() / 1000 - 1;
        let expired = context.sign_file(file, earlier);
        assert!(!context.verify_file(file, earlier, &expired));
    }
}
