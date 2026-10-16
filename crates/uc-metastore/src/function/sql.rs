use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use uc_api::{
    ColumnTypeName, CreateFunction, DependencyList, FunctionInfo, FunctionParameterInfos,
    ParameterStyle, RoutineBody, SecurityType, SqlDataAccess,
};
use uc_common::{Error, ErrorCode, Pagination, Result, now_epoch_millis, validate_sql_object_name};

use crate::FunctionOps;
use crate::sql::rows::FUNC;
use crate::sql::{
    SchemaRef, SqlMetastore, create_err_as, db_err, enum_str, insert, parse_enum, select_one,
    split_three_part,
};

/// A function row (SQL table `func`), keyed by its UUID and linked to its
/// schema by id.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct FunctionRow {
    pub(crate) id_string: String,
    pub(crate) name: String,
    pub(crate) schema_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) input_params: Option<FunctionParameterInfos>,
    /// Stored as the JSON enum repr (e.g. `"LONG"`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) data_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) full_data_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) return_params: Option<FunctionParameterInfos>,
    pub(crate) routine_body: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) routine_definition: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) routine_dependencies: Option<DependencyList>,
    pub(crate) parameter_style: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) is_deterministic: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) sql_data_access: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) is_null_call: Option<bool>,
    pub(crate) security_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) specific_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) comment: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) properties: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) external_language: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) owner: Option<String>,
    pub(crate) created_at: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) created_by: Option<String>,
    pub(crate) updated_at: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) updated_by: Option<String>,
}

fn non_empty(params: Option<FunctionParameterInfos>) -> Option<FunctionParameterInfos> {
    params.filter(|p| p.parameters.as_ref().is_some_and(|v| !v.is_empty()))
}

impl FunctionRow {
    fn into_info(self, schema: &SchemaRef) -> Result<FunctionInfo> {
        Ok(FunctionInfo {
            catalog_name: Some(schema.catalog_name.clone()),
            schema_name: Some(schema.schema_name.clone()),
            full_name: Some(schema.child_full_name(&self.name)),
            name: Some(self.name),
            // Upstream leaves out empty parameter lists.
            input_params: non_empty(self.input_params),
            data_type: self
                .data_type
                .as_deref()
                .map(parse_enum::<ColumnTypeName>)
                .transpose()?,
            full_data_type: self.full_data_type,
            return_params: non_empty(self.return_params),
            routine_body: Some(parse_enum::<RoutineBody>(&self.routine_body)?),
            routine_definition: self.routine_definition,
            routine_dependencies: self.routine_dependencies,
            parameter_style: Some(parse_enum::<ParameterStyle>(&self.parameter_style)?),
            is_deterministic: self.is_deterministic,
            sql_data_access: self
                .sql_data_access
                .as_deref()
                .map(parse_enum::<SqlDataAccess>)
                .transpose()?,
            is_null_call: self.is_null_call,
            security_type: Some(parse_enum::<SecurityType>(&self.security_type)?),
            specific_name: self.specific_name,
            comment: self.comment,
            properties: self.properties,
            owner: self.owner,
            created_at: Some(self.created_at),
            created_by: self.created_by,
            updated_at: Some(self.updated_at),
            updated_by: self.updated_by,
            function_id: Some(self.id_string),
            external_language: self.external_language,
        })
    }
}

impl SqlMetastore {
    pub(crate) async fn function_row(
        &self,
        schema_id: &str,
        name: &str,
    ) -> Result<Option<FunctionRow>> {
        select_one(
            self.db.as_ref(),
            &FUNC,
            "WHERE schema_id = $1 AND name = $2",
            &[schema_id.into(), name.into()],
        )
        .await
    }

    pub(crate) async fn resolve_function(
        &self,
        full_name: &str,
    ) -> Result<(SchemaRef, FunctionRow)> {
        let (c, s, f) = split_three_part(full_name, "function")?;
        let schema = self.resolve_schema(c, s).await?;
        let row = self
            .function_row(&schema.schema_id, f)
            .await?
            .ok_or_else(|| Error::not_found(format!("Function not found: {full_name}")))?;
        Ok((schema, row))
    }
}

#[async_trait]
impl FunctionOps for SqlMetastore {
    async fn create(&self, req: CreateFunction, created_by: Option<&str>) -> Result<FunctionInfo> {
        let name = req.name.clone().unwrap_or_default();
        validate_sql_object_name(&name)?;
        let catalog_name = req
            .catalog_name
            .as_deref()
            .ok_or_else(|| Error::catalog_not_found("Catalog not found: null"))?;
        let schema_name = req.schema_name.as_deref().ok_or_else(|| {
            Error::schema_not_found(format!("Schema not found: {catalog_name}.null"))
        })?;
        let schema = self.resolve_schema(catalog_name, schema_name).await?;
        let exists = || {
            Error::new(
                ErrorCode::ResourceAlreadyExists,
                format!("Function already exists: {name}"),
            )
        };
        if self.function_row(&schema.schema_id, &name).await?.is_some() {
            return Err(exists());
        }

        let now = now_epoch_millis();
        let id = Uuid::new_v4().to_string();
        let row = FunctionRow {
            id_string: id,
            name: name.clone(),
            schema_id: schema.schema_id.clone(),
            input_params: req.input_params.clone(),
            data_type: req.data_type.as_ref().map(enum_str),
            full_data_type: req.full_data_type.clone(),
            return_params: req.return_params.clone(),
            routine_body: enum_str(&req.routine_body),
            routine_definition: req.routine_definition.clone(),
            routine_dependencies: req.routine_dependencies.clone(),
            parameter_style: enum_str(&req.parameter_style),
            is_deterministic: req.is_deterministic,
            sql_data_access: req.sql_data_access.as_ref().map(enum_str),
            is_null_call: req.is_null_call,
            security_type: enum_str(&req.security_type),
            specific_name: req.specific_name.clone(),
            comment: req.comment.clone(),
            properties: req.properties.clone(),
            external_language: req.external_language.clone(),
            owner: created_by.map(str::to_string),
            created_at: now,
            created_by: created_by.map(str::to_string),
            updated_at: now,
            updated_by: created_by.map(str::to_string),
        };

        insert(self.db.as_ref(), &FUNC, &row)
            .await
            .map_err(create_err_as(
                ErrorCode::ResourceAlreadyExists,
                format!("Function already exists: {name}"),
            ))?;
        let mut info = row.into_info(&schema)?;
        // Upstream answers a create with the parameters as sent.
        info.input_params = req.input_params;
        info.return_params = req.return_params;
        Ok(info)
    }

    async fn get(&self, full_name: &str) -> Result<FunctionInfo> {
        let (schema, row) = self.resolve_function(full_name).await?;
        row.into_info(&schema)
    }

    async fn delete(&self, full_name: &str) -> Result<()> {
        let (_, row) = self.resolve_function(full_name).await?;
        let failed = db_err("delete function");
        let tx = self.db.begin().await.map_err(&failed)?;
        let id = [row.id_string.as_str().into()];
        for sql in [
            "DELETE FROM permission WHERE resource_id = $1",
            "DELETE FROM func WHERE id_string = $1",
        ] {
            tx.execute(sql, &id).await.map_err(&failed)?;
        }
        tx.commit().await.map_err(&failed)
    }

    async fn list(
        &self,
        catalog_name: &str,
        schema_name: &str,
        page: &Pagination,
    ) -> Result<(Vec<FunctionInfo>, Option<String>)> {
        let schema = self.resolve_schema(catalog_name, schema_name).await?;
        let (rows, next) = self
            .paged_list::<FunctionRow, _>(
                &FUNC,
                &["schema_id = $1"],
                vec![schema.schema_id.as_str().into()],
                "name",
                page,
                |r| r.name.clone(),
            )
            .await?;
        let infos: Result<Vec<FunctionInfo>> =
            rows.into_iter().map(|r| r.into_info(&schema)).collect();
        Ok((infos?, next))
    }
}
