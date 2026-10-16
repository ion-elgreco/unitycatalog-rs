//! The metadata rules of Iceberg REST creates and commits, after upstream
//! `IcebergRestCatalogService.createTable` and `updateTable`.

use std::collections::HashMap;

use iceberg::{
    TableRequirement, TableUpdate,
    spec::{
        FormatVersion, Schema, SortOrder, TableMetadata, TableMetadataBuilder, UnboundPartitionSpec,
    },
};
use serde::Deserialize;

use uc_common::{Error, ErrorCode, NormalizedUrl, Result};

use super::metadata;

/// Iceberg's `format-version` table property, read when creating a table.
const FORMAT_VERSION: &str = "format-version";

/// A create body (Iceberg `CreateTableRequest`).
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct CreateTableBody {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub location: Option<String>,
    #[serde(default)]
    pub schema: Option<Schema>,
    #[serde(default)]
    pub partition_spec: Option<UnboundPartitionSpec>,
    #[serde(default)]
    pub write_order: Option<SortOrder>,
    #[serde(default)]
    pub stage_create: Option<bool>,
    #[serde(default)]
    pub properties: Option<HashMap<String, String>>,
}

/// A commit body (Iceberg `UpdateTableRequest`).
#[derive(Debug, Default, Deserialize)]
pub struct CommitBody {
    #[serde(default)]
    pub requirements: Vec<TableRequirement>,
    #[serde(default)]
    pub updates: Vec<TableUpdate>,
}

/// An `iceberg` error's message and context.
pub fn describe(e: &iceberg::Error) -> String {
    // Displayed as `<kind>, context: { k: v, … } => <message>, source: …`.
    let display = e.to_string();
    match display
        .split_once("context: { ")
        .and_then(|(_, rest)| rest.split_once(" } =>"))
    {
        Some((context, _)) => format!("{} ({context})", e.message()),
        None => e.message().to_string(),
    }
}

fn invalid(e: iceberg::Error) -> Error {
    Error::invalid_argument(describe(&e))
}

/// Upstream `TableMetadata.newTableMetadata`: fresh ids for the schema,
/// spec and sort order; the `format-version` property picks the version
/// (2 by default) and, like Iceberg's other reserved properties, is not
/// kept; new tables write zstd Parquet.
pub fn new_table_metadata(
    schema: Schema,
    spec: Option<UnboundPartitionSpec>,
    order: Option<SortOrder>,
    location: &NormalizedUrl,
    properties: HashMap<String, String>,
) -> Result<TableMetadata> {
    let format_version = match properties.get(FORMAT_VERSION) {
        None => FormatVersion::V2,
        Some(value) => match value.trim().parse::<i32>() {
            Ok(1) => FormatVersion::V1,
            Ok(2) => FormatVersion::V2,
            Ok(3) => FormatVersion::V3,
            Ok(n) => {
                return Err(Error::invalid_argument(format!(
                    "Unsupported format version: v{n} (supported: v3)"
                )));
            }
            Err(_) => {
                return Err(Error::invalid_argument(format!(
                    "For input string: \"{value}\""
                )));
            }
        },
    };
    let reserved = iceberg::spec::TableProperties::RESERVED_PROPERTIES;
    let mut persisted = HashMap::from([(
        "write.parquet.compression-codec".to_string(),
        "zstd".to_string(),
    )]);
    persisted.extend(
        properties
            .into_iter()
            .filter(|(key, _)| !reserved.contains(&key.as_str())),
    );
    let spec = spec.unwrap_or_else(|| UnboundPartitionSpec::builder().build());
    let order = order.unwrap_or_else(SortOrder::unsorted_order);
    let builder = TableMetadataBuilder::new(
        schema,
        spec,
        order,
        location.to_string(),
        format_version,
        persisted,
    )
    .map_err(invalid)?;
    Ok(builder.build().map_err(invalid)?.metadata)
}

/// Upstream `requireAtMostOneSetLocation`.
pub fn require_at_most_one_set_location(updates: &[TableUpdate]) -> Result<()> {
    let set_locations = updates
        .iter()
        .filter(|u| matches!(u, TableUpdate::SetLocation { .. }));
    if set_locations.count() > 1 {
        return Err(Error::invalid_argument(
            "A commit may set the table location at most once, but received more than one \
             set-location update.",
        ));
    }
    Ok(())
}

/// Upstream `isStagedCreate`: a commit with an `assert-create` requirement
/// creates the table.
pub fn is_staged_create(requirements: &[TableRequirement]) -> bool {
    requirements
        .iter()
        .any(|r| matches!(r, TableRequirement::NotExist))
}

/// The Java class name of a requirement, as upstream messages name it.
fn requirement_name(requirement: &TableRequirement) -> &'static str {
    match requirement {
        TableRequirement::NotExist => "AssertTableDoesNotExist",
        TableRequirement::UuidMatch { .. } => "AssertTableUUID",
        TableRequirement::RefSnapshotIdMatch { .. } => "AssertRefSnapshotID",
        TableRequirement::LastAssignedFieldIdMatch { .. } => "AssertLastAssignedFieldId",
        TableRequirement::CurrentSchemaIdMatch { .. } => "AssertCurrentSchemaID",
        TableRequirement::LastAssignedPartitionIdMatch { .. } => "AssertLastAssignedPartitionId",
        TableRequirement::DefaultSpecIdMatch { .. } => "AssertDefaultSpecID",
        TableRequirement::DefaultSortOrderIdMatch { .. } => "AssertDefaultSortOrderID",
    }
}

/// Upstream `commitStagedCreate`: the table a create commit describes.
///
/// Iceberg's reference handler applies the updates to empty metadata. Here
/// the first schema, spec and sort order seed new metadata instead. That
/// assigns fresh ids, so it is the same table only when the ids already
/// are fresh, as stage create returns them; data files written since name
/// those ids, so other ids are refused rather than changed. Every update
/// then applies in order.
pub fn staged_create_metadata(
    full_name: &str,
    requirements: &[TableRequirement],
    updates: Vec<TableUpdate>,
) -> Result<TableMetadata> {
    if let Some(other) = requirements
        .iter()
        .find(|r| !matches!(r, TableRequirement::NotExist))
    {
        return Err(Error::invalid_argument(format!(
            "Invalid requirement for a create commit: {}",
            requirement_name(other)
        )));
    }
    let mut format_version = None;
    let (mut location, mut schema, mut spec, mut order) = (None, None, None, None);
    for update in &updates {
        match update {
            TableUpdate::UpgradeFormatVersion { format_version: v } => {
                format_version.get_or_insert(*v);
            }
            TableUpdate::SetLocation { location: l } => location = Some(l.clone()),
            TableUpdate::AddSchema { schema: s } => {
                schema.get_or_insert_with(|| s.clone());
            }
            TableUpdate::AddSpec { spec: s } => {
                spec.get_or_insert_with(|| s.clone());
            }
            TableUpdate::AddSortOrder { sort_order: o } => {
                order.get_or_insert_with(|| o.clone());
            }
            _ => {}
        }
    }
    let location = location.filter(|l| !l.is_empty()).ok_or_else(|| {
        Error::invalid_argument(format!(
            "Create commit for {full_name} must include a set-location update"
        ))
    })?;
    let schema = schema.ok_or_else(|| {
        Error::invalid_argument(format!(
            "Create commit for {full_name} must include an add-schema update"
        ))
    })?;
    let spec = spec.unwrap_or_else(|| UnboundPartitionSpec::builder().build());
    let order = order.unwrap_or_else(SortOrder::unsorted_order);
    let seed = TableMetadataBuilder::new(
        schema.clone(),
        spec.clone(),
        order.clone(),
        location,
        format_version.unwrap_or(FormatVersion::V2),
        HashMap::new(),
    )
    .and_then(TableMetadataBuilder::build)
    .map_err(invalid)?
    .metadata;
    if !same_ids(&seed, &schema, &spec, &order) {
        return Err(Error::invalid_argument(format!(
            "Create commit for {full_name} must keep the field, partition and sort order ids \
             its staged create assigned"
        )));
    }
    let mut builder = seed.into_builder(None);
    for update in updates {
        builder = update.apply(builder).map_err(invalid)?;
    }
    Ok(builder.build().map_err(invalid)?.metadata)
}

/// Whether fresh ids left the client's schema, spec and sort order as sent.
fn same_ids(
    seed: &TableMetadata,
    schema: &Schema,
    spec: &UnboundPartitionSpec,
    order: &SortOrder,
) -> bool {
    let seed_spec = seed.default_partition_spec();
    let spec_fields_match = seed_spec.fields().len() == spec.fields().len()
        && seed_spec
            .fields()
            .iter()
            .zip(spec.fields())
            .all(|(fresh, sent)| {
                fresh.source_id == sent.source_id
                    && fresh.name == sent.name
                    && fresh.transform == sent.transform
                    && sent.field_id.is_none_or(|id| id == fresh.field_id)
            });
    let seed_order = seed.default_sort_order();
    seed.current_schema().as_struct() == schema.as_struct()
        && spec.spec_id().is_none_or(|id| id == seed_spec.spec_id())
        && spec_fields_match
        && seed_order.fields == order.fields
        && (order.fields.is_empty() || seed_order.order_id == order.order_id)
}

/// Upstream's requirement check: a requirement that no longer holds is a
/// lost race (409 `CommitFailedException`).
pub fn check_requirements(requirements: &[TableRequirement], base: &TableMetadata) -> Result<()> {
    for requirement in requirements {
        requirement
            .check(Some(base))
            .map_err(|e| Error::new(ErrorCode::UpdateRequirementConflict, describe(&e)))?;
    }
    Ok(())
}

/// `base`, read from `base_location`, with `updates` applied; `None` when
/// they change nothing.
pub fn apply_updates(
    base: TableMetadata,
    base_location: &NormalizedUrl,
    updates: Vec<TableUpdate>,
) -> Result<Option<TableMetadata>> {
    let mut builder = base.into_builder(Some(metadata::client_location(base_location)));
    for update in updates {
        builder = update.apply(builder).map_err(invalid)?;
    }
    let result = builder.build().map_err(invalid)?;
    Ok((!result.changes.is_empty()).then_some(result.metadata))
}
