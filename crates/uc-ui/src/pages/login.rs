//! The login screen, as the console's `Login`, with the server's sign-in:
//! one button, which sends the browser to `/auth/login`. The server signs
//! in with its identity provider, then returns the browser to this page;
//! after a failed sign-in, to `/login?error=<why>`, and the page says so.
//! Where the browser cannot sign in, the page says why. A signed-in user
//! who opens the page goes to the catalogs.

use leptos::prelude::*;
use leptos_router::hooks::{use_location, use_navigate, use_query_map};

use crate::auth::{SignIn, logout, logout_url, use_auth};
use crate::components::Card;

/// The page to return to after the sign-in: this one, else `/`.
fn current_path() -> String {
    web_sys::window()
        .and_then(|w| {
            let location = w.location();
            Some(format!(
                "{}{}",
                location.pathname().ok()?,
                location.search().ok()?
            ))
        })
        .filter(|p| !p.starts_with("/login"))
        .unwrap_or_else(|| "/".into())
}

/// Why the server's sign-in failed, by the code of `?error=`.
fn why(code: &str) -> Option<&'static str> {
    Some(match code {
        "provider" => "The identity provider did not sign you in.",
        "expired" => "The sign-in took too long, or it did not start here. Try again.",
        "token" => "The identity provider's answer is not valid.",
        "user" => "No active user of this server has your email. An admin adds users.",
        "server" => "The server could not sign you in.",
        _ => return None,
    })
}

/// The login screen's frame: the dark page, the logo and a card.
#[component]
fn Frame(#[prop(into)] title: String, children: Children) -> impl IntoView {
    view! {
        <div class="uc-login">
            <div class="uc-login__box">
                <div class="uc-login__brand">
                    <img src="/uc-logo-horiz-reverse.svg" alt="Unity Catalog"/>
                    <p>"Sign in to Unity Catalog"</p>
                </div>
                <Card title=title>{children()}</Card>
            </div>
        </div>
    }
}

#[component]
pub fn Login() -> impl IntoView {
    let auth = use_auth();
    let navigate = use_navigate();
    Effect::new(move |_| {
        if auth.session.with(|session| session.user().is_some()) {
            navigate("/", Default::default());
        }
    });
    // A failed sign-in, once: its reason, if the code names one. Then the
    // URL loses its `?error=`.
    let failed: Option<Option<&'static str>> = use_query_map()
        .with_untracked(|query| query.get("error"))
        .filter(|_| use_location().pathname.get_untracked() == "/login")
        .map(|code| why(&code));
    if failed.is_some()
        && let Some(history) = web_sys::window().and_then(|w| w.history().ok())
    {
        let _ = history.replace_state_with_url(&wasm_bindgen::JsValue::NULL, "", Some("/login"));
    }
    let failure = move || {
        failed.map(|reason| {
            view! {
                <p class="uc-error" role="alert">
                    "Login failed. Contact your administrator."
                    {reason.map(|why| view! { <br/>{why} })}
                </p>
            }
        })
    };
    let sign_in = move |_| {
        let rd = String::from(js_sys::encode_uri_component(&current_path()));
        if let Some(window) = web_sys::window() {
            let _ = window.location().assign(&format!("/auth/login?rd={rd}"));
        }
    };
    let body = move || match auth.mode.get().map(|mode| mode.sign_in) {
        None => ().into_any(),
        Some(SignIn::Oidc) => view! {
            <p class="uc-muted">"The server signs you in with its identity provider."</p>
            <button type="button" class="uc-btn uc-btn--block" on:click=sign_in>
                "Sign in"
            </button>
            {failure}
        }
        .into_any(),
        Some(SignIn::Forward) => view! {
            <p class="uc-muted">
                "This server signs users in through the auth proxy in front of it. \
                 Open the UI through the proxy."
            </p>
        }
        .into_any(),
        Some(SignIn::None | SignIn::Off) => view! {
            <p class="uc-muted">
                "This server has no sign-in for the web UI. An admin sets one up with \
                 auth.mode."
            </p>
        }
        .into_any(),
        Some(SignIn::Token) => view! {
            <p class="uc-muted">
                "uc web signs in with the token of the uc commands: --auth_token, \
                 UC_CLI_TOKEN, or token in the [cli] table of uc.toml; uc auth login gets \
                 one. Set a valid token, then start uc web again."
            </p>
        }
        .into_any(),
    };
    view! { <Frame title="Login">{body}</Frame> }
}

/// The server knows the user but does not let them in (403), such as a
/// deactivated user: its message, and "Log out" to sign in as someone
/// else.
#[component]
pub fn Denied(message: String) -> impl IntoView {
    let logout_url = logout_url(use_auth());
    view! {
        <Frame title="No access">
            <p class="uc-muted">{message}</p>
            {move || {
                logout_url
                    .get()
                    .map(|url| {
                        view! {
                            <button type="button" class="uc-btn uc-btn--block" on:click=move |_| logout(&url)>
                                "Log out"
                            </button>
                        }
                    })
            }}
        </Frame>
    }
}
