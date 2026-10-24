//! What a catalog, schema or registered model holds: what a delete with
//! `force` removes along with it, as the server cascades it. The web UI and
//! the TUI count it before they ask to delete it all.

use futures::TryStreamExt;
use futures::future::try_join_all;
use uc_common::three_part_full_name;

use crate::Client;
use crate::error::Result;

/// How many objects of each kind an object holds.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Contents {
    pub schemas: usize,
    pub tables: usize,
    pub volumes: usize,
    pub functions: usize,
    pub models: usize,
    pub versions: usize,
}

impl Contents {
    fn plus(self, other: Self) -> Self {
        Self {
            schemas: self.schemas + other.schemas,
            tables: self.tables + other.tables,
            volumes: self.volumes + other.volumes,
            functions: self.functions + other.functions,
            models: self.models + other.models,
            versions: self.versions + other.versions,
        }
    }

    /// Such as "1 schema, 2 tables and 1 registered model"; none when it
    /// holds nothing.
    pub fn describe(&self) -> Option<String> {
        let parts: Vec<String> = [
            (self.schemas, "schema", "schemas"),
            (self.tables, "table", "tables"),
            (self.volumes, "volume", "volumes"),
            (self.functions, "function", "functions"),
            (self.models, "registered model", "registered models"),
            (self.versions, "model version", "model versions"),
        ]
        .into_iter()
        .filter(|(count, ..)| *count > 0)
        .map(|(count, one, many)| format!("{count} {}", if count == 1 { one } else { many }))
        .collect();
        match parts.as_slice() {
            [] => None,
            [only] => Some(only.clone()),
            [first @ .., last] => Some(format!("{} and {last}", first.join(", "))),
        }
    }
}

impl Client {
    /// What the catalog `catalog` holds: its schemas, and what they hold.
    pub async fn catalog_contents(&self, catalog: &str) -> Result<Contents> {
        let schemas: Vec<_> = self
            .list_schemas(catalog)
            .into_stream()
            .try_collect()
            .await?;
        let each = try_join_all(
            schemas
                .iter()
                .map(|schema| self.schema_contents(catalog, &schema.name)),
        )
        .await?;
        Ok(each.into_iter().fold(
            Contents {
                schemas: schemas.len(),
                ..Contents::default()
            },
            Contents::plus,
        ))
    }

    /// What the schema `catalog.schema` holds, with the versions of its
    /// models.
    pub async fn schema_contents(&self, catalog: &str, schema: &str) -> Result<Contents> {
        let (tables, volumes, functions, models) = futures::try_join!(
            self.list_tables(catalog, schema)
                .into_stream()
                .try_collect::<Vec<_>>(),
            self.list_volumes(catalog, schema)
                .into_stream()
                .try_collect::<Vec<_>>(),
            self.list_functions(catalog, schema)
                .into_stream()
                .try_collect::<Vec<_>>(),
            self.list_registered_models()
                .with_catalog_name(catalog.to_string())
                .with_schema_name(schema.to_string())
                .into_stream()
                .try_collect::<Vec<_>>(),
        )?;
        let versions = try_join_all(models.iter().map(|model| {
            let full_name = three_part_full_name(catalog, schema, &model.name);
            async move { self.model_contents(&full_name).await }
        }))
        .await?;
        Ok(versions.into_iter().fold(
            Contents {
                tables: tables.len(),
                volumes: volumes.len(),
                functions: functions.len(),
                models: models.len(),
                ..Contents::default()
            },
            Contents::plus,
        ))
    }

    /// What the registered model `full_name` holds: its versions.
    pub async fn model_contents(&self, full_name: &str) -> Result<Contents> {
        let versions: Vec<_> = self
            .list_model_versions(full_name)
            .into_stream()
            .try_collect()
            .await?;
        Ok(Contents {
            versions: versions.len(),
            ..Contents::default()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::Contents;

    #[test]
    fn describes_the_counts() {
        assert_eq!(Contents::default().describe(), None);
        let one = Contents {
            versions: 1,
            ..Contents::default()
        };
        assert_eq!(one.describe().unwrap(), "1 model version");
        let many = Contents {
            schemas: 2,
            tables: 2,
            volumes: 1,
            models: 2,
            versions: 1,
            ..Contents::default()
        };
        assert_eq!(
            many.describe().unwrap(),
            "2 schemas, 2 tables, 1 volume, 2 registered models and 1 model version"
        );
    }
}
