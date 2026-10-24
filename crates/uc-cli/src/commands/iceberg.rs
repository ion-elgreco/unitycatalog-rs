//! `uc iceberg`: read the Iceberg REST catalog.

use std::collections::HashMap;

use clap::Subcommand;
use serde_json::{Map, Value};

use super::options::{IcebergPaging, ThreePartName, help, three_part_name};
use super::{Context, Result, upstream};
use crate::output::sorted;

#[derive(Debug, Subcommand)]
#[command(rename_all = "snake_case")]
pub enum IcebergOp {
    /// Get the client config for a catalog
    Config {
        #[arg(long, help = help::CATALOG)]
        catalog: String,
    },
    /// List the namespaces of a catalog
    ListNamespaces {
        #[arg(long, help = help::CATALOG)]
        catalog: String,
        #[command(flatten)]
        paging: IcebergPaging,
    },
    /// List the tables of a namespace
    ListTables {
        #[arg(long, help = help::CATALOG)]
        catalog: String,
        #[arg(long, help = help::SCHEMA)]
        schema: String,
        #[command(flatten)]
        paging: IcebergPaging,
    },
    /// Load a table's metadata
    LoadTable {
        #[arg(long, help = help::FULL_NAME, value_parser = three_part_name)]
        full_name: ThreePartName,
    },
}

pub async fn run(c: &Context<'_>, op: IcebergOp) -> Result<String> {
    match op {
        IcebergOp::Config { catalog } => {
            let config = c
                .client
                .get_iceberg_config(catalog)
                .await
                .map_err(upstream("getConfig"))?;
            // `defaults` and `overrides` print also when empty.
            let mut shown = Map::new();
            shown.insert("defaults".into(), map(&config.defaults)?);
            shown.insert("overrides".into(), map(&config.overrides)?);
            if !config.endpoints.is_empty() {
                shown.insert("endpoints".into(), serde_json::to_value(&config.endpoints)?);
            }
            Ok(c.json(&Value::Object(shown)))
        }
        IcebergOp::ListNamespaces { catalog, paging } => {
            let list = c
                .client
                .list_iceberg_namespaces(catalog)
                .with_page_token(paging.page_token)
                .with_page_size(paging.max_results)
                .await
                .map_err(upstream("listNamespaces"))?;
            c.page_token(list.next_page_token.as_deref());
            Ok(c.json(&serde_json::to_value(&list.namespaces)?))
        }
        IcebergOp::ListTables {
            catalog,
            schema,
            paging,
        } => {
            let list = c
                .client
                .list_iceberg_tables(catalog, schema)
                .with_page_token(paging.page_token)
                .with_page_size(paging.max_results)
                .await
                .map_err(upstream("listTables"))?;
            c.page_token(list.next_page_token.as_deref());
            Ok(c.json(&serde_json::to_value(&list.identifiers)?))
        }
        IcebergOp::LoadTable { full_name } => {
            let table = c
                .client
                .load_iceberg_table(full_name.catalog, full_name.schema, full_name.name)
                .await
                .map_err(upstream("loadTable"))?;
            // `config` prints also when empty.
            let mut shown = Map::new();
            if let Some(location) = &table.metadata_location {
                shown.insert("metadata-location".into(), location.as_str().into());
            }
            shown.insert("metadata".into(), serde_json::to_value(&table.metadata)?);
            shown.insert("config".into(), map(&table.config)?);
            if let Some(etag) = &table.etag {
                shown.insert("etag".into(), etag.as_str().into());
            }
            Ok(c.json(&Value::Object(shown)))
        }
    }
}

/// A map of the API as a JSON object, in key order.
fn map(entries: &HashMap<String, String>) -> serde_json::Result<Value> {
    serde_json::to_value(entries).map(sorted)
}
