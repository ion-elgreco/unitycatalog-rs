// @generated — do not edit by hand.
#![allow(unused_imports)]

use super::builders::*;
use super::client::VolumeServiceClient;
use uc_client::models::volumes::v1::*;

/// A client scoped to a single `volume`.
#[derive(Clone)]
pub struct VolumeClient {
    pub(crate) volume_name: String,
    pub(crate) client: VolumeServiceClient,
}

impl VolumeClient {
    /// Create a client bound to the resource's name components.
    pub fn new(volume_name: impl Into<String>, client: VolumeServiceClient) -> Self {
        Self {
            volume_name: volume_name.into(),
            client,
        }
    }

    /// This resource's own name (the leaf component).
    pub fn name(&self) -> &str {
        &self.volume_name
    }

    /// The fully-qualified name of this resource.
    pub fn full_name(&self) -> String {
        self.volume_name.clone()
    }

    /// Get a volume.
    pub fn get(&self) -> GetVolumeBuilder {
        GetVolumeBuilder::new(self.client.clone(), &self.volume_name)
    }

    /// Change a volume.
    pub fn update(&self) -> UpdateVolumeBuilder {
        UpdateVolumeBuilder::new(self.client.clone(), &self.volume_name)
    }

    /// Delete a volume.
    pub fn delete(&self) -> DeleteVolumeBuilder {
        DeleteVolumeBuilder::new(self.client.clone(), &self.volume_name)
    }
}
