//! A registered model's page, as the console's `ModelDetails`: its
//! description and versions, its details, its grants; "Edit", "Create
//! version" and "Delete".

use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::hooks::use_navigate;
use uc_api::{
    CreateModelVersion, ModelVersionInfo, ModelVersionStatus, RegisteredModelInfo,
    UpdateRegisteredModel,
};
use uc_common::{SecurableType, three_part_full_name, wire_name};

use crate::api;
use crate::auth::use_auth;
use crate::components::edit_modal::Changes;
use crate::components::icons::{Box, CircleCheck, CircleMinus, CircleX, Plus, Trash2};
use crate::components::{
    Card, DeleteModal, DescriptionCard, EditMetadata, EntityHeader, LoadError, Loaded,
    LoadingBlock, MetaGrid, MetaRow, Modal, PermissionsCard, RawJsonCard, Tab, Tabs, use_notify,
};
use crate::contents::Holder;
use crate::kind::Kind;
use crate::load::{Key, Load, field, use_cache, use_load};
use crate::util::{format_timestamp, or_dash, pretty_json, route_params};

#[component]
pub fn ModelDetails() -> impl IntoView {
    let params = route_params(["catalog", "schema", "model"]);
    move || {
        let [catalog, schema, model] = params.get();
        view! { <ModelPage catalog schema model/> }
    }
}

/// The page of version `version` of the model at `page`.
pub fn version_href(catalog: &str, schema: &str, model: &str, version: i64) -> String {
    format!(
        "{}/version/{version}",
        Kind::Models.href(catalog, schema, model)
    )
}

#[component]
fn ModelPage(catalog: String, schema: String, model: String) -> impl IntoView {
    let full_name = three_part_full_name(&catalog, &schema, &model);
    let names = StoredValue::new((
        catalog.clone(),
        schema.clone(),
        model.clone(),
        full_name.clone(),
    ));
    let info = use_load(Key::Model(full_name.clone()), move || {
        let (_, _, _, full_name) = names.get_value();
        async move { api::client().get_registered_model(&full_name).await }
    });
    let versions = use_load(Key::Versions(full_name.clone()), move || {
        let (_, _, _, full_name) = names.get_value();
        async move { api::model_versions(&full_name).await }
    });
    let cache = use_cache();
    let auth = use_auth();
    let tab = RwSignal::new(Tab::Overview);
    let delete_open = RwSignal::new(false);
    let create_open = RwSignal::new(false);
    let owner = field(info, |m: &RegisteredModelInfo| m.owner.clone());
    let manages = Memo::new(move |_| {
        auth.session
            .with(|s| owner.with(|o| s.manages(o.as_deref())))
    });
    let comment = field(info, |m: &RegisteredModelInfo| {
        m.comment.clone().unwrap_or_default()
    });

    let save_comment = move |comment: String| {
        let (_, _, _, full_name) = names.get_value();
        async move {
            let request = UpdateRegisteredModel {
                comment: Some(comment),
                new_name: None,
            };
            let updated = api::client()
                .update_registered_model(&full_name, &request)
                .await?;
            cache.set(Key::Model(full_name), updated);
            Ok(())
        }
    };
    let save_metadata = move |changes: Changes| {
        let (catalog, schema, _, full_name) = names.get_value();
        async move {
            let request = UpdateRegisteredModel {
                comment: changes.comment,
                new_name: changes.new_name.clone(),
            };
            let updated = api::client()
                .update_registered_model(&full_name, &request)
                .await?;
            match changes.new_name {
                Some(new_name) => {
                    cache.forget(Key::Model(full_name));
                    cache.changed(Key::Entries(catalog.clone(), schema.clone(), Kind::Models));
                    cache.set(
                        Key::Model(three_part_full_name(&catalog, &schema, &new_name)),
                        updated,
                    );
                    Ok(Some(Kind::Models.href(&catalog, &schema, &new_name)))
                }
                None => {
                    cache.set(Key::Model(full_name), updated);
                    Ok(None)
                }
            }
        }
    };
    let refresh = Callback::new(move |_| {
        let (_, _, _, full_name) = names.get_value();
        cache.changed(Key::Model(full_name.clone()));
        cache.changed(Key::Versions(full_name));
    });

    let actions = view! {
        <EditMetadata name=model.clone() comment noun="model" save=save_metadata/>
        <button type="button" class="uc-btn uc-btn--sm" on:click=move |_| create_open.set(true)>
            <Plus/>
            "Create version"
        </button>
        <Show when=move || manages.get()>
            <button
                type="button"
                class="uc-btn uc-btn--destructive uc-btn--sm"
                on:click=move |_| delete_open.set(true)
            >
                <Trash2/>
                "Delete"
            </button>
        </Show>
    }
    .into_any();

    let version_rows = move || {
        match versions.get() {
        Load::Loading => view! { <LoadingBlock/> }.into_any(),
        Load::Failed(message) => view! { <LoadError message/> }.into_any(),
        Load::Ready(list) if list.is_empty() => {
            view! { <p class="uc-empty">"No versions."</p> }.into_any()
        }
        Load::Ready(list) => view! {
            <div class="uc-table-wrap">
                <table class="uc-table">
                    <thead>
                        <tr>
                            <th>"Version"</th>
                            <th>"Status"</th>
                            <th>"Source"</th>
                            <th>"Created"</th>
                        </tr>
                    </thead>
                    <tbody>
                        {list
                            .into_iter()
                            .map(|v| {
                                let (catalog, schema, model, _) = names.get_value();
                                let number = v.version.unwrap_or_default();
                                view! {
                                    <tr>
                                        <td class="uc-strong">
                                            <a class="uc-btn uc-btn--link" href=version_href(&catalog, &schema, &model, number)>
                                                {format!("v{number}")}
                                            </a>
                                        </td>
                                        <td>{status(v.status.as_ref())}</td>
                                        <td class="uc-mono">{or_dash(v.source)}</td>
                                        <td class="uc-muted">{format_timestamp(v.created_at)}</td>
                                    </tr>
                                }
                            })
                            .collect_view()}
                    </tbody>
                </table>
            </div>
        }
        .into_any(),
    }
    };
    let version_count = move || {
        versions
            .with(|v| v.ready().map(Vec::len))
            .map(|n| n.to_string())
            .unwrap_or_default()
    };

    let content = move || match tab.get() {
        Tab::Overview => view! {
            <DescriptionCard comment noun="model" save=save_comment/>
            <Card
                title="Versions"
                actions=view! { <span class="uc-badge">{version_count}</span> }.into_any()
            >
                {version_rows}
            </Card>
        }
        .into_any(),
        Tab::Details => view! {
            <Card>
                <MetaGrid rows=field(info, model_rows)/>
            </Card>
            <RawJsonCard json=field(info, pretty_json)/>
        }
        .into_any(),
        Tab::Permissions => {
            let (_, _, _, full_name) = names.get_value();
            view! { <PermissionsCard kind=SecurableType::RegisteredModel full_name owner/> }
                .into_any()
        }
    };

    view! {
        <EntityHeader
            icon=view! { <Box class="uc-icon--model"/> }.into_any()
            name=full_name.clone()
            badges=vec!["MODEL".to_string()]
            on_refresh=refresh
            actions
        />
        <div class="uc-page">
            <Loaded state=info>
                <Tabs tab/>
                {content}
            </Loaded>
        </div>
        <DeleteModal
            open=delete_open
            noun="model"
            question="Are you sure you want to delete the model"
            name=model.clone()
            label=format!("{model} model")
            delete=move |force| {
                let (_, _, _, full_name) = names.get_value();
                async move { api::client().delete_registered_model(&full_name, force).await }
            }
            key=Key::Model(full_name.clone())
            list=Key::Entries(catalog.clone(), schema.clone(), Kind::Models)
            then=Kind::Models.tab_href(&catalog, &schema)
            holder=Holder::Model(full_name.clone())
            blocked=Signal::derive(move || {
                versions
                    .with(|v| v.ready().is_some_and(|v| !v.is_empty()))
                    .then_some(
                        "Registered models cannot be deleted with existing model versions. \
                         Delete model versions and try again.",
                    )
            })
        />
        <CreateVersion open=create_open catalog schema model/>
    }
}

/// A version's status: an icon in green, gray or red, and its name.
pub fn status(status: Option<&ModelVersionStatus>) -> AnyView {
    let (class, icon) = match status {
        Some(ModelVersionStatus::Ready) => (
            "uc-inline uc-status--ready",
            view! { <CircleCheck/> }.into_any(),
        ),
        Some(ModelVersionStatus::PendingRegistration) => (
            "uc-inline uc-status--pending",
            view! { <CircleMinus/> }.into_any(),
        ),
        Some(ModelVersionStatus::FailedRegistration) => (
            "uc-inline uc-status--failed",
            view! { <CircleX/> }.into_any(),
        ),
        _ => ("uc-inline uc-muted", ().into_any()),
    };
    let text = status.map(wire_name).unwrap_or_else(|| "UNKNOWN".into());
    view! {
        <span class=class>
            {icon}
            <span>{text}</span>
        </span>
    }
    .into_any()
}

/// "Create version": the source, a run id and a comment.
#[component]
fn CreateVersion(
    open: RwSignal<bool>,
    catalog: String,
    schema: String,
    model: String,
) -> impl IntoView {
    let names = StoredValue::new((catalog, schema, model));
    let source = RwSignal::new(String::new());
    let run_id = RwSignal::new(String::new());
    let comment = RwSignal::new(String::new());
    let busy = RwSignal::new(false);
    let notify = use_notify();
    let cache = use_cache();
    let navigate = use_navigate();
    let close = move || {
        open.set(false);
        source.set(String::new());
        run_id.set(String::new());
        comment.set(String::new());
    };
    let on_ok = move || {
        let from = source.get_untracked().trim().to_string();
        if from.is_empty() {
            notify.error("Source is required");
            return;
        }
        let (catalog, schema, model) = names.get_value();
        let run = run_id.get_untracked().trim().to_string();
        let text = comment.get_untracked();
        let request = CreateModelVersion {
            model_name: model.clone(),
            catalog_name: catalog.clone(),
            schema_name: schema.clone(),
            source: from,
            run_id: (!run.is_empty()).then_some(run),
            comment: (!text.is_empty()).then_some(text),
        };
        let navigate = navigate.clone();
        busy.set(true);
        spawn_local(async move {
            match api::client().create_model_version(&request).await {
                Ok(version) => {
                    let number = version.version.unwrap_or_default();
                    notify.success(format!("Created version {number}"));
                    cache.changed(Key::Versions(three_part_full_name(
                        &catalog, &schema, &model,
                    )));
                    if busy.try_get_untracked().is_some() {
                        close();
                        navigate(
                            &version_href(&catalog, &schema, &model, number),
                            Default::default(),
                        );
                    }
                }
                Err(e) => notify.failed(&e, "Failed to create model version"),
            }
            busy.set(false);
        });
    };
    view! {
        <Modal
            open
            title="Create model version"
            ok_text="Create"
            busy
            on_cancel=move || close()
            on_ok
        >
            <div class="uc-field">
                <label class="uc-label" for="uc-version-source">
                    <span class="req">"*"</span>
                    "Source"
                </label>
                <input
                    id="uc-version-source"
                    class="uc-input"
                    type="text"
                    required
                    placeholder="s3://bucket/model"
                    prop:value=move || source.get()
                    on:input=move |e| source.set(event_target_value(&e))
                />
            </div>
            <div class="uc-field">
                <label class="uc-label" for="uc-version-run">"Run ID"</label>
                <input
                    id="uc-version-run"
                    class="uc-input"
                    type="text"
                    prop:value=move || run_id.get()
                    on:input=move |e| run_id.set(event_target_value(&e))
                />
            </div>
            <div class="uc-field">
                <label class="uc-label" for="uc-version-comment">"Comment"</label>
                <textarea
                    id="uc-version-comment"
                    class="uc-textarea"
                    prop:value=move || comment.get()
                    on:input=move |e| comment.set(event_target_value(&e))
                />
            </div>
        </Modal>
    }
}

/// The model's details, which the version page shows too.
pub fn model_rows(m: &RegisteredModelInfo) -> Vec<MetaRow> {
    vec![
        MetaRow::new("Name", or_dash(m.name.clone())),
        MetaRow::new("Owner", or_dash(m.owner.clone())),
        MetaRow::new("Created at", format_timestamp(m.created_at)),
        MetaRow::new("Created by", or_dash(m.created_by.clone())),
        MetaRow::new("Updated at", format_timestamp(m.updated_at)),
        MetaRow::new("Updated by", or_dash(m.updated_by.clone())),
        MetaRow::mono("Model ID", or_dash(m.id.clone())),
        MetaRow::mono("Storage location", or_dash(m.storage_location.clone())),
    ]
}

/// Whether a version still waits for its files.
pub fn is_pending(v: &ModelVersionInfo) -> bool {
    matches!(v.status, Some(ModelVersionStatus::PendingRegistration))
}
