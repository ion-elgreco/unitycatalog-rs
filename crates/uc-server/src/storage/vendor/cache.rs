//! A cache of vended credentials, after upstream
//! `CachingCloudCredentialVendor`: a request for the same locations,
//! privileges and identity gets the credential vended before, in place of a
//! new call to the cloud.
//!
//! Only the call to the cloud is skipped. The privileges of the caller are
//! checked, and the external location and storage credential of the path
//! are read, on every request. The key holds the identity that the vendor
//! uses: a changed role, external ID, service principal, managed identity,
//! service account or key is another key. Secrets are in the key as their
//! SHA-256 digests.
//!
//! An entry is given until `min(vend start + max age, expiry - 1 minute)`.
//! A credential without an expiry is bounded by the max age alone, and one
//! that would be given for no time is not cached. A failed vend is not
//! cached, and a stale entry is never given in place of a failed renewal.
//! Requests that miss at the same time wait for one vend.

use std::sync::Arc;
use std::time::{Duration, Instant};

use async_trait::async_trait;
use moka::future::Cache;
use moka::ops::compute::{CompResult, Op};
use sha2::{Digest, Sha256};

use uc_api::TemporaryCredentials;
use uc_common::settings::{self, Storage};
use uc_common::{Error, NormalizedUrl, Result, UriScheme, now_epoch_millis};

use super::{
    CredentialContext, CredentialVendor, DynVendor, StoragePrivilege, aws, aws::S3Store, azure, gcp,
};

/// A cached credential is never given within this time of its expiry.
const EXPIRY_MARGIN_MS: i64 = 60_000;

/// `storage.credential_cache_*`, parsed: the cache is on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CredentialCacheConfig {
    /// `storage.credential_cache_max_size`: the most credentials kept.
    pub max_size: u64,
    /// `storage.credential_cache_max_age`: the longest a credential is given
    /// again, from when it was vended.
    pub max_age: Duration,
}

impl CredentialCacheConfig {
    pub const DEFAULT_MAX_SIZE: u64 = 1000;
    pub const DEFAULT_MAX_AGE: Duration = Duration::from_secs(5 * 60);

    /// None when `storage.credential_cache_enabled` is false.
    pub fn from_settings(storage: &Storage) -> std::result::Result<Option<Self>, String> {
        let max_size = storage
            .credential_cache_max_size
            .unwrap_or(Self::DEFAULT_MAX_SIZE);
        if max_size == 0 {
            return Err(settings::invalid(
                "storage.credential_cache_max_size",
                0,
                "Expected 1 or more; set storage.credential_cache_enabled to false for no cache.",
            ));
        }
        let max_age = storage
            .credential_cache_max_age
            .unwrap_or(Self::DEFAULT_MAX_AGE);
        if max_age.is_zero() {
            return Err(settings::invalid(
                "storage.credential_cache_max_age",
                "0s",
                "Expected a positive duration; set storage.credential_cache_enabled to false for \
                 no cache.",
            ));
        }
        Ok(storage
            .credential_cache_enabled
            .then_some(Self { max_size, max_age }))
    }
}

/// The identity that a credential is vended with.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum Binding {
    /// A location of the settings, which do not change while the server
    /// runs.
    Settings,
    Aws(aws::Role),
    Azure(azure::SignInKey),
    GcpAccount(Option<String>),
    GcpKey([u8; 32]),
}

impl Binding {
    /// The identity that the vendor uses for `ctx`, as `CloudVendor` finds
    /// it. None when it cannot find one: the vend fails, or vends nothing.
    fn of(ctx: &CredentialContext) -> Option<Self> {
        if ctx.scheme == UriScheme::File {
            return None;
        }
        let Some(credential) = &ctx.credential else {
            return Some(Self::Settings);
        };
        match ctx.scheme {
            UriScheme::S3 => aws::Role::of(credential).ok().map(Self::Aws),
            UriScheme::Abfs | UriScheme::Abfss => {
                let identity = azure::Identity::of(credential).ok()?;
                Some(Self::Azure((&identity).into()))
            }
            UriScheme::Gs => match gcp::Identity::of(credential).ok()? {
                gcp::Identity::ServiceAccount(email) => Some(Self::GcpAccount(email)),
                gcp::Identity::Key(key) => Some(Self::GcpKey(
                    Sha256::digest(key.as_sensitive_bytes()).into(),
                )),
            },
            UriScheme::File => None,
        }
    }
}

/// What decides a vended credential.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct Key {
    /// Every location of the request: AWS and GCP scope a credential to all
    /// of them.
    locations: Vec<String>,
    /// Sorted, without repeats.
    privileges: Vec<StoragePrivilege>,
    binding: Binding,
}

impl Key {
    fn of(ctx: &CredentialContext) -> Option<Self> {
        let mut privileges = ctx.privileges.clone();
        privileges.sort();
        privileges.dedup();
        Some(Self {
            locations: ctx
                .locations
                .iter()
                .map(|l| l.as_str().to_string())
                .collect(),
            privileges,
            binding: Binding::of(ctx)?,
        })
    }
}

#[derive(Clone)]
struct Entry {
    credentials: TemporaryCredentials,
    /// The end of the time it is given, in epoch milliseconds.
    serve_until: i64,
}

/// The time in epoch milliseconds.
type Clock = Arc<dyn Fn() -> i64 + Send + Sync>;

/// Drops each entry at the end of the time it is given.
struct UntilServeDeadline {
    clock: Clock,
}

impl UntilServeDeadline {
    fn left(&self, entry: &Entry) -> Option<Duration> {
        let left = u64::try_from(entry.serve_until - (self.clock)()).unwrap_or(0);
        Some(Duration::from_millis(left))
    }
}

impl moka::Expiry<Key, Entry> for UntilServeDeadline {
    fn expire_after_create(&self, _key: &Key, entry: &Entry, _at: Instant) -> Option<Duration> {
        self.left(entry)
    }

    fn expire_after_update(
        &self,
        _key: &Key,
        entry: &Entry,
        _at: Instant,
        _left: Option<Duration>,
    ) -> Option<Duration> {
        self.left(entry)
    }
}

/// A vendor that gives the credentials of `inner` again; see the module.
pub struct CachingVendor {
    inner: DynVendor,
    entries: Cache<Key, Entry>,
    max_age_ms: i64,
    clock: Clock,
}

impl CachingVendor {
    pub fn new(inner: impl CredentialVendor, config: CredentialCacheConfig) -> Self {
        Self::with_clock(inner, config, Arc::new(now_epoch_millis))
    }

    fn with_clock(
        inner: impl CredentialVendor,
        config: CredentialCacheConfig,
        clock: Clock,
    ) -> Self {
        let entries = Cache::builder()
            .max_capacity(config.max_size)
            .expire_after(UntilServeDeadline {
                clock: clock.clone(),
            })
            .build();
        Self {
            inner: Arc::new(inner),
            entries,
            max_age_ms: i64::try_from(config.max_age.as_millis()).unwrap_or(i64::MAX),
            clock,
        }
    }
}

#[async_trait]
impl CredentialVendor for CachingVendor {
    async fn vend(&self, ctx: &CredentialContext) -> Result<TemporaryCredentials> {
        let Some(key) = Key::of(ctx) else {
            return self.inner.vend(ctx).await;
        };
        let fresh = |entry: &Entry| entry.serve_until > (self.clock)();
        // The entry's own deadline too: the cache drops it by the clock it
        // read when the entry came.
        if let Some(entry) = self.entries.get(&key).await
            && fresh(&entry)
        {
            return Ok(entry.credentials);
        }
        // Requests for one key take turns here, so those that miss at the
        // same time wait for one vend.
        let result = self
            .entries
            .entry(key)
            .and_try_compute_with(|cached| async move {
                if cached.is_some_and(|entry| fresh(entry.value())) {
                    return Ok(Op::Nop);
                }
                // The cloud checks the identity (the trust policy of a role,
                // say) during the call, so the max age counts from before it.
                let start = (self.clock)();
                let credentials = self.inner.vend(ctx).await?;
                let mut serve_until = start.saturating_add(self.max_age_ms);
                if let Some(expires_at) = credentials.expiration_time {
                    serve_until = serve_until.min(expires_at - EXPIRY_MARGIN_MS);
                }
                // A credential already past its time ends at once.
                Ok::<_, Error>(Op::Put(Entry {
                    credentials,
                    serve_until,
                }))
            })
            .await?;
        match result {
            CompResult::Unchanged(entry)
            | CompResult::Inserted(entry)
            | CompResult::ReplacedWith(entry) => Ok(entry.into_value().credentials),
            CompResult::StillNone(_) | CompResult::Removed(_) => {
                Err(Error::internal("the credential cache lost an entry"))
            }
        }
    }

    fn s3_store(&self) -> Option<&S3Store> {
        self.inner.s3_store()
    }

    fn azure_blob(&self) -> Option<&str> {
        self.inner.azure_blob()
    }

    fn covers(&self, path: &NormalizedUrl) -> bool {
        self.inner.covers(path)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;
    use std::sync::atomic::{AtomicI64, AtomicUsize, Ordering};

    use uc_api::{AwsCredentials, AwsIamRoleResponse, CredentialInfo};
    use uc_common::Error;

    use super::super::StorageCredential;
    use super::*;

    const MINUTE: i64 = 60_000;

    /// The fixture's clock at `minutes` after its start.
    fn at(minutes: i64) -> i64 {
        1_000 * MINUTE + minutes * MINUTE
    }

    /// A vendor that counts its vends and answers with `answers` in turn,
    /// the last one again when they run out. Each credential's session
    /// token is the number of its vend.
    struct Counting {
        vends: Arc<AtomicUsize>,
        answers: Mutex<Vec<std::result::Result<Option<i64>, ()>>>,
    }

    #[async_trait]
    impl CredentialVendor for Counting {
        async fn vend(&self, _ctx: &CredentialContext) -> Result<TemporaryCredentials> {
            let n = self.vends.fetch_add(1, Ordering::SeqCst) + 1;
            // Let other requests run during the call, as a cloud call does.
            tokio::task::yield_now().await;
            let answer = {
                let mut answers = self.answers.lock().unwrap();
                match answers.len() {
                    1 => answers[0],
                    _ => answers.remove(0),
                }
            };
            let expiration_time = answer.map_err(|()| Error::internal("down"))?;
            Ok(TemporaryCredentials {
                aws_temp_credentials: Some(AwsCredentials {
                    session_token: Some(n.to_string()),
                    ..Default::default()
                }),
                expiration_time,
                ..Default::default()
            })
        }
    }

    struct Fixture {
        vendor: CachingVendor,
        vends: Arc<AtomicUsize>,
        now: Arc<AtomicI64>,
    }

    impl Fixture {
        /// The cache with a max age of 5 minutes over a vendor that answers
        /// with `answers`: credentials expiring then, or a failure.
        fn new(answers: Vec<std::result::Result<Option<i64>, ()>>) -> Self {
            let vends = Arc::new(AtomicUsize::new(0));
            let now = Arc::new(AtomicI64::new(at(0)));
            let clock = {
                let now = now.clone();
                Arc::new(move || now.load(Ordering::SeqCst))
            };
            let inner = Counting {
                vends: vends.clone(),
                answers: Mutex::new(answers),
            };
            let config = CredentialCacheConfig {
                max_size: 100,
                max_age: Duration::from_secs(5 * 60),
            };
            Self {
                vendor: CachingVendor::with_clock(inner, config, clock),
                vends,
                now,
            }
        }

        fn set(&self, millis: i64) {
            self.now.store(millis, Ordering::SeqCst);
        }

        /// The session token, that is the vend that made the credential.
        async fn vend(&self, ctx: &CredentialContext) -> Result<String> {
            let credentials = self.vendor.vend(ctx).await?;
            Ok(credentials
                .aws_temp_credentials
                .unwrap()
                .session_token
                .unwrap())
        }

        fn vends(&self) -> usize {
            self.vends.load(Ordering::SeqCst)
        }
    }

    fn role(arn: &str, external_id: &str) -> Option<StorageCredential> {
        Some(StorageCredential {
            info: CredentialInfo {
                aws_iam_role: Some(AwsIamRoleResponse {
                    role_arn: Some(arn.into()),
                    external_id: Some(external_id.into()),
                    ..Default::default()
                }),
                ..Default::default()
            },
            secret: None,
        })
    }

    fn ctx(
        url: &str,
        privileges: &[StoragePrivilege],
        credential: Option<StorageCredential>,
    ) -> CredentialContext {
        CredentialContext::new(NormalizedUrl::parse(url).unwrap(), privileges, credential)
    }

    const READ: &[StoragePrivilege] = &[StoragePrivilege::Select];
    const WRITE: &[StoragePrivilege] = &[StoragePrivilege::Select, StoragePrivilege::Update];

    #[tokio::test]
    async fn a_request_for_the_same_credential_gets_it_again() {
        let f = Fixture::new(vec![Ok(Some(at(60)))]);
        let read = ctx("s3://b/t", READ, role("arn:r", "x"));
        assert_eq!(f.vend(&read).await.unwrap(), "1");
        assert_eq!(f.vend(&read).await.unwrap(), "1");
        // The order of the privileges does not matter.
        let reversed = &[StoragePrivilege::Update, StoragePrivilege::Select];
        let write = ctx("s3://b/t", WRITE, role("arn:r", "x"));
        assert_eq!(f.vend(&write).await.unwrap(), "2");
        assert_eq!(
            f.vend(&ctx("s3://b/t", reversed, role("arn:r", "x")))
                .await
                .unwrap(),
            "2"
        );
        assert_eq!(f.vends(), 2);
    }

    #[tokio::test]
    async fn what_decides_the_credential_is_in_the_key() {
        let f = Fixture::new(vec![Ok(Some(at(60)))]);
        let requests = [
            ctx("s3://b/t", READ, role("arn:r", "x")),
            ctx("s3://b/t", WRITE, role("arn:r", "x")),
            // Another role, external ID, path, path with the same start,
            // scheme, or a location of the settings.
            ctx("s3://b/t", READ, role("arn:other", "x")),
            ctx("s3://b/t", READ, role("arn:r", "y")),
            ctx("s3://b/t/sub", READ, role("arn:r", "x")),
            ctx("s3://b/t2", READ, role("arn:r", "x")),
            ctx("gs://b/t", READ, None),
            ctx("s3://b/t", READ, None),
        ];
        for (i, request) in requests.iter().enumerate() {
            assert_eq!(f.vend(request).await.unwrap(), (i + 1).to_string());
        }
        for (i, request) in requests.iter().enumerate() {
            assert_eq!(f.vend(request).await.unwrap(), (i + 1).to_string());
        }
        assert_eq!(f.vends(), requests.len());
    }

    #[tokio::test]
    async fn requests_that_miss_together_wait_for_one_vend() {
        let f = Fixture::new(vec![Ok(Some(at(60)))]);
        let read = ctx("s3://b/t", READ, None);
        let (a, b) = tokio::join!(f.vend(&read), f.vend(&read));
        assert_eq!((a.unwrap(), b.unwrap()), ("1".into(), "1".into()));
        assert_eq!(f.vends(), 1);
    }

    #[test]
    fn secrets_are_in_the_key_as_digests() {
        let principal = |secret: &str| StorageCredential {
            info: CredentialInfo {
                azure_service_principal: Some(Default::default()),
                ..Default::default()
            },
            secret: Some(secret.into()),
        };
        let url = "abfss://c@a.dfs.core.windows.net/t";
        let one = Key::of(&ctx(url, READ, Some(principal("one")))).unwrap();
        let two = Key::of(&ctx(url, READ, Some(principal("two")))).unwrap();
        assert_ne!(one, two);
        assert!(!format!("{one:?}").contains("one"));
        // A local path vends nothing, so it is not cached.
        assert_eq!(Key::of(&ctx("file:///tmp/t", READ, None)), None);
    }

    #[tokio::test]
    async fn an_entry_ends_at_the_max_age_or_a_minute_before_expiry() {
        // An hour-long credential: the max age ends it.
        let read = ctx("s3://b/t", READ, None);
        let f = Fixture::new(vec![Ok(Some(at(60)))]);
        assert_eq!(f.vend(&read).await.unwrap(), "1");
        f.set(at(5) - 1);
        assert_eq!(f.vend(&read).await.unwrap(), "1");
        f.set(at(5));
        assert_eq!(f.vend(&read).await.unwrap(), "2");

        // A credential that expires in 3 minutes: given for 2.
        let f = Fixture::new(vec![Ok(Some(at(3)))]);
        assert_eq!(f.vend(&read).await.unwrap(), "1");
        f.set(at(2) - 1);
        assert_eq!(f.vend(&read).await.unwrap(), "1");
        f.set(at(2));
        assert_eq!(f.vend(&read).await.unwrap(), "2");

        // Without an expiry, the max age alone.
        let f = Fixture::new(vec![Ok(None)]);
        assert_eq!(f.vend(&read).await.unwrap(), "1");
        f.set(at(5) - 1);
        assert_eq!(f.vend(&read).await.unwrap(), "1");
        f.set(at(5));
        assert_eq!(f.vend(&read).await.unwrap(), "2");

        // A credential that lives a minute or less is not cached.
        let f = Fixture::new(vec![Ok(Some(at(1)))]);
        assert_eq!(f.vend(&read).await.unwrap(), "1");
        assert_eq!(f.vend(&read).await.unwrap(), "2");
    }

    #[tokio::test]
    async fn a_failed_vend_is_not_cached_and_a_stale_entry_not_given() {
        let f = Fixture::new(vec![Ok(Some(at(60))), Err(()), Ok(Some(at(70)))]);
        let read = ctx("s3://b/t", READ, None);
        assert_eq!(f.vend(&read).await.unwrap(), "1");
        f.set(at(5));
        assert!(f.vend(&read).await.is_err());
        assert_eq!(f.vend(&read).await.unwrap(), "3");
        assert_eq!(f.vend(&read).await.unwrap(), "3");
    }

    #[test]
    fn the_settings_turn_it_on() {
        let parse = |text: &str| {
            let settings = uc_common::settings::Settings::from_toml(text).unwrap();
            CredentialCacheConfig::from_settings(&settings.storage)
        };
        assert_eq!(parse(""), Ok(None));
        assert_eq!(
            parse("[storage]\ncredential_cache_enabled = true"),
            Ok(Some(CredentialCacheConfig {
                max_size: 1000,
                max_age: Duration::from_secs(300),
            }))
        );
        assert_eq!(
            parse(
                "[storage]\ncredential_cache_enabled = true\ncredential_cache_max_size = 5\n\
                 credential_cache_max_age = \"1m\""
            ),
            Ok(Some(CredentialCacheConfig {
                max_size: 5,
                max_age: Duration::from_secs(60),
            }))
        );
        let error = parse("[storage]\ncredential_cache_max_age = \"0s\"").unwrap_err();
        assert!(
            error.starts_with("Invalid value for storage.credential_cache_max_age"),
            "{error}"
        );
        let error = parse("[storage]\ncredential_cache_max_size = 0").unwrap_err();
        assert!(
            error.starts_with("Invalid value for storage.credential_cache_max_size"),
            "{error}"
        );
    }
}
