//! The NDJSON body of `metadata`, `query` and `changes`: a protocol line, a
//! metadata line, then lines of files and of later metadata, in the
//! `parquet` or the `delta` response format.
//!
//! In the `parquet` format, a line holds what a client needs to read the
//! file as plain Parquet. In the `delta` format, it wraps the Delta log's
//! action, with the file's URL as its `path`, so that a client can rebuild
//! the log.

use delta_kernel::Version;
use serde_json::{Map, Value, json};
use sha2::{Digest, Sha256};

use super::ResponseFormat;
use super::signer::SignedUrl;
use super::table::Action;

/// Reader features of tables whose files read as plain Parquet, with the
/// file lines of the `parquet` format: none of them changes how a file
/// reads, and the server checks each file for deletion vectors.
const PARQUET_READER_FEATURES: &[&str] = &[
    "catalogManaged",
    "catalogOwned-preview",
    "columnMapping",
    "deletionVectors",
    "timestampNtz",
    "v2Checkpoint",
    "vacuumProtocolCheck",
];

/// Features of the catalog that commits to the table: a recipient reads a
/// log of its own, without the catalog.
const CATALOG_FEATURES: &[&str] = &["catalogManaged", "catalogOwned-preview"];

/// Whether the files of a table with `protocol` and `metadata` read as
/// plain Parquet: no column mapping, and no reader features besides
/// [`PARQUET_READER_FEATURES`].
pub fn parquet_compatible(protocol: &Value, metadata: &Value) -> bool {
    column_mapping(metadata).is_none()
        && features(protocol, "readerFeatures")
            .all(|feature| PARQUET_READER_FEATURES.contains(&feature))
}

/// Whether files of a table with `protocol` may have deletion vectors,
/// which only the `delta` format expresses.
pub fn deletion_vectors(protocol: &Value) -> bool {
    features(protocol, "readerFeatures").any(|feature| feature == "deletionVectors")
}

/// The table's column mapping mode, none for `none`.
pub fn column_mapping(metadata: &Value) -> Option<&str> {
    metadata["configuration"]["delta.columnMapping.mode"]
        .as_str()
        .filter(|mode| *mode != "none")
}

fn features<'a>(protocol: &'a Value, key: &str) -> impl Iterator<Item = &'a str> {
    protocol[key]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
}

/// The kind of a file line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileKind {
    /// A file of a snapshot.
    Snapshot,
    /// A file a commit added.
    Add,
    /// A file a commit removed.
    Remove,
    /// A change data file of a commit.
    Cdc,
}

/// A file of a response: the log's action, and URLs for its data and its
/// deletion vector.
pub struct SharedFile<'a> {
    pub kind: FileKind,
    pub action: &'a Action,
    pub url: SignedUrl,
    pub deletion_vector: Option<SignedUrl>,
    /// The commit (version and time) of the file, for queries of a
    /// version or of changes.
    pub commit: Option<(Version, i64)>,
}

/// The id of a file: the same in every response, in both formats.
pub fn file_id(path: &str) -> String {
    hex::encode(&Sha256::digest(path.as_bytes())[..16])
}

/// The lines of a response.
pub struct Lines {
    format: ResponseFormat,
    body: Vec<u8>,
    /// When the first file URL expires.
    min_expiration: Option<i64>,
}

impl Lines {
    pub fn new(format: ResponseFormat) -> Self {
        Self {
            format,
            body: Vec::new(),
            min_expiration: None,
        }
    }

    pub fn format(&self) -> ResponseFormat {
        self.format
    }

    fn push(&mut self, line: Value) {
        serde_json::to_writer(&mut self.body, &line).expect("JSON values serialize");
        self.body.push(b'\n');
    }

    /// The table's protocol, the kernel's `Protocol` in JSON. In the
    /// `parquet` format it is the protocol of the response instead.
    pub fn protocol(&mut self, protocol: &Value, version: Option<Version>) {
        let line = match self.format {
            ResponseFormat::Parquet => json!({ "protocol": { "minReaderVersion": 1 } }),
            ResponseFormat::Delta => {
                let mut wrapper = Map::new();
                if let Some(version) = version {
                    wrapper.insert("version".into(), version.into());
                }
                wrapper.insert("deltaProtocol".into(), without_catalog(protocol));
                json!({ "protocol": wrapper })
            }
        };
        self.push(line);
    }

    /// The table's metadata, the kernel's `Metadata` in JSON, as of
    /// `version` for queries of a version or of changes.
    /// `change_data_feed` tells `parquet` clients that they may read the
    /// table's changes.
    pub fn metadata(&mut self, metadata: &Value, version: Option<Version>, change_data_feed: bool) {
        let mut line = Map::new();
        match self.format {
            ResponseFormat::Parquet => {
                for key in ["id", "name", "description"] {
                    if let Some(value) = metadata.get(key).filter(|v| !v.is_null()) {
                        line.insert(key.into(), value.clone());
                    }
                }
                line.insert("format".into(), json!({ "provider": "parquet" }));
                let schema = metadata["schemaString"].as_str().unwrap_or_default();
                line.insert("schemaString".into(), comments_only(schema).into());
                line.insert(
                    "partitionColumns".into(),
                    metadata
                        .get("partitionColumns")
                        .cloned()
                        .unwrap_or_else(|| json!([])),
                );
                let configuration = match change_data_feed {
                    true => json!({ "enableChangeDataFeed": "true" }),
                    false => json!({}),
                };
                line.insert("configuration".into(), configuration);
            }
            ResponseFormat::Delta => {
                let metadata = metadata
                    .as_object()
                    .into_iter()
                    .flatten()
                    .filter(|(_, v)| !v.is_null())
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect::<Map<_, _>>();
                line.insert("deltaMetadata".into(), Value::Object(metadata));
            }
        }
        if let Some(version) = version {
            line.insert("version".into(), version.into());
        }
        self.push(json!({ "metaData": line }));
    }

    pub fn file(&mut self, file: SharedFile<'_>) {
        let expires_at = match &file.deletion_vector {
            Some(dv) => file.url.expires_at.min(dv.expires_at),
            None => file.url.expires_at,
        };
        self.min_expiration = Some(
            self.min_expiration
                .map_or(expires_at, |e| e.min(expires_at)),
        );
        let path = file.action.get("path").and_then(Value::as_str);
        let id = file_id(path.unwrap_or_default());
        let line = match self.format {
            ResponseFormat::Parquet => parquet_file(&file, id),
            ResponseFormat::Delta => delta_file(&file, id),
        };
        self.push(line);
    }

    /// The `endStreamAction` line: when the first URL expires.
    pub fn end_stream(&mut self) {
        let mut action = Map::new();
        if let Some(expiration) = self.min_expiration {
            action.insert("minUrlExpirationTimestamp".into(), expiration.into());
        }
        self.push(json!({ "endStreamAction": action }));
    }

    pub fn into_body(self) -> Vec<u8> {
        self.body
    }
}

fn parquet_file(file: &SharedFile<'_>, id: String) -> Value {
    let action = file.action;
    let mut line = Map::new();
    line.insert("url".into(), file.url.url.clone().into());
    line.insert("id".into(), id.into());
    line.insert(
        "partitionValues".into(),
        action
            .get("partitionValues")
            .cloned()
            .unwrap_or_else(|| json!({})),
    );
    line.insert(
        "size".into(),
        action.get("size").cloned().unwrap_or(Value::Null),
    );
    if matches!(file.kind, FileKind::Snapshot | FileKind::Add)
        && let Some(stats) = action.get("stats")
    {
        line.insert("stats".into(), stats.clone());
    }
    if let Some((version, timestamp)) = file.commit {
        line.insert("version".into(), version.into());
        line.insert("timestamp".into(), timestamp.into());
    }
    line.insert("expirationTimestamp".into(), file.url.expires_at.into());
    let key = match file.kind {
        FileKind::Snapshot => "file",
        FileKind::Add => "add",
        FileKind::Remove => "remove",
        FileKind::Cdc => "cdf",
    };
    json!({ key: line })
}

fn delta_file(file: &SharedFile<'_>, id: String) -> Value {
    let mut action = file.action.clone();
    action.insert("path".into(), file.url.url.clone().into());
    let mut line = Map::new();
    line.insert("id".into(), id.into());
    if let Some(dv) = &file.deletion_vector
        && let Some(Value::Object(descriptor)) = action.get_mut("deletionVector")
    {
        let original = descriptor
            .get("pathOrInlineDv")
            .and_then(Value::as_str)
            .unwrap_or_default();
        line.insert("deletionVectorFileId".into(), file_id(original).into());
        descriptor.insert("storageType".into(), "p".into());
        descriptor.insert("pathOrInlineDv".into(), dv.url.clone().into());
    }
    if let Some((version, timestamp)) = file.commit {
        line.insert("version".into(), version.into());
        line.insert("timestamp".into(), timestamp.into());
    }
    let expires_at = match &file.deletion_vector {
        Some(dv) => file.url.expires_at.min(dv.expires_at),
        None => file.url.expires_at,
    };
    line.insert("expirationTimestamp".into(), expires_at.into());
    let key = match file.kind {
        FileKind::Snapshot | FileKind::Add => "add",
        FileKind::Remove => "remove",
        FileKind::Cdc => "cdc",
    };
    line.insert("deltaSingleAction".into(), json!({ key: action }));
    json!({ "file": line })
}

/// `protocol` without the features of the catalog.
fn without_catalog(protocol: &Value) -> Value {
    let mut protocol = protocol.clone();
    for key in ["readerFeatures", "writerFeatures"] {
        if let Some(Value::Array(features)) = protocol.get_mut(key) {
            features.retain(|f| !f.as_str().is_some_and(|f| CATALOG_FEATURES.contains(&f)));
        }
    }
    protocol
}

/// A table schema whose fields keep only their comments as metadata, as
/// the reference server sends `parquet` clients.
fn comments_only(schema: &str) -> String {
    let Ok(mut schema) = serde_json::from_str::<Value>(schema) else {
        return schema.to_string();
    };
    if let Some(Value::Array(fields)) = schema.get_mut("fields") {
        for field in fields {
            let comment = field["metadata"].get("comment").cloned();
            let metadata = match comment {
                Some(comment) => json!({ "comment": comment }),
                None => json!({}),
            };
            field["metadata"] = metadata;
        }
    }
    schema.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lines(body: Vec<u8>) -> Vec<Value> {
        String::from_utf8(body)
            .unwrap()
            .lines()
            .map(|l| serde_json::from_str(l).unwrap())
            .collect()
    }

    fn url(url: &str, expires_at: i64) -> SignedUrl {
        SignedUrl {
            url: url.into(),
            expires_at,
        }
    }

    #[test]
    fn parquet_lines_are_plain() {
        let mut out = Lines::new(ResponseFormat::Parquet);
        out.protocol(
            &json!({ "minReaderVersion": 3, "minWriterVersion": 7 }),
            None,
        );
        let schema = r#"{"type":"struct","fields":[{"name":"a","type":"long","nullable":true,"metadata":{"comment":"c","delta.columnMapping.id":1}}]}"#;
        out.metadata(
            &json!({ "id": "t1", "name": null, "schemaString": schema, "partitionColumns": ["a"],
                     "configuration": { "delta.enableChangeDataFeed": "true" } }),
            Some(3),
            true,
        );
        let add = json!({ "path": "a=1/p.parquet", "partitionValues": { "a": "1" }, "size": 9,
                          "stats": "{\"numRecords\":2}", "dataChange": true });
        out.file(SharedFile {
            kind: FileKind::Snapshot,
            action: add.as_object().unwrap(),
            url: url("https://s/p", 50),
            deletion_vector: None,
            commit: Some((3, 1000)),
        });
        let remove = json!({ "path": "a=1/p.parquet", "partitionValues": { "a": "1" }, "size": 9 });
        out.file(SharedFile {
            kind: FileKind::Remove,
            action: remove.as_object().unwrap(),
            url: url("https://s/p", 40),
            deletion_vector: None,
            commit: Some((4, 2000)),
        });
        out.end_stream();
        let id = file_id("a=1/p.parquet");
        assert_eq!(
            lines(out.into_body()),
            [
                json!({ "protocol": { "minReaderVersion": 1 } }),
                json!({ "metaData": {
                    "id": "t1", "format": { "provider": "parquet" },
                    "schemaString": r#"{"type":"struct","fields":[{"name":"a","type":"long","nullable":true,"metadata":{"comment":"c"}}]}"#,
                    "partitionColumns": ["a"], "configuration": { "enableChangeDataFeed": "true" },
                    "version": 3,
                } }),
                json!({ "file": {
                    "url": "https://s/p", "id": id, "partitionValues": { "a": "1" }, "size": 9,
                    "stats": "{\"numRecords\":2}", "version": 3, "timestamp": 1000,
                    "expirationTimestamp": 50,
                } }),
                json!({ "remove": {
                    "url": "https://s/p", "id": id, "partitionValues": { "a": "1" }, "size": 9,
                    "version": 4, "timestamp": 2000, "expirationTimestamp": 40,
                } }),
                json!({ "endStreamAction": { "minUrlExpirationTimestamp": 40 } }),
            ]
        );
    }

    #[test]
    fn delta_lines_wrap_the_log_actions() {
        let mut out = Lines::new(ResponseFormat::Delta);
        out.protocol(
            &json!({ "minReaderVersion": 3, "minWriterVersion": 7,
                     "readerFeatures": ["catalogManaged", "deletionVectors"],
                     "writerFeatures": ["catalogManaged", "deletionVectors", "inCommitTimestamp"] }),
            None,
        );
        out.metadata(
            &json!({ "id": "t1", "name": null, "configuration": {} }),
            None,
            false,
        );
        let add = json!({ "path": "p.parquet", "partitionValues": {}, "size": 9, "dataChange": true,
                          "deletionVector": { "storageType": "u", "pathOrInlineDv": "ab^-aqEH.-t@S}K{vb[*k^",
                                              "offset": 1, "sizeInBytes": 36, "cardinality": 2 } });
        out.file(SharedFile {
            kind: FileKind::Snapshot,
            action: add.as_object().unwrap(),
            url: url("https://s/p", 50),
            deletion_vector: Some(url("https://s/dv", 30)),
            commit: None,
        });
        assert_eq!(
            lines(out.into_body()),
            [
                json!({ "protocol": { "deltaProtocol": {
                    "minReaderVersion": 3, "minWriterVersion": 7,
                    "readerFeatures": ["deletionVectors"],
                    "writerFeatures": ["deletionVectors", "inCommitTimestamp"],
                } } }),
                json!({ "metaData": { "deltaMetadata": { "id": "t1", "configuration": {} } } }),
                json!({ "file": {
                    "id": file_id("p.parquet"),
                    "deletionVectorFileId": file_id("ab^-aqEH.-t@S}K{vb[*k^"),
                    "expirationTimestamp": 30,
                    "deltaSingleAction": { "add": {
                        "path": "https://s/p", "partitionValues": {}, "size": 9, "dataChange": true,
                        "deletionVector": { "storageType": "p", "pathOrInlineDv": "https://s/dv",
                                            "offset": 1, "sizeInBytes": 36, "cardinality": 2 },
                    } },
                } }),
            ]
        );
    }

    #[test]
    fn plain_parquet_tables_have_no_reader_features_that_change_files() {
        let metadata = json!({ "configuration": {} });
        let v1 = json!({ "minReaderVersion": 1, "minWriterVersion": 2 });
        assert!(parquet_compatible(&v1, &metadata));
        let managed = json!({ "minReaderVersion": 3, "minWriterVersion": 7,
                              "readerFeatures": ["catalogManaged", "deletionVectors", "vacuumProtocolCheck"] });
        assert!(parquet_compatible(&managed, &metadata));
        let widened = json!({ "minReaderVersion": 3, "minWriterVersion": 7,
                              "readerFeatures": ["typeWidening"] });
        assert!(!parquet_compatible(&widened, &metadata));
        let mapped = json!({ "configuration": { "delta.columnMapping.mode": "name" } });
        assert!(!parquet_compatible(&v1, &mapped));
        let unmapped = json!({ "configuration": { "delta.columnMapping.mode": "none" } });
        assert!(parquet_compatible(&v1, &unmapped));
    }
}
