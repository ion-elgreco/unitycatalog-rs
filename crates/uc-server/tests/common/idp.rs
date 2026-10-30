//! An identity provider for tests: an RSA, a P-256 and a P-521 signing key
//! behind OIDC discovery and a JWK set. It counts the fetches of its keys.

use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use std::time::{SystemTime, UNIX_EPOCH};

use axum::{Json, Router, extract::State, routing::get};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use jsonwebtoken::{EncodingKey, Header};
use rsa::{RsaPrivateKey, pkcs1::EncodeRsaPrivateKey, traits::PublicKeyParts};
use serde_json::{Value, json};

pub struct Idp {
    pub issuer: String,
    rsa: EncodingKey,
    p256: EncodingKey,
    p521: p521::ecdsa::SigningKey,
    fetches: Arc<AtomicUsize>,
}

/// What the provider serves.
#[derive(Clone)]
struct Served {
    config: Value,
    jwks: Value,
    fetches: Arc<AtomicUsize>,
}

impl Idp {
    pub async fn start() -> Self {
        let rsa = RsaPrivateKey::new(&mut rand::rngs::OsRng, 2048).unwrap();
        let p256 = p256::ecdsa::SigningKey::random(&mut rand::rngs::OsRng);
        let p521 = p521::ecdsa::SigningKey::random(&mut rand::rngs::OsRng);
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let issuer = format!("http://{}", listener.local_addr().unwrap());
        let b64 = |bytes: &[u8]| URL_SAFE_NO_PAD.encode(bytes);
        let p256_point = p256.verifying_key().to_encoded_point(false);
        let p521_point = p521::ecdsa::VerifyingKey::from(&p521).to_encoded_point(false);
        let jwks = json!({ "keys": [
            { "kty": "RSA", "kid": "rsa", "use": "sig", "alg": "RS256",
              "n": b64(&rsa.n().to_bytes_be()), "e": b64(&rsa.e().to_bytes_be()) },
            { "kty": "EC", "kid": "p256", "use": "sig", "crv": "P-256",
              "x": b64(p256_point.x().unwrap()), "y": b64(p256_point.y().unwrap()) },
            { "kty": "EC", "kid": "p521", "use": "sig", "crv": "P-521",
              "x": b64(p521_point.x().unwrap()), "y": b64(p521_point.y().unwrap()) },
        ] });
        let fetches = Arc::new(AtomicUsize::new(0));
        let served = Served {
            config: json!({ "issuer": issuer, "jwks_uri": format!("{issuer}/jwks") }),
            jwks,
            fetches: fetches.clone(),
        };
        let app = Router::new()
            .route(
                "/.well-known/openid-configuration",
                get(|State(served): State<Served>| async move { Json(served.config) }),
            )
            .route(
                "/jwks",
                get(|State(served): State<Served>| async move {
                    served.fetches.fetch_add(1, Ordering::SeqCst);
                    Json(served.jwks)
                }),
            )
            .with_state(served);
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        let p256_der = p256::pkcs8::EncodePrivateKey::to_pkcs8_der(&p256).unwrap();
        Self {
            issuer,
            rsa: EncodingKey::from_rsa_der(rsa.to_pkcs1_der().unwrap().as_bytes()),
            p256: EncodingKey::from_ec_der(p256_der.as_bytes()),
            p521,
            fetches,
        }
    }

    /// Seconds since the epoch.
    pub fn now() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
    }

    /// How often the keys were fetched.
    pub fn fetches(&self) -> usize {
        self.fetches.load(Ordering::SeqCst)
    }

    /// The claims of a token: this issuer, the audience `uc`, valid for ten
    /// minutes, with `extra` over them.
    pub fn claims(&self, extra: Value) -> Value {
        let now = Self::now();
        let mut claims = json!({ "iss": self.issuer, "aud": "uc", "iat": now, "exp": now + 600 });
        claims
            .as_object_mut()
            .unwrap()
            .extend(extra.as_object().unwrap().clone());
        claims
    }

    /// An RS256 token with these claims (over the defaults).
    pub fn token(&self, extra: Value) -> String {
        self.token_with_kid("rsa", extra)
    }

    /// An RS256 token whose header names the key `kid`: the provider
    /// publishes only `rsa`.
    pub fn token_with_kid(&self, kid: &str, extra: Value) -> String {
        let mut header = Header::new(jsonwebtoken::Algorithm::RS256);
        header.kid = Some(kid.into());
        jsonwebtoken::encode(&header, &self.claims(extra), &self.rsa).unwrap()
    }

    pub fn es256(&self, extra: Value) -> String {
        let mut header = Header::new(jsonwebtoken::Algorithm::ES256);
        header.kid = Some("p256".into());
        jsonwebtoken::encode(&header, &self.claims(extra), &self.p256).unwrap()
    }

    pub fn es512(&self, extra: Value) -> String {
        use p521::ecdsa::{Signature, signature::Signer};
        let header = b64_json(&json!({ "alg": "ES512", "typ": "JWT", "kid": "p521" }));
        let message = format!("{header}.{}", b64_json(&self.claims(extra)));
        let signature: Signature = self.p521.sign(message.as_bytes());
        format!("{message}.{}", URL_SAFE_NO_PAD.encode(signature.to_bytes()))
    }
}

fn b64_json(value: &Value) -> String {
    URL_SAFE_NO_PAD.encode(value.to_string())
}
