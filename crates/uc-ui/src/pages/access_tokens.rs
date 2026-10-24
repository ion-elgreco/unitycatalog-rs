//! The signed-in user's personal access tokens, from the profile menu: a
//! list with each token's name, first characters, creation and expiry;
//! "Create Token" with a name and an expiry; and Delete. A new token shows
//! once, above the list, with a button that copies it: the server keeps
//! only its hash. Upstream's UI has no such page.

use std::time::Duration;

use js_sys::Date;
use leptos::html;
use leptos::prelude::*;
use leptos::task::spawn_local;
use uc_client::models::tokens::v1::PatInfo;
use uc_common::EpochMillis;
use wasm_bindgen::JsCast;

use crate::api;
use crate::auth::use_auth;
use crate::components::icons::{
    CheckCircleFilled, CheckOutlined, CloseOutlined, CopyOutlined, DeleteOutlined,
};
use crate::components::{Modal, use_notify};
use crate::layout::{Column, ListLayout};
use crate::load::{Key, use_cache, use_load};
use crate::util::format_timestamp;

/// A day, in milliseconds.
const DAY_MS: f64 = 24.0 * 60.0 * 60.0 * 1000.0;

/// How long the copy button says "Copied": antd's.
const COPIED_FOR: Duration = Duration::from_secs(3);

#[component]
pub fn AccessTokens() -> impl IntoView {
    let auth = use_auth();
    let signed_in = move || auth.session.with(|session| session.user().is_some());
    view! {
        <Show
            when=signed_in
            fallback=|| {
                view! {
                    <div class="uc-flex-col uc-gap-md uc-grow">
                        <h2 class="uc-title-2">"Access tokens"</h2>
                        <div class="uc-text-secondary">
                            "Access tokens belong to a signed-in user, and no one is signed in."
                        </div>
                    </div>
                }
            }
        >
            <TokenList/>
        </Show>
    }
}

#[component]
fn TokenList() -> impl IntoView {
    let tokens = use_load(Key::Tokens, api::tokens);
    let create_open = RwSignal::new(false);
    // The new token: it shows until the user closes it or leaves the page.
    let created: RwSignal<Option<PatInfo>> = RwSignal::new(None);
    // The token of the delete question.
    let deleting: RwSignal<Option<PatInfo>> = RwSignal::new(None);

    let columns = vec![
        Column {
            title: "Name",
            width: "28%",
            center: false,
            render: Box::new(|t: &PatInfo| view! { <span>{t.name.clone()}</span> }.into_any()),
            search_key: Some(Box::new(|t: &PatInfo| t.name.clone())),
        },
        Column {
            title: "Token",
            width: "16%",
            center: false,
            render: Box::new(|t: &PatInfo| {
                view! { <code class="uc-text-code">{format!("{}…", t.token_prefix)}</code> }
                    .into_any()
            }),
            search_key: None,
        },
        Column {
            title: "Created At",
            width: "22%",
            center: false,
            render: Box::new(|t: &PatInfo| {
                view! { <span>{format_timestamp(t.created_at)}</span> }.into_any()
            }),
            search_key: None,
        },
        Column {
            title: "Expires At",
            width: "22%",
            center: false,
            render: Box::new(|t: &PatInfo| expiry(t.expires_at)),
            search_key: None,
        },
        Column {
            title: "",
            width: "12%",
            center: false,
            render: Box::new(move |t: &PatInfo| {
                let token = t.clone();
                view! {
                    <button
                        type="button"
                        class="uc-btn uc-btn--text uc-btn--text-danger"
                        on:click=move |_| deleting.set(Some(token.clone()))
                    >
                        <DeleteOutlined/>
                        <span>"Delete"</span>
                    </button>
                }
                .into_any()
            }),
            search_key: None,
        },
    ];

    view! {
        <div class="uc-flex-col uc-gap-md uc-grow">
            <div class="uc-flex uc-justify-between uc-items-start uc-gap-md">
                <h2 class="uc-title-2">"Access tokens"</h2>
                <button class="uc-btn uc-btn--primary" on:click=move |_| create_open.set(true)>
                    "Create Token"
                </button>
            </div>
            <div class="uc-text-secondary">
                "Scripts and tools call the API as you with an access token: they send it in the "
                <code class="uc-text-code">"Authorization: Bearer <token>"</code>
                " header."
            </div>
            <NewToken created/>
            <ListLayout data=tokens columns/>
            <CreateToken open=create_open created/>
            <DeleteToken deleting created/>
        </div>
    }
}

/// The expiry in the list: the time, "Never", or in red once it passed.
fn expiry(expires_at: Option<EpochMillis>) -> AnyView {
    match expires_at {
        None | Some(0) => view! { <span>"Never"</span> }.into_any(),
        Some(ms) if (ms as f64) <= Date::now() => view! {
            <span class="uc-text-danger">{format!("Expired on {}", format_timestamp(ms))}</span>
        }
        .into_any(),
        Some(ms) => view! { <span>{format_timestamp(ms)}</span> }.into_any(),
    }
}

/// The new token, once: the server keeps only its hash.
#[component]
fn NewToken(created: RwSignal<Option<PatInfo>>) -> impl IntoView {
    move || {
        created.get().map(|token| {
            let name = token.name;
            let value = token.token_value.unwrap_or_default();
            view! {
                <div class="uc-alert uc-alert--success uc-alert--large" role="status">
                    <span class="uc-alert__icon">
                        <CheckCircleFilled/>
                    </span>
                    <div class="uc-alert__content">
                        <div class="uc-alert__message">"Copy your new access token now"</div>
                        <div>
                            "You cannot see the token "
                            <span class="uc-text-strong">{name}</span>
                            " again after you leave this page."
                        </div>
                        <TokenField value/>
                    </div>
                    <button
                        type="button"
                        class="uc-alert__close"
                        aria-label="Close"
                        on:click=move |_| created.set(None)
                    >
                        <CloseOutlined/>
                    </button>
                </div>
            }
        })
    }
}

/// The token in a field that selects all of it, and a button that copies
/// it.
#[component]
fn TokenField(value: String) -> impl IntoView {
    let field = NodeRef::<html::Input>::new();
    let copied = RwSignal::new(false);
    // The last copy: only its "Copied" goes back to "Copy".
    let copies = StoredValue::new(0u64);
    let notify = use_notify();
    let select = move || {
        if let Some(field) = field.get_untracked() {
            field.select();
        }
    };
    let copy = {
        let value = value.clone();
        move |_| {
            let value = value.clone();
            select();
            copies.update_value(|n| *n += 1);
            let this = copies.get_value();
            spawn_local(async move {
                if !copy_text(&value).await {
                    notify.error("Failed to copy. The token is selected: press Ctrl+C.");
                    return;
                }
                copied.set(true);
                gloo_timers::future::sleep(COPIED_FOR).await;
                if copies.try_get_value() == Some(this) {
                    copied.set(false);
                }
            });
        }
    };
    view! {
        <div class="uc-token-field">
            <input
                node_ref=field
                class="uc-input"
                type="text"
                readonly
                aria-label="Access token"
                value=value
                on:focus=move |_| select()
            />
            <button type="button" class="uc-btn" on:click=copy>
                {move || match copied.get() {
                    true => view! {
                        <span class="uc-text-success">
                            <CheckOutlined/>
                        </span>
                        <span>"Copied"</span>
                    }
                    .into_any(),
                    false => view! {
                        <CopyOutlined/>
                        <span>"Copy"</span>
                    }
                    .into_any(),
                }}
            </button>
        </div>
    }
}

/// Copy `text`: with the Clipboard API where the browser has it (https and
/// localhost), else with the `copy` command on the selected field.
async fn copy_text(text: &str) -> bool {
    let clipboard = window().navigator().clipboard();
    if !clipboard.is_undefined() {
        let written = clipboard.write_text(text);
        return wasm_bindgen_futures::JsFuture::from(written).await.is_ok();
    }
    document()
        .dyn_into::<web_sys::HtmlDocument>()
        .ok()
        .and_then(|document| document.exec_command("copy").ok())
        .unwrap_or(false)
}

/// When a new token expires.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Expiry {
    /// This many days after its creation.
    Days(u32),
    /// At the end of a date that the user chooses.
    Date,
    Never,
}

impl Expiry {
    const ALL: [Expiry; 5] = [
        Expiry::Days(7),
        Expiry::Days(30),
        Expiry::Days(90),
        Expiry::Date,
        Expiry::Never,
    ];

    /// The choice that the dialog shows first.
    const FIRST: Expiry = Expiry::Days(30);

    fn label(self) -> String {
        match self {
            Expiry::Days(days) => format!("{days} days"),
            Expiry::Date => "Custom".into(),
            Expiry::Never => "Never".into(),
        }
    }
}

#[component]
fn CreateToken(open: RwSignal<bool>, created: RwSignal<Option<PatInfo>>) -> impl IntoView {
    let name = RwSignal::new(String::new());
    let expiry = RwSignal::new(Expiry::FIRST);
    // The date of a custom expiry, as the date field writes it: `2026-12-31`.
    let date = RwSignal::new(String::new());
    let busy = RwSignal::new(false);
    let notify = use_notify();
    let cache = use_cache();
    let close = move || {
        open.set(false);
        name.set(String::new());
        expiry.set(Expiry::FIRST);
        date.set(String::new());
    };
    let on_ok = move || {
        let new_name = name.get_untracked().trim().to_string();
        if new_name.is_empty() {
            notify.error("Name is required");
            return;
        }
        let expires_at = match expiry.get_untracked() {
            Expiry::Days(days) => Some(Date::now() + f64::from(days) * DAY_MS),
            Expiry::Date => match end_of(&date.get_untracked()) {
                Some(end) if end > Date::now() => Some(end),
                Some(_) => {
                    notify.error("The date has passed");
                    return;
                }
                None => {
                    notify.error("Expiration date is required");
                    return;
                }
            },
            Expiry::Never => None,
        };
        let create = api::client()
            .create_token(new_name)
            .with_expires_at(expires_at.map(|ms| ms as EpochMillis));
        busy.set(true);
        spawn_local(async move {
            match create.await {
                Ok(token) => {
                    cache.changed(Key::Tokens);
                    // Only when the user is still on the page.
                    if busy.try_get_untracked().is_some() {
                        created.set(Some(token));
                        close();
                    }
                }
                Err(e) => notify.failed(&e, "Failed to create access token"),
            }
            busy.set(false);
        });
    };
    view! {
        <Modal
            open
            title="Create access token"
            ok_text="Create"
            busy
            on_cancel=move || close()
            on_ok
        >
            <div class="uc-form-item">
                <label>
                    <span class="req">"*"</span>
                    "Name"
                </label>
                <input
                    class="uc-input"
                    type="text"
                    prop:value=move || name.get()
                    on:input=move |e| name.set(event_target_value(&e))
                />
            </div>
            <div class="uc-form-item">
                <label>"Expiration"</label>
                <div class="uc-radio-group" role="radiogroup" aria-label="Expiration">
                    {Expiry::ALL
                        .into_iter()
                        .map(|choice| {
                            view! {
                                <button
                                    type="button"
                                    role="radio"
                                    aria-checked=move || (expiry.get() == choice).to_string()
                                    class:is-checked=move || expiry.get() == choice
                                    on:click=move |_| expiry.set(choice)
                                >
                                    {choice.label()}
                                </button>
                            }
                        })
                        .collect_view()}
                </div>
                {move || {
                    (expiry.get() == Expiry::Date)
                        .then(|| {
                            view! {
                                <input
                                    class="uc-input uc-form-hint"
                                    type="date"
                                    aria-label="Expiration date"
                                    min=today()
                                    prop:value=move || date.get()
                                    on:input=move |e| date.set(event_target_value(&e))
                                />
                            }
                        })
                }}
                <ExpiryNote expiry date/>
            </div>
        </Modal>
    }
}

/// When the new token expires, under the choice; for none, a warning.
#[component]
fn ExpiryNote(expiry: RwSignal<Expiry>, date: RwSignal<String>) -> impl IntoView {
    let on = |at: f64| {
        format!(
            "The token expires on {}.",
            format_timestamp(at as EpochMillis)
        )
    };
    move || match expiry.get() {
        Expiry::Days(days) => {
            let text = on(Date::now() + f64::from(days) * DAY_MS);
            view! { <div class="uc-form-hint uc-text-secondary">{text}</div> }.into_any()
        }
        Expiry::Date => {
            let text = match date.with(|date| end_of(date)) {
                Some(end) => on(end),
                None => "The token works until the end of the date.".to_string(),
            };
            view! { <div class="uc-form-hint uc-text-secondary">{text}</div> }.into_any()
        }
        Expiry::Never => view! {
            <div class="uc-alert uc-alert--warning uc-form-hint">
                "The token never expires. Delete it when you no longer need it."
            </div>
        }
        .into_any(),
    }
}

/// The last millisecond of `date` (`2026-12-31`, from the date field) in
/// the browser's time zone: the token works until the end of that day.
fn end_of(date: &str) -> Option<f64> {
    let mut parts = date.splitn(3, '-').map(|part| part.parse::<i32>().ok());
    let (year, month, day) = (parts.next()??, parts.next()??, parts.next()??);
    let year = u32::try_from(year).ok()?;
    let end = Date::new_with_year_month_day_hr_min_sec_milli(year, month - 1, day, 23, 59, 59, 999);
    let ms = end.get_time();
    (!ms.is_nan()).then_some(ms)
}

/// Today in the browser's time zone, as the date field writes it: the
/// first date that it offers.
fn today() -> String {
    let now = Date::new_0();
    format!(
        "{:04}-{:02}-{:02}",
        now.get_full_year(),
        now.get_month() + 1,
        now.get_date()
    )
}

/// The delete question for the token in `deleting`.
#[component]
fn DeleteToken(
    deleting: RwSignal<Option<PatInfo>>,
    created: RwSignal<Option<PatInfo>>,
) -> impl IntoView {
    let busy = RwSignal::new(false);
    let notify = use_notify();
    let cache = use_cache();
    let on_ok = move || {
        let Some(PatInfo { id, name, .. }) = deleting.get_untracked() else {
            return;
        };
        busy.set(true);
        spawn_local(async move {
            match api::client().token(&id).delete().await {
                Ok(()) => {
                    notify.success(format!("{name} token successfully deleted"));
                    cache.changed(Key::Tokens);
                    // Only when the user is still on the page.
                    if busy.try_get_untracked().is_some() {
                        deleting.set(None);
                        let shown =
                            created.with_untracked(|t| t.as_ref().is_some_and(|t| t.id == id));
                        if shown {
                            created.set(None);
                        }
                    }
                }
                Err(e) => notify.failed(&e, "Failed to delete access token"),
            }
            busy.set(false);
        });
    };
    let name = move || {
        deleting
            .try_with(|t| t.as_ref().map(|t| t.name.clone()))
            .flatten()
            .unwrap_or_default()
    };
    view! {
        <Modal
            open=Signal::derive(move || deleting.try_with(Option::is_some).unwrap_or(false))
            title="Delete access token"
            ok_text="Delete"
            danger=true
            busy
            on_cancel=move || deleting.set(None)
            on_ok
        >
            <span>"Are you sure you want to delete the token "</span>
            <span class="uc-text-strong">{name}</span>
            <span>
                "? Scripts and tools that use it can no longer call the API. This operation cannot be undone."
            </span>
        </Modal>
    }
}
