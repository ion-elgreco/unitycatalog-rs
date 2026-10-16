//! The root component: the router, the notifications and the shell.

use leptos::prelude::*;
use leptos_router::components::{ParentRoute, Route, Router, Routes};
use leptos_router::path;

use crate::auth::provide_auth;
use crate::components::{Notifications, provide_notifications};
use crate::layout::Shell;
use crate::load::provide_cache;
use crate::pages::{
    AccessTokens, CatalogDetails, CatalogsList, FunctionDetails, Login, ModelDetails,
    ModelVersionDetails, SchemaDetails, TableDetails, VolumeDetails,
};

#[component]
pub fn App() -> impl IntoView {
    provide_notifications();
    provide_cache();
    provide_auth();

    view! {
        <Router>
            <Notifications/>
            <Routes fallback=|| view! { <div style="padding:24px;">"Page not found."</div> }>
                <Route path=path!("/login") view=Login/>
                <ParentRoute path=path!("/") view=Shell>
                    <Route path=path!("") view=CatalogsList/>
                    <Route path=path!("data/:catalog") view=CatalogDetails/>
                    <Route path=path!("data/:catalog/:schema") view=SchemaDetails/>
                    <Route path=path!("data/:catalog/:schema/:table") view=TableDetails/>
                    <Route path=path!("volumes/:catalog/:schema/:volume") view=VolumeDetails/>
                    <Route path=path!("functions/:catalog/:schema/:function") view=FunctionDetails/>
                    <Route path=path!("models/:catalog/:schema/:model") view=ModelDetails/>
                    <Route path=path!("models/:catalog/:schema/:model/versions/:version") view=ModelVersionDetails/>
                    <Route path=path!("settings/tokens") view=AccessTokens/>
                </ParentRoute>
            </Routes>
        </Router>
    }
}
