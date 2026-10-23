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
use crate::auth::use_manages;
use crate::components::edit_modal::Changes;
use crate::components::icons::{CircleCheck, CircleMinus, CircleX, Plus};
use crate::components::{
    Card, DeleteButton, DeleteModal, DescriptionCard, DetailsTab, EditMetadata, EntityHeader,
    Loaded, MetaRow, Modal, PermissionsCard, Tabs, TextArea, TextField, count, loaded_list,
    use_notify,
};
use crate::contents::Holder;
use crate::kind::Kind;
use crate::load::{Key, field, use_cache, use_load};
use crate::util::{format_timestamp, or_dash, pretty_json, route_params};

#[component]
pub fn ModelDetails() -> impl IntoView {
    let params = route_params(["catalog", "schema", "model"]);
    move || {
        let [catalog, schema, model] = params.get();
        view! { <ModelPage catalog schema model/> }
    }
}

/// The page of version `version` of the model.
fn version_href(catalog: &str, schema: &str, model: &str, version: i64) -> String {
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
    let delete_open = RwSignal::new(false);
    let create_open = RwSignal::new(false);
    let owner = field(info, |m: &RegisteredModelInfo| m.owner.clone());
    let manages = use_manages(owner);
    let comment = field(info, |m: &RegisteredModelInfo| {
        m.comment.clone().unwrap_or_default()
    });

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
            let page = changes
                .new_name
                .as_deref()
                .map(|new_name| Kind::Models.href(&catalog, &schema, new_name));
            let renamed = changes.new_name.map(|new_name| {
                (
                    Key::Model(three_part_full_name(&catalog, &schema, &new_name)),
                    Key::Entries(catalog.clone(), schema.clone(), Kind::Models),
                )
            });
            cache.edited(Key::Model(full_name), updated, renamed);
            Ok(page)
        }
    };
    let save_comment = move |comment: String| {
        let saving = save_metadata(Changes {
            new_name: None,
            comment: Some(comment),
        });
        async move { saving.await.map(|_| ()) }
    };
    let refresh = Callback::new(move |_| {
        let (_, _, _, full_name) = names.get_value();
        cache.changed(Key::Model(full_name.clone()));
        cache.changed(Key::Versions(full_name));
    });

    let version_rows = move || {
        loaded_list(versions.get(), "No versions.", |list| {
            let (catalog, schema, model, _) = names.get_value();
            view! {
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
            .into_any()
        })
    };

    view! {
        <EntityHeader
            icon=Kind::Models.icon()
            name=full_name.clone()
            badges=vec!["MODEL".to_string()]
            on_refresh=refresh
            actions=view! {
                <EditMetadata name=model.clone() comment noun="model" save=save_metadata/>
                <button type="button" class="uc-btn uc-btn--sm" on:click=move |_| create_open.set(true)>
                    <Plus/>
                    "Create version"
                </button>
                <DeleteButton open=delete_open when=manages/>
            }
                .into_any()
        />
        <div class="uc-page">
            <Loaded state=info>
                <Tabs
                    overview=move || {
                        view! {
                            <DescriptionCard comment noun="model" save=save_comment/>
                            <Card
                                title="Versions"
                                actions=view! { <span class="uc-badge">{count(versions)}</span> }.into_any()
                            >
                                {version_rows}
                            </Card>
                        }
                    }
                    details=move || {
                        view! { <DetailsTab rows=field(info, model_rows) json=field(info, pretty_json)/> }
                    }
                    permissions=move || {
                        let (_, _, _, full_name) = names.get_value();
                        view! { <PermissionsCard kind=SecurableType::RegisteredModel full_name owner/> }
                    }
                />
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
fn status(status: Option<&ModelVersionStatus>) -> AnyView {
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
            <TextField
                id="uc-version-source"
                label="Source"
                value=source
                required=true
                placeholder="s3://bucket/model"
            />
            <TextField id="uc-version-run" label="Run ID" value=run_id/>
            <TextArea id="uc-version-comment" label="Comment" value=comment/>
        </Modal>
    }
}

fn model_rows(m: &RegisteredModelInfo) -> Vec<MetaRow> {
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
