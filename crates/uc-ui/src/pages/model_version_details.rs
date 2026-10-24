//! A model version's page, as the console's `ModelVersionDetails`: its
//! description and details; "Finalize" while it is pending, and "Delete".

use leptos::prelude::*;
use leptos::task::spawn_local;
use uc_client::models::models::v1::{ModelVersionInfo, RegisteredModelInfo};
use uc_common::three_part_full_name;

use super::model_details::is_pending;
use crate::api;
use crate::auth::use_manages;
use crate::components::{
    Card, DeleteButton, DeleteModal, DescriptionCard, EntityHeader, LoadError, Loaded, MetaGrid,
    MetaRow, RawJsonCard, use_notify,
};
use crate::kind::Kind;
use crate::load::{Key, field, use_cache, use_load};
use crate::util::{enum_name, format_timestamp, or_dash, pretty_json, route_params};

#[component]
pub fn ModelVersionDetails() -> impl IntoView {
    let params = route_params(["catalog", "schema", "model", "version"]);
    move || {
        let [catalog, schema, model, version] = params.get();
        view! { <VersionPage catalog schema model version/> }
    }
}

#[component]
fn VersionPage(catalog: String, schema: String, model: String, version: String) -> impl IntoView {
    let icon = Kind::Models.icon();
    // A version is a positive number, as the console takes it.
    let Some(number) = version.parse::<i64>().ok().filter(|n| *n > 0) else {
        return view! {
            <EntityHeader icon name=format!("{model} · v{version}")/>
            <div class="uc-page">
                <LoadError message=format!("Invalid model version: {version}")/>
            </div>
        }
        .into_any();
    };
    let full_name = three_part_full_name(&catalog, &schema, &model);
    let names = StoredValue::new((
        catalog.clone(),
        schema.clone(),
        model.clone(),
        full_name.clone(),
    ));
    let info = use_load(Key::Version(full_name.clone(), number), move || {
        let (_, _, _, full_name) = names.get_value();
        async move { api::client().get_model_version(full_name, number).await }
    });
    let model_info = use_load(Key::Model(full_name.clone()), move || {
        let (_, _, _, full_name) = names.get_value();
        async move { api::client().registered_model(full_name).get().await }
    });
    let cache = use_cache();
    let notify = use_notify();
    let delete_open = RwSignal::new(false);
    let finalizing = RwSignal::new(false);
    let owner = field(model_info, |m: &RegisteredModelInfo| m.owner.clone());
    let manages = use_manages(owner);
    let comment = field(info, |v: &ModelVersionInfo| {
        v.comment.clone().unwrap_or_default()
    });
    let badges = field(info, |v: &ModelVersionInfo| {
        Some(enum_name(v.status))
            .filter(|status| !status.is_empty())
            .into_iter()
            .collect::<Vec<_>>()
    });
    let pending = field(info, is_pending);

    let save_comment = move |comment: String| {
        let (_, _, _, full_name) = names.get_value();
        async move {
            let updated = api::client()
                .update_model_version(&full_name, number, comment)
                .await?;
            cache.set(Key::Version(full_name, number), updated);
            Ok(())
        }
    };
    let finalize = move |_| {
        let (_, _, _, full_name) = names.get_value();
        finalizing.set(true);
        spawn_local(async move {
            match api::client()
                .finalize_model_version(&full_name, number)
                .await
            {
                Ok(updated) => {
                    cache.set(Key::Version(full_name.clone(), number), updated);
                    cache.changed(Key::Versions(full_name));
                }
                Err(e) => notify.failed(&e, "Failed to finalize the model version"),
            }
            finalizing.set(false);
        });
    };
    let reload = vec![Key::Version(full_name.clone(), number)];

    let actions = view! {
        <Show when=move || pending.get()>
            <button
                type="button"
                class="uc-btn uc-btn--outline uc-btn--sm"
                disabled=move || finalizing.get()
                on:click=finalize
            >
                "Finalize"
            </button>
        </Show>
        <DeleteButton open=delete_open when=manages/>
    }
    .into_any();

    view! {
        <EntityHeader
            icon
            name=format!("{model} · v{version}")
            badges
            reload
            actions
        />
        <div class="uc-page">
            <Loaded state=info>
                <DescriptionCard comment noun="model version" save=save_comment/>
                <Card title="Details">
                    <MetaGrid rows=field(info, version_rows)/>
                </Card>
                <RawJsonCard json=field(info, pretty_json)/>
            </Loaded>
        </div>
        <DeleteModal
            open=delete_open
            noun="model version"
            name=version.clone()
            delete=move |_| {
                let (_, _, _, full_name) = names.get_value();
                async move { api::client().delete_model_version(full_name, number).await }
            }
            key=Key::Version(full_name.clone(), number)
            list=Key::Versions(full_name.clone())
            then=Kind::Models.href(&catalog, &schema, &model)
        />
    }
    .into_any()
}

fn version_rows(v: &ModelVersionInfo) -> Vec<MetaRow> {
    vec![
        MetaRow::new("Model", or_dash(v.model_name.clone())),
        MetaRow::new("Version", format!("v{}", v.version)),
        MetaRow::new("Status", or_dash(enum_name(v.status))),
        MetaRow::mono("Source", or_dash(v.source.clone())),
        MetaRow::mono("Run ID", or_dash(v.run_id.clone())),
        MetaRow::mono("Storage location", or_dash(v.storage_location.clone())),
        MetaRow::new("Created at", format_timestamp(v.created_at)),
        MetaRow::new("Updated at", format_timestamp(v.updated_at)),
        MetaRow::new("Comment", or_dash(v.comment.clone())),
    ]
}
