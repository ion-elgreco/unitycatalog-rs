//! `uc permission`: get, grant and revoke privileges on a securable.

use clap::Subcommand;
use uc_api::{PermissionsChange, Privilege, UpdatePermissions};
use uc_common::SecurableType;

use super::options::{PRIVILEGES, SECURABLE_TYPES, help, one_of_wire};
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
            value_parser = one_of_wire::<Privilege>(PRIVILEGES))]
        privilege: Privilege,
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
            value_parser = one_of_wire::<Privilege>(PRIVILEGES))]
        privilege: Privilege,
    },
}

pub async fn run(c: &Context<'_>, op: PermissionOp) -> Result<String> {
    // One privilege added or removed for the principal.
    let change =
        |principal: String, add: Vec<Privilege>, remove: Vec<Privilege>| UpdatePermissions {
            changes: vec![PermissionsChange {
                principal,
                add,
                remove,
            }],
        };
    let assignments = match op {
        PermissionOp::Get {
            securable_type,
            name,
            principal,
        } => c
            .client
            .get_permissions(securable_type, &name, principal.as_deref())
            .await
            .map_err(upstream("get"))?,
        PermissionOp::Create {
            securable_type,
            name,
            principal,
            privilege,
        } => {
            let request = change(principal, vec![privilege], vec![]);
            c.client
                .update_permissions(securable_type, &name, &request)
                .await
                .map_err(upstream("update"))?
        }
        PermissionOp::Delete {
            securable_type,
            name,
            principal,
            privilege,
        } => {
            let request = change(principal, vec![], vec![privilege]);
            c.client
                .update_permissions(securable_type, &name, &request)
                .await
                .map_err(upstream("update"))?
        }
    };
    Ok(c.shaped_list(
        &models::PRIVILEGE_ASSIGNMENT,
        &serde_json::to_value(&assignments)?,
        "privilege_assignments",
    ))
}
