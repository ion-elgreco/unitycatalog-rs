use std::collections::{BTreeMap, HashMap};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use uc_api::{CreateSchema, SchemaInfo, UpdateSchema};
use uc_common::{
    Error, ErrorCode, NormalizedUrl, Pagination, Result, SecurablePropertiesMap, SecurableType,
    now_epoch_millis, schema_full_name, validate_sql_object_name,
};

use crate::protection::sql::check_unprotected;
use crate::sql::rows::SCHEMA;
use crate::sql::{
    SchemaRef, SqlMetastore, create_err_as, db_err, exists_in, insert, json_text, placeholders,
    select_one, split_schema_name, touch_audit, update,
};
use crate::{PropertyChanges, SchemaOps};
use uc_storage::{Exec, Row, Value};

/// Attempts of a property patch against concurrent writers.
const PATCH_ATTEMPTS: usize = 5;

/// A schema row, keyed by its UUID and linked to its catalog by id.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct SchemaRow {
    pub(crate) id_string: String,
    pub(crate) name: String,
    pub(crate) catalog_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) comment: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) properties: Option<SecurablePropertiesMap>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) owner: Option<String>,
    pub(crate) created_at: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) created_by: Option<String>,
    pub(crate) updated_at: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) updated_by: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) storage_root: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) storage_location: Option<String>,
}

impl SchemaRow {
    fn into_info(self, catalog_name: &str) -> SchemaInfo {
        SchemaInfo {
            full_name: Some(schema_full_name(catalog_name, &self.name)),
            name: Some(self.name),
            catalog_name: Some(catalog_name.to_string()),
            comment: self.comment,
            properties: self.properties,
            owner: self.owner,
            created_at: Some(self.created_at),
            created_by: self.created_by,
            updated_at: Some(self.updated_at),
            updated_by: self.updated_by,
            schema_id: Some(self.id_string),
            storage_root: self.storage_root,
            storage_location: self.storage_location,
        }
    }
}

/// Children that keep a schema from being deleted without `force`, with
/// upstream's wording, in upstream's check order.
const SCHEMA_CHILDREN: [(&str, &str); 4] = [
    ("tbl", "tables"),
    ("volume", "volumes"),
    ("func", "functions"),
    ("registered_model", "models"),
];

/// Delete everything inside the schemas that `in_schemas` picks by
/// `schema_id` (binding `params`): models with their versions, functions,
/// volumes and tables, and their grants. As upstream, each child is dropped
/// the way a single delete drops it, so managed data gets cleanup tasks.
/// Run it in the transaction that deletes the schemas.
pub(crate) async fn drop_schema_children<E: Exec + ?Sized>(
    tx: &E,
    in_schemas: &str,
    params: &[Value],
) -> Result<()> {
    crate::table::sql::drop_managed_tables(tx, in_schemas, params).await?;
    crate::volume::sql::queue_managed_volumes(tx, in_schemas, params).await?;
    crate::model::sql::queue_models(tx, in_schemas, params).await?;
    let children = |table: &str| format!("SELECT id_string FROM {table} WHERE {in_schemas}");
    let failed = db_err("delete schema contents");
    for sql in [
        format!(
            "DELETE FROM permission WHERE resource_id IN ({}) OR resource_id IN ({}) \
             OR resource_id IN ({}) OR resource_id IN ({})",
            children("tbl"),
            children("volume"),
            children("func"),
            children("registered_model"),
        ),
        format!(
            "DELETE FROM model_version WHERE registered_model_id IN ({})",
            children("registered_model")
        ),
        format!("DELETE FROM registered_model WHERE {in_schemas}"),
        format!("DELETE FROM func WHERE {in_schemas}"),
        format!("DELETE FROM volume WHERE {in_schemas}"),
        format!("DELETE FROM tbl WHERE {in_schemas}"),
    ] {
        tx.execute(&sql, params).await.map_err(&failed)?;
    }
    Ok(())
}

/// The catalog `$1` and its schema `$2`, for [`SCHEMA_REF_COLUMNS`]; the
/// schema's columns are null when it is missing.
const CATALOG_AND_SCHEMA: &str =
    "catalog c LEFT JOIN schema s ON s.catalog_id = c.id_string AND s.name = $2";

/// The columns of a [`SchemaRef`], from [`CATALOG_AND_SCHEMA`].
const SCHEMA_REF_COLUMNS: &str =
    "c.name, c.storage_location, s.id_string, s.name, s.storage_location";

/// The [`SchemaRef`] of a row that starts with [`SCHEMA_REF_COLUMNS`], or
/// upstream's `SCHEMA_NOT_FOUND` when its schema is missing.
fn schema_ref_of(row: &Row, catalog: &str, schema: &str) -> Result<SchemaRef> {
    let failed = db_err("resolve schema");
    let Some(schema_id) = row.opt_text(2).map_err(&failed)? else {
        return Err(Error::schema_not_found(format!(
            "Schema not found: {catalog}.{schema}"
        )));
    };
    Ok(SchemaRef {
        catalog_name: row.text(0).map_err(&failed)?,
        catalog_storage_location: row.opt_text(1).map_err(&failed)?,
        schema_id,
        schema_name: row.text(3).map_err(&failed)?,
        schema_storage_location: row.opt_text(4).map_err(&failed)?,
    })
}

fn schema_exists(name: &str) -> Error {
    Error::new(
        ErrorCode::SchemaAlreadyExists,
        format!("Schema already exists: {name}"),
    )
}

impl SqlMetastore {
    /// The schema `name` in catalog `catalog_id`, if any.
    pub(crate) async fn schema_row(
        &self,
        catalog_id: &str,
        name: &str,
    ) -> Result<Option<SchemaRow>> {
        select_one(
            self.db.as_ref(),
            &SCHEMA,
            "WHERE catalog_id = $1 AND name = $2",
            &[catalog_id.into(), name.into()],
        )
        .await
    }

    /// Resolve `<catalog>.<schema>` to ids in one query, or fail with
    /// upstream's `CATALOG_NOT_FOUND` / `SCHEMA_NOT_FOUND`.
    pub(crate) async fn resolve_schema(&self, catalog: &str, schema: &str) -> Result<SchemaRef> {
        let sql =
            format!("SELECT {SCHEMA_REF_COLUMNS} FROM {CATALOG_AND_SCHEMA} WHERE c.name = $1");
        let rows = self
            .db
            .query(&sql, &[catalog.into(), schema.into()])
            .await
            .map_err(db_err("resolve schema"))?;
        let row = rows
            .first()
            .ok_or_else(|| Error::catalog_not_found(format!("Catalog not found: {catalog}")))?;
        schema_ref_of(row, catalog, schema)
    }

    /// Resolve a schema id (from a child row) to its current names.
    pub(crate) async fn schema_ref_by_id(&self, schema_id: &str) -> Result<SchemaRef> {
        self.schema_refs_by_ids(&[schema_id.to_string()])
            .await?
            .remove(schema_id)
            .ok_or_else(|| Error::schema_not_found(format!("Schema not found: {schema_id}")))
    }

    /// Batch form of [`Self::schema_ref_by_id`]: two queries for any number
    /// of ids. Ids without a schema (or catalog) are left out.
    pub(crate) async fn schema_refs_by_ids(
        &self,
        ids: &[String],
    ) -> Result<HashMap<String, SchemaRef>> {
        if ids.is_empty() {
            return Ok(HashMap::new());
        }
        let failed = db_err("schemas by id");
        let ids: Vec<Value> = ids.iter().map(Into::into).collect();
        let schemas = self
            .db
            .query(
                &format!(
                    "SELECT id_string, name, catalog_id, storage_location FROM schema \
                     WHERE id_string IN ({})",
                    placeholders(1, ids.len())
                ),
                &ids,
            )
            .await
            .map_err(&failed)?;
        let catalog_ids: Vec<Value> = schemas
            .iter()
            .map(|r| r.text(2).map(Value::Text))
            .collect::<Result<_, _>>()
            .map_err(&failed)?;
        if catalog_ids.is_empty() {
            return Ok(HashMap::new());
        }
        let catalogs = self
            .db
            .query(
                &format!(
                    "SELECT id_string, name, storage_location FROM catalog \
                     WHERE id_string IN ({})",
                    placeholders(1, catalog_ids.len())
                ),
                &catalog_ids,
            )
            .await
            .map_err(&failed)?;
        let mut by_id = HashMap::new();
        for c in &catalogs {
            let id = c.text(0).map_err(&failed)?;
            let name = c.text(1).map_err(&failed)?;
            by_id.insert(id, (name, c.opt_text(2).map_err(&failed)?));
        }
        let mut refs = HashMap::new();
        for s in &schemas {
            let catalog_id = s.text(2).map_err(&failed)?;
            let Some((catalog_name, catalog_location)) = by_id.get(&catalog_id) else {
                continue;
            };
            let schema_id = s.text(0).map_err(&failed)?;
            refs.insert(
                schema_id.clone(),
                SchemaRef {
                    catalog_name: catalog_name.clone(),
                    schema_id,
                    schema_name: s.text(1).map_err(&failed)?,
                    schema_storage_location: s.opt_text(3).map_err(&failed)?,
                    catalog_storage_location: catalog_location.clone(),
                },
            );
        }
        Ok(refs)
    }

    /// The schema row for `<catalog>.<schema>` with upstream errors.
    pub(crate) async fn require_schema_row(&self, full_name: &str) -> Result<(String, SchemaRow)> {
        let (catalog_name, schema_name) = split_schema_name(full_name)?;
        let cat = self.require_catalog_row(catalog_name).await?;
        let row = self
            .schema_row(&cat.id_string, schema_name)
            .await?
            .ok_or_else(|| Error::schema_not_found(format!("Schema not found: {full_name}")))?;
        Ok((cat.name, row))
    }
}

#[async_trait]
impl SchemaOps for SqlMetastore {
    async fn create(&self, req: CreateSchema, created_by: Option<&str>) -> Result<SchemaInfo> {
        validate_sql_object_name(&req.name)?;
        let cat = self.require_catalog_row(&req.catalog_name).await?;
        if self.schema_row(&cat.id_string, &req.name).await?.is_some() {
            return Err(schema_exists(&req.name));
        }
        let id = Uuid::new_v4().to_string();
        // A storage root gets the schema's managed location below it.
        let (storage_root, storage_location) =
            match NormalizedUrl::parse_opt(req.storage_root.as_deref())? {
                Some(root) => {
                    root.validate_not_under_managed_prefix()?;
                    let location = root.managed_schema_location(&id)?;
                    (Some(root.into_string()), Some(location.into_string()))
                }
                None => (None, None),
            };

        let now = now_epoch_millis();
        let row = SchemaRow {
            id_string: id,
            name: req.name.clone(),
            catalog_id: cat.id_string.clone(),
            comment: req.comment,
            properties: req.properties,
            owner: created_by.map(str::to_string),
            created_at: now,
            created_by: created_by.map(str::to_string),
            updated_at: now,
            updated_by: created_by.map(str::to_string),
            storage_root,
            storage_location,
        };
        insert(self.db.as_ref(), &SCHEMA, &row)
            .await
            .map_err(create_err_as(
                ErrorCode::SchemaAlreadyExists,
                format!("Schema already exists: {}", req.name),
            ))?;
        Ok(row.into_info(&cat.name))
    }

    async fn get(&self, full_name: &str) -> Result<SchemaInfo> {
        let (catalog_name, row) = self.require_schema_row(full_name).await?;
        Ok(row.into_info(&catalog_name))
    }

    async fn update(
        &self,
        full_name: &str,
        req: UpdateSchema,
        updated_by: Option<&str>,
    ) -> Result<SchemaInfo> {
        if let Some(new_name) = &req.new_name {
            validate_sql_object_name(new_name)?;
        }
        let (catalog_name, mut row) = self.require_schema_row(full_name).await?;

        // Upstream checks the new name first: any existing name fails, the
        // current one included. Children link by id, so a rename is a field
        // update.
        if let Some(new_name) = req.new_name.as_ref()
            && self.schema_row(&row.catalog_id, new_name).await?.is_some()
        {
            return Err(schema_exists(new_name));
        }
        // Nothing to change returns the schema untouched; an empty
        // properties map counts as nothing.
        let properties = req.properties.filter(|p| !p.is_empty());
        if req.new_name.is_none() && req.comment.is_none() && properties.is_none() {
            return Ok(row.into_info(&catalog_name));
        }
        if let Some(new_name) = req.new_name {
            row.name = new_name;
        }
        if let Some(c) = req.comment {
            row.comment = Some(c);
        }
        // A non-empty map replaces all properties.
        if let Some(p) = properties {
            row.properties = Some(p);
        }
        touch_audit(&mut row.updated_at, &mut row.updated_by, updated_by);

        let written = update(self.db.as_ref(), &SCHEMA, &row)
            .await
            .map_err(create_err_as(
                ErrorCode::SchemaAlreadyExists,
                format!("Schema already exists: {}", row.name),
            ))?;
        if written == 0 {
            return Err(Error::schema_not_found(format!(
                "Schema not found: {full_name}"
            )));
        }
        Ok(row.into_info(&catalog_name))
    }

    async fn delete(&self, full_name: &str, force: bool) -> Result<()> {
        let (_, schema) = self.require_schema_row(full_name).await?;
        let failed = db_err("delete schema");
        let tx = self.db.begin().await.map_err(&failed)?;
        check_unprotected(
            tx.as_ref(),
            SecurableType::Schema,
            &schema.id_string,
            full_name,
            force,
        )
        .await?;
        // Upstream checks the child kinds in this order; without `force`
        // the first kind present stops the delete.
        if !force {
            for (table, kind) in SCHEMA_CHILDREN {
                if exists_in(tx.as_ref(), table, "schema_id", &schema.id_string).await? {
                    return Err(Error::failed_precondition(format!(
                        "Cannot delete schema with {kind}"
                    )));
                }
            }
        }
        let id = [schema.id_string.as_str().into()];
        drop_schema_children(tx.as_ref(), "schema_id = $1", &id).await?;
        for sql in [
            "DELETE FROM permission WHERE resource_id = $1",
            "DELETE FROM schema WHERE id_string = $1",
        ] {
            tx.execute(sql, &id).await.map_err(&failed)?;
        }
        tx.commit().await.map_err(&failed)
    }

    async fn apply_property_changes(
        &self,
        full_name: &str,
        updates: BTreeMap<String, String>,
        removals: Vec<String>,
        updated_by: Option<&str>,
    ) -> Result<PropertyChanges> {
        // Read, patch, and write back only if no one wrote in between: every
        // write moves `updated_at` forward, so it is the row's version.
        for _ in 0..PATCH_ATTEMPTS {
            let (_, row) = self.require_schema_row(full_name).await?;
            let mut properties = row.properties.clone().unwrap_or_default();
            let mut changes = PropertyChanges::default();
            for key in &removals {
                if properties.remove(key).is_some() {
                    changes.removed.push(key.clone());
                } else {
                    changes.missing.push(key.clone());
                }
            }
            changes.updated = updates.keys().cloned().collect();
            properties.extend(updates.clone());
            // An emptied map is no properties at all, as on a new schema.
            let properties = match properties.is_empty() {
                true => Value::Null,
                false => json_text(&properties)?.into(),
            };
            let written = self
                .db
                .execute(
                    "UPDATE schema SET properties = $1, updated_by = $2, updated_at = $3 \
                     WHERE id_string = $4 AND updated_at = $5",
                    &[
                        properties,
                        updated_by.into(),
                        now_epoch_millis().max(row.updated_at + 1).into(),
                        row.id_string.as_str().into(),
                        row.updated_at.into(),
                    ],
                )
                .await
                .map_err(db_err("schema properties"))?;
            if written == 1 {
                return Ok(changes);
            }
        }
        Err(Error::new(
            ErrorCode::Aborted,
            format!("Schema {full_name} changed concurrently; try again."),
        ))
    }

    async fn list(
        &self,
        catalog_name: &str,
        page: &Pagination,
    ) -> Result<(Vec<SchemaInfo>, Option<String>)> {
        let cat = self.require_catalog_row(catalog_name).await?;
        let (rows, next) = self
            .paged_list::<SchemaRow, _>(
                &SCHEMA,
                &["catalog_id = $1"],
                vec![cat.id_string.as_str().into()],
                "name",
                page,
                |r| r.name.clone(),
            )
            .await?;
        Ok((
            rows.into_iter().map(|r| r.into_info(&cat.name)).collect(),
            next,
        ))
    }
}
