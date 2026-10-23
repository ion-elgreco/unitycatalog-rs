//! Sign-in state, as upstream's `AuthProvider` (`context/auth-context.tsx`),
//! with the server's sign-in: `GET {control}/scim2/Me` names the signed-in
//! user, and its 401 means that the server needs a sign-in, which the login
//! screen starts at `/auth/login`. The UI holds no identity provider
//! settings: the server signs users in, with its OpenID Connect provider or
//! through the auth proxy in front of it.
//!
//! A session can end while the page is open, so the UI asks again when the
//! window gets the focus, and when a call fails with 401 or 403
//! ([`check_again`]).
//!
//! `GET /auth/mode` says how the browser signs in, and where "Log out"
//! goes: the login page and the profile menu show only what works. The UI
//! asks `/scim2/Me` after that answer, and not at all when it is `off`: then
//! no one signs in.

use std::cell::Cell;

use leptos::ev;
use leptos::prelude::*;
use leptos::task::spawn_local;
use uc_api::ScimUser;
use uc_client::Method;

use crate::api;

/// How the browser signs in: the server's `auth.mode`, or `uc web`'s
/// token.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SignIn {
    /// The server signs users in with its OpenID Connect provider, from
    /// `/auth/login`.
    Oidc,
    /// The auth proxy in front of the server signs users in.
    Forward,
    /// No sign-in in the browser.
    None,
    /// `uc web`: the calls carry the token of the `uc` commands.
    Token,
    /// Authorization is off and the server has no sign-in: no one signs
    /// in, and everyone may do everything.
    Off,
}

/// The answer of `GET /auth/mode`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Mode {
    pub sign_in: SignIn,
    /// Where "Log out" goes; none hides it.
    pub logout_url: Option<String>,
}

impl Mode {
    /// What the UI does without an answer: the server's own sign-in.
    fn fallback() -> Self {
        Self {
            sign_in: SignIn::Oidc,
            logout_url: Some("/auth/logout".into()),
        }
    }

    fn from_json(body: &serde_json::Value) -> Option<Self> {
        let sign_in = match body.get("mode")?.as_str()? {
            "oidc" => SignIn::Oidc,
            "forward" => SignIn::Forward,
            "none" => SignIn::None,
            "token" => SignIn::Token,
            "off" => SignIn::Off,
            _ => return None,
        };
        let logout_url = body
            .get("logout_url")
            .and_then(|url| url.as_str())
            .map(str::to_string);
        Some(Self {
            sign_in,
            logout_url,
        })
    }
}

/// Who uses the UI, after the answer of `/scim2/Me`.
#[derive(Clone, Debug)]
pub enum Session {
    /// No answer yet.
    Unknown,
    /// 401: the server needs a sign-in.
    SignedOut,
    /// 403: the server knows the caller but does not let them in, such as
    /// a deactivated user. The server's message.
    Denied(String),
    User(Box<ScimUser>),
    /// No user: `/auth/mode` says `off`, or `/scim2/Me` answers "No user
    /// found.".
    Anonymous,
}

impl Session {
    pub fn user(&self) -> Option<&ScimUser> {
        match self {
            Session::User(user) => Some(user.as_ref()),
            _ => None,
        }
    }

    /// The signed-in user's principal, as the server names owners and
    /// grantees: their email.
    pub fn principal(&self) -> Option<&str> {
        self.user().map(|user| user.user_name.as_str())
    }

    /// Whether the user may manage `owner`'s object: its owner, or an
    /// admin. Without a user, authorization is off.
    pub fn manages(&self, owner: Option<&str>) -> bool {
        match self {
            Session::User(user) => user.is_admin() || owner == Some(user.user_name.as_str()),
            Session::Anonymous => true,
            Session::Unknown | Session::SignedOut | Session::Denied(_) => false,
        }
    }

    /// Whether the server lets the user do what the metastore admin does:
    /// `/scim2/Me` names the role `admin`. Without a user, authorization is
    /// off, and the server lets everyone.
    pub fn is_admin(&self) -> bool {
        match self {
            Session::User(user) => user.is_admin(),
            Session::Anonymous => true,
            Session::Unknown | Session::SignedOut | Session::Denied(_) => false,
        }
    }
}

#[derive(Clone, Copy)]
pub struct Auth {
    pub session: RwSignal<Session>,
    /// How the browser signs in; none until the server answers.
    pub mode: RwSignal<Option<Mode>>,
    /// Whether a call to `/scim2/Me` runs: one at a time is enough.
    checking: StoredValue<bool>,
}

thread_local! {
    /// The app's [`Auth`], for [`check_again`]: calls fail in tasks, which
    /// have no context.
    static AUTH: Cell<Option<Auth>> = const { Cell::new(None) };
}

impl Auth {
    /// Ask the server who is signed in: once it has said how users sign in,
    /// and only if someone can.
    pub fn refresh(&self) {
        let auth = *self;
        let asks = auth.mode.with_untracked(|mode| {
            mode.as_ref()
                .is_some_and(|mode| mode.sign_in != SignIn::Off)
        });
        if !asks || auth.checking.get_value() {
            return;
        }
        auth.checking.set_value(true);
        spawn_local(async move {
            let answer = api::client().me().await;
            auth.checking.set_value(false);
            let session = match answer {
                Ok(user) => Session::User(Box::new(user)),
                Err(e) => match e.status() {
                    Some(401) => Session::SignedOut,
                    Some(403) => Session::Denied(e.message()),
                    // The server or the network failed: keep what the UI
                    // knows.
                    None | Some(502..=504)
                        if !auth
                            .session
                            .with_untracked(|s| matches!(s, Session::Unknown)) =>
                    {
                        return;
                    }
                    _ => Session::Anonymous,
                },
            };
            auth.session.set(session);
        });
    }
}

/// Ask how users sign in, then who is signed in; put the sign-in state in
/// the context.
pub fn provide_auth() {
    let auth = Auth {
        session: RwSignal::new(Session::Unknown),
        mode: RwSignal::new(None),
        checking: StoredValue::new(false),
    };
    provide_context(auth);
    AUTH.set(Some(auth));
    spawn_local(async move {
        let answer = api::client()
            .raw_json(Method::GET, "/auth/mode", &[], None)
            .await;
        let mode = answer
            .ok()
            .and_then(|body| Mode::from_json(&body))
            .unwrap_or_else(Mode::fallback);
        let off = mode.sign_in == SignIn::Off;
        auth.mode.set(Some(mode));
        if off {
            auth.session.set(Session::Anonymous);
        } else {
            auth.refresh();
        }
    });
    // The app's lifetime: the listener stays.
    let _ = window_event_listener(ev::focus, move |_| auth.refresh());
}

pub fn use_auth() -> Auth {
    expect_context::<Auth>()
}

/// Whether the user may manage the object of `owner`: its owner, or an
/// admin (see [`Session::manages`]).
pub fn use_manages(owner: Signal<Option<String>>) -> Memo<bool> {
    let auth = use_auth();
    Memo::new(move |_| {
        auth.session
            .with(|s| owner.with(|o| s.manages(o.as_deref())))
    })
}

/// A call failed with 401 or 403: the session may have ended, so ask the
/// server again. A 401 then shows the login page, a 403 the "No access"
/// page.
pub fn check_again() {
    if let Some(auth) = AUTH.get() {
        auth.refresh();
    }
}

/// Where "Log out" goes, if anywhere: the server's `/auth/logout`, which
/// expires the cookie and returns to the UI or, behind an auth proxy, goes
/// to the proxy's sign-out, which alone ends that session.
pub fn logout_url(auth: Auth) -> Signal<Option<String>> {
    Signal::derive(move || auth.mode.with(|mode| mode.as_ref()?.logout_url.clone()))
}

/// Upstream's `logout`: to `url`.
pub fn logout(url: &str) {
    if let Some(window) = web_sys::window() {
        let _ = window.location().assign(url);
    }
}
