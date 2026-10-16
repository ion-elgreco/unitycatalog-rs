//! Storage URLs — upstream `NormalizedURL`, `UriScheme` and the managed
//! storage layout of `ExternalLocationUtils`.
//!
//! Every storage path the server stores or compares goes through
//! [`NormalizedUrl`], so equal locations have equal strings:
//! `s3://bucket/path/` and `s3://bucket/path` are both `s3://bucket/path`,
//! and `/tmp/x` and `file:/tmp/x` are both `file:///tmp/x`.

use std::fmt;

use crate::{Error, Result};

/// The reserved directory under a storage root that holds managed data.
pub const MANAGED_STORAGE_PREFIX: &str = "__unitystorage";

/// Path segment before the id of a managed entity: `.../<segment>/<id>`.
/// Upstream `ManagedResourceType`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ManagedResource {
    Table,
    Volume,
    RegisteredModel,
    ModelVersion,
}

impl ManagedResource {
    pub const ALL: [Self; 4] = [
        Self::Table,
        Self::Volume,
        Self::RegisteredModel,
        Self::ModelVersion,
    ];

    /// The upstream enum name, as stored in storage cleanup tasks.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Table => "TABLE",
            Self::Volume => "VOLUME",
            Self::RegisteredModel => "REGISTERED_MODEL",
            Self::ModelVersion => "MODEL_VERSION",
        }
    }

    pub fn parse(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|r| r.as_str() == name)
    }

    pub fn path_segment(self) -> &'static str {
        match self {
            Self::Table => "tables",
            Self::Volume => "volumes",
            Self::RegisteredModel => "models",
            Self::ModelVersion => "versions",
        }
    }
}

/// Supported storage URL schemes. Upstream `UriScheme`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UriScheme {
    Abfs,
    Abfss,
    Gs,
    S3,
    File,
}

impl UriScheme {
    /// `None` is a local path without a scheme. Any other scheme is
    /// upstream's `Unsupported URI scheme`.
    fn from_scheme(scheme: Option<&str>) -> Result<Option<Self>> {
        Ok(match scheme {
            None => None,
            Some("abfs") => Some(Self::Abfs),
            Some("abfss") => Some(Self::Abfss),
            Some("gs") => Some(Self::Gs),
            Some("s3") => Some(Self::S3),
            Some("file") => Some(Self::File),
            Some(other) => {
                return Err(Error::invalid_argument(format!(
                    "Unsupported URI scheme: {other}"
                )));
            }
        })
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Abfs => "abfs",
            Self::Abfss => "abfss",
            Self::Gs => "gs",
            Self::S3 => "s3",
            Self::File => "file",
        }
    }

    pub fn is_cloud(self) -> bool {
        !matches!(self, Self::File)
    }
}

/// A storage URL in upstream's normal form. See the module docs.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct NormalizedUrl(String);

impl fmt::Display for NormalizedUrl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl NormalizedUrl {
    /// Normalize a path or URL, with upstream's rules and error messages:
    ///
    /// - `file:/tmp/x`, `file:///tmp/x/` and `/tmp/x` become `file:///tmp/x`
    /// - `s3://bucket/a/` becomes `s3://bucket/a`
    /// - `ftp://h/x` is `Unsupported URI scheme: ftp`
    /// - a value without `/` is `Ambiguous path`
    pub fn parse(input: &str) -> Result<Self> {
        if input.trim().is_empty() {
            return Err(Error::invalid_argument("Path cannot be null or empty"));
        }
        if !input.contains('/') {
            return Err(Error::invalid_argument(format!("Ambiguous path: {input}")));
        }
        let uri = Uri::parse(input)
            .ok_or_else(|| Error::invalid_argument(format!("Unsupported path: {input}")))?
            .normalize();
        let normalized = match UriScheme::from_scheme(uri.scheme.as_deref())? {
            // A local path without a scheme: make it absolute.
            None => {
                let path = absolute_local_path(input)?;
                let as_uri = format!("file://{}", encode_path(&path));
                let uri = Uri::parse(&as_uri)
                    .ok_or_else(|| {
                        Error::invalid_argument(format!("Unsupported local path: {input}"))
                    })?
                    .normalize();
                validate_local_file_uri(&local_file_uri_to_string(&uri.to_string()), input)?
            }
            // A host is read as the first directory (file://tmp/x is /tmp/x).
            Some(UriScheme::File) => {
                validate_local_file_uri(&local_file_uri_to_string(&uri.to_string()), input)?
            }
            Some(_) => remove_extra_slashes(&uri.to_string()).to_string(),
        };
        Ok(Self(normalized))
    }

    /// Like [`Self::parse`], passing `None` through.
    pub fn parse_opt(input: Option<&str>) -> Result<Option<Self>> {
        input.map(Self::parse).transpose()
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn into_string(self) -> String {
        self.0
    }

    pub fn scheme(&self) -> UriScheme {
        let scheme = self.0.split_once(':').map(|(s, _)| s);
        UriScheme::from_scheme(scheme)
            .ok()
            .flatten()
            .unwrap_or(UriScheme::File)
    }

    /// The decoded path part (`/a/b` for `s3://bucket/a/b`).
    pub fn path(&self) -> String {
        Uri::parse(&self.0)
            .map(|u| percent_decode(&u.path))
            .unwrap_or_default()
    }

    /// The `scheme://authority` part: the bucket or container.
    pub fn storage_base(&self) -> NormalizedUrl {
        let uri = Uri::parse(&self.0).expect("normalized URL parses");
        let base = format!(
            "{}://{}",
            uri.scheme.unwrap_or_default(),
            uri.authority.unwrap_or_default()
        );
        Self(remove_extra_slashes(&base).to_string())
    }

    /// Whether this names a whole cloud bucket or container (no path below
    /// it). Local paths never are.
    pub fn is_cloud_storage_root(&self) -> bool {
        self.scheme().is_cloud() && self.path().chars().all(|c| c == '/')
    }

    /// Upstream `validateNotSameOrUnderManagedStoragePrefix`.
    pub fn validate_not_under_managed_prefix(&self) -> Result<()> {
        if self.0.contains(MANAGED_STORAGE_PREFIX) {
            return Err(Error::invalid_argument(format!(
                "Input path '{self}' contains managed storage prefix {MANAGED_STORAGE_PREFIX}."
            )));
        }
        Ok(())
    }

    /// Upstream `LocalStorageLocationValidator.isCanonical`: a local URL
    /// spelled the one way its decoded path is (`tableA`, not `table%41`).
    /// Locations compare as strings while the file system opens the decoded
    /// path, so each local directory must have one accepted spelling.
    pub fn is_canonical_local(&self) -> bool {
        self.scheme() != UriScheme::File
            || format!("file://{}", encode_path(&self.path())) == self.0
    }

    /// `true` when `self` is `other` or a path below it.
    pub fn is_same_or_under(&self, other: &NormalizedUrl) -> bool {
        self.0 == other.0
            || (self.0.starts_with(&other.0) && self.0.as_bytes().get(other.0.len()) == Some(&b'/'))
    }

    /// `true` when either is the same as, or below, the other.
    pub fn overlaps(&self, other: &NormalizedUrl) -> bool {
        self.is_same_or_under(other) || other.is_same_or_under(self)
    }

    /// This URL with `/`-joined segments appended.
    pub fn join(&self, segments: &[&str]) -> Result<NormalizedUrl> {
        let mut s = self.0.clone();
        for seg in segments {
            s.push('/');
            s.push_str(seg);
        }
        Self::parse(&s)
    }

    /// Storage location of a catalog with a storage root:
    /// `<root>/__unitystorage/catalogs/<id>`.
    pub fn managed_catalog_location(&self, catalog_id: &str) -> Result<NormalizedUrl> {
        self.join(&[MANAGED_STORAGE_PREFIX, "catalogs", catalog_id])
    }

    /// Storage location of a schema with a storage root:
    /// `<root>/__unitystorage/schemas/<id>`.
    pub fn managed_schema_location(&self, schema_id: &str) -> Result<NormalizedUrl> {
        self.join(&[MANAGED_STORAGE_PREFIX, "schemas", schema_id])
    }

    /// Storage location of a managed entity under a parent storage location
    /// (which already includes `__unitystorage`): `<parent>/<segment>/<id>`.
    pub fn managed_child_location(&self, kind: ManagedResource, id: &str) -> Result<NormalizedUrl> {
        self.join(&[kind.path_segment(), id])
    }

    /// A fallback storage root from server properties
    /// (`storage-root.tables`, `storage-root.models`): `<root>/__unitystorage`.
    pub fn managed_fallback_root(&self) -> Result<NormalizedUrl> {
        self.join(&[MANAGED_STORAGE_PREFIX])
    }
}

/// Upstream `localFileURIToString`: make `file:/x` and `file://x` start with
/// `file:///`, and drop trailing slashes.
fn local_file_uri_to_string(uri: &str) -> String {
    match uri.strip_prefix("file:") {
        Some(path) if path.starts_with('/') => format!("file://{}", remove_extra_slashes(path)),
        _ => uri.to_string(),
    }
}

/// Upstream `validateLocalFileURI`: reject a local URI that does not name one
/// plain path (an encoded `/` or dot segment, a `..` above the root, NUL, a
/// query or fragment, or an opaque URI).
fn validate_local_file_uri(file_uri: &str, input: &str) -> Result<String> {
    let bad = || Error::invalid_argument(format!("Unsupported local path: {input}"));
    let uri = Uri::parse(file_uri).ok_or_else(bad)?.normalize();
    if uri.is_opaque() || uri.query.is_some() || uri.fragment.is_some() {
        return Err(bad());
    }
    let decoded = percent_decode(&uri.path);
    if decoded.contains('\0') {
        return Err(bad());
    }
    let decoded_segments: Vec<&str> = decoded.split('/').collect();
    let raw_segments = uri.path.split('/').count();
    if decoded_segments.len() != raw_segments
        || decoded_segments.iter().any(|s| *s == "." || *s == "..")
    {
        return Err(bad());
    }
    Ok(local_file_uri_to_string(&uri.to_string()))
}

/// Upstream `removeExtraSlashes`: keep at most one leading slash, drop all
/// trailing slashes.
fn remove_extra_slashes(path: &str) -> &str {
    if path.len() <= 1 {
        return path;
    }
    let bytes = path.as_bytes();
    let mut start = 0;
    while start < bytes.len() && bytes[start] == b'/' {
        start += 1;
    }
    start = start.saturating_sub(1);
    let mut end = bytes.len() - 1;
    while end > start && bytes[end] == b'/' {
        end -= 1;
    }
    &path[start..=end]
}

fn absolute_local_path(input: &str) -> Result<String> {
    // A local path is taken literally: a `?` or `#` is part of the path.
    if input.starts_with('/') {
        return Ok(input.to_string());
    }
    let cwd =
        std::env::current_dir().map_err(|e| Error::internal(format!("current directory: {e}")))?;
    Ok(format!(
        "{}/{}",
        cwd.display().to_string().trim_end_matches('/'),
        input
    ))
}

/// Percent-encode the characters `java.io.File.toURI` encodes in a path.
fn encode_path(path: &str) -> String {
    let mut out = String::with_capacity(path.len());
    for c in path.chars() {
        match c {
            'A'..='Z'
            | 'a'..='z'
            | '0'..='9'
            | '/'
            | '-'
            | '_'
            | '.'
            | '~'
            | '!'
            | '$'
            | '&'
            | '\''
            | '('
            | ')'
            | '*'
            | '+'
            | ','
            | ';'
            | '='
            | ':'
            | '@' => out.push(c),
            c => {
                let mut buf = [0u8; 4];
                for b in c.encode_utf8(&mut buf).bytes() {
                    out.push_str(&format!("%{b:02X}"));
                }
            }
        }
    }
    out
}

fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%'
            && i + 2 < bytes.len()
            && let (Some(h), Some(l)) = (hex(bytes[i + 1]), hex(bytes[i + 2]))
        {
            out.push(h * 16 + l);
            i += 3;
            continue;
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn hex(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

/// The parts of a URI reference, split the way `java.net.URI` splits them.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Uri {
    scheme: Option<String>,
    authority: Option<String>,
    path: String,
    query: Option<String>,
    fragment: Option<String>,
}

impl Uri {
    /// Split a URI reference. `None` for text `java.net.URI` rejects:
    /// spaces and other illegal characters, or a bad `%` escape.
    fn parse(s: &str) -> Option<Self> {
        if s.chars()
            .any(|c| c.is_control() || c.is_whitespace() || "\"<>\\^`{|}".contains(c))
        {
            return None;
        }
        let bytes = s.as_bytes();
        for (i, b) in bytes.iter().enumerate() {
            if *b == b'%'
                && !(i + 2 < bytes.len()
                    && hex(bytes[i + 1]).is_some()
                    && hex(bytes[i + 2]).is_some())
            {
                return None;
            }
        }
        let (rest, fragment) = match s.split_once('#') {
            Some((r, f)) => (r, Some(f.to_string())),
            None => (s, None),
        };
        let (rest, query) = match rest.split_once('?') {
            Some((r, q)) => (r, Some(q.to_string())),
            None => (rest, None),
        };
        let (scheme, rest) = match rest.find(':') {
            Some(i)
                if i > 0
                    && rest[..i].starts_with(|c: char| c.is_ascii_alphabetic())
                    && rest[..i]
                        .chars()
                        .all(|c| c.is_ascii_alphanumeric() || "+-.".contains(c))
                    && !rest[..i].contains('/') =>
            {
                (Some(rest[..i].to_ascii_lowercase()), &rest[i + 1..])
            }
            _ => (None, rest),
        };
        let (authority, path) = match rest.strip_prefix("//") {
            Some(after) => {
                let end = after.find('/').unwrap_or(after.len());
                (Some(after[..end].to_string()), after[end..].to_string())
            }
            None => (None, rest.to_string()),
        };
        Some(Self {
            scheme,
            authority,
            path,
            query,
            fragment,
        })
    }

    /// An opaque URI has a scheme and a path that does not start with `/`
    /// (e.g. `file:a/b`).
    fn is_opaque(&self) -> bool {
        self.scheme.is_some() && self.authority.is_none() && !self.path.starts_with('/')
    }

    /// `java.net.URI.normalize`: collapse repeated slashes, drop `.`
    /// segments, resolve `segment/..`. A `..` that cannot be resolved stays.
    fn normalize(mut self) -> Self {
        if self.is_opaque() {
            return self;
        }
        let absolute = self.path.starts_with('/');
        let trailing = self.path.len() > 1 && self.path.ends_with('/');
        let mut out: Vec<&str> = Vec::new();
        for seg in self.path.split('/').filter(|s| !s.is_empty()) {
            match seg {
                "." => {}
                ".." => match out.last() {
                    Some(&last) if last != ".." => {
                        out.pop();
                    }
                    _ => out.push(".."),
                },
                s => out.push(s),
            }
        }
        let mut path = out.join("/");
        if absolute {
            path.insert(0, '/');
        }
        if trailing && !path.ends_with('/') && !out.is_empty() {
            path.push('/');
        }
        self.path = path;
        self
    }
}

impl fmt::Display for Uri {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(s) = &self.scheme {
            write!(f, "{s}:")?;
        }
        if let Some(a) = &self.authority {
            write!(f, "//{a}")?;
        }
        f.write_str(&self.path)?;
        if let Some(q) = &self.query {
            write!(f, "?{q}")?;
        }
        if let Some(fr) = &self.fragment {
            write!(f, "#{fr}")?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    fn n(s: &str) -> String {
        NormalizedUrl::parse(s).unwrap().into_string()
    }

    #[rstest]
    #[case("file:/tmp/myfile", "file:///tmp/myfile")]
    #[case("file:///tmp/myfile/", "file:///tmp/myfile")]
    #[case("file://tmp/x", "file:///tmp/x")]
    #[case("/local/path/to/file", "file:///local/path/to/file")]
    #[case("/tmp//a///b/", "file:///tmp/a/b")]
    #[case("/tmp/a/../b/./c", "file:///tmp/b/c")]
    // A local path is taken literally, so `%` is encoded (Java `Path.toUri`).
    #[case("/tmp/my%20file", "file:///tmp/my%2520file")]
    #[case("file:///tmp/my%20file", "file:///tmp/my%20file")]
    #[case("s3://my-bucket/my-file", "s3://my-bucket/my-file")]
    #[case("s3://my-bucket/my-file/", "s3://my-bucket/my-file")]
    #[case("s3://bucket//a//b", "s3://bucket/a/b")]
    #[case("s3://bucket/a/../b", "s3://bucket/b")]
    #[case("gs://my-bucket/my-file", "gs://my-bucket/my-file")]
    #[case(
        "abfs://my-container@my-storage.dfs.core.windows.net/my-file",
        "abfs://my-container@my-storage.dfs.core.windows.net/my-file"
    )]
    #[case(
        "abfss://c@a.dfs.core.windows.net/x/",
        "abfss://c@a.dfs.core.windows.net/x"
    )]
    fn normalizes_like_upstream(#[case] input: &str, #[case] expected: &str) {
        assert_eq!(n(input), expected);
    }

    #[rstest]
    #[case("", "Path cannot be null or empty")]
    #[case("bucket", "Ambiguous path: bucket")]
    #[case("ftp://example.com/file", "Unsupported URI scheme: ftp")]
    #[case("s3://bucket/a b", "Unsupported path: s3://bucket/a b")]
    #[case("/tmp/my file", "Unsupported path: /tmp/my file")]
    #[case(
        "file:///data/root/%2e%2e/etc",
        "Unsupported local path: file:///data/root/%2e%2e/etc"
    )]
    #[case("file:///data/a%2Fb", "Unsupported local path: file:///data/a%2Fb")]
    #[case("file:///../etc", "Unsupported local path: file:///../etc")]
    #[case("file:///tmp/x?y=1", "Unsupported local path: file:///tmp/x?y=1")]
    fn rejects_like_upstream(#[case] input: &str, #[case] message: &str) {
        assert_eq!(NormalizedUrl::parse(input).unwrap_err().message(), message);
    }

    #[test]
    fn canonical_local_spellings() {
        assert!(
            NormalizedUrl::parse("file:///tmp/el-x/my%20t")
                .unwrap()
                .is_canonical_local()
        );
        assert!(
            !NormalizedUrl::parse("file:///tmp/el%2Dx/t")
                .unwrap()
                .is_canonical_local()
        );
        assert!(
            NormalizedUrl::parse("s3://b/el%2Dx")
                .unwrap()
                .is_canonical_local()
        );
    }

    #[test]
    fn relative_local_paths_become_absolute() {
        let url = n("some/dir");
        assert!(
            url.starts_with("file:///") && url.ends_with("/some/dir"),
            "{url}"
        );
    }

    #[test]
    fn bucket_roots_and_bases() {
        assert!(
            NormalizedUrl::parse("s3://bucket/")
                .unwrap()
                .is_cloud_storage_root()
        );
        assert!(
            NormalizedUrl::parse("s3://bucket")
                .unwrap()
                .is_cloud_storage_root()
        );
        assert!(
            !NormalizedUrl::parse("s3://bucket/a")
                .unwrap()
                .is_cloud_storage_root()
        );
        assert!(
            !NormalizedUrl::parse("file:///")
                .unwrap()
                .is_cloud_storage_root()
        );
        assert_eq!(
            NormalizedUrl::parse("s3://bucket/a/b")
                .unwrap()
                .storage_base()
                .as_str(),
            "s3://bucket"
        );
    }

    #[test]
    fn managed_layout() {
        let root = NormalizedUrl::parse("s3://bucket/root/").unwrap();
        assert_eq!(
            root.managed_catalog_location("c1").unwrap().as_str(),
            "s3://bucket/root/__unitystorage/catalogs/c1"
        );
        let schema = root.managed_schema_location("s1").unwrap();
        assert_eq!(
            schema.as_str(),
            "s3://bucket/root/__unitystorage/schemas/s1"
        );
        assert_eq!(
            schema
                .managed_child_location(ManagedResource::Volume, "v1")
                .unwrap()
                .as_str(),
            "s3://bucket/root/__unitystorage/schemas/s1/volumes/v1"
        );
        assert!(schema.validate_not_under_managed_prefix().is_err());
        assert!(root.validate_not_under_managed_prefix().is_ok());
    }

    #[test]
    fn overlap_uses_path_boundaries() {
        let data = NormalizedUrl::parse("s3://b/data").unwrap();
        let data2 = NormalizedUrl::parse("s3://b/data2").unwrap();
        let sub = NormalizedUrl::parse("s3://b/data/x").unwrap();
        assert!(!data.overlaps(&data2));
        assert!(sub.is_same_or_under(&data));
        assert!(data.overlaps(&sub));
    }
}
