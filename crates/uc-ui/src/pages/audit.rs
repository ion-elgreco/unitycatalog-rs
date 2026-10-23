//! The audit log, from "Manage" in the sidebar, for a signed-in admin: the
//! calls that changed the catalog, newest first, with their principal,
//! method, path and status. A filter by principal, method, path and time;
//! "Load more" for the next page.

use js_sys::Date;
use leptos::prelude::*;
use leptos::task::spawn_local;
use uc_api::AuditEvent;
use uc_client::AuditFilter;
use uc_common::EpochMillis;

use crate::api;
use crate::components::{
    AdminOnly, Empty, EmptyState, LoadError, LoadingBlock, Select, TextField, use_notify,
};
use crate::load::Load;
use crate::util::format_timestamp;

/// The events of one call.
const PAGE: i32 = 50;

/// The methods that the log records: the ones that change the catalog.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Method {
    Any,
    Post,
    Put,
    Patch,
    Delete,
}

impl Method {
    const ALL: [Method; 5] = [
        Method::Any,
        Method::Post,
        Method::Put,
        Method::Patch,
        Method::Delete,
    ];

    /// The method's name in the log; none for any.
    fn name(self) -> Option<&'static str> {
        match self {
            Method::Any => None,
            Method::Post => Some("POST"),
            Method::Put => Some("PUT"),
            Method::Patch => Some("PATCH"),
            Method::Delete => Some("DELETE"),
        }
    }

    fn text(self) -> String {
        self.name().unwrap_or("Any").to_string()
    }
}

#[component]
pub fn AuditLog() -> impl IntoView {
    view! {
        <AdminOnly title="Audit log" signed_in=true>
            <AuditList/>
        </AdminOnly>
    }
}

/// A time from a `datetime-local` field, in epoch milliseconds; none for
/// an empty field.
fn time_of(text: &str) -> Option<i64> {
    if text.is_empty() {
        return None;
    }
    let ms = Date::parse(text);
    (!ms.is_nan()).then_some(ms as i64)
}

/// A minute, in milliseconds.
const MINUTE_MS: i64 = 60 * 1000;

#[component]
fn AuditList() -> impl IntoView {
    let principal = RwSignal::new(String::new());
    let method = RwSignal::new(Method::Any);
    let path = RwSignal::new(String::new());
    let since = RwSignal::new(String::new());
    let until = RwSignal::new(String::new());
    let events: RwSignal<Load<Vec<AuditEvent>>> = RwSignal::new(Load::Loading);
    // The token of the next page, when there is one.
    let next: RwSignal<Option<String>> = RwSignal::new(None);
    let loading_more = RwSignal::new(false);
    // The last load: only its answer counts.
    let loads = StoredValue::new(0u64);
    let notify = use_notify();
    let filter = move || AuditFilter {
        principal: Some(principal.get_untracked().trim().to_string()).filter(|p| !p.is_empty()),
        method: method.get_untracked().name().map(str::to_string),
        path_prefix: Some(path.get_untracked().trim().to_string()).filter(|p| !p.is_empty()),
        since: time_of(&since.get_untracked()),
        // The field names a minute: through its end.
        until: time_of(&until.get_untracked()).map(|t| t + MINUTE_MS - 1),
        max_results: Some(PAGE),
        page_token: None,
    };
    // The first page for the filter, or the page after `token`.
    let load = move |token: Option<String>| {
        loads.update_value(|n| *n += 1);
        let this = loads.get_value();
        let mut filter = filter();
        filter.page_token = token.clone();
        let more = token.is_some();
        match more {
            true => loading_more.set(true),
            false => events.set(Load::Loading),
        }
        spawn_local(async move {
            let answer = api::client().list_audit_events(&filter).await;
            if loads.try_get_value() != Some(this) {
                return;
            }
            match answer {
                Ok(list) => {
                    next.set(list.next_page_token.map(|t| t.0).filter(|t| !t.is_empty()));
                    if more {
                        events.update(|events| {
                            if let Load::Ready(items) = events {
                                items.extend(list.events);
                            }
                        });
                    } else {
                        events.set(Load::Ready(list.events));
                    }
                }
                Err(e) if more => notify.failed(&e, "Failed to load more events"),
                Err(e) => events.set(Load::Failed(api::failure(
                    &e,
                    "Failed to list audit events",
                ))),
            }
            loading_more.set(false);
        });
    };
    load(None);
    let apply = move |e: leptos::ev::SubmitEvent| {
        e.prevent_default();
        load(None);
    };
    let empty = Empty::new(
        "No events.",
        "The log records every call that changes the catalog: creates, edits, grants and deletes.",
    );
    let table = move || match events.get() {
        Load::Loading => view! { <LoadingBlock/> }.into_any(),
        Load::Failed(message) => view! { <LoadError message/> }.into_any(),
        Load::Ready(list) if list.is_empty() => {
            view! { <EmptyState empty=empty.clone()/> }.into_any()
        }
        Load::Ready(list) => view! {
            <div class="uc-table-wrap">
                <table class="uc-table">
                    <thead>
                        <tr>
                            <th>"Time"</th>
                            <th>"Principal"</th>
                            <th>"Method"</th>
                            <th>"Path"</th>
                            <th>"Status"</th>
                        </tr>
                    </thead>
                    <tbody>
                        {list.into_iter().map(|event| view! { <EventRow event/> }).collect_view()}
                    </tbody>
                </table>
            </div>
        }
        .into_any(),
    };
    view! {
        <div class="uc-page">
            <h1 class="uc-page__title">"Audit log"</h1>
            <p class="uc-muted">
                "The calls that changed the catalog, the newest first: who made each one, what it did, and how the server answered."
            </p>
            <form class="uc-form-row uc-form-row--filter" on:submit=apply>
                <TextField
                    id="uc-audit-principal"
                    label="Principal"
                    value=principal
                    placeholder="ada@example.com"
                    autocomplete="off"
                />
                <Select
                    id="uc-audit-method"
                    label="Method"
                    choices=&Method::ALL
                    value=method
                    name=|m: &Method| m.text()
                    fixed=true
                />
                <TextField
                    id="uc-audit-path"
                    label="Path starts with"
                    value=path
                    placeholder="/api/2.1/unity-catalog/tables"
                    autocomplete="off"
                />
                <TimeField id="uc-audit-since" label="From" value=since/>
                <TimeField id="uc-audit-until" label="To" value=until/>
                <button type="submit" class="uc-btn">"Apply"</button>
            </form>
            {table}
            {move || {
                next.get()
                    .map(|token| {
                        view! {
                            <button
                                type="button"
                                class="uc-btn uc-btn--outline uc-self-start"
                                disabled=move || loading_more.get()
                                on:click=move |_| load(Some(token.clone()))
                            >
                                {move || if loading_more.get() { "Loading…" } else { "Load more" }}
                            </button>
                        }
                    })
            }}
        </div>
    }
}

/// A date and a time, in the browser's time zone.
#[component]
fn TimeField(id: &'static str, label: &'static str, value: RwSignal<String>) -> impl IntoView {
    view! {
        <div class="uc-field uc-field--fixed">
            <label class="uc-label" for=id>{label}</label>
            <input
                id=id
                class="uc-input"
                type="datetime-local"
                prop:value=move || value.get()
                on:input=move |e| value.set(event_target_value(&e))
            />
        </div>
    }
}

#[component]
fn EventRow(event: AuditEvent) -> impl IntoView {
    let status_class = match event.status {
        200..=399 => "uc-badge uc-badge--success",
        _ => "uc-badge uc-badge--destructive",
    };
    view! {
        <tr>
            <td class="uc-muted uc-nowrap">{format_timestamp(Some(event.timestamp as EpochMillis))}</td>
            <td class="uc-strong uc-nowrap">{event.principal}</td>
            <td>
                <span class="uc-badge">{event.method}</span>
            </td>
            <td class="uc-mono">{event.path}</td>
            <td>
                <span class=status_class>{event.status}</span>
            </td>
        </tr>
    }
}
