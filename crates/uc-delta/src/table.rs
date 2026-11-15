//! The Delta API view of a table. Upstream `TableRepository`'s Delta
//! response building and `ColumnUtils`' column conversions.

use std::collections::BTreeMap;

use delta_kernel::schema::{StructField, StructType};
use serde_json::{Value, json};
use tracing::warn;

use uc_api::{
    ColumnInfo, CreateTable, DataSourceFormat, TableInfo, TableType,
    delta_api::{
        DeltaCommitWire, DeltaCreateTableRequest, DeltaLoadTableResponse,
        DeltaMaintenanceOperation, DeltaTableMetadata, DeltaTableType,
    },
};
use uc_common::{Error, ErrorCode, NormalizedUrl, Result};

use crate::commits::StoredCommit;
use crate::contract::{self as contract, LAST_COMMIT_TIMESTAMP, LAST_UPDATE_VERSION};
use crate::fields::{self, CheckedSchema, KeyStyle};

/// Upstream `requireDeltaTable`: only Delta tables are served.
pub fn require_delta(format: Option<DataSourceFormat>, full_name: &str) -> Result<()> {
    if format != Some(DataSourceFormat::Delta) {
        return Err(Error::new(
            ErrorCode::UnsupportedTableFormat,
            format!("Table is not a Delta table: {full_name}"),
        ));
    }
    Ok(())
}

/// Upstream `computeEtag`: changes whenever the table's metadata does.
pub fn etag(info: &TableInfo) -> String {
    match info.updated_at {
        Some(at) => format!("etag-{at}"),
        None => format!("etag-{}", info.table_id.as_deref().unwrap_or_default()),
    }
}

/// A column's `type_json`, as JSON.
fn type_json(column: &ColumnInfo) -> std::result::Result<Value, String> {
    let type_json = column
        .type_json
        .as_deref()
        .filter(|t| !t.is_empty())
        .ok_or_else(|| {
            format!(
                "Column {} has null/empty typeJson",
                column.name.as_deref().unwrap_or("null")
            )
        })?;
    serde_json::from_str(type_json).map_err(|e| e.to_string())
}

/// A column's `type_json` as the kernel's struct field.
pub fn struct_field(column: &ColumnInfo) -> std::result::Result<StructField, String> {
    serde_json::from_value(type_json(column)?).map_err(|e| e.to_string())
}

/// The stored columns as the kernel's struct type.
pub fn stored_schema(columns: &[ColumnInfo]) -> std::result::Result<StructType, String> {
    let fields = columns
        .iter()
        .map(struct_field)
        .collect::<std::result::Result<Vec<_>, _>>()?;
    StructType::try_new(fields).map_err(fields::kernel_message)
}

/// Upstream `ColumnUtils.toColumnInfo`: a checked Delta field as a UC
/// column, like Spark's `UCSingleCatalog` writes it. `type_json` is the
/// field's camelCase JSON.
pub fn column_info(field: &StructField, type_json: &Value, position: i32) -> Result<ColumnInfo> {
    let type_name = fields::column_type_name(field.data_type()).map_err(Error::invalid_argument)?;
    Ok(ColumnInfo {
        name: Some(field.name().clone()),
        type_text: Some(
            fields::catalog_string(field.data_type()).map_err(Error::invalid_argument)?,
        ),
        type_json: Some(type_json.to_string()),
        type_name: Some(type_name),
        position: Some(position),
        comment: fields::comment(field).map(str::to_string),
        nullable: Some(field.is_nullable()),
        ..Default::default()
    })
}

/// Read and check a Delta struct type (`columns`) in kebab case as upstream
/// `ColumnUtils.validateStructType`, at `path` in messages.
pub fn struct_type(columns: Option<&Value>, path: &str) -> Result<CheckedSchema> {
    let columns = columns
        .filter(|c| !c.is_null())
        .ok_or_else(|| Error::invalid_argument(format!("{path} is required.")))?;
    let type_jsons = match columns.get("fields") {
        None | Some(Value::Null) => None,
        Some(Value::Array(items)) => Some(
            items
                .iter()
                .map(|f| fields::normalize(f, KeyStyle::Kebab, KeyStyle::Camel))
                .collect::<std::result::Result<Vec<_>, _>>()
                .map_err(Error::invalid_argument)?,
        ),
        Some(_) => {
            return Err(Error::invalid_argument(format!(
                "{path}.fields must be an array."
            )));
        }
    };
    let schema =
        fields::check_fields(type_jsons.as_deref(), path).map_err(Error::invalid_argument)?;
    Ok(CheckedSchema {
        schema,
        type_jsons: type_jsons.unwrap_or_default(),
    })
}

/// Upstream `ColumnUtils.toColumnInfos`: positions follow the list order.
pub fn column_infos(checked: &CheckedSchema) -> Result<Vec<ColumnInfo>> {
    checked
        .schema
        .fields()
        .zip(&checked.type_jsons)
        .enumerate()
        .map(|(i, (field, type_json))| column_info(field, type_json, i as i32))
        .collect()
}

/// Upstream `DeltaCreateTableMapper.toCreateTable`: the UC create request
/// for a Delta API create, and its checked UniForm fields. MANAGED tables
/// must meet the catalog-managed contract.
pub fn create_table_request(
    catalog: &str,
    schema: &str,
    req: &DeltaCreateTableRequest,
    allow_missing_dv: bool,
) -> Result<(CreateTable, Option<crate::uniform::UniformIceberg>)> {
    let invalid = Error::invalid_argument;
    let name = req.name.clone().filter(|n| !n.trim().is_empty());
    let name = name.ok_or_else(|| invalid("Table name is required."))?;
    let location = req.location.clone().filter(|l| !l.trim().is_empty());
    let location = location.ok_or_else(|| invalid("Table location is required."))?;
    let checked = struct_type(req.columns.as_ref(), "columns")?;
    // A column-mapped table starts with the identities that set-columns
    // requires on every update.
    let mode = req
        .properties
        .as_ref()
        .and_then(|p| p.get(crate::column_mapping::MODE));
    crate::column_mapping::validate_identities(
        &checked.schema,
        crate::column_mapping::parse_mode(mode.map(String::as_str))?,
    )?;
    let table_type = match req.table_type {
        Some(DeltaTableType::Managed) => TableType::Managed,
        Some(DeltaTableType::External) => TableType::External,
        None => return Err(invalid("table-type is required.")),
    };
    if req.protocol.is_none() {
        return Err(invalid("protocol is required."));
    }
    let last_commit_timestamp = req
        .last_commit_timestamp_ms
        .ok_or_else(|| invalid("last-commit-timestamp-ms is required."))?;
    let client_properties = req.properties.clone().unwrap_or_default();
    // EXTERNAL tables mirror what the client wrote; MANAGED ones must meet
    // the contract.
    let protocol = match table_type {
        TableType::Managed => Some(contract::validate(
            req.protocol.as_ref(),
            req.domain_metadata.as_ref(),
            &client_properties,
            allow_missing_dv,
        )?),
        _ => None,
    };
    contract::validate_uniform_consistency(&client_properties, req.uniform.is_some())?;
    let uniform = crate::uniform::from_delta_api(req.uniform.as_ref())?;
    crate::uniform::validate_create(uniform.as_ref(), &NormalizedUrl::parse(&location)?)?;

    let mut columns = column_infos(&checked)?;
    apply_partition_columns(
        &mut columns,
        req.partition_columns.as_deref().unwrap_or_default(),
    )?;
    if let Some(protocol) = protocol {
        contract::validate_table_configuration(
            protocol,
            checked.schema,
            partition_columns(&columns),
            &client_properties,
        )?;
    }

    // Upstream `buildStoredProperties`: client properties, overlaid with the
    // protocol and domain-metadata projections and the snapshot stamps.
    let mut properties = client_properties;
    contract::derive_from_protocol(&mut properties, req.protocol.as_ref());
    contract::derive_from_domain_metadata(&mut properties, req.domain_metadata.as_ref());
    properties.insert(
        LAST_COMMIT_TIMESTAMP.into(),
        last_commit_timestamp.to_string(),
    );
    properties.insert(LAST_UPDATE_VERSION.into(), "0".into());

    let create = CreateTable {
        name,
        catalog_name: catalog.to_string(),
        schema_name: schema.to_string(),
        table_type,
        data_source_format: Some(DataSourceFormat::Delta),
        columns,
        storage_location: Some(location),
        comment: req.comment.clone(),
        properties: Some(properties),
        view_definition: None,
        view_dependencies: None,
    };
    Ok((create, uniform))
}

/// Upstream `ColumnUtils.applyPartitionColumns`: stamp each partition
/// column's index. Names match case-insensitively.
pub fn apply_partition_columns(
    columns: &mut [ColumnInfo],
    partition_columns: &[String],
) -> Result<()> {
    let mut seen = std::collections::HashSet::new();
    for (index, name) in partition_columns.iter().enumerate() {
        let lower = name.to_lowercase();
        if !seen.insert(lower.clone()) {
            return Err(Error::invalid_argument(format!(
                "partition-columns contains duplicate entry (case-insensitive): {name}"
            )));
        }
        let column = columns
            .iter_mut()
            .find(|c| c.name.as_deref().map(str::to_lowercase).as_deref() == Some(lower.as_str()))
            .ok_or_else(|| {
                Error::invalid_argument(format!(
                    "partition-columns references unknown column: {name}"
                ))
            })?;
        column.partition_index = Some(index as i32);
    }
    Ok(())
}

/// The table's columns as a Delta struct type. Unreadable columns give an
/// empty schema, as upstream (best effort).
fn columns_struct(columns: &[ColumnInfo], full_name: &str) -> Value {
    let fields: std::result::Result<Vec<Value>, String> = columns
        .iter()
        .map(|c| fields::normalize(&type_json(c)?, KeyStyle::Camel, KeyStyle::Kebab))
        .collect();
    match fields {
        Ok(fields) => json!({ "type": "struct", "fields": fields }),
        Err(e) => {
            warn!(table = full_name, error = %e, "cannot parse columns; returning an empty schema");
            json!({ "type": "struct", "fields": [] })
        }
    }
}

/// Partition columns by index; a gap means a corrupt spec, which upstream
/// reports as no partition columns.
pub fn partition_columns(columns: &[ColumnInfo]) -> Vec<String> {
    let mut partitioned: Vec<&ColumnInfo> = columns
        .iter()
        .filter(|c| c.partition_index.is_some())
        .collect();
    partitioned.sort_by_key(|c| c.partition_index);
    if partitioned
        .iter()
        .enumerate()
        .any(|(i, c)| c.partition_index != Some(i as i32))
    {
        return vec![];
    }
    partitioned.iter().filter_map(|c| c.name.clone()).collect()
}

/// Upstream `buildTableMetadata`.
pub fn table_metadata(info: &TableInfo, full_name: &str) -> DeltaTableMetadata {
    let columns = info.columns.clone().unwrap_or_default();
    let properties: BTreeMap<String, String> = info.properties.clone().unwrap_or_default();
    let long = |key: &str| properties.get(key).and_then(|v| v.parse::<i64>().ok());
    let location = info.storage_location.clone().unwrap_or_default();
    DeltaTableMetadata {
        etag: etag(info),
        table_type: match info.table_type {
            Some(TableType::External) => DeltaTableType::External,
            _ => DeltaTableType::Managed,
        },
        table_uuid: info.table_id.clone().unwrap_or_default(),
        location: NormalizedUrl::parse(&location)
            .map(NormalizedUrl::into_string)
            .unwrap_or(location),
        created_time: info.created_at.unwrap_or_default(),
        updated_time: info.updated_at.unwrap_or_default(),
        columns: columns_struct(&columns, full_name),
        partition_columns: partition_columns(&columns),
        last_commit_version: long(LAST_UPDATE_VERSION),
        last_commit_timestamp_ms: long(LAST_COMMIT_TIMESTAMP),
        properties,
    }
}

/// Upstream `buildLoadTableResponse`. `commits` is the unbackfilled
/// commits and latest version of a MANAGED table.
pub fn load_response(
    info: &TableInfo,
    full_name: &str,
    commits: Option<(Vec<StoredCommit>, i64)>,
) -> DeltaLoadTableResponse {
    let (commits, latest_table_version) = match commits {
        Some((commits, latest)) => (
            Some(
                commits
                    .into_iter()
                    .map(|c| DeltaCommitWire {
                        version: Some(c.commit_version),
                        timestamp: Some(c.commit_timestamp),
                        file_name: Some(c.commit_filename),
                        file_size: Some(c.commit_filesize),
                        file_modification_timestamp: Some(c.commit_file_modification_timestamp),
                    })
                    .collect(),
            ),
            Some(latest),
        ),
        None => (None, None),
    };
    DeltaLoadTableResponse {
        metadata: table_metadata(info, full_name),
        commits,
        uniform: None,
        latest_table_version,
        allowed_maintenance_operations: vec![
            DeltaMaintenanceOperation::DataReorganization,
            DeltaMaintenanceOperation::DataCleanup,
            DeltaMaintenanceOperation::MetadataCleanup,
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn column(name: &str, partition: Option<i32>) -> ColumnInfo {
        ColumnInfo {
            name: Some(name.into()),
            partition_index: partition,
            ..Default::default()
        }
    }

    #[test]
    fn partition_columns_follow_their_indexes() {
        let cols = [
            column("a", Some(1)),
            column("b", None),
            column("c", Some(0)),
        ];
        assert_eq!(partition_columns(&cols), ["c", "a"]);
        assert!(
            partition_columns(&[column("a", Some(1))]).is_empty(),
            "a gap is corrupt"
        );
        let mut cols = vec![column("Id", None), column("day", None)];
        apply_partition_columns(&mut cols, &["DAY".into(), "id".into()]).unwrap();
        assert_eq!(
            (cols[0].partition_index, cols[1].partition_index),
            (Some(1), Some(0))
        );
        let err = apply_partition_columns(&mut cols, &["x".into()]).unwrap_err();
        assert_eq!(
            err.message(),
            "partition-columns references unknown column: x"
        );
        let err = apply_partition_columns(&mut cols, &["id".into(), "ID".into()]).unwrap_err();
        assert_eq!(
            err.message(),
            "partition-columns contains duplicate entry (case-insensitive): ID"
        );
    }

    #[test]
    fn columns_round_trip_through_type_json() {
        let columns = json!({ "type": "struct", "fields": [
            { "name": "id", "type": "long", "nullable": false, "metadata": {} },
            { "name": "amount", "type": { "type": "decimal", "precision": 10, "scale": 2 },
              "nullable": true, "metadata": { "comment": "money" } },
        ] });
        let checked = struct_type(Some(&columns), "columns").unwrap();
        let cols = column_infos(&checked).unwrap();
        let col = &cols[1];
        assert_eq!(col.type_text.as_deref(), Some("decimal(10,2)"));
        assert_eq!(col.comment.as_deref(), Some("money"));
        assert_eq!(col.position, Some(1));
        assert_eq!(
            &struct_field(col).unwrap(),
            checked.schema.field_at_index(1).unwrap()
        );
        let served = columns_struct(&cols, "c.s.t");
        assert_eq!(served["fields"][1]["type"], "decimal(10,2)");
        assert_eq!(served["fields"][1]["metadata"]["comment"], "money");
    }
}
