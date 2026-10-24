//! Functions.

use futures::TryStreamExt;

use uc_client::models::functions::v1::{
    CreateFunction, FunctionParameterInfo, FunctionParameterInfos, FunctionParameterMode,
    FunctionParameterType, ParameterStyle, RoutineBody, SecurityType, SqlDataAccess,
};
use uc_client::models::tables::v1::{ColumnTypeName, Dependency, DependencyList, TableDependency};

use crate::common::{catalog_and_schema, client, external_table, rejected};

/// A parameter `name` of type `long` at `position`.
fn long_parameter(name: &str, position: i32) -> FunctionParameterInfo {
    FunctionParameterInfo {
        name: name.into(),
        type_text: "long".into(),
        type_json: Some(r#""long""#.into()),
        type_name: ColumnTypeName::Long.into(),
        position: Some(position),
        ..Default::default()
    }
}

#[tokio::test]
async fn sql_functions() {
    let uc = client().await;
    let storage = catalog_and_schema(&uc).await;
    external_table(&uc, &storage, "t").await;
    let parameters = FunctionParameterInfos {
        parameters: vec![FunctionParameterInfo {
            parameter_mode: Some(FunctionParameterMode::In.into()),
            parameter_type: Some(FunctionParameterType::Param.into()),
            parameter_default: Some("0".into()),
            comment: Some("the number".into()),
            ..long_parameter("x", 0)
        }],
        ..Default::default()
    };
    let dependencies = DependencyList {
        dependencies: vec![Dependency {
            table: TableDependency {
                table_full_name: "c.s.t".into(),
                ..Default::default()
            }
            .into(),
            ..Default::default()
        }],
        ..Default::default()
    };
    let created = uc
        .create_function(CreateFunction {
            name: "inc".into(),
            catalog_name: "c".into(),
            schema_name: "s".into(),
            input_params: parameters.clone().into(),
            data_type: Some(ColumnTypeName::Long.into()),
            full_data_type: Some("long".into()),
            routine_body: RoutineBody::Sql.into(),
            routine_definition: Some("x + 1".into()),
            routine_dependencies: dependencies.clone().into(),
            parameter_style: ParameterStyle::S.into(),
            is_deterministic: Some(true),
            sql_data_access: Some(SqlDataAccess::ReadsSqlData.into()),
            is_null_call: Some(false),
            security_type: SecurityType::Definer.into(),
            specific_name: Some("inc".into()),
            comment: Some("adds one".into()),
            properties: Some(r#"{"team":"data"}"#.into()),
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(created.full_name, "c.s.inc");
    assert_eq!(created.input_params.as_option(), Some(&parameters));
    assert_eq!(
        created.routine_dependencies.as_option(),
        Some(&dependencies)
    );
    assert_eq!(created.data_type, Some(ColumnTypeName::Long.into()));
    assert_eq!(created.full_data_type.as_deref(), Some("long"));
    assert_eq!(created.routine_body, RoutineBody::Sql);
    assert_eq!(created.routine_definition.as_deref(), Some("x + 1"));
    assert_eq!(created.parameter_style, ParameterStyle::S);
    assert_eq!(created.security_type, SecurityType::Definer);
    assert_eq!(
        created.sql_data_access,
        Some(SqlDataAccess::ReadsSqlData.into())
    );
    assert_eq!(
        (created.is_deterministic, created.is_null_call),
        (Some(true), Some(false))
    );
    assert_eq!(created.specific_name.as_deref(), Some("inc"));
    assert_eq!(created.comment.as_deref(), Some("adds one"));
    assert_eq!(created.properties.as_deref(), Some(r#"{"team":"data"}"#));
    assert!(!created.function_id.is_empty());
    assert_eq!(uc.function("c.s.inc").get().await.unwrap(), created);

    uc.function("c.s.inc").delete().await.unwrap();
    rejected(uc.function("c.s.inc").get().await, 404, "NOT_FOUND");
}

/// A table function in Python: its result columns are `return_params`.
#[tokio::test]
async fn external_table_functions() {
    let uc = client().await;
    let _storage = catalog_and_schema(&uc).await;
    let columns = FunctionParameterInfos {
        parameters: vec![FunctionParameterInfo {
            parameter_type: Some(FunctionParameterType::Column.into()),
            ..long_parameter("n", 0)
        }],
        ..Default::default()
    };
    let created = uc
        .create_function(CreateFunction {
            name: "numbers".into(),
            catalog_name: "c".into(),
            schema_name: "s".into(),
            data_type: Some(ColumnTypeName::TableType.into()),
            full_data_type: Some("table<n: long>".into()),
            return_params: columns.clone().into(),
            routine_body: RoutineBody::External.into(),
            routine_definition: Some("return [(1,), (2,)]".into()),
            external_language: Some("python".into()),
            parameter_style: ParameterStyle::S.into(),
            sql_data_access: Some(SqlDataAccess::NoSql.into()),
            security_type: SecurityType::Definer.into(),
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(created.return_params.as_option(), Some(&columns));
    assert_eq!(created.routine_body, RoutineBody::External);
    assert_eq!(created.external_language.as_deref(), Some("python"));
    assert!(created.input_params.is_unset());

    uc.create_function(CreateFunction {
        name: "other".into(),
        catalog_name: "c".into(),
        schema_name: "s".into(),
        routine_body: RoutineBody::Sql.into(),
        routine_definition: Some("1".into()),
        parameter_style: ParameterStyle::S.into(),
        security_type: SecurityType::Definer.into(),
        ..Default::default()
    })
    .await
    .unwrap();
    let first = uc
        .list_functions("c", "s")
        .with_max_results(1)
        .await
        .unwrap();
    assert_eq!(first.functions, [created]);
    let second = uc
        .list_functions("c", "s")
        .with_max_results(1)
        .with_page_token(first.next_page_token)
        .await
        .unwrap();
    assert_eq!(second.functions[0].name, "other");
    let all: Vec<_> = uc
        .list_functions("c", "s")
        .into_stream()
        .try_collect()
        .await
        .unwrap();
    assert_eq!(all.len(), 2);
}
