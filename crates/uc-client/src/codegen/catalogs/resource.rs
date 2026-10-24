// @generated — do not edit by hand.
#![allow(unused_imports)]

use super::builders::*;
use super::client::CatalogServiceClient;
use uc_client::models::catalogs::v1::*;

/// A client scoped to a single `catalog`.
#[derive(Clone)]
pub struct CatalogClient {
    pub(crate) catalog_name: String,
    pub(crate) client: CatalogServiceClient,
}

impl CatalogClient {
    /// Create a client bound to the resource's name components.
    pub fn new(catalog_name: impl Into<String>, client: CatalogServiceClient) -> Self {
        Self {
            catalog_name: catalog_name.into(),
            client,
        }
    }

    /// This resource's own name (the leaf component).
    pub fn name(&self) -> &str {
        &self.catalog_name
    }

    /// The fully-qualified name of this resource.
    pub fn full_name(&self) -> String {
        self.catalog_name.clone()
    }

    /// Get a catalog.
    pub fn get(&self) -> GetCatalogBuilder {
        GetCatalogBuilder::new(self.client.clone(), &self.catalog_name)
    }

    /// Change a catalog.
    pub fn update(&self) -> UpdateCatalogBuilder {
        UpdateCatalogBuilder::new(self.client.clone(), &self.catalog_name)
    }

    /// Delete a catalog.
    pub fn delete(&self) -> DeleteCatalogBuilder {
        DeleteCatalogBuilder::new(self.client.clone(), &self.catalog_name)
    }
}
