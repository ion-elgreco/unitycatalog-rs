//! `uc function`: create, list, get and delete functions; call Python
//! functions with [`crate::data::python`].

use clap::Subcommand;
use uc_client::models::functions::v1::{
    CreateFunction, FunctionParameterInfos, ParameterStyle, RoutineBody, SecurityType,
    SqlDataAccess,
};
use uc_client::models::tables::v1::ColumnTypeName;

use super::columns::{TYPE_NAMES, parse_input_params};
use super::options::{Paging, ThreePartName, help, one_of_wire, three_part_name, wire_name};
use super::{Context, Result, upstream};
use crate::data::python;
use crate::output::models;

#[derive(Debug, Subcommand)]
#[command(rename_all = "snake_case")]
pub enum FunctionOp {
    /// Create a Python function
    Create {
        #[arg(long, help = help::FULL_NAME, value_parser = three_part_name)]
        full_name: ThreePartName,
        /// The parameters, as "name TYPE" separated by commas: "a INT, b STRING"
        #[arg(long)]
        input_params: String,
        /// The data type of the result
        #[arg(long, value_parser = one_of_wire::<ColumnTypeName>(TYPE_NAMES),
            ignore_case = true)]
        data_type: ColumnTypeName,
        #[arg(long, help = help::COMMENT)]
        comment: Option<String>,
        /// The routine definition: the body of the function
        #[arg(long)]
        def: Option<String>,
        /// The language of the function
        #[arg(long, default_value = "python")]
        language: String,
    },
    /// List the functions of a schema
    List {
        #[arg(long, help = help::CATALOG)]
        catalog: String,
        #[arg(long, help = help::SCHEMA)]
        schema: String,
        #[command(flatten)]
        paging: Paging,
    },
    /// Get a function
    Get {
        #[arg(long, help = help::FULL_NAME)]
        full_name: String,
    },
    /// Delete a function
    Delete {
        #[arg(long, help = help::FULL_NAME)]
        full_name: String,
    },
    /// Call a Python function with python3
    Call {
        #[arg(long, help = help::FULL_NAME)]
        full_name: String,
        /// The arguments, separated by commas: "1,2"
        #[arg(long, allow_hyphen_values = true)]
        input_params: String,
    },
}

pub async fn run(c: &Context<'_>, op: FunctionOp) -> Result<String> {
    match op {
        FunctionOp::Create {
            full_name,
            input_params,
            data_type,
            comment,
            def,
            language,
        } => {
            let parameters = parse_input_params(&input_params)?;
            let function_info = CreateFunction {
                name: full_name.name.clone(),
                catalog_name: full_name.catalog,
                schema_name: full_name.schema,
                input_params: FunctionParameterInfos {
                    parameters,
                    ..Default::default()
                }
                .into(),
                data_type: Some(data_type.into()),
                full_data_type: Some(wire_name(&data_type)),
                routine_body: RoutineBody::External.into(),
                routine_definition: Some(def.unwrap_or_default()),
                parameter_style: ParameterStyle::S.into(),
                is_deterministic: Some(true),
                sql_data_access: Some(SqlDataAccess::NoSql.into()),
                is_null_call: Some(true),
                security_type: SecurityType::Definer.into(),
                specific_name: Some(full_name.name),
                comment,
                external_language: Some(language),
                ..Default::default()
            };
            let created = c
                .client
                .create_function(function_info)
                .await
                .map_err(upstream("createFunction"))?;
            Ok(c.shaped(&models::FUNCTION, &serde_json::to_value(&created)?))
        }
        FunctionOp::List {
            catalog,
            schema,
            paging,
        } => {
            let list = c
                .client
                .list_functions(catalog, schema)
                .with_max_results(paging.max_results)
                .with_page_token(paging.page_token)
                .await
                .map_err(upstream("listFunctions"))?;
            Ok(c.shaped_list(
                &models::FUNCTION,
                &serde_json::to_value(&list)?,
                "functions",
            ))
        }
        FunctionOp::Get { full_name } => {
            let info = c
                .client
                .function(full_name)
                .get()
                .await
                .map_err(upstream("getFunction"))?;
            Ok(c.shaped(&models::FUNCTION, &serde_json::to_value(&info)?))
        }
        FunctionOp::Delete { full_name } => {
            c.client
                .function(full_name)
                .delete()
                .await
                .map_err(upstream("deleteFunction"))?;
            Ok(String::new())
        }
        FunctionOp::Call {
            full_name,
            input_params,
        } => python::call_function(c, &full_name, &input_params).await,
    }
}
