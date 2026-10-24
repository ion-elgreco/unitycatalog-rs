// @generated — do not edit by hand.
#![allow(unused_imports)]

use super::builders::*;
use super::client::SchemaServiceClient;
use uc_client::models::schemas::v1::*;

/// A client scoped to a single `schema`.
#[derive(Clone)]
pub struct SchemaClient {
    pub(crate) schema_name: String,
    pub(crate) client: SchemaServiceClient,
}

impl SchemaClient {
    /// Create a client bound to the resource's name components.
    pub fn new(schema_name: impl Into<String>, client: SchemaServiceClient) -> Self {
        Self {
            schema_name: schema_name.into(),
            client,
        }
    }

    /// This resource's own name (the leaf component).
    pub fn name(&self) -> &str {
        &self.schema_name
    }

    /// The fully-qualified name of this resource.
    pub fn full_name(&self) -> String {
        self.schema_name.clone()
    }

    /// Get a schema.
    pub fn get(&self) -> GetSchemaBuilder {
        GetSchemaBuilder::new(self.client.clone(), &self.schema_name)
    }

    /// Change a schema.
    pub fn update(&self) -> UpdateSchemaBuilder {
        UpdateSchemaBuilder::new(self.client.clone(), &self.schema_name)
    }

    /// Delete a schema.
    pub fn delete(&self) -> DeleteSchemaBuilder {
        DeleteSchemaBuilder::new(self.client.clone(), &self.schema_name)
    }
}
