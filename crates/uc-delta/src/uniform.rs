//! UniForm: Delta tables that Iceberg readers can read through converted
//! Iceberg metadata. Upstream `DeltaUniformUtils`.

use uc_api::{DeltaUniform, delta_api::DeltaUniformMetadata};
use uc_common::{Error, NormalizedUrl, Result};

/// Longest metadata location accepted, as upstream's column.
pub const MAX_METADATA_LOCATION_CHARS: usize = 65535;

/// The UniForm Iceberg fields of a request, checked and normalized once.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UniformIceberg {
    pub metadata_location: NormalizedUrl,
    pub converted_delta_version: i64,
    pub converted_delta_timestamp_ms: i64,
    pub base_converted_delta_version: Option<i64>,
}

fn invalid(message: impl Into<String>) -> Error {
    Error::invalid_argument(message)
}

/// Upstream `requireBasicShape`.
fn basic_shape(
    location: Option<NormalizedUrl>,
    version: Option<i64>,
    timestamp_ms: Option<i64>,
    base: Option<i64>,
) -> Result<UniformIceberg> {
    let location = location.ok_or_else(|| {
        invalid("uniform.iceberg.metadata-location is required when uniform is set.")
    })?;
    let version = version.ok_or_else(|| {
        invalid("uniform.iceberg.converted-delta-version is required when uniform is set.")
    })?;
    let timestamp_ms = timestamp_ms.ok_or_else(|| {
        invalid("uniform.iceberg.converted-delta-timestamp is required when uniform is set.")
    })?;
    let chars = location.as_str().chars().count();
    if chars > MAX_METADATA_LOCATION_CHARS {
        return Err(invalid(format!(
            "uniform.iceberg.metadata-location length ({chars}) exceeds the maximum allowed \
             ({MAX_METADATA_LOCATION_CHARS})."
        )));
    }
    Ok(UniformIceberg {
        metadata_location: location,
        converted_delta_version: version,
        converted_delta_timestamp_ms: timestamp_ms,
        base_converted_delta_version: base,
    })
}

/// Upstream `getUniformFields(DeltaUniformMetadata)`: the Delta API block.
pub fn from_delta_api(uniform: Option<&DeltaUniformMetadata>) -> Result<Option<UniformIceberg>> {
    let Some(uniform) = uniform else {
        return Ok(None);
    };
    let iceberg = uniform
        .iceberg
        .as_ref()
        .ok_or_else(|| invalid("uniform.iceberg is required when uniform is set."))?;
    let location = NormalizedUrl::parse_opt(iceberg.metadata_location.as_deref())?;
    basic_shape(
        location,
        iceberg.converted_delta_version,
        iceberg.converted_delta_timestamp,
        iceberg.base_converted_delta_version,
    )
    .map(Some)
}

/// Upstream `getUniformFields(DeltaCommit)`: the `/delta/preview/commits`
/// block, whose timestamp is an ISO-8601 instant. With a commit, the
/// converted version must be the commit's.
pub fn from_commit(
    uniform: Option<&DeltaUniform>,
    commit_version: Option<i64>,
) -> Result<Option<UniformIceberg>> {
    let Some(uniform) = uniform else {
        return Ok(None);
    };
    let iceberg = uniform
        .iceberg
        .as_ref()
        .ok_or_else(|| invalid("Field cannot be null in uniform: iceberg"))?;
    let location = NormalizedUrl::parse_opt(iceberg.metadata_location.as_deref())?;
    let timestamp_ms = iceberg
        .converted_delta_timestamp
        .as_deref()
        .map(|t| {
            chrono::DateTime::parse_from_rfc3339(t)
                .map(|d| d.timestamp_millis())
                .map_err(|_| invalid(format!("Text '{t}' could not be parsed as an instant")))
        })
        .transpose()?;
    let fields = basic_shape(
        location,
        iceberg.converted_delta_version,
        timestamp_ms,
        iceberg.base_converted_delta_version,
    )?;
    if let Some(version) = commit_version {
        require_version_equals(&fields, version)?;
    }
    Ok(Some(fields))
}

/// Upstream `requireConvertedDeltaVersionEquals`.
pub fn require_version_equals(fields: &UniformIceberg, commit_version: i64) -> Result<()> {
    if fields.converted_delta_version != commit_version {
        return Err(invalid(format!(
            "uniform.iceberg.converted-delta-version ({}) must equal commit version ({commit_version})",
            fields.converted_delta_version
        )));
    }
    Ok(())
}

/// Upstream `validateCreate`: at create time the converted version is 0 or
/// 1, there is no base version, and the metadata is inside the table.
pub fn validate_create(
    fields: Option<&UniformIceberg>,
    table_location: &NormalizedUrl,
) -> Result<()> {
    let Some(fields) = fields else { return Ok(()) };
    let version = fields.converted_delta_version;
    if version != 0 && version != 1 {
        return Err(invalid(format!(
            "uniform.iceberg.converted-delta-version must be 0 or 1 at create time, got {version}."
        )));
    }
    if fields.base_converted_delta_version.is_some() {
        return Err(invalid(
            "uniform.iceberg.base-converted-delta-version must not be set at create time: the \
             table has no prior converted-delta-version for it to reference.",
        ));
    }
    require_subpath(&fields.metadata_location, table_location)
}

/// Upstream `requireMetadataLocationSubpath`: the Iceberg metadata must be
/// strictly inside the table's location.
pub fn require_subpath(
    metadata_location: &NormalizedUrl,
    table_location: &NormalizedUrl,
) -> Result<()> {
    if !metadata_location
        .as_str()
        .starts_with(&format!("{table_location}/"))
    {
        return Err(invalid(format!(
            "uniform.iceberg.metadata-location ('{metadata_location}') must be a subpath of the \
             table location ('{table_location}')."
        )));
    }
    Ok(())
}
