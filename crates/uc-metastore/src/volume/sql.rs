use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use uc_api::{CreateVolumeRequestContent, UpdateVolumeRequestContent, VolumeInfo, VolumeType};
use uc_common::{
    Error, ErrorCode, ManagedResource, NormalizedUrl, Pagination, Result, now_epoch_millis,
    validate_sql_object_name,
};

use crate::VolumeOps;
use crate::cleanup::sql::queue_cleanup;
use crate::sql::rows::VOLUME;
use crate::sql::{
    SchemaRef, SqlMetastore, by_id, create_err_as, db_err, enum_str, insert, parse_enum,
    select_one, split_three_part, touch_audit, update,
};
use uc_storage::{Exec, Value};

/// A volume row, keyed by its UUID and linked to its schema by id.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct VolumeRow {
    pub(crate) id_string: String,
    pub(crate) name: String,
    pub(crate) schema_id: String,
    pub(crate) volume_type: String,
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
}

impl VolumeRow {
    fn into_info(self, schema: &SchemaRef) -> Result<VolumeInfo> {
        Ok(VolumeInfo {
            catalog_name: Some(schema.catalog_name.clone()),
            schema_name: Some(schema.schema_name.clone()),
            full_name: Some(schema.child_full_name(&self.name)),
            name: Some(self.name),
            comment: self.comment,
            owner: self.owner,
            created_at: Some(self.created_at),
            created_by: self.created_by,
            updated_at: Some(self.updated_at),
            updated_by: self.updated_by,
            volume_id: Some(self.id_string),
            volume_type: Some(parse_enum::<VolumeType>(&self.volume_type)?),
            storage_location: self.storage_location,
        })
    }
}

/// Before dropping the volumes that `filter` picks (binding `params`), in
/// the same transaction: MANAGED volumes queue storage cleanup.
pub(crate) async fn queue_managed_volumes<E: Exec + ?Sized>(
    ex: &E,
    filter: &str,
    params: &[Value],
) -> Result<()> {
    let managed = format!("({filter}) AND volume_type = 'MANAGED'");
    queue_cleanup(ex, &VOLUME, ManagedResource::Volume, &managed, params).await
}

fn volume_exists(name: &str) -> Error {
    Error::new(
        ErrorCode::ResourceAlreadyExists,
        format!("Volume already exists: {name}"),
    )
}

impl SqlMetastore {
    async fn volume_row(&self, schema_id: &str, name: &str) -> Result<Option<VolumeRow>> {
        select_one(
            self.db.as_ref(),
            &VOLUME,
            "WHERE schema_id = $1 AND name = $2",
            &[schema_id.into(), name.into()],
        )
        .await
    }

    pub(crate) async fn resolve_volume(&self, full_name: &str) -> Result<(SchemaRef, VolumeRow)> {
        let (c, s, v) = split_three_part(full_name, "volume")?;
        let schema = self.resolve_schema(c, s).await?;
        let row = self
            .volume_row(&schema.schema_id, v)
            .await?
            .ok_or_else(|| Error::not_found(format!("Volume not found: {full_name}")))?;
        Ok((schema, row))
    }
}

#[async_trait]
impl VolumeOps for SqlMetastore {
    async fn create(
        &self,
        req: CreateVolumeRequestContent,
        created_by: Option<&str>,
    ) -> Result<VolumeInfo> {
        validate_sql_object_name(&req.name)?;
        let schema = self
            .resolve_schema(&req.catalog_name, &req.schema_name)
            .await?;
        if self
            .volume_row(&schema.schema_id, &req.name)
            .await?
            .is_some()
        {
            return Err(volume_exists(&req.name));
        }

        // Upstream `VolumeRepository.createVolume`: the server places a
        // MANAGED volume; an EXTERNAL one names its own path.
        let id = Uuid::new_v4().to_string();
        let storage_location = match req.volume_type {
            VolumeType::Managed => {
                if req.storage_location.is_some() {
                    return Err(Error::invalid_argument(
                        "Storage location should not be specified for managed volume",
                    ));
                }
                schema
                    .managed_parent_location(&self.roots, ManagedResource::Volume)?
                    .managed_child_location(ManagedResource::Volume, &id)?
            }
            VolumeType::External => {
                let raw = req.storage_location.as_deref().ok_or_else(|| {
                    Error::invalid_argument("Storage location is required for external volume")
                })?;
                let url = NormalizedUrl::parse(raw)?;
                if url.is_cloud_storage_root() {
                    return Err(Error::invalid_argument(format!(
                        "External volume storage location must include a non-empty path prefix: {raw}"
                    )));
                }
                self.validate_not_overlap_with_managed_storage(&url).await?;
                url
            }
        };

        let now = now_epoch_millis();
        let row = VolumeRow {
            id_string: id,
            name: req.name.clone(),
            schema_id: schema.schema_id.clone(),
            volume_type: enum_str(&req.volume_type),
            comment: req.comment,
            owner: created_by.map(str::to_string),
            created_at: now,
            created_by: created_by.map(str::to_string),
            updated_at: now,
            updated_by: created_by.map(str::to_string),
            storage_location: Some(storage_location.into_string()),
        };

        insert(self.db.as_ref(), &VOLUME, &row)
            .await
            .map_err(create_err_as(
                ErrorCode::ResourceAlreadyExists,
                format!("Volume already exists: {}", req.name),
            ))?;
        row.into_info(&schema)
    }

    async fn get(&self, full_name: &str) -> Result<VolumeInfo> {
        let (schema, row) = self.resolve_volume(full_name).await?;
        row.into_info(&schema)
    }

    async fn get_by_id(&self, volume_id: &str) -> Result<VolumeInfo> {
        let row: VolumeRow = by_id(self.db.as_ref(), &VOLUME, volume_id)
            .await?
            .ok_or_else(|| Error::not_found(format!("Volume not found: {volume_id}")))?;
        let schema = self.schema_ref_by_id(&row.schema_id).await?;
        row.into_info(&schema)
    }

    async fn update(
        &self,
        full_name: &str,
        req: UpdateVolumeRequestContent,
        updated_by: Option<&str>,
    ) -> Result<VolumeInfo> {
        if let Some(new_name) = &req.new_name {
            validate_sql_object_name(new_name)?;
        }
        let (schema, mut row) = self.resolve_volume(full_name).await?;

        // Upstream rejects any new name that exists, the current one included.
        if let Some(new_name) = req.new_name.as_ref()
            && self
                .volume_row(&schema.schema_id, new_name)
                .await?
                .is_some()
        {
            return Err(volume_exists(new_name));
        }
        // Nothing to change returns the volume untouched.
        if req.new_name.is_none() && req.comment.is_none() {
            return row.into_info(&schema);
        }
        if let Some(new_name) = req.new_name {
            row.name = new_name;
        }
        if let Some(c) = req.comment {
            row.comment = Some(c);
        }
        touch_audit(&mut row.updated_at, &mut row.updated_by, updated_by);

        let written = update(self.db.as_ref(), &VOLUME, &row)
            .await
            .map_err(create_err_as(
                ErrorCode::ResourceAlreadyExists,
                format!("Volume already exists: {}", row.name),
            ))?;
        if written == 0 {
            return Err(Error::not_found(format!("Volume not found: {full_name}")));
        }
        row.into_info(&schema)
    }

    async fn delete(&self, full_name: &str) -> Result<()> {
        let (_, row) = self.resolve_volume(full_name).await?;
        // Upstream `deleteVolume`: a MANAGED volume queues storage cleanup in
        // the same transaction as the drop.
        let failed = db_err("delete volume");
        let tx = self.db.begin().await.map_err(&failed)?;
        let id = [row.id_string.as_str().into()];
        queue_managed_volumes(tx.as_ref(), "id_string = $1", &id).await?;
        for sql in [
            "DELETE FROM permission WHERE resource_id = $1",
            "DELETE FROM volume WHERE id_string = $1",
        ] {
            tx.execute(sql, &id).await.map_err(&failed)?;
        }
        tx.commit().await.map_err(&failed)
    }

    async fn list(
        &self,
        catalog_name: &str,
        schema_name: &str,
        page: &Pagination,
    ) -> Result<(Vec<VolumeInfo>, Option<String>)> {
        let schema = self.resolve_schema(catalog_name, schema_name).await?;
        let (rows, next) = self
            .paged_list::<VolumeRow, _>(
                &VOLUME,
                &["schema_id = $1"],
                vec![schema.schema_id.as_str().into()],
                "name",
                page,
                |r| r.name.clone(),
            )
            .await?;
        let infos: Result<Vec<VolumeInfo>> =
            rows.into_iter().map(|r| r.into_info(&schema)).collect();
        Ok((infos?, next))
    }
}
