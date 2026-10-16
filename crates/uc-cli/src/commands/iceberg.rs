//! `uc iceberg`: read the Iceberg REST catalog.

use clap::Subcommand;

use super::options::{IcebergPaging, ThreePartName, help, three_part_name};
use super::{Context, Result, upstream};

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
                .iceberg_config(&catalog)
                .await
                .map_err(upstream("getConfig"))?;
            Ok(c.json(&serde_json::to_value(&config)?))
        }
        IcebergOp::ListNamespaces { catalog, paging } => {
            let list = c
                .client
                .list_iceberg_namespaces(&catalog, &paging.into())
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
                .list_iceberg_tables(&catalog, &schema, &paging.into())
                .await
                .map_err(upstream("listTables"))?;
            c.page_token(list.next_page_token.as_deref());
            Ok(c.json(&serde_json::to_value(&list.identifiers)?))
        }
        IcebergOp::LoadTable { full_name } => {
            let table = c
                .client
                .load_iceberg_table(&full_name.catalog, &full_name.schema, &full_name.name)
                .await
                .map_err(upstream("loadTable"))?;
            Ok(c.json(&serde_json::to_value(&table)?))
        }
    }
}
