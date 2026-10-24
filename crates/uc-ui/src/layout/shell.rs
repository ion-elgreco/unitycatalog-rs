//! The app's shell, as upstream's `AppProvider` in `App.tsx`. A dark header
//! has the logo, the Catalogs menu item, a link to the API docs and the
//! user's avatar when one is signed in. Below it, the schema browser is on
//! the left and the page on the right. Signed out, the login screen takes
//! its place. The avatar's menu also opens the user's access tokens.

use leptos::prelude::*;
use leptos_router::components::Outlet;
use leptos_router::hooks::use_location;

use crate::auth::{Session, logout, logout_url, use_auth};
use crate::components::icons::{ExportOutlined, UserOutlined};
use crate::components::{Dropdown, Placement, SchemaBrowser};
use crate::pages::{Denied, Login};

/// What the shell shows. A new answer of the same kind, such as the
/// check when the window gets the focus, keeps the page as it is.
#[derive(Clone, PartialEq)]
enum Screen {
    Blank,
    Login,
    Denied(String),
    App,
}

#[component]
pub fn Shell() -> impl IntoView {
    let auth = use_auth();
    let screen = Memo::new(move |_| {
        auth.session.with(|session| match session {
            Session::Unknown => Screen::Blank,
            Session::SignedOut => Screen::Login,
            Session::Denied(message) => Screen::Denied(message.clone()),
            Session::User(_) | Session::Anonymous => Screen::App,
        })
    });
    move || match screen.get() {
        Screen::Blank => ().into_any(),
        Screen::Login => view! { <Login/> }.into_any(),
        Screen::Denied(message) => view! { <Denied message/> }.into_any(),
        Screen::App => view! { <Layout/> }.into_any(),
    }
}

#[component]
fn Layout() -> impl IntoView {
    let auth = use_auth();
    let signed_in = move || auth.session.with(|session| session.user().is_some());
    // "Catalogs" is selected on every page but the user's own.
    let pathname = use_location().pathname;
    let on_catalogs = move || !pathname.with(|path| path.starts_with("/settings/"));
    view! {
        <div class="uc-layout">
            <header class="uc-header">
                <div class="uc-header__left">
                    <a class="uc-header__logo" href="/">
                        <img src="/uc-logo-reverse.svg" height="32" alt="Unity Catalog"/>
                    </a>
                    <nav class="uc-header__menu">
                        <a class="uc-header__menu-item" class:is-selected=on_catalogs href="/">
                            "Catalogs"
                        </a>
                    </nav>
                </div>
                <div class="uc-header__right">
                    // The server's Swagger UI, outside the app: the router
                    // leaves links with a target to the browser.
                    <a
                        class="uc-header__link"
                        href="/docs"
                        target="_blank"
                        rel="noopener"
                        title="The server's REST API in Swagger UI, in a new tab"
                    >
                        "API docs"
                        <ExportOutlined/>
                    </a>
                    <Show when=signed_in>
                        <ProfileMenu/>
                    </Show>
                </div>
            </header>
            <div class="uc-content">
                <aside class="uc-sidebar">
                    <SchemaBrowser/>
                </aside>
                <main class="uc-main">
                    <Outlet/>
                </main>
            </div>
        </div>
    }
}

/// Upstream's avatar dropdown: the user's name and email, and "Log out"
/// where the server says that sign-out goes. Here also the user's access
/// tokens: a link, which the router follows after the menu closes.
#[component]
fn ProfileMenu() -> impl IntoView {
    let auth = use_auth();
    let logout_url = logout_url(auth);
    let name = move || {
        auth.session.with(|session| {
            session
                .user()
                .and_then(|user| user.display_name.clone())
                .unwrap_or_default()
        })
    };
    let email = move || {
        auth.session.with(|session| {
            session
                .user()
                .and_then(|user| user.emails.first().map(|e| e.value.clone()))
                .unwrap_or_default()
        })
    };
    view! {
        <Dropdown
            button_class="uc-avatar"
            title="Account"
            placement=Placement::BottomRight
            button=view! { <UserOutlined/> }.into_any()
        >
            <div class="uc-dropdown__item uc-dropdown__user">
                <span>{name}</span>
                <span>{email}</span>
            </div>
            <div class="uc-dropdown__divider"></div>
            <a class="uc-dropdown__item" role="menuitem" href="/settings/tokens">
                "Access tokens"
            </a>
            {move || {
                logout_url
                    .get()
                    .map(|url| {
                        view! {
                            <div class="uc-dropdown__divider"></div>
                            <button
                                type="button"
                                class="uc-dropdown__item"
                                role="menuitem"
                                on:click=move |_| logout(&url)
                            >
                                "Log out"
                            </button>
                        }
                    })
            }}
        </Dropdown>
    }
}
