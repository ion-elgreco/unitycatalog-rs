// @generated — do not edit by hand.
#![allow(unused_imports)]

use super::builders::*;
use super::client::ShareServiceClient;
use uc_client::models::shares::v1::*;

/// A client scoped to a single `share`.
#[derive(Clone)]
pub struct ShareClient {
    pub(crate) share_name: String,
    pub(crate) client: ShareServiceClient,
}

impl ShareClient {
    /// Create a client bound to the resource's name components.
    pub fn new(share_name: impl Into<String>, client: ShareServiceClient) -> Self {
        Self {
            share_name: share_name.into(),
            client,
        }
    }

    /// This resource's own name (the leaf component).
    pub fn name(&self) -> &str {
        &self.share_name
    }

    /// The fully-qualified name of this resource.
    pub fn full_name(&self) -> String {
        self.share_name.clone()
    }

    /// Get a share.
    pub fn get(&self) -> GetShareBuilder {
        GetShareBuilder::new(self.client.clone(), &self.share_name)
    }

    /// Change a share: its comment, its name and its tables.
    pub fn update(&self) -> UpdateShareBuilder {
        UpdateShareBuilder::new(self.client.clone(), &self.share_name)
    }

    /// Delete a share.
    pub fn delete(&self) -> DeleteShareBuilder {
        DeleteShareBuilder::new(self.client.clone(), &self.share_name)
    }
}
