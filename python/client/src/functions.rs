//! `unitycatalog._core.functions`.

use pyo3::prelude::*;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};
use uc_api::{
    ColumnTypeName, FunctionParameterMode, FunctionParameterType, ParameterStyle, RoutineBody,
    SecurityType, SqlDataAccess,
};

use crate::str_enum::StrEnum;
use crate::tables::{Dependency, dependencies_from_api};
use crate::utils::repr;

/// A parameter of a function, or a column of the table that it returns.
#[gen_stub_pyclass]
#[pyclass(
    frozen,
    get_all,
    eq,
    from_py_object,
    module = "unitycatalog._core.functions"
)]
#[derive(Clone, PartialEq)]
pub struct FunctionParameterInfo {
    /// The name of the parameter.
    pub name: String,
    /// The type as SQL text, such as `"bigint"`.
    pub type_text: String,
    /// The name of the type.
    pub type_name: StrEnum<ColumnTypeName>,
    /// The type as JSON, as Spark writes it.
    pub type_json: Option<String>,
    /// The precision of a `DECIMAL` type.
    pub type_precision: Option<i32>,
    /// The scale of a `DECIMAL` type.
    pub type_scale: Option<i32>,
    /// The format of an `INTERVAL` type.
    pub type_interval_type: Option<String>,
    /// The position of the parameter, from 0.
    pub position: Option<i32>,
    /// The mode of the parameter.
    pub parameter_mode: Option<StrEnum<FunctionParameterMode>>,
    /// A parameter, or a column of the returned table.
    pub parameter_type: Option<StrEnum<FunctionParameterType>>,
    /// The default value, as SQL text.
    pub parameter_default: Option<String>,
    /// A description.
    pub comment: Option<String>,
}

#[gen_stub_pymethods]
#[pymethods]
impl FunctionParameterInfo {
    #[new]
    #[pyo3(signature = (
        name, type_text, type_name, *, type_json=None, type_precision=None, type_scale=None,
        type_interval_type=None, position=None, parameter_mode=None, parameter_type=None,
        parameter_default=None, comment=None
    ))]
    fn new(
        name: String,
        type_text: String,
        type_name: StrEnum<ColumnTypeName>,
        type_json: Option<String>,
        type_precision: Option<i32>,
        type_scale: Option<i32>,
        type_interval_type: Option<String>,
        position: Option<i32>,
        parameter_mode: Option<StrEnum<FunctionParameterMode>>,
        parameter_type: Option<StrEnum<FunctionParameterType>>,
        parameter_default: Option<String>,
        comment: Option<String>,
    ) -> Self {
        Self {
            name,
            type_text,
            type_name,
            type_json,
            type_precision,
            type_scale,
            type_interval_type,
            position,
            parameter_mode,
            parameter_type,
            parameter_default,
            comment,
        }
    }

    fn __repr__(slf: &Bound<'_, Self>) -> PyResult<String> {
        let fields = [
            "name",
            "type_text",
            "type_name",
            "type_json",
            "type_precision",
            "type_scale",
            "type_interval_type",
            "position",
            "parameter_mode",
            "parameter_type",
            "parameter_default",
            "comment",
        ];
        repr(slf.as_any(), &fields)
    }
}

impl From<uc_api::FunctionParameterInfo> for FunctionParameterInfo {
    fn from(p: uc_api::FunctionParameterInfo) -> Self {
        Self {
            name: p.name,
            type_text: p.type_text,
            type_name: StrEnum(p.type_name),
            type_json: p.type_json,
            type_precision: p.type_precision,
            type_scale: p.type_scale,
            type_interval_type: p.type_interval_type,
            position: p.position,
            parameter_mode: p.parameter_mode.map(StrEnum),
            parameter_type: p.parameter_type.map(StrEnum),
            parameter_default: p.parameter_default,
            comment: p.comment,
        }
    }
}

impl From<FunctionParameterInfo> for uc_api::FunctionParameterInfo {
    fn from(p: FunctionParameterInfo) -> Self {
        Self {
            name: p.name,
            type_text: p.type_text,
            type_name: p.type_name.0,
            type_json: p.type_json,
            type_precision: p.type_precision,
            type_scale: p.type_scale,
            type_interval_type: p.type_interval_type,
            position: p.position,
            parameter_mode: p.parameter_mode.map(|m| m.0),
            parameter_type: p.parameter_type.map(|t| t.0),
            parameter_default: p.parameter_default,
            comment: p.comment,
        }
    }
}

/// The parameters in the API's `{"parameters": [...]}`.
pub fn parameters_from_api(list: uc_api::FunctionParameterInfos) -> Vec<FunctionParameterInfo> {
    let parameters = list.parameters.unwrap_or_default();
    parameters
        .into_iter()
        .map(FunctionParameterInfo::from)
        .collect()
}

/// `parameters` as the API's `{"parameters": [...]}`.
pub fn parameters_to_api(parameters: Vec<FunctionParameterInfo>) -> uc_api::FunctionParameterInfos {
    uc_api::FunctionParameterInfos {
        parameters: Some(parameters.into_iter().map(Into::into).collect()),
    }
}

/// A function in a schema.
#[gen_stub_pyclass]
#[pyclass(
    frozen,
    get_all,
    eq,
    skip_from_py_object,
    module = "unitycatalog._core.functions"
)]
#[derive(Clone, PartialEq)]
pub struct FunctionInfo {
    /// The name of the function, in its schema.
    pub name: Option<String>,
    /// The name of the catalog.
    pub catalog_name: Option<String>,
    /// The name of the schema.
    pub schema_name: Option<String>,
    /// The parameters.
    pub input_params: Option<Vec<FunctionParameterInfo>>,
    /// The type of the result.
    pub data_type: Option<StrEnum<ColumnTypeName>>,
    /// The type of the result as SQL text.
    pub full_data_type: Option<String>,
    /// The columns of the returned table, for a table function.
    pub return_params: Option<Vec<FunctionParameterInfo>>,
    /// The language of the body: SQL, or `external_language`.
    pub routine_body: Option<StrEnum<RoutineBody>>,
    /// The body.
    pub routine_definition: Option<String>,
    /// The tables and functions that the body uses.
    pub routine_dependencies: Option<Vec<Dependency>>,
    /// The parameter style.
    pub parameter_style: Option<StrEnum<ParameterStyle>>,
    /// Whether the function gives the same result for the same arguments.
    pub is_deterministic: Option<bool>,
    /// How the function accesses SQL data.
    pub sql_data_access: Option<StrEnum<SqlDataAccess>>,
    /// Whether the function returns null when an argument is null.
    pub is_null_call: Option<bool>,
    /// Whose rights the function runs with.
    pub security_type: Option<StrEnum<SecurityType>>,
    /// The specific name of the function.
    pub specific_name: Option<String>,
    /// A description.
    pub comment: Option<String>,
    /// Key-value properties, as a JSON object in a string.
    pub properties: Option<String>,
    /// The full name: `catalog.schema.function`.
    pub full_name: Option<String>,
    /// The username of the owner.
    pub owner: Option<String>,
    /// The creation time, in epoch milliseconds.
    pub created_at: Option<i64>,
    /// The username of the creator.
    pub created_by: Option<String>,
    /// The time of the last update, in epoch milliseconds.
    pub updated_at: Option<i64>,
    /// The username of the last updater.
    pub updated_by: Option<String>,
    /// The unique identifier.
    pub function_id: Option<String>,
    /// The language of an `EXTERNAL` body, such as `"python"`.
    pub external_language: Option<String>,
}

#[gen_stub_pymethods]
#[pymethods]
impl FunctionInfo {
    fn __repr__(slf: &Bound<'_, Self>) -> PyResult<String> {
        let fields = [
            "name",
            "catalog_name",
            "schema_name",
            "input_params",
            "data_type",
            "full_data_type",
            "return_params",
            "routine_body",
            "routine_definition",
            "routine_dependencies",
            "parameter_style",
            "is_deterministic",
            "sql_data_access",
            "is_null_call",
            "security_type",
            "specific_name",
            "comment",
            "properties",
            "full_name",
            "owner",
            "created_at",
            "created_by",
            "updated_at",
            "updated_by",
            "function_id",
            "external_language",
        ];
        repr(slf.as_any(), &fields)
    }
}

impl From<uc_api::FunctionInfo> for FunctionInfo {
    fn from(f: uc_api::FunctionInfo) -> Self {
        Self {
            name: f.name,
            catalog_name: f.catalog_name,
            schema_name: f.schema_name,
            input_params: f.input_params.map(parameters_from_api),
            data_type: f.data_type.map(StrEnum),
            full_data_type: f.full_data_type,
            return_params: f.return_params.map(parameters_from_api),
            routine_body: f.routine_body.map(StrEnum),
            routine_definition: f.routine_definition,
            routine_dependencies: f.routine_dependencies.map(dependencies_from_api),
            parameter_style: f.parameter_style.map(StrEnum),
            is_deterministic: f.is_deterministic,
            sql_data_access: f.sql_data_access.map(StrEnum),
            is_null_call: f.is_null_call,
            security_type: f.security_type.map(StrEnum),
            specific_name: f.specific_name,
            comment: f.comment,
            properties: f.properties,
            full_name: f.full_name,
            owner: f.owner,
            created_at: f.created_at,
            created_by: f.created_by,
            updated_at: f.updated_at,
            updated_by: f.updated_by,
            function_id: f.function_id,
            external_language: f.external_language,
        }
    }
}

pyo3_stub_gen::module_doc!(
    "unitycatalog._core.functions",
    "The classes of the function methods."
);

pub fn register_module(parent: &Bound<'_, PyModule>) -> PyResult<()> {
    register_submodule!(parent, "functions", [FunctionInfo, FunctionParameterInfo])
}
