//! The database store (`store = "database"`): each secret in its own row,
//! sealed with the keys of `secrets.key_file`. The sealed secret is its own
//! reference.
//!
//! A secret is sealed with HPKE (RFC 9180): DHKEM(P-256, HKDF-SHA256),
//! HKDF-SHA256 and AES-256-GCM, to the public key of the file's first key,
//! with the credential's id as associated data. Only the private key opens
//! it, and only for that credential. The file may hold several keys, newest
//! first: the first seals, and each opens what it sealed. A sealed secret
//! names its key, so [`SecretStore::refresh`] can seal the secrets of older
//! keys again with the first one.

use std::fmt;

use async_trait::async_trait;
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use hpke::{
    Deserializable, Kem as _, OpModeR, OpModeS, Serializable, aead::AesGcm256, kdf::HkdfSha256,
    kem::DhP256HkdfSha256,
};
use p256::SecretKey;
use p256::elliptic_curve::{Generate, sec1::ToSec1Point};
use p256::pkcs8::{DecodePrivateKey, EncodePrivateKey, LineEnding};
use sha2::{Digest, Sha256};

use super::{SecretRef, SecretStore, SecretValue};
use uc_common::{Error, Result};

type Kem = DhP256HkdfSha256;

/// The first part of a sealed secret, which is
/// `v1.<key id>.<encapsulated key>.<ciphertext>`, each part after the first
/// in base64url without padding.
const VERSION: &str = "v1";
/// HPKE's `info`: what the keys seal.
const INFO: &[u8] = b"unitycatalog-rs secret of a storage credential";

/// The RFC 7638 thumbprint of the public key of `key`.
fn key_id(key: &SecretKey) -> String {
    let point = key.public_key().to_sec1_point(false);
    let coordinate = |c: Option<&[u8]>| URL_SAFE_NO_PAD.encode(c.unwrap_or_default());
    let (x, y) = (
        coordinate(point.x().map(|x| x.as_slice())),
        coordinate(point.y().map(|y| y.as_slice())),
    );
    let members = format!(r#"{{"crv":"P-256","kty":"EC","x":"{x}","y":"{y}"}}"#);
    URL_SAFE_NO_PAD.encode(Sha256::digest(members.as_bytes()))
}

struct SealingKey {
    id: String,
    private: <Kem as hpke::Kem>::PrivateKey,
    public: <Kem as hpke::Kem>::PublicKey,
}

/// The database store, with the keys of `secrets.key_file`, newest first.
pub struct DatabaseStore {
    /// Never empty.
    keys: Vec<SealingKey>,
}

impl fmt::Debug for DatabaseStore {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DatabaseStore")
            .field("key_ids", &self.ids().collect::<Vec<_>>())
            .finish()
    }
}

/// Why a sealed secret did not open.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OpenError {
    /// The text is not a sealed secret.
    Malformed,
    /// No key of the file has this id.
    UnknownKey(String),
    /// Its key does not open it: the text or its row changed.
    Corrupt,
}

impl fmt::Display for OpenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Malformed => write!(f, "it is not a sealed secret"),
            Self::UnknownKey(id) => write!(f, "its key {id} is not in secrets.key_file"),
            Self::Corrupt => write!(f, "its key does not open it"),
        }
    }
}

impl OpenError {
    /// A missing key is the settings' fault; anything else is the data's.
    fn into_error(self) -> Error {
        match self {
            Self::UnknownKey(_) => Error::failed_precondition(self.to_string()),
            Self::Malformed | Self::Corrupt => Error::internal(self.to_string()),
        }
    }
}

impl DatabaseStore {
    /// `keys`, newest first: the first seals.
    pub fn new(keys: Vec<SecretKey>) -> Result<Self> {
        if keys.is_empty() {
            return Err(Error::invalid_argument("A secrets key file needs a key."));
        }
        let keys = keys
            .iter()
            .map(|key| {
                let private = <Kem as hpke::Kem>::PrivateKey::from_bytes(&key.to_bytes())
                    .map_err(|e| Error::internal(format!("cannot use a secrets key: {e}")))?;
                Ok(SealingKey {
                    id: key_id(key),
                    public: Kem::sk_to_pk(&private),
                    private,
                })
            })
            .collect::<Result<_>>()?;
        Ok(Self { keys })
    }

    /// The keys of `pem`, P-256 keys in PEM, newest first: PKCS#8, as
    /// `uc-server keygen --secrets` and `openssl genpkey` make them, or
    /// SEC1, as Helm's `genPrivateKey "ecdsa"`.
    pub fn from_pem(pem: &str) -> Result<Self> {
        let keys = uc_common::pem::blocks(pem)
            .map(|block| {
                SecretKey::from_pkcs8_pem(block)
                    .or_else(|_| SecretKey::from_sec1_pem(block))
                    .map_err(|_| {
                        Error::invalid_argument("a key in it is not a P-256 private key in PEM")
                    })
            })
            .collect::<Result<Vec<_>>>()?;
        match keys.is_empty() {
            true => Err(Error::invalid_argument(
                "it holds no P-256 private key in PEM",
            )),
            false => Self::new(keys),
        }
    }

    /// A store with a new key in memory: what it seals cannot be opened
    /// once the process ends.
    pub fn generate() -> Result<Self> {
        Self::new(vec![new_key()?])
    }

    /// A new key, as PKCS#8 PEM: for `secrets.key_file`.
    pub fn generate_key_pem() -> Result<String> {
        let pem = new_key()?
            .to_pkcs8_pem(LineEnding::LF)
            .map_err(|e| Error::internal(format!("cannot encode a secrets key: {e}")))?;
        Ok(pem.to_string())
    }

    /// The ids of the keys, the sealing key first.
    pub fn ids(&self) -> impl Iterator<Item = &str> {
        self.keys.iter().map(|key| key.id.as_str())
    }

    /// The id of the key that seals.
    pub fn current_id(&self) -> &str {
        &self.keys[0].id
    }

    /// `secret`, sealed with the first key for the row `context`: only
    /// [`Self::open`] with the same context opens it.
    pub fn seal(&self, secret: &[u8], context: &str) -> Result<String> {
        let key = &self.keys[0];
        let (encapsulated, ciphertext) = hpke::single_shot_seal::<AesGcm256, HkdfSha256, Kem>(
            &OpModeS::Base,
            &key.public,
            INFO,
            secret,
            context.as_bytes(),
        )
        .map_err(|e| Error::internal(format!("cannot seal a secret: {e}")))?;
        Ok(format!(
            "{VERSION}.{}.{}.{}",
            key.id,
            URL_SAFE_NO_PAD.encode(encapsulated.to_bytes()),
            URL_SAFE_NO_PAD.encode(ciphertext)
        ))
    }

    /// The secret that [`Self::seal`] sealed for `context`.
    pub fn open(&self, sealed: &str, context: &str) -> std::result::Result<SecretValue, OpenError> {
        let [version, id, encapsulated, ciphertext] = parts(sealed)?;
        if version != VERSION {
            return Err(OpenError::Malformed);
        }
        let key = self
            .keys
            .iter()
            .find(|key| key.id == id)
            .ok_or_else(|| OpenError::UnknownKey(id.to_string()))?;
        let decode = |part: &str| {
            URL_SAFE_NO_PAD
                .decode(part)
                .map_err(|_| OpenError::Malformed)
        };
        let encapsulated = <Kem as hpke::Kem>::EncappedKey::from_bytes(&decode(encapsulated)?)
            .map_err(|_| OpenError::Malformed)?;
        let secret = SecretValue::new(
            hpke::single_shot_open::<AesGcm256, HkdfSha256, Kem>(
                &OpModeR::Base,
                &key.private,
                &encapsulated,
                INFO,
                &decode(ciphertext)?,
                context.as_bytes(),
            )
            .map_err(|_| OpenError::Corrupt)?,
        );
        // The stored secrets are text.
        secret
            .sensitive_value_to_str()
            .map_err(|_| OpenError::Corrupt)?;
        Ok(secret)
    }

    /// Whether the first key sealed `sealed`.
    pub fn is_current(&self, sealed: &str) -> bool {
        parts(sealed).is_ok_and(|[_, id, ..]| id == self.current_id())
    }
}

fn new_key() -> Result<SecretKey> {
    SecretKey::try_generate()
        .map_err(|e| Error::internal(format!("cannot make a secrets key: {e}")))
}

fn parts(sealed: &str) -> std::result::Result<[&str; 4], OpenError> {
    let parts: Vec<&str> = sealed.split('.').collect();
    parts.try_into().map_err(|_| OpenError::Malformed)
}

#[async_trait]
impl SecretStore for DatabaseStore {
    async fn put(&self, owner: &str, secret: &SecretValue) -> Result<SecretRef> {
        self.seal(secret.as_sensitive_bytes(), owner)
            .map(SecretRef::new)
    }

    async fn get(&self, owner: &str, reference: &SecretRef) -> Result<SecretValue> {
        self.open(reference.as_str(), owner)
            .map_err(OpenError::into_error)
    }

    /// The row held the secret: nothing else to remove.
    async fn delete(&self, _: &str, _: &SecretRef) -> Result<()> {
        Ok(())
    }

    fn refreshes(&self) -> bool {
        true
    }

    async fn refresh(&self, owner: &str, reference: &SecretRef) -> Result<Option<SecretRef>> {
        if self.is_current(reference.as_str()) {
            return Ok(None);
        }
        let secret = self.get(owner, reference).await?;
        self.put(owner, &secret).await.map(Some)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store(keys: &[&SecretKey]) -> DatabaseStore {
        DatabaseStore::new(keys.iter().map(|&key| key.clone()).collect()).unwrap()
    }

    fn open(
        store: &DatabaseStore,
        sealed: &str,
        row: &str,
    ) -> std::result::Result<String, OpenError> {
        store
            .open(sealed, row)
            .map(|secret| secret.as_sensitive_str().to_string())
    }

    #[test]
    fn a_sealed_secret_opens_only_with_its_key_and_row() {
        let (old, new) = (new_key().unwrap(), new_key().unwrap());
        let before = store(&[&old]);
        let sealed = before.seal(b"s3cret", "row-1").unwrap();
        assert!(!sealed.contains("s3cret"));
        assert_ne!(sealed, before.seal(b"s3cret", "row-1").unwrap());
        assert_eq!(open(&before, &sealed, "row-1").unwrap(), "s3cret");
        assert_eq!(open(&before, &sealed, "row-2"), Err(OpenError::Corrupt));

        // A new key first: the old one still opens, the new one seals.
        let rotated = store(&[&new, &old]);
        assert_eq!(open(&rotated, &sealed, "row-1").unwrap(), "s3cret");
        assert!(!rotated.is_current(&sealed));
        let resealed = rotated.seal(b"s3cret", "row-1").unwrap();
        assert!(rotated.is_current(&resealed));
        assert_eq!(
            open(&before, &resealed, "row-1"),
            Err(OpenError::UnknownKey(rotated.current_id().to_string()))
        );

        // Without the old key, its secrets stay closed.
        let only_new = store(&[&new]);
        assert_eq!(
            open(&only_new, &sealed, "row-1"),
            Err(OpenError::UnknownKey(before.current_id().to_string()))
        );
        assert_eq!(
            open(&only_new, "plain text", "row-1"),
            Err(OpenError::Malformed)
        );
        let mut changed = resealed.clone();
        let at = changed.rfind('.').unwrap() + 1;
        let flipped = if changed[at..].starts_with('A') {
            "B"
        } else {
            "A"
        };
        changed.replace_range(at..at + 1, flipped);
        assert_eq!(open(&rotated, &changed, "row-1"), Err(OpenError::Corrupt));
    }

    #[tokio::test]
    async fn refresh_seals_the_secrets_of_older_keys_again() {
        let (old, new) = (new_key().unwrap(), new_key().unwrap());
        let before = store(&[&old]);
        let secret = SecretValue::from("s3cret");
        let reference = before.put("row-1", &secret).await.unwrap();
        assert_eq!(before.refresh("row-1", &reference).await.unwrap(), None);

        let rotated = store(&[&new, &old]);
        let refreshed = rotated
            .refresh("row-1", &reference)
            .await
            .unwrap()
            .expect("a new reference");
        assert!(rotated.is_current(refreshed.as_str()));
        assert_eq!(rotated.get("row-1", &refreshed).await.unwrap(), secret);
        // A missing key is a settings error, a changed row a data error.
        let error = before.get("row-1", &refreshed).await.unwrap_err();
        assert_eq!(error.code(), uc_common::ErrorCode::FailedPrecondition);
        let error = rotated.get("row-2", &refreshed).await.unwrap_err();
        assert_eq!(error.code(), uc_common::ErrorCode::Internal);
    }

    /// keygen's PKCS#8 and Helm's SEC1 mix in one file; an RSA key is no
    /// secrets key.
    #[test]
    fn a_key_file_holds_p256_keys() {
        let new = DatabaseStore::generate_key_pem().unwrap();
        let old = new_key().unwrap().to_sec1_pem(LineEnding::LF).unwrap();
        let both = DatabaseStore::from_pem(&format!("{new}\n{}", old.as_str())).unwrap();
        let only_new = DatabaseStore::from_pem(&new).unwrap();
        assert_eq!(both.ids().count(), 2);
        assert_eq!(both.current_id(), only_new.current_id());

        let error = DatabaseStore::from_pem("not a key").unwrap_err();
        assert!(
            error.message().contains("no P-256 private key"),
            "{error:?}"
        );
        let rsa =
            "-----BEGIN RSA PRIVATE KEY-----\nMIIBOgIBAAJBAK\n-----END RSA PRIVATE KEY-----\n";
        let error = DatabaseStore::from_pem(rsa).unwrap_err();
        assert!(
            error.message().contains("not a P-256 private key"),
            "{error:?}"
        );
    }

    /// The EC private key of RFC 7517, appendix A.2; its thumbprint is the
    /// SHA-256 of `{"crv":"P-256","kty":"EC","x":…,"y":…}`.
    #[test]
    fn the_key_id_is_the_rfc_7638_thumbprint() {
        let d = URL_SAFE_NO_PAD
            .decode("870MB6gfuTJ4HtUnUvYMyJpr5eUZNP4Bk43bVdj3eAE")
            .unwrap();
        let key = SecretKey::from_slice(&d).unwrap();
        assert_eq!(key_id(&key), "cn-I_WNMClehiVp51i_0VpOENW1upEerA8sEam5hn-s");
    }
}
