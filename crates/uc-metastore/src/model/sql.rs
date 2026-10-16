use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use uc_api::{
    CreateModelVersion, CreateRegisteredModel, ModelVersionInfo, ModelVersionStatus,
    RegisteredModelInfo, UpdateModelVersion, UpdateRegisteredModel,
};
use uc_common::{
    Error, ErrorCode, ManagedResource, NormalizedUrl, Pagination, Result, now_epoch_millis,
    three_part_full_name, validate_sql_object_name,
};

use crate::ModelOps;
use crate::cleanup::sql::queue_cleanup;
use crate::sql::rows::{MODEL_VERSION, REGISTERED_MODEL};
use crate::sql::{
    SchemaRef, SqlMetastore, create_err_as, create_storage_dir, db_err, enum_str, insert,
    parse_enum, select, select_one, split_three_part, touch_audit, update,
};
use uc_storage::{Exec, Value};

/// A registered model row, keyed by its UUID and linked to its schema by id.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct RegisteredModelRow {
    pub(crate) id_string: String,
    pub(crate) name: String,
    pub(crate) schema_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) comment: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) owner: Option<String>,
    pub(crate) created_at: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) created_by: Option<String>,
    pub(crate) updated_at: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) updated_by: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) storage_location: Option<String>,
    /// Highest version number ever given out; versions are never reused.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) max_version_number: Option<i64>,
}

/// Before dropping the models that `filter` picks (binding `params`), in the
/// same transaction: each model queues storage cleanup. Versions live under
/// the model's directory, so its task covers them.
pub(crate) async fn queue_models<E: Exec + ?Sized>(
    ex: &E,
    filter: &str,
    params: &[Value],
) -> Result<()> {
    queue_cleanup(
        ex,
        &REGISTERED_MODEL,
        ManagedResource::RegisteredModel,
        filter,
        params,
    )
    .await
}

impl RegisteredModelRow {
    fn into_info(self, schema: &SchemaRef) -> RegisteredModelInfo {
        RegisteredModelInfo {
            catalog_name: Some(schema.catalog_name.clone()),
            schema_name: Some(schema.schema_name.clone()),
            full_name: Some(schema.child_full_name(&self.name)),
            name: Some(self.name),
            storage_location: self.storage_location,
            comment: self.comment,
            owner: self.owner,
            created_at: Some(self.created_at),
            created_by: self.created_by,
            updated_at: Some(self.updated_at),
            updated_by: self.updated_by,
            id: Some(self.id_string),
        }
    }
}

/// A model version row, keyed by its UUID and linked to its model by id.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ModelVersionRow {
    pub(crate) id_string: String,
    pub(crate) registered_model_id: String,
    pub(crate) version: i64,
    pub(crate) source: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) run_id: Option<String>,
    /// Serialized as the spec enum repr.
    pub(crate) status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) storage_location: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) comment: Option<String>,
    pub(crate) created_at: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) created_by: Option<String>,
    pub(crate) updated_at: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) updated_by: Option<String>,
}

impl ModelVersionRow {
    fn into_info(self, schema: &SchemaRef, model_name: &str) -> Result<ModelVersionInfo> {
        Ok(ModelVersionInfo {
            model_name: Some(model_name.to_string()),
            catalog_name: Some(schema.catalog_name.clone()),
            schema_name: Some(schema.schema_name.clone()),
            version: Some(self.version),
            source: Some(self.source),
            run_id: self.run_id,
            status: Some(parse_enum::<ModelVersionStatus>(&self.status)?),
            storage_location: self.storage_location,
            comment: self.comment,
            created_at: Some(self.created_at),
            created_by: self.created_by,
            updated_at: Some(self.updated_at),
            updated_by: self.updated_by,
            id: Some(self.id_string),
        })
    }
}

fn model_exists(full_name: &str) -> Error {
    Error::new(
        ErrorCode::AlreadyExists,
        format!("Registered model already exists: {full_name}"),
    )
}

fn version_not_found(full_name: &str, version: i64) -> Error {
    Error::not_found(format!("Model version not found: {full_name}/{version}"))
}

impl SqlMetastore {
    async fn model_row(&self, schema_id: &str, name: &str) -> Result<Option<RegisteredModelRow>> {
        select_one(
            self.db.as_ref(),
            &REGISTERED_MODEL,
            "WHERE schema_id = $1 AND name = $2",
            &[schema_id.into(), name.into()],
        )
        .await
    }

    /// Resolve `<catalog>.<schema>.<model>` with upstream's errors.
    pub(crate) async fn resolve_model(
        &self,
        full_name: &str,
    ) -> Result<(SchemaRef, RegisteredModelRow)> {
        let (c, s, m) = split_three_part(full_name, "registered model")?;
        self.resolve_model_parts(c, s, m).await
    }

    async fn resolve_model_parts(
        &self,
        catalog: &str,
        schema: &str,
        model: &str,
    ) -> Result<(SchemaRef, RegisteredModelRow)> {
        let schema_ref = self.resolve_schema(catalog, schema).await?;
        let row = self
            .model_row(&schema_ref.schema_id, model)
            .await?
            .ok_or_else(|| {
                Error::not_found(format!(
                    "Registered model not found: {}",
                    three_part_full_name(catalog, schema, model)
                ))
            })?;
        Ok((schema_ref, row))
    }

    async fn version_row(&self, model_id: &str, version: i64) -> Result<Option<ModelVersionRow>> {
        select_one(
            self.db.as_ref(),
            &MODEL_VERSION,
            "WHERE registered_model_id = $1 AND version = $2",
            &[model_id.into(), version.into()],
        )
        .await
    }

    async fn resolve_version(
        &self,
        full_name: &str,
        version: i64,
    ) -> Result<(SchemaRef, RegisteredModelRow, ModelVersionRow)> {
        let (schema, model) = self.resolve_model(full_name).await?;
        let row = self
            .version_row(&model.id_string, version)
            .await?
            .ok_or_else(|| version_not_found(full_name, version))?;
        Ok((schema, model, row))
    }

    async fn save_version(&self, row: ModelVersionRow) -> Result<ModelVersionRow> {
        let written = update(self.db.as_ref(), &MODEL_VERSION, &row)
            .await
            .map_err(db_err("update model version"))?;
        if written == 0 {
            return Err(Error::internal("model version update found no row"));
        }
        Ok(row)
    }
}

#[async_trait]
impl ModelOps for SqlMetastore {
    async fn create_model(
        &self,
        req: CreateRegisteredModel,
        created_by: Option<&str>,
    ) -> Result<RegisteredModelInfo> {
        validate_sql_object_name(&req.name)?;
        let schema = self
            .resolve_schema(&req.catalog_name, &req.schema_name)
            .await?;
        let full_name = schema.child_full_name(&req.name);
        if self
            .model_row(&schema.schema_id, &req.name)
            .await?
            .is_some()
        {
            return Err(model_exists(&full_name));
        }

        // Upstream: `<schema or catalog location>/models/<id>`, falling back
        // to the `storage-root.models` server setting.
        let id = Uuid::new_v4().to_string();
        let storage_location = schema
            .managed_parent_location(&self.roots, ManagedResource::RegisteredModel)?
            .managed_child_location(ManagedResource::RegisteredModel, &id)?;
        create_storage_dir(&storage_location)?;

        let now = now_epoch_millis();
        let row = RegisteredModelRow {
            id_string: id,
            name: req.name.clone(),
            schema_id: schema.schema_id.clone(),
            comment: req.comment,
            owner: created_by.map(str::to_string),
            created_at: now,
            created_by: created_by.map(str::to_string),
            updated_at: now,
            updated_by: created_by.map(str::to_string),
            storage_location: Some(storage_location.into_string()),
            max_version_number: Some(0),
        };

        insert(self.db.as_ref(), &REGISTERED_MODEL, &row)
            .await
            .map_err(create_err_as(
                ErrorCode::AlreadyExists,
                format!("Registered model already exists: {full_name}"),
            ))?;
        Ok(row.into_info(&schema))
    }

    async fn get_model(&self, full_name: &str) -> Result<RegisteredModelInfo> {
        let (schema, row) = self.resolve_model(full_name).await?;
        Ok(row.into_info(&schema))
    }

    async fn update_model(
        &self,
        full_name: &str,
        req: UpdateRegisteredModel,
        updated_by: Option<&str>,
    ) -> Result<RegisteredModelInfo> {
        if let Some(new_name) = &req.new_name {
            validate_sql_object_name(new_name)?;
        }
        if req.new_name.is_none() && req.comment.is_none() {
            return Err(Error::invalid_argument("No updated fields defined."));
        }
        // Upstream checks the new name before the model itself.
        let (c, s, _) = split_three_part(full_name, "registered model")?;
        if let Some(new_name) = &req.new_name {
            let schema = self.resolve_schema(c, s).await?;
            if self.model_row(&schema.schema_id, new_name).await?.is_some() {
                return Err(model_exists(&schema.child_full_name(new_name)));
            }
        }
        let (schema, mut existing) = self.resolve_model(full_name).await?;

        // Versions link to the model by id, so a rename is a field update.
        if let Some(new_name) = req.new_name {
            existing.name = new_name;
        }
        if let Some(c) = req.comment {
            existing.comment = Some(c);
        }
        touch_audit(
            &mut existing.updated_at,
            &mut existing.updated_by,
            updated_by,
        );

        let new_full = schema.child_full_name(&existing.name);
        let written = update(self.db.as_ref(), &REGISTERED_MODEL, &existing)
            .await
            .map_err(create_err_as(
                ErrorCode::AlreadyExists,
                format!("Registered model already exists: {new_full}"),
            ))?;
        if written == 0 {
            return Err(Error::internal("model update found no row"));
        }
        Ok(existing.into_info(&schema))
    }

    async fn delete_model(&self, full_name: &str, force: bool) -> Result<()> {
        let (_, model) = self.resolve_model(full_name).await?;
        let has_versions = self
            .exists_where("model_version", "registered_model_id", &model.id_string)
            .await?;
        if has_versions && !force {
            return Err(Error::aborted(format!(
                "Unable to delete a registered model with existing model versions: {full_name}"
            )));
        }
        let failed = db_err("delete registered model");
        let tx = self.db.begin().await.map_err(&failed)?;
        let id = [model.id_string.as_str().into()];
        queue_models(tx.as_ref(), "id_string = $1", &id).await?;
        for sql in [
            "DELETE FROM model_version WHERE registered_model_id = $1",
            "DELETE FROM permission WHERE resource_id = $1",
            "DELETE FROM registered_model WHERE id_string = $1",
        ] {
            tx.execute(sql, &id).await.map_err(&failed)?;
        }
        tx.commit().await.map_err(&failed)
    }

    async fn list_models(
        &self,
        catalog_name: Option<&str>,
        schema_name: Option<&str>,
        page: &Pagination,
    ) -> Result<(Vec<RegisteredModelInfo>, Option<String>)> {
        // Upstream `listRegisteredModels`: empty strings count as absent, and
        // a catalog without a schema (or the reverse) is an error.
        let catalog_name = catalog_name.filter(|s| !s.is_empty());
        let schema_name = schema_name.filter(|s| !s.is_empty());
        match (catalog_name, schema_name) {
            (Some(catalog), Some(schema)) => {
                let schema = self.resolve_schema(catalog, schema).await?;
                let (rows, next) = self
                    .paged_list::<RegisteredModelRow, _>(
                        &REGISTERED_MODEL,
                        &["schema_id = $1"],
                        vec![schema.schema_id.as_str().into()],
                        "name",
                        page,
                        |r| r.name.clone(),
                    )
                    .await?;
                Ok((
                    rows.into_iter().map(|r| r.into_info(&schema)).collect(),
                    next,
                ))
            }
            (Some(_), None) => Err(Error::invalid_argument(
                "Cannot specify catalog w/o schema for list registered models.",
            )),
            (None, Some(_)) => Err(Error::invalid_argument(
                "Cannot specify schema w/o catalog for list registered models.",
            )),
            (None, None) => {
                // Model names repeat across schemas, so the metastore-wide
                // list pages by id, as upstream does.
                if let Some(token) = page.page_token.as_deref() {
                    Uuid::parse_str(token).map_err(|_| {
                        Error::invalid_argument(format!("Invalid page token received: {token}"))
                    })?;
                }
                let (rows, next) = self
                    .paged_list::<RegisteredModelRow, _>(
                        &REGISTERED_MODEL,
                        &[],
                        vec![],
                        "id_string",
                        page,
                        |r| r.id_string.clone(),
                    )
                    .await?;
                let schema_ids: Vec<String> = rows.iter().map(|r| r.schema_id.clone()).collect();
                let schemas = self.schema_refs_by_ids(&schema_ids).await?;
                let infos = rows
                    .into_iter()
                    .filter_map(|r| {
                        let schema = schemas.get(&r.schema_id)?.clone();
                        Some(r.into_info(&schema))
                    })
                    .collect();
                Ok((infos, next))
            }
        }
    }

    async fn create_version(
        &self,
        req: CreateModelVersion,
        created_by: Option<&str>,
    ) -> Result<ModelVersionInfo> {
        let (schema, model) = self
            .resolve_model_parts(&req.catalog_name, &req.schema_name, &req.model_name)
            .await?;
        let model_full = schema.child_full_name(&model.name);

        // Upstream: the next version is the model's max version number + 1,
        // so numbers are never reused after a delete. Concurrent creates race;
        // the unique (model, version) index rejects the loser.
        let max = match model.max_version_number {
            Some(n) => n,
            None => {
                let rows = self
                    .db
                    .query(
                        "SELECT MAX(version) FROM model_version WHERE registered_model_id = $1",
                        &[model.id_string.as_str().into()],
                    )
                    .await
                    .map_err(db_err("model versions"))?;
                match rows.first() {
                    Some(r) => r.opt_int(0).map_err(db_err("model versions"))?,
                    None => None,
                }
                .unwrap_or(0)
            }
        };
        if max < 0 {
            return Err(Error::new(
                ErrorCode::OutOfRange,
                format!("Registered model has invalid max model version: {max}"),
            ));
        }
        let next_version = max + 1;

        // `<model location>/versions/<version id>`.
        let id = Uuid::new_v4().to_string();
        let storage_location = match model.storage_location.as_deref() {
            Some(loc) => {
                let url = NormalizedUrl::parse(loc)?
                    .managed_child_location(ManagedResource::ModelVersion, &id)?;
                create_storage_dir(&url)?;
                Some(url.into_string())
            }
            None => None,
        };

        let now = now_epoch_millis();
        let row = ModelVersionRow {
            id_string: id,
            registered_model_id: model.id_string.clone(),
            version: next_version,
            source: req.source,
            run_id: req.run_id,
            status: enum_str(&ModelVersionStatus::PendingRegistration),
            storage_location,
            comment: req.comment,
            created_at: now,
            created_by: created_by.map(str::to_string),
            updated_at: now,
            updated_by: created_by.map(str::to_string),
        };

        // The version and the model's highest number in one transaction.
        let failed = db_err("create model version");
        let tx = self.db.begin().await.map_err(&failed)?;
        insert(tx.as_ref(), &MODEL_VERSION, &row)
            .await
            .map_err(create_err_as(
                ErrorCode::AlreadyExists,
                format!("Model version already exists: {model_full}/{next_version}"),
            ))?;
        tx.execute(
            "UPDATE registered_model SET max_version_number = $1 WHERE id_string = $2",
            &[next_version.into(), model.id_string.as_str().into()],
        )
        .await
        .map_err(&failed)?;
        tx.commit().await.map_err(&failed)?;
        row.into_info(&schema, &model.name)
    }

    async fn get_version(&self, full_name: &str, version: i64) -> Result<ModelVersionInfo> {
        let (schema, model, row) = self.resolve_version(full_name, version).await?;
        row.into_info(&schema, &model.name)
    }

    async fn update_version(
        &self,
        full_name: &str,
        version: i64,
        req: UpdateModelVersion,
        updated_by: Option<&str>,
    ) -> Result<ModelVersionInfo> {
        // Upstream `updateModelVersion`: a comment is the only updatable field.
        if version < 1 {
            return Err(Error::invalid_argument(format!(
                "No valid model version specified: {version}"
            )));
        }
        let comment = req
            .comment
            .ok_or_else(|| Error::invalid_argument("No updated fields defined."))?;
        let (schema, model, mut row) = self.resolve_version(full_name, version).await?;
        row.comment = Some(comment);
        touch_audit(&mut row.updated_at, &mut row.updated_by, updated_by);
        self.save_version(row)
            .await?
            .into_info(&schema, &model.name)
    }

    async fn delete_version(&self, full_name: &str, version: i64) -> Result<()> {
        let (_, _, row) = self.resolve_version(full_name, version).await?;
        let failed = db_err("delete model version");
        let tx = self.db.begin().await.map_err(&failed)?;
        let id = [row.id_string.as_str().into()];
        queue_cleanup(
            tx.as_ref(),
            &MODEL_VERSION,
            ManagedResource::ModelVersion,
            "id_string = $1",
            &id,
        )
        .await?;
        tx.execute("DELETE FROM model_version WHERE id_string = $1", &id)
            .await
            .map_err(&failed)?;
        tx.commit().await.map_err(&failed)
    }

    async fn finalize_version(
        &self,
        full_name: &str,
        version: i64,
        updated_by: Option<&str>,
    ) -> Result<ModelVersionInfo> {
        let (schema, model, mut row) = self.resolve_version(full_name, version).await?;
        if row.status != enum_str(&ModelVersionStatus::PendingRegistration) {
            return Err(Error::invalid_argument(format!(
                "Model version not in a pending registration state: {full_name}/{version}"
            )));
        }
        row.status = enum_str(&ModelVersionStatus::Ready);
        touch_audit(&mut row.updated_at, &mut row.updated_by, updated_by);
        self.save_version(row)
            .await?
            .into_info(&schema, &model.name)
    }

    async fn list_versions(
        &self,
        full_name: &str,
        page: &Pagination,
    ) -> Result<(Vec<ModelVersionInfo>, Option<String>)> {
        // Upstream `listModelVersions`: the token is the last version
        // number and must parse as an integer; versions compare as numbers.
        let size = page.page_size()?;
        let after: i64 = match page.page_token.as_deref() {
            None => 0,
            Some(t) => t.trim().parse().map_err(|_| {
                Error::invalid_argument(format!("Invalid page token received: {t}"))
            })?,
        };
        let (schema, model) = self.resolve_model(full_name).await?;
        let rows: Vec<ModelVersionRow> = select(
            self.db.as_ref(),
            &MODEL_VERSION,
            "WHERE registered_model_id = $1 AND version > $2 ORDER BY version LIMIT $3",
            &[model.id_string.as_str().into(), after.into(), size.into()],
        )
        .await?;
        let next = page.next_token(&rows, |r| r.version)?;
        let infos: Result<Vec<ModelVersionInfo>> = rows
            .into_iter()
            .map(|r| r.into_info(&schema, &model.name))
            .collect();
        Ok((infos?, next))
    }
}
