// @generated — do not edit by hand.
#![allow(unused_imports)]

use super::builders::*;
use super::client::TableServiceClient;
use uc_client::models::tables::v1::*;

/// A client scoped to a single `table`.
#[derive(Clone)]
pub struct TableClient {
    pub(crate) table_name: String,
    pub(crate) client: TableServiceClient,
}

impl TableClient {
    /// Create a client bound to the resource's name components.
    pub fn new(table_name: impl Into<String>, client: TableServiceClient) -> Self {
        Self {
            table_name: table_name.into(),
            client,
        }
    }

    /// This resource's own name (the leaf component).
    pub fn name(&self) -> &str {
        &self.table_name
    }

    /// The fully-qualified name of this resource.
    pub fn full_name(&self) -> String {
        self.table_name.clone()
    }

    /// Get a table.
    pub fn get(&self) -> GetTableBuilder {
        GetTableBuilder::new(self.client.clone(), &self.table_name)
    }

    /// Change the comments of a table and its columns.
    pub fn update(&self) -> UpdateTableBuilder {
        UpdateTableBuilder::new(self.client.clone(), &self.table_name)
    }

    /// Delete a table.
    pub fn delete(&self) -> DeleteTableBuilder {
        DeleteTableBuilder::new(self.client.clone(), &self.table_name)
    }
}
