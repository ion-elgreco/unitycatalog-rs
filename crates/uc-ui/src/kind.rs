//! What a schema holds: tables, volumes, functions and registered models.
//! They are the tabs of the schema page and the rows under a schema in the
//! sidebar.

use leptos::prelude::*;
use uc_common::EpochMillis;

use crate::api;
use crate::components::icons::{
    DeploymentUnitOutlined, FolderOutlined, FunctionOutlined, TableOutlined,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Kind {
    Tables,
    Volumes,
    Functions,
    Models,
}

/// An object in a schema, as the lists show it.
#[derive(Clone, Debug)]
pub struct Entry {
    pub name: String,
    pub created_at: Option<EpochMillis>,
}

impl Kind {
    pub const ALL: [Kind; 4] = [Kind::Tables, Kind::Volumes, Kind::Functions, Kind::Models];

    /// Its value of `?tab=` on the schema page.
    pub fn tab(self) -> &'static str {
        match self {
            Kind::Tables => "tables",
            Kind::Volumes => "volumes",
            Kind::Functions => "functions",
            Kind::Models => "models",
        }
    }

    pub fn from_tab(tab: &str) -> Option<Kind> {
        Kind::ALL.into_iter().find(|kind| kind.tab() == tab)
    }

    pub fn label(self) -> &'static str {
        match self {
            Kind::Tables => "Tables",
            Kind::Volumes => "Volumes",
            Kind::Functions => "Functions",
            Kind::Models => "Models",
        }
    }

    pub fn icon(self) -> AnyView {
        match self {
            Kind::Tables => view! { <TableOutlined/> }.into_any(),
            Kind::Volumes => view! { <FolderOutlined/> }.into_any(),
            Kind::Functions => view! { <FunctionOutlined/> }.into_any(),
            Kind::Models => view! { <DeploymentUnitOutlined/> }.into_any(),
        }
    }

    /// The schema page, on this kind's tab.
    pub fn tab_href(self, catalog: &str, schema: &str) -> String {
        format!("/data/{catalog}/{schema}?tab={}", self.tab())
    }

    /// The page of the object `name` in `catalog.schema`.
    pub fn href(self, catalog: &str, schema: &str, name: &str) -> String {
        match self {
            Kind::Tables => format!("/data/{catalog}/{schema}/{name}"),
            Kind::Volumes => format!("/volumes/{catalog}/{schema}/{name}"),
            Kind::Functions => format!("/functions/{catalog}/{schema}/{name}"),
            Kind::Models => format!("/models/{catalog}/{schema}/{name}"),
        }
    }

    /// The objects of this kind in `catalog.schema`.
    pub async fn entries(self, catalog: &str, schema: &str) -> api::Result<Vec<Entry>> {
        let entries = match self {
            Kind::Tables => api::tables(catalog, schema)
                .await?
                .into_iter()
                .map(|t| entry(t.name, t.created_at))
                .collect(),
            Kind::Volumes => api::volumes(catalog, schema)
                .await?
                .into_iter()
                .map(|v| entry(v.name, v.created_at))
                .collect(),
            Kind::Functions => api::functions(catalog, schema)
                .await?
                .into_iter()
                .map(|f| entry(f.name, f.created_at))
                .collect(),
            Kind::Models => api::models(catalog, schema)
                .await?
                .into_iter()
                .map(|m| entry(m.name, m.created_at))
                .collect(),
        };
        Ok(entries)
    }
}

fn entry(name: Option<String>, created_at: Option<EpochMillis>) -> Entry {
    Entry {
        name: name.unwrap_or_default(),
        created_at,
    }
}
