use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use uc_common::{EpochMillis, PageToken};

use crate::{ColumnTypeName, DependencyList};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "UPPERCASE")]
pub enum FunctionParameterType {
    Param,
    Column,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "UPPERCASE")]
pub enum FunctionParameterMode {
    In,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "UPPERCASE")]
pub enum RoutineBody {
    Sql,
    External,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SqlDataAccess {
    ContainsSql,
    ReadsSqlData,
    NoSql,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "UPPERCASE")]
pub enum SecurityType {
    Definer,
}

/// Upstream uses the single-character enum `"S"` for SQL parameter style.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub enum ParameterStyle {
    S,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct FunctionParameterInfo {
    pub name: String,
    pub type_text: String,
    /// Required by the spec, but upstream accepts it absent.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub type_json: Option<String>,
    pub type_name: ColumnTypeName,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub type_precision: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub type_scale: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub type_interval_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameter_mode: Option<FunctionParameterMode>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameter_type: Option<FunctionParameterType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameter_default: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct FunctionParameterInfos {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameters: Option<Vec<FunctionParameterInfo>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
/// The spec marks most fields required, but upstream stores whatever is
/// sent, so they are optional here. Without `routine_body`,
/// `parameter_style` or `security_type` upstream fails with a 500; here the
/// request is rejected.
pub struct CreateFunction {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub catalog_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schema_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_params: Option<FunctionParameterInfos>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data_type: Option<ColumnTypeName>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub full_data_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub return_params: Option<FunctionParameterInfos>,
    pub routine_body: RoutineBody,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub routine_definition: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub routine_dependencies: Option<DependencyList>,
    pub parameter_style: ParameterStyle,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_deterministic: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sql_data_access: Option<SqlDataAccess>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_null_call: Option<bool>,
    pub security_type: SecurityType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub specific_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
    /// JSON-encoded key/value map (string-of-JSON per upstream spec).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub properties: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external_language: Option<String>,
}

/// Upstream wraps the create payload in `{"function_info": {...}}`.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateFunctionRequest {
    pub function_info: CreateFunction,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct FunctionInfo {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub catalog_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schema_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_params: Option<FunctionParameterInfos>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data_type: Option<ColumnTypeName>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub full_data_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub return_params: Option<FunctionParameterInfos>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub routine_body: Option<RoutineBody>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub routine_definition: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub routine_dependencies: Option<DependencyList>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameter_style: Option<ParameterStyle>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_deterministic: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sql_data_access: Option<SqlDataAccess>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_null_call: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub security_type: Option<SecurityType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub specific_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub properties: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub full_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<EpochMillis>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_by: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<EpochMillis>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_by: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub function_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external_language: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct ListFunctionsResponse {
    pub functions: Vec<FunctionInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_page_token: Option<PageToken>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::assert_wire_names;
    use serde_json::json;

    #[test]
    fn enums_match_upstream() {
        assert_wire_names(&[
            (FunctionParameterType::Param, "PARAM"),
            (FunctionParameterType::Column, "COLUMN"),
        ]);
        assert_wire_names(&[(FunctionParameterMode::In, "IN")]);
        assert_wire_names(&[
            (RoutineBody::Sql, "SQL"),
            (RoutineBody::External, "EXTERNAL"),
        ]);
        assert_wire_names(&[
            (SqlDataAccess::ContainsSql, "CONTAINS_SQL"),
            (SqlDataAccess::ReadsSqlData, "READS_SQL_DATA"),
            (SqlDataAccess::NoSql, "NO_SQL"),
        ]);
        assert_wire_names(&[(SecurityType::Definer, "DEFINER")]);
        assert_wire_names(&[(ParameterStyle::S, "S")]);
    }

    #[test]
    fn create_needs_only_body_style_and_security() {
        let info =
            json!({ "routine_body": "SQL", "parameter_style": "S", "security_type": "DEFINER" });
        let create: CreateFunctionRequest =
            serde_json::from_value(json!({ "function_info": info })).unwrap();
        assert!(create.function_info.name.is_none());
        for field in ["routine_body", "parameter_style", "security_type"] {
            let mut info = info.clone();
            info.as_object_mut().unwrap().remove(field);
            let err =
                serde_json::from_value::<CreateFunctionRequest>(json!({ "function_info": info }))
                    .unwrap_err();
            assert_eq!(err.to_string(), format!("missing field `{field}`"));
        }
    }

    #[test]
    fn parameters_may_leave_out_type_json() {
        let param: FunctionParameterInfo = serde_json::from_value(json!({
            "name": "x", "type_text": "int", "type_name": "INT",
        }))
        .unwrap();
        assert!(param.type_json.is_none());
    }
}
