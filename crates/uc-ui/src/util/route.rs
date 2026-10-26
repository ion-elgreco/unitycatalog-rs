//! The console's URLs: the paths of the pages, built from the names they
//! show, and read back from the URL for the tree, the sidebar's links and
//! the path at the top of the page.

use leptos::prelude::*;
use leptos_router::hooks::{use_location, use_params_map};
use percent_encoding::percent_decode_str;

use crate::components::icons::{Cloud, KeyRound, ScrollText, Search, Users, UsersRound};
use crate::kind::Kind;

/// The catalog page.
pub fn catalog_href(catalog: &str) -> String {
    format!("/catalog/{catalog}")
}

/// The schema page.
pub fn schema_href(catalog: &str, schema: &str) -> String {
    format!("/catalog/{catalog}/{schema}")
}

/// A page outside the catalogs: the user's access tokens, the pages under
/// "Manage" in the sidebar, and the search page.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Section {
    Tokens,
    Credentials,
    ExternalLocations,
    Users,
    Groups,
    Audit,
    Search,
}

impl Section {
    /// The sidebar's "Manage" links, in order.
    pub const MANAGE: [Section; 5] = [
        Section::Credentials,
        Section::ExternalLocations,
        Section::Users,
        Section::Groups,
        Section::Audit,
    ];

    pub fn title(self) -> &'static str {
        match self {
            Section::Tokens => "Access tokens",
            Section::Credentials => "Credentials",
            Section::ExternalLocations => "External locations",
            Section::Users => "Users",
            Section::Groups => "Groups",
            Section::Audit => "Audit log",
            Section::Search => "Search",
        }
    }

    /// Its page.
    pub fn href(self) -> &'static str {
        match self {
            Section::Tokens => "/settings/tokens",
            Section::Credentials => "/credentials",
            Section::ExternalLocations => "/external-locations",
            Section::Users => "/users",
            Section::Groups => "/groups",
            Section::Audit => "/audit",
            Section::Search => "/search",
        }
    }

    /// The page of one of its items: a credential, an external location
    /// or a group, by name.
    pub fn item_href(self, name: &str) -> String {
        format!("{}/{name}", self.href())
    }

    /// Whether the server keeps the page for admins: the users, the groups
    /// and the audit log.
    pub fn admin_only(self) -> bool {
        matches!(self, Section::Users | Section::Groups | Section::Audit)
    }

    pub fn icon(self) -> AnyView {
        match self {
            Section::Tokens | Section::Credentials => view! { <KeyRound/> }.into_any(),
            Section::ExternalLocations => view! { <Cloud/> }.into_any(),
            Section::Users => view! { <Users/> }.into_any(),
            Section::Groups => view! { <UsersRound/> }.into_any(),
            Section::Audit => view! { <ScrollText/> }.into_any(),
            Section::Search => view! { <Search/> }.into_any(),
        }
    }

    /// The section whose URLs start with `segment`; the tokens page is
    /// under `settings`.
    fn from_segment(segment: &str) -> Option<Section> {
        match segment {
            "credentials" => Some(Section::Credentials),
            "external-locations" => Some(Section::ExternalLocations),
            "users" => Some(Section::Users),
            "groups" => Some(Section::Groups),
            "audit" => Some(Section::Audit),
            "search" => Some(Section::Search),
            _ => None,
        }
    }
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
/// a model's version; or a page outside the catalogs, and its item.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Route {
    pub catalog: Option<String>,
    pub schema: Option<String>,
    pub object: Option<(Kind, String)>,
    pub version: Option<String>,
    pub section: Option<Section>,
    /// The item of a section's page: a credential's, an external
    /// location's or a group's name.
    pub item: Option<String>,
}

impl Route {
    fn parse(path: &str) -> Self {
        let mut parts = path.trim_start_matches('/').split('/').map(decode);
        let first = parts.next().unwrap_or_default();
        if first == "settings" {
            let section = (parts.next().as_deref() == Some("tokens")).then_some(Section::Tokens);
            return Self {
                section,
                ..Self::default()
            };
        }
        if let Some(section) = Section::from_segment(&first) {
            return Self {
                section: Some(section),
                item: parts.next().filter(|s| !s.is_empty()),
                ..Self::default()
            };
        }
        if first != "catalog" {
            return Self::default();
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
            section: None,
            item: None,
        }
    }

    pub fn in_catalog(&self, catalog: &str) -> bool {
        self.catalog.as_deref() == Some(catalog)
    }

    pub fn in_schema(&self, catalog: &str, schema: &str) -> bool {
        self.in_catalog(catalog) && self.schema.as_deref() == Some(schema)
    }

    pub fn in_section(&self, section: Section) -> bool {
        self.section == Some(section)
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
    use super::{Route, Section};
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
        let route = Route::parse("/catalog/caf%C3%A9%20bar/a%2Fb");
        assert_eq!(route.catalog.as_deref(), Some("café bar"));
        assert_eq!(route.schema.as_deref(), Some("a/b"));
    }

    #[test]
    fn routes_name_the_sections() {
        let route = Route::parse("/settings/tokens");
        assert_eq!(route.section, Some(Section::Tokens));
        assert_eq!(route.catalog, None);
        let route = Route::parse("/credentials");
        assert_eq!(route.section, Some(Section::Credentials));
        assert_eq!(route.item, None);
        let route = Route::parse("/external-locations/lake%20east");
        assert_eq!(route.section, Some(Section::ExternalLocations));
        assert_eq!(route.item.as_deref(), Some("lake east"));
        let route = Route::parse("/groups/analysts");
        assert!(route.in_section(Section::Groups));
        assert_eq!(route.item.as_deref(), Some("analysts"));
        assert_eq!(Route::parse("/audit").section, Some(Section::Audit));
        assert_eq!(Route::parse("/search").section, Some(Section::Search));
        assert_eq!(Section::Groups.item_href("analysts"), "/groups/analysts");
    }
}
