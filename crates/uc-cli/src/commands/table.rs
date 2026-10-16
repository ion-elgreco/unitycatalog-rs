//! `uc table`: list, get and delete tables; create, read and write Delta
//! tables with [`crate::data::delta`].

use clap::{Args, Subcommand};
use uc_api::{DataSourceFormat, TableType};

use super::options::{
    Paging, Properties, TABLE_TYPES, ThreePartName, help, one_of_wire, properties, three_part_name,
};
use super::{Context, Result, upstream};
use crate::data::delta;
use crate::output::models;

#[derive(Debug, Subcommand)]
#[command(rename_all = "snake_case")]
pub enum TableOp {
    /// Create a table; for DELTA, with a Delta table at its location
    Create(TableCreate),
    /// List the tables of a schema
    List {
        #[arg(long, help = help::CATALOG)]
        catalog: String,
        #[arg(long, help = help::SCHEMA)]
        schema: String,
        #[command(flatten)]
        paging: Paging,
    },
    /// Get a table
    Get {
        #[arg(long, help = help::FULL_NAME)]
        full_name: String,
    },
    /// Delete a table
    Delete {
        #[arg(long, help = help::FULL_NAME)]
        full_name: String,
    },
    /// Print the rows of a Delta table
    Read {
        #[arg(long, help = help::FULL_NAME)]
        full_name: String,
        /// The maximum number of rows to print
        #[arg(long, default_value_t = 100)]
        max_results: usize,
    },
    /// Write 15 random rows to a Delta table
    Write {
        #[arg(long, help = help::FULL_NAME)]
        full_name: String,
    },
}

const FORMATS: &[&str] = &[
    "DELTA", "ICEBERG", "CSV", "JSON", "AVRO", "PARQUET", "ORC", "TEXT",
];

// `table create`.
#[derive(Debug, Args)]
#[command(rename_all = "snake_case")]
pub struct TableCreate {
    #[arg(long, help = help::FULL_NAME, value_parser = three_part_name)]
    pub full_name: ThreePartName,
    /// The columns, in the SQL-like format "column_name column_data_type",
    /// separated by commas: "id INT, name STRING". Supported data types are
    /// BOOLEAN, BYTE, SHORT, INT, LONG, FLOAT, DOUBLE, DATE, TIMESTAMP,
    /// TIMESTAMP_NTZ, STRING, BINARY and DECIMAL
    #[arg(long)]
    pub columns: String,
    /// The format of the data source
    #[arg(long, default_value = "DELTA", value_parser = one_of_wire::<DataSourceFormat>(FORMATS), ignore_case = true)]
    pub format: DataSourceFormat,
    #[arg(long, help = help::PROPERTIES, value_parser = properties)]
    pub properties: Option<Properties>,
    #[arg(long, help = help::STORAGE_LOCATION)]
    pub storage_location: Option<String>,
    /// The type of the table
    #[arg(long, default_value = "EXTERNAL", value_parser = one_of_wire::<TableType>(TABLE_TYPES), ignore_case = true)]
    pub table_type: TableType,
}

pub async fn run(c: &Context<'_>, op: TableOp) -> Result<String> {
    match op {
        TableOp::Create(create) => delta::create_table(c, create).await,
        TableOp::List {
            catalog,
            schema,
            paging,
        } => {
            let list = c
                .client
                .list_tables(&catalog, &schema, &paging.into())
                .await
                .map_err(upstream("listTables"))?;
            Ok(c.shaped_list(&models::TABLE, &serde_json::to_value(&list)?, "tables"))
        }
        TableOp::Get { full_name } => {
            let info = c
                .client
                .get_table(&full_name, true, true)
                .await
                .map_err(upstream("getTable"))?;
            Ok(c.shaped(&models::TABLE, &serde_json::to_value(&info)?))
        }
        TableOp::Read {
            full_name,
            max_results,
        } => delta::read_table(c, &full_name, max_results).await,
        TableOp::Write { full_name } => delta::write_table(c, &full_name).await,
        TableOp::Delete { full_name } => {
            c.client
                .delete_table(&full_name)
                .await
                .map_err(upstream("deleteTable"))?;
            Ok(String::new())
        }
    }
}
