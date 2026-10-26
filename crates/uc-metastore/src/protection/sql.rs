//! Protections, and the check that deletes run before they remove anything.

use async_trait::async_trait;

use uc_api::Protection;
use uc_common::{
    Error, Result, SecurableType, now_epoch_millis, schema_full_name, three_part_full_name,
};
use uc_storage::Exec;

use crate::protection::{ProtectionOps, label, protectable};
use crate::sql::{SqlMetastore, db_err};

/// The most protected securables that a refused delete names.
const MAX_NAMED: usize = 20;

/// The securables in a schema that can be protected, and their tables.
const CHILDREN: [(SecurableType, &str); 3] = [
    (SecurableType::Table, "tbl"),
    (SecurableType::Volume, "volume"),
    (SecurableType::RegisteredModel, "registered_model"),
];

impl SqlMetastore {
    /// The table and id of the securable `full_name` of `kind`.
    async fn protectable_id(
        &self,
        kind: SecurableType,
        full_name: &str,
    ) -> Result<(&'static str, String)> {
        let table = protectable(kind)?;
        let id = match kind {
            SecurableType::Catalog => self.require_catalog_row(full_name).await?.id_string,
            SecurableType::Schema => self.require_schema_row(full_name).await?.1.id_string,
            SecurableType::Table => self.resolve_table(full_name).await?.1.id_string,
            SecurableType::Volume => self.resolve_volume(full_name).await?.1.id_string,
            _ => self.resolve_model(full_name).await?.1.id_string,
        };
        Ok((table, id))
    }

    async fn protection_of(&self, id: &str) -> Result<Protection> {
        let failed = db_err("get protection");
        let rows = self
            .db
            .query(
                "SELECT created_at, created_by FROM protection WHERE securable_id = $1",
                &[id.into()],
            )
            .await
            .map_err(&failed)?;
        let Some(row) = rows.first() else {
            return Ok(Protection::default());
        };
        Ok(Protection {
            protected: true,
            protected_at: Some(row.int(0).map_err(&failed)?),
            protected_by: row.opt_text(1).map_err(&failed)?,
        })
    }
}

#[async_trait]
impl ProtectionOps for SqlMetastore {
    async fn get(&self, kind: SecurableType, full_name: &str) -> Result<Protection> {
        let (_, id) = self.protectable_id(kind, full_name).await?;
        self.protection_of(&id).await
    }

    async fn set(
        &self,
        kind: SecurableType,
        full_name: &str,
        protected: bool,
        by: Option<&str>,
    ) -> Result<Protection> {
        let (table, id) = self.protectable_id(kind, full_name).await?;
        let failed = db_err("set protection");
        let tx = self.db.begin().await.map_err(&failed)?;
        if protected {
            // The row lock of the deletes: a delete that holds it ends
            // first, and one that comes later sees the protection.
            let lock = format!(
                "SELECT id_string FROM {table} WHERE id_string = $1{}",
                tx.for_update()
            );
            let rows = tx
                .query(&lock, &[id.as_str().into()])
                .await
                .map_err(&failed)?;
            if rows.is_empty() {
                return Err(Error::not_found(format!(
                    "{} was deleted: {full_name}",
                    capitalized(&label(kind))
                )));
            }
            tx.execute(
                "INSERT INTO protection (securable_id, securable_type, created_at, created_by) \
                 VALUES ($1, $2, $3, $4) ON CONFLICT DO NOTHING",
                &[
                    id.as_str().into(),
                    kind.as_str().into(),
                    now_epoch_millis().into(),
                    by.into(),
                ],
            )
            .await
            .map_err(&failed)?;
        } else {
            tx.execute(
                "DELETE FROM protection WHERE securable_id = $1",
                &[id.as_str().into()],
            )
            .await
            .map_err(&failed)?;
        }
        tx.commit().await.map_err(&failed)?;
        self.protection_of(&id).await
    }
}

/// Refuse the delete of the securable `id` of `kind` when it is protected,
/// or, with `contents`, when a securable in the catalog or schema is. Runs
/// in the delete's transaction: it first locks the rows that the delete
/// removes, so that no protection is set on them until the delete ends.
/// `full_name` names the deleted securable in the error.
pub(crate) async fn check_unprotected<E: Exec + ?Sized>(
    tx: &E,
    kind: SecurableType,
    id: &str,
    full_name: &str,
    contents: bool,
) -> Result<()> {
    let failed = db_err("check protection");
    let table = protectable(kind)?;
    lock(tx, table, "id_string = $1", id).await?;
    let mut protected = Vec::new();
    let own = tx
        .query(
            "SELECT 1 FROM protection WHERE securable_id = $1",
            &[id.into()],
        )
        .await
        .map_err(&failed)?;
    if !own.is_empty() {
        protected.push((kind, full_name.to_string()));
    }
    match (kind, contents) {
        (SecurableType::Schema, true) => {
            for (_, child) in CHILDREN {
                lock(tx, child, "schema_id = $1", id).await?;
            }
            protected.extend(protected_in(tx, "s.id_string = $1", id, false).await?);
        }
        (SecurableType::Catalog, true) => {
            lock(tx, "schema", "catalog_id = $1", id).await?;
            let in_catalog = "schema_id IN (SELECT id_string FROM schema WHERE catalog_id = $1)";
            for (_, child) in CHILDREN {
                lock(tx, child, in_catalog, id).await?;
            }
            protected.extend(protected_in(tx, "s.catalog_id = $1", id, true).await?);
        }
        _ => {}
    }
    match protected.is_empty() {
        true => Ok(()),
        false => Err(refused(kind, full_name, protected)),
    }
}

/// Lock the rows of `table` that match `filter`, with `id` as `$1`. Turso
/// needs no locks: its transactions run one at a time.
async fn lock<E: Exec + ?Sized>(tx: &E, table: &str, filter: &str, id: &str) -> Result<()> {
    let suffix = tx.for_update();
    if suffix.is_empty() {
        return Ok(());
    }
    let sql = format!("SELECT id_string FROM {table} WHERE {filter}{suffix}");
    tx.query(&sql, &[id.into()])
        .await
        .map_err(db_err("lock for delete"))?;
    Ok(())
}

/// The protected securables in the schemas that match `filter` (on schema
/// `s`, with `id` as `$1`), with their full names; the schemas themselves
/// too with `schemas`.
async fn protected_in<E: Exec + ?Sized>(
    tx: &E,
    filter: &str,
    id: &str,
    schemas: bool,
) -> Result<Vec<(SecurableType, String)>> {
    let failed = db_err("check protection");
    let mut found = Vec::new();
    if schemas {
        let sql = format!(
            "SELECT c.name, s.name FROM protection p \
             JOIN schema s ON s.id_string = p.securable_id \
             JOIN catalog c ON c.id_string = s.catalog_id WHERE {filter}"
        );
        for row in tx.query(&sql, &[id.into()]).await.map_err(&failed)? {
            let catalog = row.text(0).map_err(&failed)?;
            let schema = row.text(1).map_err(&failed)?;
            found.push((SecurableType::Schema, schema_full_name(&catalog, &schema)));
        }
    }
    for (kind, table) in CHILDREN {
        let sql = format!(
            "SELECT c.name, s.name, x.name FROM protection p \
             JOIN {table} x ON x.id_string = p.securable_id \
             JOIN schema s ON s.id_string = x.schema_id \
             JOIN catalog c ON c.id_string = s.catalog_id WHERE {filter}"
        );
        for row in tx.query(&sql, &[id.into()]).await.map_err(&failed)? {
            let catalog = row.text(0).map_err(&failed)?;
            let schema = row.text(1).map_err(&failed)?;
            let name = row.text(2).map_err(&failed)?;
            found.push((kind, three_part_full_name(&catalog, &schema, &name)));
        }
    }
    Ok(found)
}

/// The error of a refused delete of `full_name` of `kind`, naming the
/// protected securables.
fn refused(
    kind: SecurableType,
    full_name: &str,
    mut protected: Vec<(SecurableType, String)>,
) -> Error {
    if let [(only, name)] = protected.as_slice()
        && *only == kind
        && name == full_name
    {
        return Error::resource_protected(format!(
            "Cannot delete {} {full_name}: it is protected. Remove its protection first.",
            label(kind)
        ));
    }
    protected.sort_by(|a, b| a.1.cmp(&b.1));
    let count = protected.len();
    let mut named: Vec<String> = protected
        .iter()
        .take(MAX_NAMED)
        .map(|(kind, name)| format!("{} {name}", label(*kind)))
        .collect();
    if count > MAX_NAMED {
        named.push(format!("and {} more", count - MAX_NAMED));
    }
    Error::resource_protected(format!(
        "Cannot delete {} {full_name}: these securables are protected: {}. Remove their \
         protection first.",
        label(kind),
        named.join(", ")
    ))
}

fn capitalized(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_refused_delete_names_the_protected_securables() {
        let one = refused(
            SecurableType::Table,
            "a.b.t",
            vec![(SecurableType::Table, "a.b.t".into())],
        );
        assert_eq!(
            one.message(),
            "Cannot delete table a.b.t: it is protected. Remove its protection first."
        );
        let some = refused(
            SecurableType::Catalog,
            "a",
            vec![
                (SecurableType::Table, "a.b.t".into()),
                (SecurableType::RegisteredModel, "a.b.m".into()),
                (SecurableType::Catalog, "a".into()),
            ],
        );
        assert_eq!(
            some.message(),
            "Cannot delete catalog a: these securables are protected: catalog a, registered \
             model a.b.m, table a.b.t. Remove their protection first."
        );
        let many: Vec<_> = (0..25)
            .map(|i| (SecurableType::Table, format!("a.b.t{i:02}")))
            .collect();
        let message = refused(SecurableType::Schema, "a.b", many)
            .message()
            .to_string();
        assert!(message.contains("table a.b.t19, and 5 more."), "{message}");
    }
}
