//! What a schema holds: tables, volumes, functions and registered models.
//! They are the groups under a schema in the tree and the picker on the
//! schema page.

use leptos::prelude::*;
use uc_common::{EpochMillis, SecurableType};

use crate::api;
use crate::components::icons::{Box, FunctionSquare, HardDrive, Table};

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
    pub comment: Option<String>,
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

    /// One object, such as "table".
    pub fn noun(self) -> &'static str {
        match self {
            Kind::Tables => "table",
            Kind::Volumes => "volume",
            Kind::Functions => "function",
            Kind::Models => "model",
        }
    }

    /// The part of an object's URL that names its kind.
    fn segment(self) -> &'static str {
        match self {
            Kind::Tables => "table",
            Kind::Volumes => "volume",
            Kind::Functions => "function",
            Kind::Models => "model",
        }
    }

    pub fn securable(self) -> SecurableType {
        match self {
            Kind::Tables => SecurableType::Table,
            Kind::Volumes => SecurableType::Volume,
            Kind::Functions => SecurableType::Function,
            Kind::Models => SecurableType::RegisteredModel,
        }
    }

    /// The class that colors the kind's icon.
    pub fn color(self) -> &'static str {
        match self {
            Kind::Tables => "uc-icon--table",
            Kind::Volumes => "uc-icon--volume",
            Kind::Functions => "uc-icon--function",
            Kind::Models => "uc-icon--model",
        }
    }

    pub fn icon(self) -> AnyView {
        let class = self.color();
        match self {
            Kind::Tables => view! { <Table class/> }.into_any(),
            Kind::Volumes => view! { <HardDrive class/> }.into_any(),
            Kind::Functions => view! { <FunctionSquare class/> }.into_any(),
            Kind::Models => view! { <Box class/> }.into_any(),
        }
    }

    /// The schema page, on this kind.
    pub fn tab_href(self, catalog: &str, schema: &str) -> String {
        format!("/catalog/{catalog}/{schema}?tab={}", self.tab())
    }

    /// The page of the object `name` in `catalog.schema`.
    pub fn href(self, catalog: &str, schema: &str, name: &str) -> String {
        format!("/catalog/{catalog}/{schema}/{}/{name}", self.segment())
    }

    /// The objects of this kind in `catalog.schema`.
    pub async fn entries(self, catalog: &str, schema: &str) -> api::Result<Vec<Entry>> {
        let entries = match self {
            Kind::Tables => api::tables(catalog, schema)
                .await?
                .into_iter()
                .map(|t| entry(t.name, t.comment, t.created_at))
                .collect(),
            Kind::Volumes => api::volumes(catalog, schema)
                .await?
                .into_iter()
                .map(|v| entry(v.name, v.comment, v.created_at))
                .collect(),
            Kind::Functions => api::functions(catalog, schema)
                .await?
                .into_iter()
                .map(|f| entry(f.name, f.comment, f.created_at))
                .collect(),
            Kind::Models => api::models(catalog, schema)
                .await?
                .into_iter()
                .map(|m| entry(m.name, m.comment, m.created_at))
                .collect(),
        };
        Ok(entries)
    }
}

fn entry(name: Option<String>, comment: Option<String>, created_at: Option<EpochMillis>) -> Entry {
    Entry {
        name: name.unwrap_or_default(),
        comment: comment.filter(|c| !c.is_empty()),
        created_at,
    }
}

/// The catalog page.
pub fn catalog_href(catalog: &str) -> String {
    format!("/catalog/{catalog}")
}

/// The schema page.
pub fn schema_href(catalog: &str, schema: &str) -> String {
    format!("/catalog/{catalog}/{schema}")
}
