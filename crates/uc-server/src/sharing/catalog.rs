//! The shares of the metastore as the engine's share catalog.

use async_trait::async_trait;
use opensharing_api::{Error, Result};
use opensharing_engine::{
    ShareCatalog,
    catalog::{
        AssetKind, AssetRef, DataFormat, Page, PageRequest, ResolvedLocation, Share, SharedAsset,
        SharedObject,
    },
    service::paginate,
};
use uc_api::{
    DataSourceFormat, HistoryDataSharingStatus, ShareInfo, SharedDataObject, SharedObjectType,
};
use uc_metastore::DynMetastore;

use super::{Recipient, internal, metastore_error};

pub struct MetastoreShares {
    metastore: DynMetastore,
}

impl MetastoreShares {
    pub fn new(metastore: DynMetastore) -> Self {
        Self { metastore }
    }

    async fn share(&self, name: &str) -> Result<ShareInfo> {
        (self.metastore.sharing().get_share(name).await).map_err(metastore_error)
    }
}

fn kind(object: &SharedDataObject) -> AssetKind {
    match object.data_object_type {
        SharedObjectType::Table => AssetKind::Table,
        SharedObjectType::Volume => AssetKind::Volume,
        SharedObjectType::AgentSkill => AssetKind::AgentSkill,
    }
}

fn share(info: ShareInfo) -> Share {
    Share {
        name: info.name,
        id: info.id,
        comment: info.comment,
        ..Default::default()
    }
}

/// The location of a table or volume; the metastore has one for each.
fn location(storage_location: Option<String>, full_name: &str) -> Result<ResolvedLocation> {
    let raw = storage_location
        .ok_or_else(|| internal(format!("'{full_name}' has no storage location")))?;
    ResolvedLocation::parse(raw)
}

#[async_trait]
impl ShareCatalog<Recipient> for MetastoreShares {
    async fn list_shares(&self, page: PageRequest, recipient: &Recipient) -> Result<Page<Share>> {
        let mut names = recipient.shares.clone();
        names.sort();
        let names = paginate(names, page.max_results, page.page_token.as_deref())?;
        let mut items = Vec::with_capacity(names.items.len());
        for name in &names.items {
            match self.share(name).await {
                Ok(info) => items.push(share(info)),
                // A share deleted after it was granted.
                Err(Error::NotFound) => {}
                Err(e) => return Err(e),
            }
        }
        Ok(Page {
            items,
            next_page_token: names.next_page_token,
        })
    }

    async fn get_share(&self, name: &str, _: &Recipient) -> Result<Share> {
        Ok(share(self.share(name).await?))
    }

    async fn list_objects(&self, name: &str, _: &Recipient) -> Result<Vec<SharedObject>> {
        let info = self.share(name).await?;
        Ok((info.objects.iter())
            .map(|object| SharedObject {
                schema: object.shared_as_schema.clone(),
                name: object.shared_as_name.clone(),
                kind: kind(object),
                id: None,
            })
            .collect())
    }

    async fn resolve(&self, asset: &AssetRef, _: &Recipient) -> Result<SharedAsset> {
        let info = self.share(&asset.share).await?;
        let object = (info.objects.iter())
            .find(|o| {
                o.shared_as_schema == asset.schema
                    && o.shared_as_name == asset.name
                    && kind(o) == asset.kind
            })
            .ok_or(Error::NotFound)?;
        let source = &object.source_full_name;
        Ok(match asset.kind {
            AssetKind::Table => {
                let table = (self.metastore.tables().get(source).await).map_err(metastore_error)?;
                let format = match table.data_source_format {
                    Some(DataSourceFormat::Delta) => DataFormat::Delta,
                    Some(DataSourceFormat::Iceberg) => DataFormat::Iceberg,
                    _ => return Err(Error::NotImplemented("serving this table format")),
                };
                SharedAsset {
                    reference: asset.clone(),
                    id: table.table_id,
                    location: location(table.storage_location, source)?,
                    format: Some(format),
                    history_shared: object.history_data_sharing_status
                        == HistoryDataSharingStatus::Enabled,
                    start_version: object.start_version.and_then(|v| u64::try_from(v).ok()),
                    source_name: Some(source.clone()),
                }
            }
            AssetKind::Volume | AssetKind::AgentSkill => {
                let volume =
                    (self.metastore.volumes().get(source).await).map_err(metastore_error)?;
                SharedAsset {
                    reference: asset.clone(),
                    id: volume.volume_id,
                    location: location(volume.storage_location, source)?,
                    format: None,
                    history_shared: false,
                    start_version: None,
                    source_name: Some(source.clone()),
                }
            }
        })
    }
}
