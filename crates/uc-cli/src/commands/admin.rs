//! `uc admin`: back up and restore the catalog; list audit events.

use clap::Subcommand;
use uc_api::BackupResponse;
use uc_client::AuditFilter;

use super::options::help;
use super::{Context, Result, upstream};
use crate::output::{Format, jackson_pretty};

#[derive(Debug, Subcommand)]
#[command(rename_all = "snake_case")]
pub enum AdminOp {
    /// Print a snapshot of the catalog, as JSON
    Backup,
    /// Restore a snapshot
    Restore {
        /// The snapshot file; - for standard input
        #[arg(long)]
        path: String,
    },
    /// List audit events
    Audit {
        /// Only events of this principal
        #[arg(long)]
        principal: Option<String>,
        /// Only events of this HTTP method
        #[arg(long)]
        method: Option<String>,
        /// Only events of paths starting with this
        #[arg(long)]
        path_prefix: Option<String>,
        #[arg(long, help = help::MAX_RESULTS)]
        max_results: Option<i32>,
    },
}

pub async fn run(c: &Context<'_>, op: AdminOp) -> Result<String> {
    match op {
        // A snapshot is for `admin restore`: JSON whatever the format.
        AdminOp::Backup => {
            let snapshot = c.client.backup().await.map_err(upstream("backup"))?;
            let snapshot = serde_json::to_value(&snapshot)?;
            Ok(match c.format {
                Format::JsonPretty => jackson_pretty(&snapshot),
                _ => snapshot.to_string(),
            })
        }
        AdminOp::Restore { path } => {
            let text = if path == "-" {
                std::io::read_to_string(std::io::stdin())?
            } else {
                std::fs::read_to_string(&path)?
            };
            let snapshot: BackupResponse = serde_json::from_str(&text)?;
            let report = c
                .client
                .restore(&snapshot)
                .await
                .map_err(upstream("restore"))?;
            Ok(c.json(&serde_json::to_value(&report)?))
        }
        AdminOp::Audit {
            principal,
            method,
            path_prefix,
            max_results,
        } => {
            let filter = AuditFilter {
                principal,
                method,
                path_prefix,
                max_results,
                ..AuditFilter::default()
            };
            let events = c
                .client
                .list_audit_events(&filter)
                .await
                .map_err(upstream("audit"))?;
            Ok(c.json(&serde_json::to_value(&events.events)?))
        }
    }
}
