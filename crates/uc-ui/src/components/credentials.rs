//! The Credentials card of a table or volume, as upstream's console: a
//! button vends temporary credentials for the object's storage location,
//! and the card shows them, with their expiry. Secrets stay hidden until
//! shown, and each value has a copy button.

use leptos::prelude::*;
use leptos::task::spawn_local;
use uc_api::{
    GenerateTemporaryTableCredential, GenerateTemporaryVolumeCredential, TableOperation,
    TemporaryCredentials, VolumeOperation,
};

use super::card::Card;
use super::icons::{Copy, Eye, EyeOff, KeyRound, LoaderCircle};
use crate::api;
use crate::components::use_notify;
use crate::util::{copy_text, format_timestamp};

/// What the card vends credentials for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Vendable {
    Table,
    Volume,
}

impl Vendable {
    fn noun(self) -> &'static str {
        match self {
            Vendable::Table => "table",
            Vendable::Volume => "volume",
        }
    }

    /// The text of the button that vends read-write credentials.
    fn write_label(self) -> &'static str {
        match self {
            Vendable::Table => "Vend READ WRITE creds",
            Vendable::Volume => "Vend WRITE creds",
        }
    }

    async fn vend(self, id: String, write: bool) -> api::Result<TemporaryCredentials> {
        let client = api::client();
        match self {
            Vendable::Table => {
                let request = GenerateTemporaryTableCredential {
                    table_id: id,
                    operation: match write {
                        true => TableOperation::ReadWrite,
                        false => TableOperation::Read,
                    },
                };
                client.generate_temporary_table_credentials(&request).await
            }
            Vendable::Volume => {
                let request = GenerateTemporaryVolumeCredential {
                    volume_id: id,
                    operation: match write {
                        true => VolumeOperation::WriteVolume,
                        false => VolumeOperation::ReadVolume,
                    },
                };
                client.generate_temporary_volume_credentials(&request).await
            }
        }
    }
}

#[component]
pub fn CredentialsCard(kind: Vendable, #[prop(into)] id: Signal<Option<String>>) -> impl IntoView {
    let notify = use_notify();
    let vended: RwSignal<Option<(bool, TemporaryCredentials)>> = RwSignal::new(None);
    // Which button's call runs: `Some(write)`.
    let busy: RwSignal<Option<bool>> = RwSignal::new(None);
    let vend = move |write: bool| {
        let Some(id) = id.get_untracked() else {
            notify.error(format!("The {} has no id.", kind.noun()));
            return;
        };
        busy.set(Some(write));
        spawn_local(async move {
            match kind.vend(id, write).await {
                Ok(credentials) => {
                    if vended.try_get_untracked().is_some() {
                        vended.set(Some((write, credentials)));
                    }
                }
                Err(e) => notify.failed(&e, "Failed to vend credentials"),
            }
            if busy.try_get_untracked().is_some() {
                busy.set(None);
            }
        });
    };
    let button = move |write: bool, label: &'static str, class: &'static str| {
        view! {
            <button
                type="button"
                class=class
                disabled=move || busy.get().is_some()
                on:click=move |_| vend(write)
            >
                <Show when=move || busy.get() == Some(write)>
                    <LoaderCircle class="uc-spin"/>
                </Show>
                {label}
            </button>
        }
    };
    view! {
        <Card
            icon=view! { <KeyRound/> }.into_any()
            title="Credentials"
            actions=view! {
                {button(false, "Vend READ creds", "uc-btn")}
                {button(true, kind.write_label(), "uc-btn uc-btn--outline")}
            }
                .into_any()
        >
            <p class="uc-muted">
                {format!(
                    "Vend temporary credentials scoped to this {}'s storage location. \
                     They work until they expire; the server logs who asked.",
                    kind.noun(),
                )}
            </p>
            {move || {
                vended
                    .get()
                    .map(|(write, credentials)| view! { <Vended write credentials/> })
            }}
        </Card>
    }
}

/// The vended credentials.
#[component]
fn Vended(write: bool, credentials: TemporaryCredentials) -> impl IntoView {
    let operation = if write { "read and write" } else { "read" };
    let aws = credentials.aws_temp_credentials;
    let sas = credentials
        .azure_user_delegation_sas
        .and_then(|sas| sas.sas_token);
    let gcp = credentials.gcp_oauth_token.and_then(|t| t.oauth_token);
    view! {
        <dl class="uc-meta">
            <dt>"Operation"</dt>
            <dd>{operation}</dd>
            <dt>"Expires"</dt>
            <dd>{format_timestamp(credentials.expiration_time)}</dd>
            {credentials
                .url
                .map(|url| {
                    view! {
                        <dt>"URL"</dt>
                        <dd>
                            <Plain value=url/>
                        </dd>
                    }
                })}
            {aws
                .map(|aws| {
                    view! {
                        <dt>"Access key ID"</dt>
                        <dd>
                            <Plain value=aws.access_key_id.unwrap_or_default()/>
                        </dd>
                        <dt>"Secret access key"</dt>
                        <dd>
                            <Secret value=aws.secret_access_key.unwrap_or_default()/>
                        </dd>
                        <dt>"Session token"</dt>
                        <dd>
                            <Secret value=aws.session_token.unwrap_or_default()/>
                        </dd>
                    }
                })}
            {sas
                .map(|sas| {
                    view! {
                        <dt>"SAS token"</dt>
                        <dd>
                            <Secret value=sas/>
                        </dd>
                    }
                })}
            {gcp
                .map(|token| {
                    view! {
                        <dt>"OAuth token"</dt>
                        <dd>
                            <Secret value=token/>
                        </dd>
                    }
                })}
        </dl>
    }
}

/// A value in the code font, with its copy button.
#[component]
fn Plain(value: String) -> impl IntoView {
    view! {
        <div class="uc-secret">
            <code title=value.clone()>{value.clone()}</code>
            <CopyButton value/>
        </div>
    }
}

/// A secret: dots until shown, with its copy button.
#[component]
fn Secret(value: String) -> impl IntoView {
    let shown = RwSignal::new(false);
    let text = {
        let value = value.clone();
        move || match shown.get() {
            true => value.clone(),
            false => "•".repeat(24),
        }
    };
    view! {
        <div class="uc-secret">
            <code>{text}</code>
            <button
                type="button"
                class="uc-btn uc-btn--ghost uc-btn--icon uc-btn--xs"
                title=move || if shown.get() { "Hide" } else { "Show" }
                aria-label=move || if shown.get() { "Hide" } else { "Show" }
                on:click=move |_| shown.update(|s| *s = !*s)
            >
                <Show when=move || shown.get() fallback=|| view! { <Eye/> }>
                    <EyeOff/>
                </Show>
            </button>
            <CopyButton value/>
        </div>
    }
}

/// Copies `value`, and says so in a toast.
#[component]
pub fn CopyButton(value: String) -> impl IntoView {
    let notify = use_notify();
    let copy = move |_| {
        let value = value.clone();
        spawn_local(async move {
            match copy_text(&value).await {
                true => notify.success("Copied"),
                false => notify.error("Failed to copy"),
            }
        });
    };
    view! {
        <button
            type="button"
            class="uc-btn uc-btn--ghost uc-btn--icon uc-btn--xs"
            title="Copy"
            aria-label="Copy"
            on:click=copy
        >
            <Copy/>
        </button>
    }
}
