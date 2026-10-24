// @generated — do not edit by hand.
#![allow(unused_imports)]

use super::builders::*;
use super::client::RecipientServiceClient;
use uc_client::models::recipients::v1::*;

/// A client scoped to a single `recipient`.
#[derive(Clone)]
pub struct RecipientClient {
    pub(crate) recipient_name: String,
    pub(crate) client: RecipientServiceClient,
}

impl RecipientClient {
    /// Create a client bound to the resource's name components.
    pub fn new(recipient_name: impl Into<String>, client: RecipientServiceClient) -> Self {
        Self {
            recipient_name: recipient_name.into(),
            client,
        }
    }

    /// This resource's own name (the leaf component).
    pub fn name(&self) -> &str {
        &self.recipient_name
    }

    /// The fully-qualified name of this resource.
    pub fn full_name(&self) -> String {
        self.recipient_name.clone()
    }

    /// Get a recipient. The response does not hold the bearer token.
    pub fn get(&self) -> GetRecipientBuilder {
        GetRecipientBuilder::new(self.client.clone(), &self.recipient_name)
    }

    /// Change a recipient: its comment and the shares it can read.
    pub fn update(&self) -> UpdateRecipientBuilder {
        UpdateRecipientBuilder::new(self.client.clone(), &self.recipient_name)
    }

    /// Delete a recipient.
    pub fn delete(&self) -> DeleteRecipientBuilder {
        DeleteRecipientBuilder::new(self.client.clone(), &self.recipient_name)
    }

    /// Replace the bearer token of a recipient. The old token stops working. The response holds the
    /// new token.
    pub fn rotate_recipient_token(&self) -> RotateRecipientTokenBuilder {
        RotateRecipientTokenBuilder::new(self.client.clone(), &self.recipient_name)
    }

    /// Replace the bearer token of a recipient and return a Delta Sharing profile with the new token.
    /// The old token stops working.
    pub fn create_recipient_profile(&self) -> CreateRecipientProfileBuilder {
        CreateRecipientProfileBuilder::new(self.client.clone(), &self.recipient_name)
    }
}
