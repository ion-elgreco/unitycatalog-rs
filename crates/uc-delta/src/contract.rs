//! The UC catalog-managed Delta table contract and the table-property
//! projections of the Delta API. Upstream `UcManagedDeltaContract`,
//! `DeltaConsts` and `DeltaPropertyMapper`.

use std::collections::{BTreeMap, BTreeSet};

use uc_api::delta_api::{
    DeltaClusteringDomainMetadata, DeltaDomainMetadataUpdates, DeltaProtocol,
    DeltaRowTrackingDomainMetadata,
};
use uc_common::{Error, Result, SecurablePropertiesMap};

/// The table property that ties a MANAGED Delta table to its UC table id.
/// Upstream `TableProperties.UC_TABLE_ID`.
pub const UC_TABLE_ID_PROPERTY: &str = "io.unitycatalog.tableId";

/// Upstream `UcManagedDeltaContract.validateTableIdProperty`.
pub fn validate_table_id_property(
    properties: Option<&SecurablePropertiesMap>,
    expected: &str,
) -> Result<()> {
    let actual = properties
        .and_then(|p| p.get(UC_TABLE_ID_PROPERTY))
        .ok_or_else(|| {
            Error::invalid_argument(format!(
                "Properties does not contain {UC_TABLE_ID_PROPERTY}."
            ))
        })?;
    if actual != expected {
        return Err(Error::invalid_argument(format!(
            "the table id ({expected}) does not match the properties \
             {UC_TABLE_ID_PROPERTY}({actual})."
        )));
    }
    Ok(())
}

pub const LAST_UPDATE_VERSION: &str = "delta.lastUpdateVersion";
pub const LAST_COMMIT_TIMESTAMP: &str = "delta.lastCommitTimestamp";
pub const FEATURE_PREFIX: &str = "delta.feature.";
pub const MIN_READER_VERSION: &str = "delta.minReaderVersion";
pub const MIN_WRITER_VERSION: &str = "delta.minWriterVersion";
/// Clustering columns, a JSON list of column paths.
pub const CLUSTERING_COLUMNS: &str = "clusteringColumns";
pub const ROW_ID_HIGH_WATER_MARK: &str = "delta.rowTracking.rowIdHighWaterMark";
pub const UNIVERSAL_FORMAT_ENABLED_FORMATS: &str = "delta.universalFormat.enabledFormats";
pub const ENABLE_ICEBERG_COMPAT_V2: &str = "delta.enableIcebergCompatV2";
pub const ENABLE_DELETION_VECTORS: &str = "delta.enableDeletionVectors";

pub const CLUSTERING_DOMAIN: &str = "delta.clustering";
pub const ROW_TRACKING_DOMAIN: &str = "delta.rowTracking";

/// Reader version 3 negotiates reader features.
pub const REQUIRED_MIN_READER_VERSION: i32 = 3;
/// Writer version 7 negotiates writer features.
pub const REQUIRED_MIN_WRITER_VERSION: i32 = 7;

/// `(feature, reader-writer?)`: reader-writer features go in both lists,
/// writer-only features in `writer-features` only.
const REQUIRED_FEATURES: &[(&str, bool)] = &[
    ("catalogManaged", true),
    ("v2Checkpoint", true),
    ("vacuumProtocolCheck", true),
    ("deletionVectors", true),
    ("inCommitTimestamp", false),
];
const SUGGESTED_FEATURES: &[(&str, bool)] = &[
    ("columnMapping", true),
    ("domainMetadata", false),
    ("rowTracking", false),
];

fn reader_features(features: &[(&str, bool)]) -> Vec<String> {
    features
        .iter()
        .filter(|(_, rw)| *rw)
        .map(|(f, _)| f.to_string())
        .collect()
}

fn writer_features(features: &[(&str, bool)]) -> Vec<String> {
    features.iter().map(|(f, _)| f.to_string()).collect()
}

pub fn required_reader_features() -> Vec<String> {
    reader_features(REQUIRED_FEATURES)
}
pub fn required_writer_features() -> Vec<String> {
    writer_features(REQUIRED_FEATURES)
}
pub fn suggested_reader_features() -> Vec<String> {
    reader_features(SUGGESTED_FEATURES)
}
pub fn suggested_writer_features() -> Vec<String> {
    writer_features(SUGGESTED_FEATURES)
}

/// Required properties with fixed values.
pub const REQUIRED_FIXED_PROPERTIES: &[(&str, &str)] = &[
    (ENABLE_DELETION_VECTORS, "true"),
    ("delta.checkpointPolicy", "v2"),
    ("delta.enableInCommitTimestamps", "true"),
    ("delta.checkpoint.writeStatsAsStruct", "true"),
    ("delta.checkpoint.writeStatsAsJson", "true"),
];

/// Suggested properties; `None` means the client generates the value.
pub fn suggested_properties() -> BTreeMap<String, Option<String>> {
    [
        ("delta.columnMapping.mode", Some("name")),
        ("delta.columnMapping.maxColumnId", None),
        ("delta.enableRowTracking", Some("true")),
        ("delta.rowTracking.materializedRowIdColumnName", None),
        (
            "delta.rowTracking.materializedRowCommitVersionColumnName",
            None,
        ),
        ("delta.randomizeFilePrefixes", Some("true")),
        ("delta.parquet.compression.codec", Some("zstd")),
    ]
    .into_iter()
    .map(|(k, v)| (k.to_string(), v.map(str::to_string)))
    .collect()
}

/// The properties a staging table's first commit must carry.
pub fn required_properties(table_id: &str) -> BTreeMap<String, Option<String>> {
    let mut props: BTreeMap<String, Option<String>> = REQUIRED_FIXED_PROPERTIES
        .iter()
        .map(|(k, v)| (k.to_string(), Some(v.to_string())))
        .collect();
    props.insert(UC_TABLE_ID_PROPERTY.to_string(), Some(table_id.to_string()));
    props
}

fn invalid(message: impl Into<String>) -> Error {
    Error::invalid_argument(message)
}

/// Whether the deletion-vector requirement is waived: the server allows it
/// (`tables.allow_missing_deletion_vectors`) and the
/// table enables IcebergCompatV2.
pub fn skip_deletion_vectors(
    properties: &BTreeMap<String, String>,
    allow_missing_dv: bool,
) -> bool {
    allow_missing_dv && properties.get(ENABLE_ICEBERG_COMPAT_V2).map(String::as_str) == Some("true")
}

/// Upstream `UcManagedDeltaContract.validate`: the protocol, domain
/// metadata and properties of a MANAGED table.
pub fn validate(
    protocol: Option<&DeltaProtocol>,
    domain_metadata: Option<&DeltaDomainMetadataUpdates>,
    properties: &BTreeMap<String, String>,
    allow_missing_dv: bool,
) -> Result<()> {
    let skip_dv = skip_deletion_vectors(properties, allow_missing_dv);
    let protocol = protocol.ok_or_else(|| invalid("protocol is required."))?;
    let readers: BTreeSet<&str> = protocol
        .reader_features
        .iter()
        .flatten()
        .map(String::as_str)
        .collect();
    let writers: BTreeSet<&str> = protocol
        .writer_features
        .iter()
        .flatten()
        .map(String::as_str)
        .collect();
    for feature in protocol.reader_features.iter().flatten() {
        if !writers.contains(feature.as_str()) {
            return Err(invalid(format!(
                "Feature '{feature}' is in reader-features but not writer-features. Per the Delta \
                 protocol, every reader-feature must also be a writer-feature."
            )));
        }
    }
    let version = |v: Option<i32>| v.map_or("null".to_string(), |v| v.to_string());
    if protocol
        .min_reader_version
        .is_none_or(|v| v < REQUIRED_MIN_READER_VERSION)
    {
        return Err(invalid(format!(
            "MANAGED table minReaderVersion must be at least {REQUIRED_MIN_READER_VERSION} (got {}).",
            version(protocol.min_reader_version)
        )));
    }
    if protocol
        .min_writer_version
        .is_none_or(|v| v < REQUIRED_MIN_WRITER_VERSION)
    {
        return Err(invalid(format!(
            "MANAGED table minWriterVersion must be at least {REQUIRED_MIN_WRITER_VERSION} (got {}).",
            version(protocol.min_writer_version)
        )));
    }
    let dv = |f: &str| skip_dv && f == "deletionVectors";
    for required in required_reader_features() {
        if !dv(&required) && !readers.contains(required.as_str()) {
            return Err(invalid(format!(
                "MANAGED table missing required reader-feature '{required}'."
            )));
        }
    }
    for required in required_writer_features() {
        if !dv(&required) && !writers.contains(required.as_str()) {
            return Err(invalid(format!(
                "MANAGED table missing required writer-feature '{required}'."
            )));
        }
    }
    validate_domain_metadata(&writers, domain_metadata)?;
    for (key, expected) in REQUIRED_FIXED_PROPERTIES {
        if skip_dv && *key == ENABLE_DELETION_VECTORS {
            continue;
        }
        let actual = properties.get(*key);
        if actual.map(String::as_str) != Some(*expected) {
            return Err(invalid(format!(
                "MANAGED table required property '{key}' must be '{expected}' (got '{}').",
                actual.map_or("null", String::as_str)
            )));
        }
    }
    if properties
        .get(UC_TABLE_ID_PROPERTY)
        .is_none_or(|id| id.trim().is_empty())
    {
        return Err(invalid(format!(
            "MANAGED table required property '{UC_TABLE_ID_PROPERTY}' is missing."
        )));
    }
    Ok(())
}

/// Upstream `validateDomainMetadataAgainstWriterFeatures`: each domain
/// needs its writer feature.
pub fn validate_domain_metadata(
    writer_features: &BTreeSet<&str>,
    domain_metadata: Option<&DeltaDomainMetadataUpdates>,
) -> Result<()> {
    let Some(dm) = domain_metadata else {
        return Ok(());
    };
    if dm.delta_clustering.is_some() && !writer_features.contains("clustering") {
        return Err(invalid(
            "domain-metadata.delta.clustering requires the 'clustering' writer feature.",
        ));
    }
    if dm.delta_row_tracking.is_some() && !writer_features.contains("rowTracking") {
        return Err(invalid(
            "domain-metadata.delta.rowTracking requires the 'rowTracking' writer feature.",
        ));
    }
    Ok(())
}

/// Upstream `deriveFromProtocol`: `delta.minReaderVersion`,
/// `delta.minWriterVersion` and `delta.feature.<name> = supported`.
pub fn derive_from_protocol(
    props: &mut BTreeMap<String, String>,
    protocol: Option<&DeltaProtocol>,
) {
    let Some(protocol) = protocol else { return };
    if let Some(v) = protocol.min_reader_version {
        props.insert(MIN_READER_VERSION.into(), v.to_string());
    }
    if let Some(v) = protocol.min_writer_version {
        props.insert(MIN_WRITER_VERSION.into(), v.to_string());
    }
    for feature in protocol
        .reader_features
        .iter()
        .chain(protocol.writer_features.iter())
        .flatten()
    {
        props.insert(format!("{FEATURE_PREFIX}{feature}"), "supported".into());
    }
}

/// The stored property projected from a domain.
pub fn domain_property(domain: &str) -> Option<&'static str> {
    match domain {
        CLUSTERING_DOMAIN => Some(CLUSTERING_COLUMNS),
        ROW_TRACKING_DOMAIN => Some(ROW_ID_HIGH_WATER_MARK),
        _ => None,
    }
}

/// Upstream `deriveFromDomainMetadata`.
pub fn derive_from_domain_metadata(
    props: &mut BTreeMap<String, String>,
    domain_metadata: Option<&DeltaDomainMetadataUpdates>,
) {
    let Some(dm) = domain_metadata else { return };
    if let Some(columns) = dm
        .delta_clustering
        .as_ref()
        .and_then(|c| c.clustering_columns.as_ref())
    {
        let json = serde_json::to_string(columns).unwrap_or_default();
        props.insert(CLUSTERING_COLUMNS.into(), json);
    }
    if let Some(mark) = dm
        .delta_row_tracking
        .as_ref()
        .and_then(|r| r.row_id_high_water_mark)
    {
        props.insert(ROW_ID_HIGH_WATER_MARK.into(), mark.to_string());
    }
}

/// Upstream `synthesizeDomainMetadataFromProperties`: the domain metadata
/// the stored properties still project.
pub fn domain_metadata_from_properties(
    props: &BTreeMap<String, String>,
) -> Result<DeltaDomainMetadataUpdates> {
    let mut dm = DeltaDomainMetadataUpdates::default();
    if let Some(json) = props.get(CLUSTERING_COLUMNS) {
        let columns: Vec<Vec<String>> = serde_json::from_str(json).map_err(|e| {
            Error::internal(format!("Corrupt stored property {CLUSTERING_COLUMNS}: {e}"))
        })?;
        dm.delta_clustering = Some(DeltaClusteringDomainMetadata {
            clustering_columns: Some(columns),
        });
    }
    if let Some(mark) = props.get(ROW_ID_HIGH_WATER_MARK) {
        let mark: i64 = mark.parse().map_err(|e| {
            Error::internal(format!(
                "Corrupt stored property {ROW_ID_HIGH_WATER_MARK}: {e}"
            ))
        })?;
        dm.delta_row_tracking = Some(DeltaRowTrackingDomainMetadata {
            row_id_high_water_mark: Some(mark),
        });
    }
    Ok(dm)
}

/// Upstream `extractFeaturesFromProperties`: the `delta.feature.*` names.
pub fn features_from_properties(props: &BTreeMap<String, String>) -> BTreeSet<String> {
    props
        .keys()
        .filter_map(|k| k.strip_prefix(FEATURE_PREFIX))
        .map(str::to_string)
        .collect()
}

/// Upstream `DeltaUniformUtils.isIcebergEnabled`.
pub fn iceberg_enabled(props: &BTreeMap<String, String>) -> bool {
    props
        .get(UNIVERSAL_FORMAT_ENABLED_FORMATS)
        .is_some_and(|v| v.split(',').any(|f| f == "iceberg"))
}

/// Upstream `DeltaUniformUtils.validateConsistency`: UniForm metadata is
/// present exactly when the properties enable UniForm Iceberg.
pub fn validate_uniform_consistency(
    props: &BTreeMap<String, String>,
    uniform_present: bool,
) -> Result<()> {
    match (iceberg_enabled(props), uniform_present) {
        (true, false) => Err(invalid(format!(
            "Uniform metadata must be set when '{UNIVERSAL_FORMAT_ENABLED_FORMATS}' includes 'iceberg'."
        ))),
        (false, true) => Err(invalid(format!(
            "Uniform metadata must not be set unless '{UNIVERSAL_FORMAT_ENABLED_FORMATS}' includes 'iceberg'."
        ))),
        _ => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn protocol() -> DeltaProtocol {
        DeltaProtocol {
            min_reader_version: Some(3),
            min_writer_version: Some(7),
            reader_features: Some(required_reader_features()),
            writer_features: Some(required_writer_features()),
        }
    }

    fn properties() -> BTreeMap<String, String> {
        required_properties("t-1")
            .into_iter()
            .map(|(k, v)| (k, v.unwrap()))
            .collect()
    }

    #[test]
    fn the_contract_follows_upstream() {
        assert!(validate(Some(&protocol()), None, &properties(), false).is_ok());
        let cases: Vec<(DeltaProtocol, BTreeMap<String, String>, &str)> = vec![
            (
                DeltaProtocol {
                    min_reader_version: Some(2),
                    ..protocol()
                },
                properties(),
                "MANAGED table minReaderVersion must be at least 3 (got 2).",
            ),
            (
                DeltaProtocol {
                    writer_features: Some(vec!["catalogManaged".into()]),
                    ..protocol()
                },
                properties(),
                "Feature 'v2Checkpoint' is in reader-features but not writer-features.",
            ),
            (
                DeltaProtocol {
                    reader_features: Some(vec![]),
                    ..protocol()
                },
                properties(),
                "MANAGED table missing required reader-feature 'catalogManaged'.",
            ),
            (
                protocol(),
                {
                    let mut p = properties();
                    p.insert("delta.checkpointPolicy".into(), "classic".into());
                    p
                },
                "MANAGED table required property 'delta.checkpointPolicy' must be 'v2' (got 'classic').",
            ),
            (
                protocol(),
                {
                    let mut p = properties();
                    p.remove(UC_TABLE_ID_PROPERTY);
                    p
                },
                "MANAGED table required property 'io.unitycatalog.tableId' is missing.",
            ),
        ];
        for (protocol, props, message) in cases {
            let err = validate(Some(&protocol), None, &props, false).unwrap_err();
            assert!(err.message().starts_with(message), "{}", err.message());
        }
        // IcebergCompatV2 tables may skip deletion vectors when the server allows it.
        let mut props = properties();
        props.remove(ENABLE_DELETION_VECTORS);
        props.insert(ENABLE_ICEBERG_COMPAT_V2.into(), "true".into());
        let no_dv = |mut v: Vec<String>| {
            v.retain(|f| f != "deletionVectors");
            Some(v)
        };
        let p = DeltaProtocol {
            reader_features: no_dv(required_reader_features()),
            writer_features: no_dv(required_writer_features()),
            ..protocol()
        };
        assert!(validate(Some(&p), None, &props, true).is_ok());
        assert!(validate(Some(&p), None, &props, false).is_err());
    }

    #[test]
    fn domains_need_their_writer_features() {
        let dm = DeltaDomainMetadataUpdates {
            delta_clustering: Some(DeltaClusteringDomainMetadata {
                clustering_columns: Some(vec![]),
            }),
            ..Default::default()
        };
        let err = validate(Some(&protocol()), Some(&dm), &properties(), false).unwrap_err();
        assert_eq!(
            err.message(),
            "domain-metadata.delta.clustering requires the 'clustering' writer feature."
        );
        let mut props = BTreeMap::new();
        derive_from_domain_metadata(
            &mut props,
            Some(&DeltaDomainMetadataUpdates {
                delta_clustering: Some(DeltaClusteringDomainMetadata {
                    clustering_columns: Some(vec![vec!["a".into(), "b".into()]]),
                }),
                delta_row_tracking: Some(DeltaRowTrackingDomainMetadata {
                    row_id_high_water_mark: Some(7),
                }),
            }),
        );
        assert_eq!(props[CLUSTERING_COLUMNS], r#"[["a","b"]]"#);
        assert_eq!(
            domain_metadata_from_properties(&props)
                .unwrap()
                .delta_row_tracking
                .unwrap()
                .row_id_high_water_mark,
            Some(7)
        );
    }
}
