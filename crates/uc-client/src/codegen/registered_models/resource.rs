// @generated — do not edit by hand.
#![allow(unused_imports)]

use super::builders::*;
use super::client::RegisteredModelServiceClient;
use uc_client::models::models::v1::*;

/// A client scoped to a single `registered_model`.
#[derive(Clone)]
pub struct RegisteredModelClient {
    pub(crate) registered_model_name: String,
    pub(crate) client: RegisteredModelServiceClient,
}

impl RegisteredModelClient {
    /// Create a client bound to the resource's name components.
    pub fn new(
        registered_model_name: impl Into<String>,
        client: RegisteredModelServiceClient,
    ) -> Self {
        Self {
            registered_model_name: registered_model_name.into(),
            client,
        }
    }

    /// This resource's own name (the leaf component).
    pub fn name(&self) -> &str {
        &self.registered_model_name
    }

    /// The fully-qualified name of this resource.
    pub fn full_name(&self) -> String {
        self.registered_model_name.clone()
    }

    /// Get a registered model.
    pub fn get(&self) -> GetRegisteredModelBuilder {
        GetRegisteredModelBuilder::new(self.client.clone(), &self.registered_model_name)
    }

    /// Change a registered model.
    pub fn update(&self) -> UpdateRegisteredModelBuilder {
        UpdateRegisteredModelBuilder::new(self.client.clone(), &self.registered_model_name)
    }

    /// Delete a registered model. A model with versions needs `force`.
    pub fn delete(&self) -> DeleteRegisteredModelBuilder {
        DeleteRegisteredModelBuilder::new(self.client.clone(), &self.registered_model_name)
    }
}
