//! The root component: the router, the toasts and the shell. The routes
//! are the console's (`web/src/routes`), with the access tokens page, the
//! pages under "Manage" and the search page added.

use leptos::prelude::*;
use leptos_router::components::{ParentRoute, Route, Router, Routes};
use leptos_router::path;

use crate::auth::provide_auth;
use crate::components::{Notifications, provide_notifications};
use crate::layout::Shell;
use crate::load::provide_cache;
use crate::pages::{
    AccessTokens, AuditLog, CatalogDetails, CatalogsList, CredentialDetails, Credentials,
    ExternalLocationDetails, ExternalLocations, FunctionDetails, GroupDetails, Groups, Login,
    ModelDetails, ModelVersionDetails, SchemaDetails, SearchResults, TableDetails, Users,
    VolumeDetails,
};
use crate::theme::provide_theme;

#[component]
pub fn App() -> impl IntoView {
    provide_theme();
    provide_notifications();
    provide_cache();
    provide_auth();

    view! {
        <Router>
            <Notifications/>
            <Routes fallback=|| view! { <div class="uc-page">"Page not found."</div> }>
                <Route path=path!("/login") view=Login/>
                <ParentRoute path=path!("/") view=Shell>
                    <Route path=path!("") view=CatalogsList/>
                    <Route path=path!("catalog/:catalog") view=CatalogDetails/>
                    <Route path=path!("catalog/:catalog/:schema") view=SchemaDetails/>
                    <Route path=path!("catalog/:catalog/:schema/table/:table") view=TableDetails/>
                    <Route path=path!("catalog/:catalog/:schema/volume/:volume") view=VolumeDetails/>
                    <Route path=path!("catalog/:catalog/:schema/function/:function") view=FunctionDetails/>
                    <Route path=path!("catalog/:catalog/:schema/model/:model") view=ModelDetails/>
                    <Route path=path!("catalog/:catalog/:schema/model/:model/version/:version") view=ModelVersionDetails/>
                    <Route path=path!("settings/tokens") view=AccessTokens/>
                    <Route path=path!("credentials") view=Credentials/>
                    <Route path=path!("credentials/:name") view=CredentialDetails/>
                    <Route path=path!("external-locations") view=ExternalLocations/>
                    <Route path=path!("external-locations/:name") view=ExternalLocationDetails/>
                    <Route path=path!("users") view=Users/>
                    <Route path=path!("groups") view=Groups/>
                    <Route path=path!("groups/:name") view=GroupDetails/>
                    <Route path=path!("audit") view=AuditLog/>
                    <Route path=path!("search") view=SearchResults/>
                </ParentRoute>
            </Routes>
        </Router>
    }
}
