//! A function's page, as the console's `FunctionDetails`: its description,
//! parameters and definition, its details, its grants; "Delete".

use leptos::prelude::*;
use uc_api::FunctionInfo;
use uc_common::{SecurableType, three_part_full_name, wire_name};

use crate::api;
use crate::auth::use_auth;
use crate::components::icons::{FunctionSquare, Trash2};
use crate::components::{
    Card, CodeBlock, DeleteModal, EntityHeader, Loaded, MetaGrid, MetaRow, PermissionsCard,
    RawJsonCard, Tab, Tabs,
};
use crate::kind::Kind;
use crate::load::{Key, field, use_cache, use_load};
use crate::util::{format_timestamp, or_dash, pretty_json, route_params};

#[component]
pub fn FunctionDetails() -> impl IntoView {
    let params = route_params(["catalog", "schema", "function"]);
    move || {
        let [catalog, schema, function] = params.get();
        view! { <FunctionPage catalog schema function/> }
    }
}

#[component]
fn FunctionPage(catalog: String, schema: String, function: String) -> impl IntoView {
    let full_name = three_part_full_name(&catalog, &schema, &function);
    let name = StoredValue::new(full_name.clone());
    let info = use_load(Key::Function(full_name.clone()), move || {
        let full_name = name.get_value();
        async move { api::client().get_function(&full_name).await }
    });
    let cache = use_cache();
    let auth = use_auth();
    let tab = RwSignal::new(Tab::Overview);
    let delete_open = RwSignal::new(false);
    let owner = field(info, |f: &FunctionInfo| f.owner.clone());
    let manages = Memo::new(move |_| {
        auth.session
            .with(|s| owner.with(|o| s.manages(o.as_deref())))
    });
    let comment = field(info, |f: &FunctionInfo| {
        f.comment.clone().unwrap_or_default()
    });
    let badges = field(info, |f: &FunctionInfo| {
        let language = f
            .external_language
            .clone()
            .or_else(|| f.routine_body.as_ref().map(wire_name));
        let returns = f
            .full_data_type
            .clone()
            .or_else(|| f.data_type.as_ref().map(wire_name));
        [language, returns]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>()
    });
    let parameters = field(info, |f: &FunctionInfo| {
        f.input_params
            .as_ref()
            .and_then(|p| p.parameters.clone())
            .unwrap_or_default()
    });
    let definition = field(info, |f: &FunctionInfo| {
        f.routine_definition.clone().unwrap_or_default()
    });
    let refresh = Callback::new(move |_| cache.changed(Key::Function(name.get_value())));

    let actions = view! {
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

    let parameter_rows = move || {
        let parameters = parameters.get();
        if parameters.is_empty() {
            return view! { <p class="uc-empty">"No parameters."</p> }.into_any();
        }
        view! {
            <div class="uc-table-wrap">
                <table class="uc-table">
                    <thead>
                        <tr>
                            <th>"#"</th>
                            <th>"Name"</th>
                            <th>"Type"</th>
                            <th>"Comment"</th>
                        </tr>
                    </thead>
                    <tbody>
                        {parameters
                            .into_iter()
                            .enumerate()
                            .map(|(i, p)| {
                                let position = p.position.map_or(i.to_string(), |p| p.to_string());
                                let type_text = if p.type_text.is_empty() {
                                    wire_name(&p.type_name)
                                } else {
                                    p.type_text
                                };
                                view! {
                                    <tr>
                                        <td class="uc-muted">{position}</td>
                                        <td class="uc-strong">{p.name}</td>
                                        <td class="uc-mono">{type_text}</td>
                                        <td class="uc-muted">{or_dash(p.comment)}</td>
                                    </tr>
                                }
                            })
                            .collect_view()}
                    </tbody>
                </table>
            </div>
        }
        .into_any()
    };

    let content = move || match tab.get() {
        Tab::Overview => view! {
            <Card title="Description">
                <Show
                    when=move || comment.with(|c| !c.is_empty())
                    fallback=|| view! { <p class="uc-empty">"No description provided."</p> }
                >
                    <p class="uc-prewrap">{move || comment.get()}</p>
                </Show>
            </Card>
            <Card title="Input parameters">{parameter_rows}</Card>
            <Card title="Routine definition">
                <CodeBlock text=definition/>
            </Card>
        }
        .into_any(),
        Tab::Details => view! {
            <Card>
                <MetaGrid rows=field(info, function_rows)/>
            </Card>
            <RawJsonCard json=field(info, pretty_json)/>
        }
        .into_any(),
        Tab::Permissions => view! {
            <PermissionsCard kind=SecurableType::Function full_name=name.get_value() owner/>
        }
        .into_any(),
    };

    view! {
        <EntityHeader
            icon=view! { <FunctionSquare class="uc-icon--function"/> }.into_any()
            name=full_name.clone()
            badges
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
            noun="function"
            question="Are you sure you want to delete the function"
            name=function.clone()
            label=format!("{function} function")
            delete=move |_| {
                let full_name = name.get_value();
                async move { api::client().delete_function(&full_name).await }
            }
            key=Key::Function(full_name.clone())
            list=Key::Entries(catalog.clone(), schema.clone(), Kind::Functions)
            then=Kind::Functions.tab_href(&catalog, &schema)
        />
    }
}

fn function_rows(f: &FunctionInfo) -> Vec<MetaRow> {
    vec![
        MetaRow::new("Name", or_dash(f.name.clone())),
        MetaRow::new("Owner", or_dash(f.owner.clone())),
        MetaRow::new(
            "Return type",
            or_dash(
                f.full_data_type
                    .clone()
                    .or_else(|| f.data_type.as_ref().map(wire_name)),
            ),
        ),
        MetaRow::new(
            "Routine body",
            or_dash(f.routine_body.as_ref().map(wire_name)),
        ),
        MetaRow::new("External language", or_dash(f.external_language.clone())),
        MetaRow::new(
            "SQL data access",
            or_dash(f.sql_data_access.as_ref().map(wire_name)),
        ),
        MetaRow::new(
            "Deterministic",
            or_dash(f.is_deterministic.map(|d| d.to_string())),
        ),
        MetaRow::new("Created at", format_timestamp(f.created_at)),
        MetaRow::new("Created by", or_dash(f.created_by.clone())),
        MetaRow::new("Updated at", format_timestamp(f.updated_at)),
        MetaRow::new("Updated by", or_dash(f.updated_by.clone())),
        MetaRow::mono("Function ID", or_dash(f.function_id.clone())),
    ]
}
