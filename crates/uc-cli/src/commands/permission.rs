//! `uc permission`: get, grant and revoke privileges on a securable.

use clap::Subcommand;
use uc_client::models::permissions::v1::PermissionsChange;
use uc_common::SecurableType;

use super::options::{PRIVILEGES, SECURABLE_TYPES, help, one_of, one_of_wire};
use super::{Context, Result, upstream};
use crate::output::models;

#[derive(Debug, Subcommand)]
#[command(rename_all = "snake_case")]
pub enum PermissionOp {
    /// Get the privileges on a securable
    Get {
        #[arg(long, help = help::SECURABLE_TYPE, ignore_case = true, hide_possible_values = true,
            value_parser = one_of_wire::<SecurableType>(SECURABLE_TYPES))]
        securable_type: SecurableType,
        #[arg(long, help = help::NAME)]
        name: String,
        #[arg(long, help = help::PRINCIPAL)]
        principal: Option<String>,
    },
    /// Grant a privilege on a securable
    Create {
        #[arg(long, help = help::SECURABLE_TYPE, ignore_case = true, hide_possible_values = true,
            value_parser = one_of_wire::<SecurableType>(SECURABLE_TYPES))]
        securable_type: SecurableType,
        #[arg(long, help = help::NAME)]
        name: String,
        #[arg(long, help = help::PRINCIPAL)]
        principal: String,
        #[arg(long, help = help::PRIVILEGE, ignore_case = true, hide_possible_values = true,
            value_parser = one_of(PRIVILEGES))]
        privilege: String,
    },
    /// Revoke a privilege on a securable
    Delete {
        #[arg(long, help = help::SECURABLE_TYPE, ignore_case = true, hide_possible_values = true,
            value_parser = one_of_wire::<SecurableType>(SECURABLE_TYPES))]
        securable_type: SecurableType,
        #[arg(long, help = help::NAME)]
        name: String,
        #[arg(long, help = help::PRINCIPAL)]
        principal: String,
        #[arg(long, help = help::PRIVILEGE, ignore_case = true, hide_possible_values = true,
            value_parser = one_of(PRIVILEGES))]
        privilege: String,
    },
}

pub async fn run(c: &Context<'_>, op: PermissionOp) -> Result<String> {
    // One privilege added or removed for the principal.
    let change = |principal: String, add: Vec<String>, remove: Vec<String>| PermissionsChange {
        principal,
        add,
        remove,
        ..Default::default()
    };
    let assignments = match op {
        PermissionOp::Get {
            securable_type,
            name,
            principal,
        } => c
            .client
            .get_permissions(securable_type.as_str(), name)
            .with_principal(principal)
            .await
            .map_err(upstream("get"))?,
        PermissionOp::Create {
            securable_type,
            name,
            principal,
            privilege,
        } => c
            .client
            .update_permissions(
                securable_type.as_str(),
                name,
                vec![change(principal, vec![privilege], vec![])],
            )
            .await
            .map_err(upstream("update"))?,
        PermissionOp::Delete {
            securable_type,
            name,
            principal,
            privilege,
        } => c
            .client
            .update_permissions(
                securable_type.as_str(),
                name,
                vec![change(principal, vec![], vec![privilege])],
            )
            .await
            .map_err(upstream("update"))?,
    };
    Ok(c.shaped_list(
        &models::PRIVILEGE_ASSIGNMENT,
        &serde_json::to_value(&assignments)?,
        "privilege_assignments",
    ))
}
