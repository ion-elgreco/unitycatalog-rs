//! The console's URLs: the paths of the pages, built from the names they
//! show, and read back from the URL for the tree and the path at the top
//! of the page.

use leptos::prelude::*;
use leptos_router::hooks::{use_location, use_params_map};
use percent_encoding::percent_decode_str;

use crate::kind::Kind;

/// The catalog page.
pub fn catalog_href(catalog: &str) -> String {
    format!("/catalog/{catalog}")
}

/// The schema page.
pub fn schema_href(catalog: &str, schema: &str) -> String {
    format!("/catalog/{catalog}/{schema}")
}

/// The route's parameters `names`, as one value that changes only when one
/// of them does. A page builds its content again from them: the router
/// keeps the page when only the parameters change, such as from one table
/// to another.
pub fn route_params<const N: usize>(names: [&'static str; N]) -> Memo<[String; N]> {
    let params = use_params_map();
    Memo::new(move |_| params.with(|params| names.map(|name| params.get(name).unwrap_or_default())))
}

/// What the URL names: the catalog, the schema, an object of a kind, and
/// a model's version; or a page of the user's own, such as the tokens.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Route {
    pub catalog: Option<String>,
    pub schema: Option<String>,
    pub object: Option<(Kind, String)>,
    pub version: Option<String>,
    /// The title of a page outside the catalogs.
    pub page: Option<&'static str>,
}

impl Route {
    fn parse(path: &str) -> Self {
        let mut parts = path.trim_start_matches('/').split('/').map(decode);
        match parts.next().as_deref() {
            Some("catalog") => {}
            Some("settings") => {
                let page = (parts.next().as_deref() == Some("tokens")).then_some("Access tokens");
                return Self {
                    page,
                    ..Self::default()
                };
            }
            _ => return Self::default(),
        }
        let catalog = parts.next().filter(|s| !s.is_empty());
        let schema = parts.next().filter(|s| !s.is_empty());
        let kind = parts
            .next()
            .and_then(|segment| Kind::from_segment(&segment));
        let name = parts.next().filter(|s| !s.is_empty());
        let version = (parts.next().as_deref() == Some("version"))
            .then(|| parts.next().filter(|s| !s.is_empty()))
            .flatten();
        Self {
            catalog,
            schema,
            object: kind.zip(name),
            version,
            page: None,
        }
    }

    pub fn in_catalog(&self, catalog: &str) -> bool {
        self.catalog.as_deref() == Some(catalog)
    }

    pub fn in_schema(&self, catalog: &str, schema: &str) -> bool {
        self.in_catalog(catalog) && self.schema.as_deref() == Some(schema)
    }
}

/// A part of the URL, as the name it stands for: its `%XX` escapes
/// decoded.
fn decode(part: &str) -> String {
    percent_decode_str(part).decode_utf8_lossy().into_owned()
}

/// Read the URL once for the tree and the path at the top of the page: put
/// the route in the context.
pub fn provide_route() {
    let pathname = use_location().pathname;
    let route = Memo::new(move |_| pathname.with(|path| Route::parse(path)));
    provide_context(route);
}

pub fn use_route() -> Memo<Route> {
    expect_context::<Memo<Route>>()
}

#[cfg(test)]
mod tests {
    use super::Route;
    use crate::kind::Kind;

    #[test]
    fn routes_name_the_objects() {
        assert_eq!(Route::parse("/"), Route::default());
        assert_eq!(Route::parse("/settings/other"), Route::default());
        let route = Route::parse("/catalog/unity");
        assert_eq!(route.catalog.as_deref(), Some("unity"));
        assert_eq!(route.schema, None);
        let route = Route::parse("/catalog/unity/default/table/marksheet");
        assert_eq!(route.schema.as_deref(), Some("default"));
        assert_eq!(route.object, Some((Kind::Tables, "marksheet".into())));
        let route = Route::parse("/catalog/unity/default/model/iris/version/2");
        assert_eq!(route.object, Some((Kind::Models, "iris".into())));
        assert_eq!(route.version.as_deref(), Some("2"));
        let route = Route::parse("/catalog/unity/default/view/x");
        assert_eq!(route.object, None);
        let route = Route::parse("/settings/tokens");
        assert_eq!(route.page, Some("Access tokens"));
        assert_eq!(route.catalog, None);
        let route = Route::parse("/catalog/caf%C3%A9%20bar/a%2Fb");
        assert_eq!(route.catalog.as_deref(), Some("café bar"));
        assert_eq!(route.schema.as_deref(), Some("a/b"));
    }
}
