//! The app's shell, as upstream's console lays it out. A dark header has a
//! button that hides the sidebar, the logo, the search box, the theme
//! switcher, a link to the API docs and the account menu when a user is
//! signed in. Below it,
//! on the gray page: the path of the page shown, then the catalog tree in
//! a card on the left, with the "Manage" links under it, and the page on
//! the right. Signed out, the login screen takes its place.

use leptos::prelude::*;
use leptos_router::components::Outlet;

use crate::auth::{Session, SignIn, logout, logout_url, use_auth};
use crate::components::icons::{
    ExternalLink, KeyRound, LogOut, Monitor, Moon, PanelLeftClose, PanelLeftOpen, Sun, User,
};
use crate::components::{CatalogTree, Crumbs, Dropdown};
use crate::layout::SearchBox;
use crate::pages::{Denied, Login};
use crate::theme::{ColorMode, use_theme};
use crate::util::{Section, provide_route, use_route};

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
        // The console's skeleton while the server says who is signed in.
        Screen::Blank => view! {
            <div class="uc-boot uc-skeleton" role="status" aria-label="Loading">
                <div class="uc-skeleton__bar"></div>
            </div>
        }
        .into_any(),
        Screen::Login => view! { <Login/> }.into_any(),
        Screen::Denied(message) => view! { <Denied message/> }.into_any(),
        Screen::App => view! { <Layout/> }.into_any(),
    }
}

#[component]
fn Layout() -> impl IntoView {
    provide_route();
    let auth = use_auth();
    let logout_url = logout_url(auth);
    let signed_in = move || auth.session.with(|session| session.user().is_some());
    // The server signs users in: the sidebar says who.
    let auth_enabled = move || {
        auth.mode.with(|mode| {
            mode.as_ref()
                .is_some_and(|mode| !matches!(mode.sign_in, SignIn::Off | SignIn::None))
        })
    };
    let nav_open = RwSignal::new(true);
    // Scrolled, the page is cut level with the sidebar's top; at the top,
    // the focus rings of its first row show above that line.
    let scrolled = RwSignal::new(false);
    let toggle_label = move || {
        if nav_open.get() {
            "Collapse navigation"
        } else {
            "Expand navigation"
        }
    };
    view! {
        <div class="uc-app">
            <header class="uc-header">
                <div class="uc-header__group">
                    <button
                        type="button"
                        class="uc-header__btn"
                        aria-expanded=move || nav_open.get().to_string()
                        aria-controls="uc-nav"
                        aria-label=toggle_label
                        title=toggle_label
                        on:click=move |_| nav_open.update(|open| *open = !*open)
                    >
                        <Show when=move || nav_open.get() fallback=|| view! { <PanelLeftOpen/> }>
                            <PanelLeftClose/>
                        </Show>
                    </button>
                    <a class="uc-header__logo" href="/" title="Unity Catalog">
                        <img src="/uc-logo-reverse.svg" alt="Unity Catalog"/>
                    </a>
                </div>
                <SearchBox/>
                <div class="uc-header__group">
                    <ThemeSwitcher/>
                    // The server's Swagger UI, outside the app: the router
                    // leaves links with a target to the browser.
                    <a
                        class="uc-header__btn"
                        href="/docs"
                        target="_blank"
                        rel="noopener"
                        title="API docs, in a new tab"
                        aria-label="API docs"
                    >
                        <ExternalLink/>
                    </a>
                    <Show when=signed_in>
                        <AccountMenu/>
                    </Show>
                </div>
            </header>
            <div class="uc-body">
                <Path/>
                <div class="uc-columns">
                    <aside
                        id="uc-nav"
                        class="uc-sidebar"
                        class:is-closed=move || !nav_open.get()
                        aria-hidden=move || (!nav_open.get()).to_string()
                    >
                        <div class="uc-sidebar__head">
                            <span class="uc-sidebar__title">"Catalogs"</span>
                            <Show when=auth_enabled>
                                <Show
                                    when=signed_in
                                    fallback=|| view! { <span class="uc-badge">"anonymous"</span> }
                                >
                                    <span class="uc-badge uc-badge--primary">"authenticated"</span>
                                </Show>
                            </Show>
                        </div>
                        <div class="uc-sidebar__tree">
                            <CatalogTree/>
                        </div>
                        <ManageNav/>
                        <Show when=signed_in>
                            {move || {
                                logout_url
                                    .get()
                                    .map(|url| {
                                        view! {
                                            <div class="uc-sidebar__foot">
                                                <button
                                                    type="button"
                                                    class="uc-btn uc-btn--outline uc-btn--block"
                                                    on:click=move |_| logout(&url)
                                                >
                                                    <LogOut/>
                                                    "Log out"
                                                </button>
                                            </div>
                                        }
                                    })
                            }}
                        </Show>
                    </aside>
                    <main
                        class="uc-main"
                        class:is-scrolled=move || scrolled.get()
                        on:scroll=move |e| {
                            let now = event_target::<web_sys::Element>(&e).scroll_top() > 0;
                            if scrolled.get_untracked() != now {
                                scrolled.set(now);
                            }
                        }
                    >
                        <Outlet/>
                    </main>
                </div>
            </div>
        </div>
    }
}

/// "Manage", under the tree: the credentials, the external locations and,
/// for admins, the users, the groups and the audit log. The audit log
/// takes a signed-in admin, so it hides without a user.
#[component]
fn ManageNav() -> impl IntoView {
    let auth = use_auth();
    let route = use_route();
    let admin = Memo::new(move |_| auth.session.with(|s| s.is_admin()));
    let signed_in_admin =
        Memo::new(move |_| auth.session.with(|s| s.user().is_some() && s.is_admin()));
    let shows = move |section: Section| match section {
        Section::Audit => signed_in_admin.get(),
        section if section.admin_only() => admin.get(),
        _ => true,
    };
    view! {
        <nav class="uc-sidebar__nav" aria-label="Manage">
            <span class="uc-sidebar__title">"Manage"</span>
            <ul>
                {Section::MANAGE
                    .into_iter()
                    .map(|section| {
                        let active = move || route.with(|r| r.in_section(section));
                        view! {
                            <Show when=move || shows(section)>
                                <li>
                                    <a
                                        class="uc-sidebar__link"
                                        class:is-active=active
                                        aria-current=move || active().then_some("page")
                                        href=section.href()
                                    >
                                        {section.icon()}
                                        {section.title()}
                                    </a>
                                </li>
                            </Show>
                        }
                    })
                    .collect_view()}
            </ul>
        </nav>
    }
}

/// The path of the page shown, from the URL: "Catalog > unity > default >
/// marksheet", or "Catalog > ml > models > iris > v2".
#[component]
fn Path() -> impl IntoView {
    let route = use_route();
    move || view! { <Crumbs route=route.get()/> }
}

/// Light, dark or the system's theme, as the console's `ThemeSwitcher`.
#[component]
fn ThemeSwitcher() -> impl IntoView {
    let theme = use_theme();
    view! {
        <div class="uc-theme" role="group" aria-label="Theme">
            {ColorMode::ALL
                .into_iter()
                .map(|mode| {
                    let label = mode.label();
                    let icon = match mode {
                        ColorMode::Light => view! { <Sun/> }.into_any(),
                        ColorMode::Dark => view! { <Moon/> }.into_any(),
                        ColorMode::Auto => view! { <Monitor/> }.into_any(),
                    };
                    view! {
                        <button
                            type="button"
                            title=label
                            aria-label=format!("{label} theme")
                            aria-pressed=move || (theme.mode.get() == mode).to_string()
                            class:is-active=move || theme.mode.get() == mode
                            on:click=move |_| theme.set(mode)
                        >
                            {icon}
                        </button>
                    }
                })
                .collect_view()}
        </div>
    }
}

/// The account menu, as the console's: the user's name and email, their
/// access tokens, and "Log out" where the server says that sign-out goes.
#[component]
fn AccountMenu() -> impl IntoView {
    let auth = use_auth();
    let logout_url = logout_url(auth);
    let email = move || {
        auth.session.with(|session| {
            session
                .user()
                .map(|user| {
                    user.emails
                        .first()
                        .map_or_else(|| user.user_name.clone(), |e| e.value.clone())
                })
                .unwrap_or_default()
        })
    };
    let name = move || {
        auth.session.with(|session| {
            session
                .user()
                .and_then(|user| user.display_name.clone())
                .filter(|name| !name.is_empty())
                .unwrap_or_else(email)
        })
    };
    view! {
        <Dropdown button_class="uc-avatar" title="Account" button=view! { <User/> }.into_any()>
            <div class="uc-menu__label">
                <span class="uc-truncate">{name}</span>
                <small>{email}</small>
            </div>
            <div class="uc-menu__sep"></div>
            <a class="uc-menu__item" role="menuitem" href="/settings/tokens">
                <KeyRound/>
                "Access tokens"
            </a>
            {move || {
                logout_url
                    .get()
                    .map(|url| {
                        view! {
                            <div class="uc-menu__sep"></div>
                            <button
                                type="button"
                                class="uc-menu__item"
                                role="menuitem"
                                on:click=move |_| logout(&url)
                            >
                                <LogOut/>
                                "Log out"
                            </button>
                        }
                    })
            }}
        </Dropdown>
    }
}
