//! A model version's page, as the console's `ModelVersionDetails`: its
//! description and details; "Finalize" while it is pending, and "Delete".

use leptos::prelude::*;
use leptos::task::spawn_local;
use uc_api::{ModelVersionInfo, RegisteredModelInfo, UpdateModelVersion};
use uc_common::{three_part_full_name, wire_name};

use super::model_details::is_pending;
use crate::api;
use crate::auth::use_manages;
use crate::components::icons::CircleCheck;
use crate::components::{
    Card, DeleteButton, DeleteModal, DescriptionCard, EntityHeader, LoadError, Loaded, MetaGrid,
    MetaRow, RawJsonCard, use_notify,
};
use crate::kind::Kind;
use crate::load::{Key, field, use_cache, use_load};
use crate::util::{format_timestamp, or_dash, pretty_json, route_params};

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
    let Ok(number) = version.parse::<i64>() else {
        return view! {
            <EntityHeader icon name=format!("{model} · v{version}")/>
            <div class="uc-page">
                <LoadError message=format!("\"{version}\" is not a version number.")/>
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
        async move { api::client().get_model_version(&full_name, number).await }
    });
    let model_info = use_load(Key::Model(full_name.clone()), move || {
        let (_, _, _, full_name) = names.get_value();
        async move { api::client().get_registered_model(&full_name).await }
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
        v.status
            .as_ref()
            .map(wire_name)
            .into_iter()
            .collect::<Vec<_>>()
    });
    let pending = field(info, is_pending);

    let save_comment = move |comment: String| {
        let (_, _, _, full_name) = names.get_value();
        async move {
            let request = UpdateModelVersion {
                comment: Some(comment),
            };
            let updated = api::client()
                .update_model_version(&full_name, number, &request)
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
                    notify.success(format!("Finalized version {number}"));
                    cache.set(Key::Version(full_name.clone(), number), updated);
                    cache.changed(Key::Versions(full_name));
                }
                Err(e) => notify.failed(&e, "Failed to finalize the model version"),
            }
            finalizing.set(false);
        });
    };
    let refresh = Callback::new(move |_| {
        let (_, _, _, full_name) = names.get_value();
        cache.changed(Key::Version(full_name, number));
    });

    let actions = view! {
        <Show when=move || pending.get()>
            <button
                type="button"
                class="uc-btn uc-btn--outline uc-btn--sm"
                disabled=move || finalizing.get()
                on:click=finalize
            >
                <CircleCheck/>
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
            on_refresh=refresh
            actions
        />
        <div class="uc-page">
            <Loaded state=info>
                <DescriptionCard comment noun="model version" save=save_comment/>
                <Card title="Version details">
                    <MetaGrid rows=field(info, version_rows)/>
                </Card>
                <RawJsonCard json=field(info, pretty_json)/>
            </Loaded>
        </div>
        <DeleteModal
            open=delete_open
            noun="model version"
            question="Are you sure you want to delete model version"
            name=number.to_string()
            label=format!("Model version {number}")
            delete=move |_| {
                let (_, _, _, full_name) = names.get_value();
                async move { api::client().delete_model_version(&full_name, number).await }
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
        MetaRow::new("Version", or_dash(v.version.map(|n| format!("v{n}")))),
        MetaRow::new("Status", or_dash(v.status.as_ref().map(wire_name))),
        MetaRow::mono("Source", or_dash(v.source.clone())),
        MetaRow::mono("Run ID", or_dash(v.run_id.clone())),
        MetaRow::mono("Storage location", or_dash(v.storage_location.clone())),
        MetaRow::new("Created at", format_timestamp(v.created_at)),
        MetaRow::new("Created by", or_dash(v.created_by.clone())),
        MetaRow::new("Updated at", format_timestamp(v.updated_at)),
        MetaRow::new("Updated by", or_dash(v.updated_by.clone())),
        MetaRow::mono("Version ID", or_dash(v.id.clone())),
    ]
}
