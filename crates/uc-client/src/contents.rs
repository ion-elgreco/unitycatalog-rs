//! What a catalog, schema or registered model holds: what a delete with
//! `force` removes along with it, as the server cascades it. The web UI and
//! the TUI count it before they ask to delete it all.

use std::future::Future;

use futures::future::try_join_all;
use uc_common::{PageToken, three_part_full_name};

use crate::{Client, Page, Result};

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
        let schemas = all(|page| async move {
            let list = self.list_schemas(catalog, &page).await?;
            Ok((list.schemas, list.next_page_token))
        })
        .await?;
        let names: Vec<String> = schemas.into_iter().filter_map(|s| s.name).collect();
        let each = try_join_all(
            names
                .iter()
                .map(|schema| self.schema_contents(catalog, schema)),
        )
        .await?;
        Ok(each.into_iter().fold(
            Contents {
                schemas: names.len(),
                ..Contents::default()
            },
            Contents::plus,
        ))
    }

    /// What the schema `catalog.schema` holds, with the versions of its
    /// models.
    pub async fn schema_contents(&self, catalog: &str, schema: &str) -> Result<Contents> {
        let (tables, volumes, functions, models) = futures::try_join!(
            all(|page| async move {
                let list = self.list_tables(catalog, schema, &page).await?;
                Ok((list.tables, list.next_page_token))
            }),
            all(|page| async move {
                let list = self.list_volumes(catalog, schema, &page).await?;
                Ok((list.volumes, list.next_page_token))
            }),
            all(|page| async move {
                let list = self.list_functions(catalog, schema, &page).await?;
                Ok((list.functions, list.next_page_token))
            }),
            all(|page| async move {
                let list = self
                    .list_registered_models(Some(catalog), Some(schema), &page)
                    .await?;
                Ok((list.registered_models, list.next_page_token))
            }),
        )?;
        let versions = try_join_all(models.iter().filter_map(|model| {
            let full_name = three_part_full_name(catalog, schema, model.name.as_deref()?);
            Some(async move { self.model_contents(&full_name).await })
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
        let versions = all(|page| async move {
            let list = self.list_model_versions(full_name, &page).await?;
            Ok((list.model_versions, list.next_page_token))
        })
        .await?;
        Ok(Contents {
            versions: versions.len(),
            ..Contents::default()
        })
    }
}

/// Every item of a list: `page` gets one page, and the token of the next.
async fn all<T, F>(mut page: impl FnMut(Page) -> F) -> Result<Vec<T>>
where
    F: Future<Output = Result<(Vec<T>, Option<PageToken>)>>,
{
    let mut items = Vec::new();
    let mut next = Page::default();
    loop {
        let (batch, token) = page(next.clone()).await?;
        items.extend(batch);
        match token {
            Some(PageToken(token)) if !token.is_empty() => next.page_token = Some(token),
            _ => return Ok(items),
        }
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
