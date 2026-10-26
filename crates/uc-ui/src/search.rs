//! The search: the types of objects that it finds, and the page of each
//! result. The box in the header shows the first results while you type;
//! the search page lists all of them, by type.

use leptos::prelude::*;
use percent_encoding::{NON_ALPHANUMERIC, utf8_percent_encode};
use uc_client::models::search::v1::{SearchObjectsResponse, SearchResult};

use crate::api;
use crate::kind::{Kind, catalog_icon, schema_icon};
use crate::util::{catalog_href, schema_href};

/// The shortest text that the UI searches for: on Turso, the server finds
/// no word of one character.
pub const MIN_CHARS: usize = 2;

/// A type of object that the search finds, or all of them.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SearchType {
    #[default]
    All,
    Catalog,
    Schema,
    Object(Kind),
}

impl SearchType {
    /// The tabs of the search page, in order.
    pub const ALL: [SearchType; 7] = [
        SearchType::All,
        SearchType::Catalog,
        SearchType::Schema,
        SearchType::Object(Kind::Tables),
        SearchType::Object(Kind::Volumes),
        SearchType::Object(Kind::Functions),
        SearchType::Object(Kind::Models),
    ];

    /// Its `securable_type` in the API, and its `?type=` on the search
    /// page; none for all.
    pub fn param(self) -> Option<&'static str> {
        match self {
            SearchType::All => None,
            SearchType::Catalog => Some("catalog"),
            SearchType::Schema => Some("schema"),
            SearchType::Object(Kind::Tables) => Some("table"),
            SearchType::Object(Kind::Volumes) => Some("volume"),
            SearchType::Object(Kind::Functions) => Some("function"),
            SearchType::Object(Kind::Models) => Some("registered_model"),
        }
    }

    /// The type that `param` names; all for another value.
    pub fn from_param(param: &str) -> SearchType {
        SearchType::ALL
            .into_iter()
            .find(|t| t.param() == Some(param))
            .unwrap_or_default()
    }

    /// Its tab on the search page.
    pub fn label(self) -> &'static str {
        match self {
            SearchType::All => "All",
            SearchType::Catalog => "Catalogs",
            SearchType::Schema => "Schemas",
            SearchType::Object(kind) => kind.label(),
        }
    }

    /// The badge of a result of this type.
    pub fn noun(self) -> &'static str {
        match self {
            SearchType::All => "Object",
            SearchType::Catalog => "Catalog",
            SearchType::Schema => "Schema",
            SearchType::Object(Kind::Tables) => "Table",
            SearchType::Object(Kind::Volumes) => "Volume",
            SearchType::Object(Kind::Functions) => "Function",
            SearchType::Object(Kind::Models) => "Model",
        }
    }

    pub fn icon(self) -> Option<AnyView> {
        match self {
            SearchType::All => None,
            SearchType::Catalog => Some(catalog_icon()),
            SearchType::Schema => Some(schema_icon()),
            SearchType::Object(kind) => Some(kind.icon()),
        }
    }

    pub fn of(result: &SearchResult) -> SearchType {
        SearchType::from_param(&result.securable_type)
    }
}

/// The page of a result.
pub fn result_href(result: &SearchResult) -> String {
    let catalog = &result.catalog_name;
    match SearchType::of(result) {
        SearchType::All | SearchType::Catalog => catalog_href(catalog),
        SearchType::Schema => schema_href(catalog, &result.name),
        SearchType::Object(kind) => kind.href(
            catalog,
            result.schema_name.as_deref().unwrap_or_default(),
            &result.name,
        ),
    }
}

/// Where a result is: its catalog and schema; none for a catalog.
pub fn result_parent(result: &SearchResult) -> Option<String> {
    match SearchType::of(result) {
        SearchType::All | SearchType::Catalog => None,
        SearchType::Schema => Some(result.catalog_name.clone()),
        SearchType::Object(_) => Some(format!(
            "{}.{}",
            result.catalog_name,
            result.schema_name.as_deref().unwrap_or_default()
        )),
    }
}

/// The search page for `text` and `kind`.
pub fn page_href(text: &str, kind: SearchType) -> String {
    let mut href = format!(
        "/search?q={}",
        utf8_percent_encode(text.trim(), NON_ALPHANUMERIC)
    );
    if let Some(param) = kind.param() {
        href.push_str("&type=");
        href.push_str(param);
    }
    href
}

/// Whether the UI sends `text` to the server.
pub fn searchable(text: &str) -> bool {
    text.trim().chars().count() >= MIN_CHARS
}

/// A page of the results for `text` of the type `kind`.
pub async fn search(
    text: &str,
    kind: SearchType,
    max_results: i32,
    page_token: Option<String>,
) -> api::Result<SearchObjectsResponse> {
    api::client()
        .search_objects(text.trim())
        .with_securable_type(kind.param().map(str::to_string))
        .with_max_results(max_results)
        .with_page_token(page_token)
        .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn types_round_trip_their_params() {
        for kind in SearchType::ALL {
            let param = kind.param().unwrap_or("other");
            assert_eq!(SearchType::from_param(param), kind);
        }
        assert_eq!(
            page_href("sales orders/x", SearchType::Object(Kind::Models)),
            "/search?q=sales%20orders%2Fx&type=registered_model"
        );
        assert_eq!(page_href(" a ", SearchType::All), "/search?q=a");
        assert!(!searchable(" a ") && searchable("ab"));
    }
}
