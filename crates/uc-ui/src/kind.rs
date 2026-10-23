//! What a schema holds: tables, volumes, functions and registered models.
//! They are the groups under a schema in the tree and the picker on the
//! schema page.

use leptos::prelude::*;

use crate::api;
use crate::components::icons::{Box, Database, FunctionSquare, HardDrive, Notebook, Table};
use crate::util::schema_href;

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

    /// One object, such as "table"; also the part of an object's URL that
    /// names its kind.
    pub fn noun(self) -> &'static str {
        match self {
            Kind::Tables => "table",
            Kind::Volumes => "volume",
            Kind::Functions => "function",
            Kind::Models => "model",
        }
    }

    /// The kind that the part `segment` of an object's URL names.
    pub fn from_segment(segment: &str) -> Option<Kind> {
        Kind::ALL.into_iter().find(|kind| kind.noun() == segment)
    }

    /// Its name in the "New" dialog's picker.
    pub fn new_label(self) -> &'static str {
        match self {
            Kind::Tables => "Table",
            Kind::Volumes => "Volume",
            Kind::Functions => "Function",
            Kind::Models => "Registered model",
        }
    }

    /// The class that colors the kind's icon.
    fn color(self) -> &'static str {
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
        format!("{}?tab={}", schema_href(catalog, schema), self.tab())
    }

    /// The page of the object `name` in `catalog.schema`.
    pub fn href(self, catalog: &str, schema: &str, name: &str) -> String {
        format!("{}/{}/{name}", schema_href(catalog, schema), self.noun())
    }

    /// The objects of this kind in `catalog.schema`.
    pub async fn entries(self, catalog: &str, schema: &str) -> api::Result<Vec<Entry>> {
        let entries = match self {
            Kind::Tables => api::tables(catalog, schema)
                .await?
                .into_iter()
                .map(|t| entry(t.name, t.comment))
                .collect(),
            Kind::Volumes => api::volumes(catalog, schema)
                .await?
                .into_iter()
                .map(|v| entry(v.name, v.comment))
                .collect(),
            Kind::Functions => api::functions(catalog, schema)
                .await?
                .into_iter()
                .map(|f| entry(f.name, f.comment))
                .collect(),
            Kind::Models => api::models(catalog, schema)
                .await?
                .into_iter()
                .map(|m| entry(m.name, m.comment))
                .collect(),
        };
        Ok(entries)
    }
}

fn entry(name: Option<String>, comment: Option<String>) -> Entry {
    Entry {
        name: name.unwrap_or_default(),
        comment: comment.filter(|c| !c.is_empty()),
    }
}

/// A catalog's icon, in its color.
pub fn catalog_icon() -> AnyView {
    view! { <Notebook class="uc-icon--catalog"/> }.into_any()
}

/// A schema's icon, in its color.
pub fn schema_icon() -> AnyView {
    view! { <Database class="uc-icon--schema"/> }.into_any()
}
