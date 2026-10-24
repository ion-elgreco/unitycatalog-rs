// @generated — do not edit by hand.
#![allow(unused_imports)]

use super::builders::*;
use super::client::FunctionServiceClient;
use uc_client::models::functions::v1::*;

/// A client scoped to a single `function`.
#[derive(Clone)]
pub struct FunctionClient {
    pub(crate) function_name: String,
    pub(crate) client: FunctionServiceClient,
}

impl FunctionClient {
    /// Create a client bound to the resource's name components.
    pub fn new(function_name: impl Into<String>, client: FunctionServiceClient) -> Self {
        Self {
            function_name: function_name.into(),
            client,
        }
    }

    /// This resource's own name (the leaf component).
    pub fn name(&self) -> &str {
        &self.function_name
    }

    /// The fully-qualified name of this resource.
    pub fn full_name(&self) -> String {
        self.function_name.clone()
    }

    /// Get a function.
    pub fn get(&self) -> GetFunctionBuilder {
        GetFunctionBuilder::new(self.client.clone(), &self.function_name)
    }

    /// Delete a function.
    pub fn delete(&self) -> DeleteFunctionBuilder {
        DeleteFunctionBuilder::new(self.client.clone(), &self.function_name)
    }
}
