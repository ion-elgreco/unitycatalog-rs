use std::collections::{BTreeSet, HashMap};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use uc_api::{PermissionsList, Privilege, PrivilegeAssignment, UpdatePermissions};
use uc_common::{Error, ErrorCode, NormalizedUrl, Result, SecurableType};

use crate::sql::rows::{METASTORE, PERMISSION, STAGING_TABLE};
use crate::sql::{
    SqlMetastore, by_id, char_len, create_err, db_err, insert, placeholders, select, select_one,
    starts_with, update,
};
use crate::{
    PermissionOps,
    permissions::{Lineage, Node, PathOwner},
};
use uc_storage::{Exec, Value};

/// The grants of one principal on one securable, which is named by id.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct PermissionRow {
    id_string: String,
    securable_type: String,
    resource_id: String,
    principal: String,
    /// The spec-wire strings (e.g. `"CREATE CATALOG"`).
    privileges: Vec<Privilege>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct MetastoreRow {
    pub(crate) metastore_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    owner: Option<String>,
}

fn assignments(rows: Vec<PermissionRow>) -> PermissionsList {
    PermissionsList {
        privilege_assignments: rows
            .into_iter()
            .map(|r| PrivilegeAssignment {
                principal: r.principal,
                privileges: r.privileges,
            })
            .collect(),
    }
}

/// `url` and each of its `/`-bounded prefixes: the locations that contain
/// it.
fn containing(url: &str) -> Vec<Value> {
    let mut out = vec![Value::from(url)];
    for (i, c) in url.char_indices() {
        if c == '/' && i > 0 {
            out.push(url[..i].into());
        }
    }
    out
}

#[async_trait]
impl PermissionOps for SqlMetastore {
    async fn get(
        &self,
        securable_type: SecurableType,
        full_name: &str,
        principal_filter: Option<&str>,
    ) -> Result<PermissionsList> {
        let resource_id = self.securable_id(securable_type, full_name).await?;
        let rows = match principal_filter {
            Some(p) => {
                select(
                    self.db.as_ref(),
                    &PERMISSION,
                    "WHERE resource_id = $1 AND principal = $2",
                    &[resource_id.into(), p.into()],
                )
                .await?
            }
            None => {
                select(
                    self.db.as_ref(),
                    &PERMISSION,
                    "WHERE resource_id = $1 ORDER BY principal",
                    &[resource_id.into()],
                )
                .await?
            }
        };
        Ok(assignments(rows))
    }

    async fn update(
        &self,
        securable_type: SecurableType,
        full_name: &str,
        req: UpdatePermissions,
    ) -> Result<PermissionsList> {
        // Upstream `updateAuthorization`: the securable and every principal
        // must exist before anything changes.
        let resource_id = self.securable_id(securable_type, full_name).await?;
        let mut changes = Vec::with_capacity(req.changes.len());
        for mut change in req.changes {
            change.principal = self.require_user(&change.principal).await?;
            changes.push(change);
        }
        let principals: BTreeSet<String> = changes.iter().map(|c| c.principal.clone()).collect();
        let failed = db_err("update permissions");
        let tx = self.db.begin().await.map_err(&failed)?;
        for change in changes {
            apply_permission_change(tx.as_ref(), securable_type, &resource_id, change).await?;
        }
        tx.commit().await.map_err(&failed)?;
        // The answer lists only this request's principals that still hold
        // privileges.
        if principals.is_empty() {
            return Ok(assignments(Vec::new()));
        }
        let mut params: Vec<Value> = vec![resource_id.into()];
        params.extend(principals.iter().map(Into::into));
        let rows = select(
            self.db.as_ref(),
            &PERMISSION,
            &format!(
                "WHERE resource_id = $1 AND principal IN ({}) ORDER BY principal",
                placeholders(2, principals.len())
            ),
            &params,
        )
        .await?;
        Ok(assignments(rows))
    }

    async fn metastore_id(&self) -> Result<String> {
        Ok(self.metastore_row().await?.metastore_id)
    }

    async fn lineage(&self, kind: SecurableType, name: &str) -> Result<Lineage> {
        let node = |kind, id: &str, owner: &Option<String>| {
            Some(Node {
                kind,
                id: id.to_string(),
                owner: owner.clone(),
            })
        };
        Ok(match kind {
            SecurableType::Metastore => Lineage::default(),
            SecurableType::Catalog => {
                let row = self.require_catalog_row(name).await?;
                let catalog = node(kind, &row.id_string, &row.owner);
                Lineage {
                    catalog: catalog.clone(),
                    schema: None,
                    object: catalog,
                }
            }
            SecurableType::Schema => {
                let (catalog_name, row) = self.require_schema_row(name).await?;
                let catalog = self.require_catalog_row(&catalog_name).await?;
                let schema = node(kind, &row.id_string, &row.owner);
                Lineage {
                    catalog: node(SecurableType::Catalog, &catalog.id_string, &catalog.owner),
                    schema: schema.clone(),
                    object: schema,
                }
            }
            SecurableType::Table
            | SecurableType::Volume
            | SecurableType::Function
            | SecurableType::RegisteredModel => {
                let (schema_ref, id, owner) = match kind {
                    SecurableType::Table => {
                        let (s, r) = self.resolve_table(name).await?;
                        (s, r.id_string, r.owner)
                    }
                    SecurableType::Volume => {
                        let (s, r) = self.resolve_volume(name).await?;
                        (s, r.id_string, r.owner)
                    }
                    SecurableType::Function => {
                        let (s, r) = self.resolve_function(name).await?;
                        (s, r.id_string, r.owner)
                    }
                    _ => {
                        let (s, r) = self.resolve_model(name).await?;
                        (s, r.id_string, r.owner)
                    }
                };
                let schema: Option<crate::schema::sql::SchemaRow> = by_id(
                    self.db.as_ref(),
                    &crate::sql::rows::SCHEMA,
                    &schema_ref.schema_id,
                )
                .await?;
                let catalog = self.require_catalog_row(&schema_ref.catalog_name).await?;
                Lineage {
                    catalog: node(SecurableType::Catalog, &catalog.id_string, &catalog.owner),
                    schema: node(
                        SecurableType::Schema,
                        &schema_ref.schema_id,
                        &schema.and_then(|s| s.owner),
                    ),
                    object: node(kind, &id, &owner),
                }
            }
            SecurableType::ExternalLocation => {
                let row = self.location_row(name).await?.ok_or_else(|| {
                    Error::not_found(format!("External location not found: {name}"))
                })?;
                Lineage {
                    object: node(kind, &row.id_string, &row.owner),
                    ..Default::default()
                }
            }
            SecurableType::Credential => {
                let row = self.credential_row(name).await?.ok_or_else(|| {
                    Error::not_found(format!("Storage credential not found: {name}"))
                })?;
                Lineage {
                    object: node(kind, &row.id_string, &row.owner),
                    ..Default::default()
                }
            }
        })
    }

    async fn lineage_by_id(&self, kind: SecurableType, id: &str) -> Result<Lineage> {
        match kind {
            SecurableType::Table => self.table_lineage(id).await,
            SecurableType::Volume => self.child_lineage(kind, "volume", id).await,
            SecurableType::RegisteredModel => {
                self.child_lineage(kind, "registered_model", id).await
            }
            SecurableType::Function => self.child_lineage(kind, "func", id).await,
            other => Err(Error::internal(format!("no lineage by id for {other:?}"))),
        }
    }

    async fn table_lineage(&self, table_id: &str) -> Result<Lineage> {
        let not_found = || Error::table_not_found(format!("Table not found: {table_id}"));
        if self.exists_where("tbl", "id_string", table_id).await? {
            return self
                .child_lineage(SecurableType::Table, "tbl", table_id)
                .await;
        }
        let staging: crate::table::sql::StagingRow =
            by_id(self.db.as_ref(), &STAGING_TABLE, table_id)
                .await?
                .ok_or_else(not_found)?;
        let (schema_id, schema_owner, catalog_id) = self
            .node_row("schema", Some("catalog_id"), &staging.schema_id)
            .await?
            .ok_or_else(not_found)?;
        let (catalog_id, catalog_owner, _) = self
            .node_row("catalog", None, &catalog_id.unwrap_or_default())
            .await?
            .ok_or_else(not_found)?;
        Ok(Lineage {
            catalog: Some(Node {
                kind: SecurableType::Catalog,
                id: catalog_id,
                owner: catalog_owner,
            }),
            schema: Some(Node {
                kind: SecurableType::Schema,
                id: schema_id,
                owner: schema_owner,
            }),
            object: Some(Node {
                kind: SecurableType::Table,
                id: table_id.to_string(),
                owner: staging.created_by,
            }),
        })
    }

    async fn grants_of(&self, principal: &str) -> Result<HashMap<String, Vec<Privilege>>> {
        let rows: Vec<PermissionRow> = select(
            self.db.as_ref(),
            &PERMISSION,
            "WHERE principal = $1",
            &[principal.into()],
        )
        .await?;
        Ok(rows
            .into_iter()
            .map(|r| (r.resource_id, r.privileges))
            .collect())
    }

    async fn clear_principal(&self, principal: &str) -> Result<()> {
        self.db
            .execute(
                "DELETE FROM permission WHERE principal = $1",
                &[principal.into()],
            )
            .await
            .map_err(db_err("clear principal grants"))?;
        Ok(())
    }

    async fn metastore_owner(&self) -> Result<Option<String>> {
        Ok(self.metastore_row().await?.owner)
    }

    async fn path_owner(&self, url: &NormalizedUrl) -> Result<PathOwner> {
        let failed = db_err("path owner");
        let u = url.as_str();
        let under = format!("{u}/");
        // `$1` and `$2`: what is under the path; then what contains it.
        let mut params: Vec<Value> = vec![under.as_str().into(), char_len(&under)];
        let containing = containing(u);
        let inside = |field: &str| format!("{field} IN ({})", placeholders(3, containing.len()));
        params.extend(containing.iter().cloned());

        let cleanup = self
            .db
            .query(
                &format!(
                    "SELECT id_string FROM storage_cleanup_task WHERE {} OR {} LIMIT 1",
                    inside("storage_location"),
                    starts_with("storage_location", 1, 2)
                ),
                &params,
            )
            .await
            .map_err(&failed)?;
        if !cleanup.is_empty() {
            return Err(Error::new(
                ErrorCode::PermissionDenied,
                "Input path overlaps pending storage cleanup.",
            ));
        }
        let entities = [
            ("external_location", "url"),
            ("tbl", "storage_location"),
            ("volume", "storage_location"),
            ("registered_model", "storage_location"),
        ];
        for (table, field) in entities {
            let below = format!(
                "SELECT id_string FROM {table} WHERE {} LIMIT 1",
                starts_with(field, 1, 2)
            );
            if !self
                .db
                .query(&below, &params[..2])
                .await
                .map_err(&failed)?
                .is_empty()
            {
                return Err(Error::new(
                    ErrorCode::PermissionDenied,
                    format!("Input path '{u}' overlaps with other entities."),
                ));
            }
        }
        let multiple = || {
            Error::invalid_argument(format!("Input path '{u}' overlaps with multiple entities."))
        };
        // Only the containing locations, renumbered from `$1`.
        let at = |field: &str| format!("{field} IN ({})", placeholders(1, containing.len()));
        let mut data = Vec::new();
        for (kind, table) in [
            (SecurableType::Table, "tbl"),
            (SecurableType::Volume, "volume"),
            (SecurableType::RegisteredModel, "registered_model"),
        ] {
            let sql = format!(
                "SELECT id_string FROM {table} WHERE {} LIMIT 2",
                at("storage_location")
            );
            for hit in self.db.query(&sql, &containing).await.map_err(&failed)? {
                data.push((kind, table, hit.text(0).map_err(&failed)?));
            }
        }
        match data.as_slice() {
            [] => {}
            [(kind, table, id)] => {
                return Ok(PathOwner::Data(
                    *kind,
                    self.child_lineage(*kind, table, id).await?,
                ));
            }
            _ => return Err(multiple()),
        }
        let sql = format!(
            "SELECT id_string, owner FROM external_location WHERE {} LIMIT 2",
            at("url")
        );
        let locations = self.db.query(&sql, &containing).await.map_err(&failed)?;
        match locations.as_slice() {
            [] => Ok(PathOwner::Unowned),
            [location] => Ok(PathOwner::ExternalLocation(Lineage {
                object: Some(Node {
                    kind: SecurableType::ExternalLocation,
                    id: location.text(0).map_err(&failed)?,
                    owner: location.opt_text(1).map_err(&failed)?,
                }),
                ..Default::default()
            })),
            _ => Err(multiple()),
        }
    }

    async fn set_metastore_owner(&self, owner: &str) -> Result<()> {
        self.metastore_row().await?;
        self.db
            .execute("UPDATE metastore SET owner = $1", &[owner.into()])
            .await
            .map_err(db_err("set metastore owner"))?;
        Ok(())
    }
}

/// Add and remove privileges of one principal on one securable, in `tx`:
/// the number of privileges that it added or removed.
pub(crate) async fn apply_permission_change<E: Exec + ?Sized>(
    tx: &E,
    securable_type: SecurableType,
    resource_id: &str,
    change: uc_api::PermissionsChange,
) -> Result<usize> {
    let key = crate::permissions::permission_key(resource_id, &change.principal);
    let existing: Option<PermissionRow> = select_one(
        tx,
        &PERMISSION,
        &format!("WHERE id_string = $1{}", tx.for_update()),
        &[key.as_str().into()],
    )
    .await?;

    let mut set: BTreeSet<Privilege> = existing
        .as_ref()
        .map(|r| r.privileges.iter().copied().collect())
        .unwrap_or_default();
    let mut changed = 0;
    for p in change.add {
        changed += usize::from(set.insert(p));
    }
    for p in change.remove {
        changed += usize::from(set.remove(&p));
    }
    if changed == 0 {
        return Ok(0);
    }

    if set.is_empty() {
        tx.execute("DELETE FROM permission WHERE id_string = $1", &[key.into()])
            .await
            .map_err(db_err("delete permission"))?;
        return Ok(changed);
    }

    let row = PermissionRow {
        id_string: key,
        securable_type: securable_type.as_str().to_string(),
        resource_id: resource_id.to_string(),
        principal: change.principal,
        privileges: set.into_iter().collect(),
    };
    match existing {
        Some(_) => update(tx, &PERMISSION, &row)
            .await
            .map(drop)
            .map_err(db_err("update permission"))?,
        None => insert(tx, &PERMISSION, &row).await.map_err(create_err)?,
    }
    Ok(changed)
}

impl SqlMetastore {
    /// The id, owner and (with `parent`) the parent id of the row `id` of
    /// `table`.
    async fn node_row(
        &self,
        table: &str,
        parent: Option<&str>,
        id: &str,
    ) -> Result<Option<(String, Option<String>, Option<String>)>> {
        let failed = db_err("lineage");
        let sql = format!(
            "SELECT id_string, owner, {} FROM {table} WHERE id_string = $1",
            parent.unwrap_or("NULL")
        );
        let rows = self.db.query(&sql, &[id.into()]).await.map_err(&failed)?;
        rows.first()
            .map(|r| Ok((r.text(0)?, r.opt_text(1)?, r.opt_text(2)?)))
            .transpose()
            .map_err(&failed)
    }

    /// The lineage of the schema child `id` of `kind`, stored in `table`.
    async fn child_lineage(&self, kind: SecurableType, table: &str, id: &str) -> Result<Lineage> {
        let missing = |table: &str, id: &str| Error::not_found(format!("{table} not found: {id}"));
        let (object_id, object_owner, schema_id) = self
            .node_row(table, Some("schema_id"), id)
            .await?
            .ok_or_else(|| missing(table, id))?;
        let schema_id = schema_id.unwrap_or_default();
        let (schema_id, schema_owner, catalog_id) = self
            .node_row("schema", Some("catalog_id"), &schema_id)
            .await?
            .ok_or_else(|| missing("schema", &schema_id))?;
        let catalog_id = catalog_id.unwrap_or_default();
        let (catalog_id, catalog_owner, _) = self
            .node_row("catalog", None, &catalog_id)
            .await?
            .ok_or_else(|| missing("catalog", &catalog_id))?;
        let node = |kind, id, owner| Some(Node { kind, id, owner });
        Ok(Lineage {
            catalog: node(SecurableType::Catalog, catalog_id, catalog_owner),
            schema: node(SecurableType::Schema, schema_id, schema_owner),
            object: node(kind, object_id, object_owner),
        })
    }

    /// The id of the securable named `name`, or the error upstream gives
    /// when it does not exist. The metastore has one id, whatever the name.
    pub(crate) async fn securable_id(&self, kind: SecurableType, name: &str) -> Result<String> {
        Ok(match kind {
            SecurableType::Metastore => self.metastore_row().await?.metastore_id,
            SecurableType::Catalog => self.require_catalog_row(name).await?.id_string,
            SecurableType::Schema => self.require_schema_row(name).await?.1.id_string,
            SecurableType::Table => self.resolve_table(name).await?.1.id_string,
            SecurableType::Volume => self.resolve_volume(name).await?.1.id_string,
            SecurableType::Function => self.resolve_function(name).await?.1.id_string,
            SecurableType::RegisteredModel => self.resolve_model(name).await?.1.id_string,
            SecurableType::ExternalLocation => {
                self.location_row(name)
                    .await?
                    .ok_or_else(|| {
                        Error::not_found(format!("External location not found: {name}"))
                    })?
                    .id_string
            }
            SecurableType::Credential => {
                self.credential_row(name)
                    .await?
                    .ok_or_else(|| {
                        Error::not_found(format!("Storage credential not found: {name}"))
                    })?
                    .id_string
            }
        })
    }

    /// Upstream `getUserByEmail`: grants go to users that exist, named by
    /// user name or email. They are kept under the user's principal (its
    /// email), the name a signed-in caller has.
    async fn require_user(&self, principal: &str) -> Result<String> {
        use crate::scim::ScimOps;
        match self.find_user_by_email(principal).await? {
            Some(user) => Ok(user.principal().to_string()),
            None => Err(Error::not_found(format!("User not found: {principal}"))),
        }
    }

    /// The metastore row, made on first use. Replicas that make it at once
    /// keep the first.
    pub(crate) async fn metastore_row(&self) -> Result<MetastoreRow> {
        if let Some(row) = select_one(self.db.as_ref(), &METASTORE, "LIMIT 1", &[]).await? {
            return Ok(row);
        }
        self.db
            .execute(
                "INSERT INTO metastore (metastore_id) VALUES ($1) ON CONFLICT DO NOTHING",
                &[Uuid::new_v4().to_string().into()],
            )
            .await
            .map_err(db_err("create metastore"))?;
        select_one(self.db.as_ref(), &METASTORE, "LIMIT 1", &[])
            .await?
            .ok_or_else(|| Error::internal("no metastore row"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn containing_locations() {
        assert_eq!(
            containing("s3://b/a/c"),
            ["s3://b/a/c", "s3:", "s3:/", "s3://b", "s3://b/a"].map(Value::from)
        );
    }
}
