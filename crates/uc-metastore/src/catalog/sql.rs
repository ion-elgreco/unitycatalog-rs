use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use uc_api::{CatalogInfo, CreateCatalog, UpdateCatalog};
use uc_common::{
    Error, ErrorCode, NormalizedUrl, Pagination, Result, SecurablePropertiesMap, SecurableType,
    now_epoch_millis, validate_sql_object_name,
};

use crate::CatalogOps;
use crate::protection::sql::check_unprotected;
use crate::sql::rows::CATALOG;
use crate::sql::{
    SqlMetastore, create_err_as, db_err, exists_in, insert, select_one, touch_audit, update,
};

/// A catalog row, keyed by its UUID.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct CatalogRow {
    pub(crate) id_string: String,
    pub(crate) name: String,
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

impl From<CatalogRow> for CatalogInfo {
    fn from(r: CatalogRow) -> Self {
        Self {
            name: Some(r.name),
            comment: r.comment,
            properties: r.properties,
            owner: r.owner,
            created_at: Some(r.created_at),
            created_by: r.created_by,
            updated_at: Some(r.updated_at),
            updated_by: r.updated_by,
            id: Some(r.id_string),
            storage_root: r.storage_root,
            storage_location: r.storage_location,
        }
    }
}

fn catalog_exists(name: &str) -> Error {
    Error::new(
        ErrorCode::CatalogAlreadyExists,
        format!("Catalog already exists: {name}"),
    )
}

impl SqlMetastore {
    /// The catalog named `name`, if any.
    pub(crate) async fn catalog_row(&self, name: &str) -> Result<Option<CatalogRow>> {
        select_one(
            self.db.as_ref(),
            &CATALOG,
            "WHERE name = $1",
            &[name.into()],
        )
        .await
    }

    /// The catalog named `name`, or upstream's `CATALOG_NOT_FOUND`.
    pub(crate) async fn require_catalog_row(&self, name: &str) -> Result<CatalogRow> {
        self.catalog_row(name)
            .await?
            .ok_or_else(|| Error::catalog_not_found(format!("Catalog not found: {name}")))
    }
}

#[async_trait]
impl CatalogOps for SqlMetastore {
    async fn create(&self, req: CreateCatalog, created_by: Option<&str>) -> Result<CatalogInfo> {
        validate_sql_object_name(&req.name)?;
        let id = Uuid::new_v4().to_string();
        // A storage root gets the catalog's managed location below it.
        let (storage_root, storage_location) =
            match NormalizedUrl::parse_opt(req.storage_root.as_deref())? {
                Some(root) => {
                    root.validate_not_under_managed_prefix()?;
                    let location = root.managed_catalog_location(&id)?;
                    (Some(root.into_string()), Some(location.into_string()))
                }
                None => (None, None),
            };
        if self.catalog_row(&req.name).await?.is_some() {
            return Err(catalog_exists(&req.name));
        }

        let now = now_epoch_millis();
        let row = CatalogRow {
            id_string: id,
            name: req.name.clone(),
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
        insert(self.db.as_ref(), &CATALOG, &row)
            .await
            .map_err(create_err_as(
                ErrorCode::CatalogAlreadyExists,
                format!("Catalog already exists: {}", req.name),
            ))?;
        Ok(row.into())
    }

    async fn get(&self, name: &str) -> Result<CatalogInfo> {
        self.require_catalog_row(name).await.map(Into::into)
    }

    async fn update(
        &self,
        name: &str,
        req: UpdateCatalog,
        updated_by: Option<&str>,
    ) -> Result<CatalogInfo> {
        if let Some(new_name) = &req.new_name {
            validate_sql_object_name(new_name)?;
        }
        let mut row = self.require_catalog_row(name).await?;

        // Upstream: nothing to change returns the catalog untouched; an
        // empty properties map counts as nothing.
        let properties = req.properties.filter(|p| !p.is_empty());
        if req.new_name.is_none() && req.comment.is_none() && properties.is_none() {
            return Ok(row.into());
        }
        // Children link to the catalog by id, so a rename is a field update.
        // Upstream rejects any new name that exists, the current one included.
        if let Some(new_name) = req.new_name.as_ref() {
            if self.catalog_row(new_name).await?.is_some() {
                return Err(catalog_exists(new_name));
            }
            row.name = new_name.clone();
        }
        if let Some(c) = req.comment {
            row.comment = Some(c);
        }
        // A non-empty map replaces all properties.
        if let Some(p) = properties {
            row.properties = Some(p);
        }
        touch_audit(&mut row.updated_at, &mut row.updated_by, updated_by);

        let written = update(self.db.as_ref(), &CATALOG, &row)
            .await
            .map_err(create_err_as(
                ErrorCode::CatalogAlreadyExists,
                format!("Catalog already exists: {}", row.name),
            ))?;
        if written == 0 {
            return Err(Error::catalog_not_found(format!(
                "Catalog not found: {name}"
            )));
        }
        Ok(row.into())
    }

    async fn delete(&self, name: &str, force: bool) -> Result<()> {
        let catalog = self.require_catalog_row(name).await?;
        // One transaction: everything inside each schema, the schemas, then
        // the catalog. Protected securables refuse the delete first.
        let failed = db_err("delete catalog");
        let tx = self.db.begin().await.map_err(&failed)?;
        check_unprotected(
            tx.as_ref(),
            SecurableType::Catalog,
            &catalog.id_string,
            name,
            force,
        )
        .await?;
        let has_schemas =
            exists_in(tx.as_ref(), "schema", "catalog_id", &catalog.id_string).await?;
        if has_schemas && !force {
            return Err(Error::failed_precondition(
                "Cannot delete catalog with schemas. Use force=true to force deletion.",
            ));
        }
        let id = [catalog.id_string.as_str().into()];
        let schemas = "SELECT id_string FROM schema WHERE catalog_id = $1";
        crate::schema::sql::drop_schema_children(
            tx.as_ref(),
            &format!("schema_id IN ({schemas})"),
            &id,
        )
        .await?;
        for sql in [
            format!("DELETE FROM permission WHERE resource_id IN ({schemas})"),
            "DELETE FROM schema WHERE catalog_id = $1".to_string(),
            "DELETE FROM permission WHERE resource_id = $1".to_string(),
            "DELETE FROM catalog WHERE id_string = $1".to_string(),
        ] {
            tx.execute(&sql, &id).await.map_err(&failed)?;
        }
        tx.commit().await.map_err(&failed)
    }

    async fn list(&self, page: &Pagination) -> Result<(Vec<CatalogInfo>, Option<String>)> {
        let (rows, next) = self
            .paged_list::<CatalogRow, _>(&CATALOG, &[], vec![], "name", page, |r| r.name.clone())
            .await?;
        Ok((rows.into_iter().map(Into::into).collect(), next))
    }
}
