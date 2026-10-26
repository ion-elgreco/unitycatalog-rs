//! `uc protection`: protect a securable against deletes, or remove its
//! protection.

use clap::Subcommand;
use uc_common::SecurableType;

use super::options::{PROTECTABLE_TYPES, help, one_of_wire};
use super::{Context, Result, upstream};
use crate::output::models;

#[derive(Debug, Subcommand)]
#[command(rename_all = "snake_case")]
pub enum ProtectionOp {
    /// Show whether a securable is protected against deletes
    Get {
        #[arg(long, help = help::PROTECTABLE_TYPE, ignore_case = true, hide_possible_values = true,
            value_parser = one_of_wire::<SecurableType>(PROTECTABLE_TYPES))]
        securable_type: SecurableType,
        #[arg(long, help = help::NAME)]
        name: String,
    },
    /// Protect a securable: it cannot be deleted, also not with its catalog
    /// or schema, until its protection is removed
    Create {
        #[arg(long, help = help::PROTECTABLE_TYPE, ignore_case = true, hide_possible_values = true,
            value_parser = one_of_wire::<SecurableType>(PROTECTABLE_TYPES))]
        securable_type: SecurableType,
        #[arg(long, help = help::NAME)]
        name: String,
    },
    /// Remove the protection of a securable
    Delete {
        #[arg(long, help = help::PROTECTABLE_TYPE, ignore_case = true, hide_possible_values = true,
            value_parser = one_of_wire::<SecurableType>(PROTECTABLE_TYPES))]
        securable_type: SecurableType,
        #[arg(long, help = help::NAME)]
        name: String,
    },
}

pub async fn run(c: &Context<'_>, op: ProtectionOp) -> Result<String> {
    let protection = match op {
        ProtectionOp::Get {
            securable_type,
            name,
        } => c
            .client
            .get_protection(securable_type.as_str(), name)
            .await
            .map_err(upstream("getProtection"))?,
        ProtectionOp::Create {
            securable_type,
            name,
        } => c
            .client
            .update_protection(securable_type.as_str(), name, true)
            .await
            .map_err(upstream("updateProtection"))?,
        ProtectionOp::Delete {
            securable_type,
            name,
        } => c
            .client
            .update_protection(securable_type.as_str(), name, false)
            .await
            .map_err(upstream("updateProtection"))?,
    };
    Ok(c.shaped(&models::PROTECTION, &serde_json::to_value(&protection)?))
}
