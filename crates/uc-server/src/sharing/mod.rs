//! Delta Sharing reads of shared tables, with the Delta kernel.
//!
//! - [`table`]: snapshots, data files and commits of a table's Delta log.
//! - [`signer`]: URLs that let a recipient read one data file for a while.
//! - [`lines`]: the NDJSON lines of the protocol, in its `parquet` and
//!   `delta` response formats.
//! - [`predicate`]: `jsonPredicateHints` as a kernel predicate, to skip
//!   files.
//!
//! The protocol: <https://github.com/delta-io/delta-sharing/blob/main/PROTOCOL.md>.

pub mod lines;
pub mod predicate;
pub mod signer;
pub mod table;

use std::collections::HashSet;

use axum::http::HeaderMap;

use uc_common::{Error, ErrorCode, Result};

/// The request header of a client's capabilities, and the response header
/// of the server's answer to them.
pub const CAPABILITIES_HEADER: &str = "delta-sharing-capabilities";

/// The request header that picks the scheme of file ids; the response
/// repeats it.
pub const FILE_ID_HASH_HEADER: &str = "fileidhash";

/// The response header of a table's version.
pub const TABLE_VERSION_HEADER: &str = "delta-table-version";

/// The format of the actions of `metadata`, `query` and `changes`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResponseFormat {
    /// Plain actions that clients read without a Delta library: the
    /// default, and only for tables whose files read as plain Parquet.
    Parquet,
    /// Delta log actions, for clients that rebuild the log.
    Delta,
}

impl ResponseFormat {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Parquet => "parquet",
            Self::Delta => "delta",
        }
    }
}

/// What a client says it can read, in [`CAPABILITIES_HEADER`]:
/// `responseformat=delta,parquet;readerfeatures=deletionvectors`. Keys
/// and values are case-insensitive.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Capabilities {
    /// The formats the client reads, in its order; `parquet` without the
    /// header.
    pub formats: Vec<ResponseFormat>,
    /// The Delta reader features the client supports, in lowercase.
    pub reader_features: HashSet<String>,
    /// `includeendstreamaction=true`: the response ends with an
    /// `endStreamAction` line.
    pub end_stream_action: bool,
}

impl Default for Capabilities {
    fn default() -> Self {
        Self {
            formats: vec![ResponseFormat::Parquet],
            reader_features: HashSet::new(),
            end_stream_action: false,
        }
    }
}

impl Capabilities {
    pub fn from_headers(headers: &HeaderMap) -> Result<Self> {
        let Some(value) = headers.get(CAPABILITIES_HEADER) else {
            return Ok(Self::default());
        };
        let value = value
            .to_str()
            .map_err(|_| Error::invalid_argument(format!("Invalid {CAPABILITIES_HEADER}")))?;
        Self::parse(value)
    }

    fn parse(value: &str) -> Result<Self> {
        let mut capabilities = Self::default();
        let pairs = value
            .split(';')
            .filter_map(|pair| pair.split_once('='))
            .map(|(key, values)| (key.trim().to_lowercase(), values.trim().to_lowercase()));
        for (key, values) in pairs {
            let values = values.split(',').map(str::trim).filter(|v| !v.is_empty());
            match key.as_str() {
                "responseformat" => {
                    capabilities.formats = values
                        .map(|format| match format {
                            "parquet" => Ok(ResponseFormat::Parquet),
                            "delta" => Ok(ResponseFormat::Delta),
                            other => Err(Error::invalid_argument(format!(
                                "Unsupported responseformat: {other}"
                            ))),
                        })
                        .collect::<Result<_>>()?;
                    if capabilities.formats.is_empty() {
                        capabilities.formats = vec![ResponseFormat::Parquet];
                    }
                }
                "readerfeatures" => {
                    capabilities.reader_features = values.map(str::to_string).collect();
                }
                "includeendstreamaction" => {
                    capabilities.end_stream_action = values.into_iter().any(|v| v == "true");
                }
                // `asyncquery` and keys of later protocol versions: queries
                // are answered at once.
                _ => {}
            }
        }
        Ok(capabilities)
    }

    /// The format of the answer for a table: one format if the client asks
    /// for one, else `parquet` when the table's files read as plain Parquet
    /// (`parquet_compatible`).
    pub fn format(&self, parquet_compatible: bool) -> ResponseFormat {
        match self.formats.as_slice() {
            [only] => *only,
            formats if parquet_compatible && formats.contains(&ResponseFormat::Parquet) => {
                ResponseFormat::Parquet
            }
            _ => ResponseFormat::Delta,
        }
    }

    pub fn supports(&self, reader_feature: &str) -> bool {
        self.reader_features
            .contains(&reader_feature.to_lowercase())
    }

    /// The [`CAPABILITIES_HEADER`] value of a response in `format`.
    pub fn response_header(&self, format: ResponseFormat) -> String {
        let mut value = format!("responseformat={}", format.as_str());
        if self.end_stream_action {
            value.push_str(";includeendstreamaction=true");
        }
        value
    }
}

/// The [`FILE_ID_HASH_HEADER`] of a request, in lowercase: `parquet` or
/// `delta`. Both name the same ids: a file's id is the same in both
/// formats.
pub fn file_id_hash(headers: &HeaderMap) -> Result<Option<String>> {
    let Some(value) = headers.get(FILE_ID_HASH_HEADER) else {
        return Ok(None);
    };
    let value = value.to_str().unwrap_or_default().trim().to_lowercase();
    match value.as_str() {
        "parquet" | "delta" => Ok(Some(value)),
        _ => Err(Error::new(
            ErrorCode::InvalidArgument,
            format!("Unsupported {FILE_ID_HASH_HEADER}: '{value}'. Supported: parquet, delta"),
        )),
    }
}

/// A protocol timestamp (`2022-01-01T00:00:00Z`) in epoch milliseconds.
/// Clients also send `2022-01-01 00:00:00` (UTC), as the reference server
/// accepts.
pub fn parse_timestamp(name: &str, value: &str) -> Result<i64> {
    let parsed = chrono::DateTime::parse_from_rfc3339(value)
        .map(|t| t.timestamp_millis())
        .or_else(|_| {
            chrono::NaiveDateTime::parse_from_str(value, "%Y-%m-%d %H:%M:%S%.f")
                .map(|t| t.and_utc().timestamp_millis())
        });
    parsed.map_err(|_| {
        Error::invalid_argument(format!(
            "Invalid {name}: {value}. Expected an ISO 8601 timestamp in UTC, such as \
             2022-01-01T00:00:00Z"
        ))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capabilities_are_case_insensitive() {
        let c = Capabilities::parse(
            "responseFormat=Delta,Parquet;readerFeatures=DeletionVectors,columnMapping;\
             includeEndStreamAction=true;asyncquery=true",
        )
        .unwrap();
        assert_eq!(c.formats, [ResponseFormat::Delta, ResponseFormat::Parquet]);
        assert!(c.supports("deletionVectors") && c.supports("columnmapping"));
        assert!(c.end_stream_action);
        assert_eq!(
            c.response_header(ResponseFormat::Parquet),
            "responseformat=parquet;includeendstreamaction=true"
        );
        assert!(Capabilities::parse("responseformat=avro").is_err());
    }

    #[test]
    fn one_format_is_kept_and_two_prefer_parquet() {
        let only = |f| Capabilities {
            formats: vec![f],
            ..Default::default()
        };
        assert_eq!(
            only(ResponseFormat::Delta).format(true),
            ResponseFormat::Delta
        );
        assert_eq!(
            only(ResponseFormat::Parquet).format(false),
            ResponseFormat::Parquet
        );
        let both = Capabilities {
            formats: vec![ResponseFormat::Delta, ResponseFormat::Parquet],
            ..Default::default()
        };
        assert_eq!(both.format(true), ResponseFormat::Parquet);
        assert_eq!(both.format(false), ResponseFormat::Delta);
    }

    #[test]
    fn timestamps_parse_in_both_forms() {
        assert_eq!(parse_timestamp("t", "1970-01-01T00:00:01Z").unwrap(), 1000);
        assert_eq!(
            parse_timestamp("t", "1970-01-01T01:00:01+01:00").unwrap(),
            1000
        );
        assert_eq!(parse_timestamp("t", "1970-01-01 00:00:01.5").unwrap(), 1500);
        assert!(parse_timestamp("t", "yesterday").is_err());
    }
}
