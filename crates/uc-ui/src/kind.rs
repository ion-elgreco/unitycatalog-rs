//! What a schema holds: tables, metric views, volumes, functions and
//! registered models. They are the groups under a schema in the tree and
//! the picker on the schema page.

use leptos::prelude::*;
use uc_client::models::functions::v1::FunctionInfo;
use uc_common::EpochMillis;

use crate::api;
use crate::components::Label;
use crate::components::icons::{
    Box, ChartNoAxesCombined, Database, FunctionSquare, HardDrive, Notebook, Table,
};
use crate::metric_view::is_metric_view;
use crate::util::{enum_name, or_dash, schema_href};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Kind {
    Tables,
    MetricViews,
    Volumes,
    Functions,
    Models,
}

/// An object in a schema, as the lists show it.
#[derive(Clone, Debug)]
pub struct Entry {
    pub name: String,
    pub comment: Option<String>,
    pub owner: Option<String>,
    pub created_at: EpochMillis,
    /// The values of the kind's own columns, in the order of
    /// [`Kind::columns`].
    pub cells: Vec<Label>,
}

impl Kind {
    pub const ALL: [Kind; 5] = [
        Kind::Tables,
        Kind::MetricViews,
        Kind::Volumes,
        Kind::Functions,
        Kind::Models,
    ];

    /// Its value of `?tab=` on the schema page.
    pub fn tab(self) -> &'static str {
        match self {
            Kind::Tables => "tables",
            Kind::MetricViews => "views",
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
            Kind::MetricViews => "Metric views",
            Kind::Volumes => "Volumes",
            Kind::Functions => "Functions",
            Kind::Models => "Models",
        }
    }

    /// The part of an object's URL that names its kind.
    pub fn segment(self) -> &'static str {
        match self {
            Kind::Tables => "table",
            Kind::MetricViews => "view",
            Kind::Volumes => "volume",
            Kind::Functions => "function",
            Kind::Models => "model",
        }
    }

    /// The kind that the part `segment` of an object's URL names.
    pub fn from_segment(segment: &str) -> Option<Kind> {
        Kind::ALL.into_iter().find(|kind| kind.segment() == segment)
    }

    /// One object, such as "table".
    pub fn noun(self) -> &'static str {
        match self {
            Kind::MetricViews => "metric view",
            other => other.segment(),
        }
    }

    /// Its name in the "New" dialog's picker.
    pub fn new_label(self) -> &'static str {
        match self {
            Kind::Tables => "Table",
            Kind::MetricViews => "Metric view",
            Kind::Volumes => "Volume",
            Kind::Functions => "Function",
            Kind::Models => "Registered model",
        }
    }

    /// The titles of the columns that the schema page shows for this kind
    /// only, between the name and the owner.
    pub fn columns(self) -> &'static [&'static str] {
        match self {
            Kind::Tables => &["Type", "Format"],
            Kind::MetricViews => &[],
            Kind::Volumes => &["Type"],
            Kind::Functions => &["Language", "Returns"],
            Kind::Models => &[],
        }
    }

    /// The hint under an empty list of this kind: what the objects are,
    /// and how to make one.
    pub fn empty_hint(self) -> &'static str {
        match self {
            Kind::Tables => {
                "A table is data in Delta, Parquet or another format, with its columns. Click New to register one: managed under the storage root, or external at a location of yours."
            }
            Kind::MetricViews => {
                "A metric view names the dimensions and measures of a table, in YAML, for engines to query. Click New to register one."
            }
            Kind::Volumes => {
                "A volume is a folder of files that are not a table, such as images or exports. Click New to register one."
            }
            Kind::Functions => {
                "A function is a routine that engines call in their queries. Click New to register a SQL function."
            }
            Kind::Models => {
                "A registered model holds the versions of a machine learning model. Click New to register one; its versions come from MLflow or the API."
            }
        }
    }

    /// The class that colors the kind's icon.
    fn color(self) -> &'static str {
        match self {
            Kind::Tables => "uc-icon--table",
            Kind::MetricViews => "uc-icon--view",
            Kind::Volumes => "uc-icon--volume",
            Kind::Functions => "uc-icon--function",
            Kind::Models => "uc-icon--model",
        }
    }

    pub fn icon(self) -> AnyView {
        let class = self.color();
        match self {
            Kind::Tables => view! { <Table class/> }.into_any(),
            Kind::MetricViews => view! { <ChartNoAxesCombined class/> }.into_any(),
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
        format!("{}/{}/{name}", schema_href(catalog, schema), self.segment())
    }

    /// The objects of this kind in `catalog.schema`.
    pub async fn entries(self, catalog: &str, schema: &str) -> api::Result<Vec<Entry>> {
        let entries = match self {
            // The API lists the metric views with the tables.
            Kind::Tables => api::tables(catalog, schema)
                .await?
                .into_iter()
                .filter(|t| !is_metric_view(t))
                .map(|t| {
                    let cells = vec![
                        or_dash(enum_name(t.table_type)).into(),
                        Label::format(t.data_source_format),
                    ];
                    entry(t.name, t.comment, t.owner, t.created_at, cells)
                })
                .collect(),
            Kind::MetricViews => api::tables(catalog, schema)
                .await?
                .into_iter()
                .filter(is_metric_view)
                .map(|t| entry(t.name, t.comment, t.owner, t.created_at, Vec::new()))
                .collect(),
            Kind::Volumes => api::volumes(catalog, schema)
                .await?
                .into_iter()
                .map(|v| {
                    let cells = vec![or_dash(enum_name(v.volume_type)).into()];
                    entry(v.name, v.comment, v.owner, v.created_at, cells)
                })
                .collect(),
            Kind::Functions => api::functions(catalog, schema)
                .await?
                .into_iter()
                .map(|f| {
                    let cells = vec![
                        or_dash(function_language(&f)).into(),
                        or_dash(function_returns(&f)).into(),
                    ];
                    entry(f.name, f.comment, f.owner, f.created_at, cells)
                })
                .collect(),
            Kind::Models => api::models(catalog, schema)
                .await?
                .into_iter()
                .map(|m| entry(m.name, m.comment, m.owner, m.created_at, Vec::new()))
                .collect(),
        };
        Ok(entries)
    }
}

fn entry(
    name: String,
    comment: Option<String>,
    owner: Option<String>,
    created_at: EpochMillis,
    cells: Vec<Label>,
) -> Entry {
    Entry {
        name,
        comment: comment.filter(|c| !c.is_empty()),
        owner,
        created_at,
        cells,
    }
}

/// A function's language: the external one, else its routine body, such
/// as `SQL`.
pub fn function_language(f: &FunctionInfo) -> Option<String> {
    f.external_language
        .clone()
        .or_else(|| Some(enum_name(f.routine_body)).filter(|body| !body.is_empty()))
}

/// The type that a function returns.
pub fn function_returns(f: &FunctionInfo) -> Option<String> {
    f.full_data_type
        .clone()
        .or_else(|| f.data_type.map(enum_name))
}

/// A catalog's icon, in its color.
pub fn catalog_icon() -> AnyView {
    view! { <Notebook class="uc-icon--catalog"/> }.into_any()
}

/// A schema's icon, in its color.
pub fn schema_icon() -> AnyView {
    view! { <Database class="uc-icon--schema"/> }.into_any()
}
