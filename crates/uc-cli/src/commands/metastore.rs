//! `uc metastore`: the metastore summary.

use clap::Subcommand;

use super::{Context, Result, upstream};
use crate::output::models;

#[derive(Debug, Subcommand)]
pub enum MetastoreOp {
    /// Get the metastore summary
    Get,
}

pub async fn run(c: &Context<'_>, op: MetastoreOp) -> Result<String> {
    match op {
        MetastoreOp::Get => {
            let summary = c
                .client
                .get_metastore_summary()
                .await
                .map_err(upstream("summary"))?;
            Ok(c.shaped(&models::METASTORE_SUMMARY, &serde_json::to_value(&summary)?))
        }
    }
}
