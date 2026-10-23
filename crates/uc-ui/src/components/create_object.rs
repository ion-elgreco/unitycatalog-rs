//! "New" on a schema's page, as the console's `CreateSchemaObjectAction`:
//! an external table, a volume, a function or a registered model, from one
//! dialog with a type picker. After the call, the UI goes to the object's
//! page.

use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::hooks::use_navigate;
use uc_api::{
    ColumnInfo, ColumnTypeName, CreateFunction, CreateFunctionRequest, CreateRegisteredModel,
    CreateTable, CreateVolumeRequestContent, DataSourceFormat, ParameterStyle, RoutineBody,
    SecurityType, SqlDataAccess, TableType, VolumeType,
};
use uc_common::wire_name;

use crate::api;
use crate::components::{Modal, use_notify};
use crate::kind::Kind;
use crate::load::{Key, use_cache};

/// The column types the dialog offers: the scalar ones.
const SCALARS: [ColumnTypeName; 9] = [
    ColumnTypeName::String,
    ColumnTypeName::Int,
    ColumnTypeName::Long,
    ColumnTypeName::Double,
    ColumnTypeName::Float,
    ColumnTypeName::Boolean,
    ColumnTypeName::Date,
    ColumnTypeName::Timestamp,
    ColumnTypeName::Binary,
];

const FORMATS: [DataSourceFormat; 7] = [
    DataSourceFormat::Delta,
    DataSourceFormat::Parquet,
    DataSourceFormat::Csv,
    DataSourceFormat::Json,
    DataSourceFormat::Avro,
    DataSourceFormat::Orc,
    DataSourceFormat::Text,
];

/// A scalar type's name in Delta's schema, for `type_json`.
fn delta_type(t: ColumnTypeName) -> &'static str {
    match t {
        ColumnTypeName::Int => "integer",
        ColumnTypeName::Long => "long",
        ColumnTypeName::Double => "double",
        ColumnTypeName::Float => "float",
        ColumnTypeName::Boolean => "boolean",
        ColumnTypeName::Date => "date",
        ColumnTypeName::Timestamp => "timestamp",
        ColumnTypeName::Binary => "binary",
        _ => "string",
    }
}

/// A column of a scalar type, as the API wants it.
fn column(name: &str, t: ColumnTypeName) -> ColumnInfo {
    let type_json = serde_json::json!({
        "name": name,
        "type": delta_type(t),
        "nullable": true,
        "metadata": {},
    });
    ColumnInfo {
        name: Some(name.to_string()),
        type_text: Some(wire_name(&t).to_lowercase()),
        type_json: Some(type_json.to_string()),
        type_name: Some(t),
        position: Some(0),
        nullable: Some(true),
        ..Default::default()
    }
}

/// The language of a new function.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Language {
    Python,
    Sql,
}

#[component]
pub fn CreateObjectModal(
    open: RwSignal<bool>,
    /// The kind the dialog opens on; the picker changes it.
    kind: RwSignal<Kind>,
    catalog: String,
    schema: String,
) -> impl IntoView {
    let name = RwSignal::new(String::new());
    let comment = RwSignal::new(String::new());
    let location = RwSignal::new(String::new());
    let column_name = RwSignal::new("value".to_string());
    let column_type = RwSignal::new(ColumnTypeName::String);
    let format = RwSignal::new(DataSourceFormat::Delta);
    let volume_type = RwSignal::new(VolumeType::Managed);
    let language = RwSignal::new(Language::Python);
    let return_type = RwSignal::new(ColumnTypeName::String);
    let definition = RwSignal::new(String::new());
    let busy = RwSignal::new(false);
    let notify = use_notify();
    let cache = use_cache();
    let navigate = use_navigate();
    let close = move || {
        open.set(false);
        name.set(String::new());
        comment.set(String::new());
        location.set(String::new());
        definition.set(String::new());
    };
    let needs_location = move || {
        kind.get() == Kind::Tables
            || (kind.get() == Kind::Volumes && volume_type.get() == VolumeType::External)
    };
    let on_ok = {
        let catalog = catalog.clone();
        let schema = schema.clone();
        move || {
            let kind = kind.get_untracked();
            let new_name = name.get_untracked().trim().to_string();
            if new_name.is_empty() {
                notify.error("Name is required");
                return;
            }
            let text = comment.get_untracked();
            let comment = (!text.is_empty()).then_some(text);
            let path = location.get_untracked().trim().to_string();
            if untrack(needs_location) && path.is_empty() {
                notify.error("Storage location is required");
                return;
            }
            let body = definition.get_untracked();
            if kind == Kind::Functions && body.trim().is_empty() {
                notify.error("Definition is required");
                return;
            }
            let catalog = catalog.clone();
            let schema = schema.clone();
            let page = kind.href(&catalog, &schema, &new_name);
            let list = Key::Entries(catalog.clone(), schema.clone(), kind);
            let navigate = navigate.clone();
            busy.set(true);
            spawn_local(async move {
                let client = api::client();
                let answer = match kind {
                    Kind::Tables => {
                        let request = CreateTable {
                            name: new_name.clone(),
                            catalog_name: catalog,
                            schema_name: schema,
                            table_type: TableType::External,
                            columns: vec![column(
                                column_name.get_untracked().trim(),
                                column_type.get_untracked(),
                            )],
                            data_source_format: Some(format.get_untracked()),
                            storage_location: Some(path),
                            comment,
                            properties: None,
                            view_definition: None,
                            view_dependencies: None,
                        };
                        client.create_table(&request).await.map(|_| ())
                    }
                    Kind::Volumes => {
                        let volume_type = volume_type.get_untracked();
                        let request = CreateVolumeRequestContent {
                            catalog_name: catalog,
                            schema_name: schema,
                            name: new_name.clone(),
                            volume_type,
                            comment,
                            storage_location: (volume_type == VolumeType::External).then_some(path),
                        };
                        client.create_volume(&request).await.map(|_| ())
                    }
                    Kind::Functions => {
                        let language = language.get_untracked();
                        let return_type = return_type.get_untracked();
                        let request = CreateFunctionRequest {
                            function_info: CreateFunction {
                                name: Some(new_name.clone()),
                                catalog_name: Some(catalog),
                                schema_name: Some(schema),
                                input_params: None,
                                data_type: Some(return_type),
                                full_data_type: Some(wire_name(&return_type)),
                                return_params: None,
                                routine_body: match language {
                                    Language::Python => RoutineBody::External,
                                    Language::Sql => RoutineBody::Sql,
                                },
                                routine_definition: Some(body),
                                routine_dependencies: None,
                                parameter_style: ParameterStyle::S,
                                is_deterministic: Some(true),
                                sql_data_access: Some(match language {
                                    Language::Python => SqlDataAccess::NoSql,
                                    Language::Sql => SqlDataAccess::ContainsSql,
                                }),
                                is_null_call: Some(false),
                                security_type: SecurityType::Definer,
                                specific_name: Some(new_name.clone()),
                                comment,
                                properties: None,
                                external_language: (language == Language::Python)
                                    .then(|| "python".to_string()),
                            },
                        };
                        client.create_function(&request).await.map(|_| ())
                    }
                    Kind::Models => {
                        let request = CreateRegisteredModel {
                            name: new_name.clone(),
                            catalog_name: catalog,
                            schema_name: schema,
                            comment,
                        };
                        client.create_registered_model(&request).await.map(|_| ())
                    }
                };
                match answer {
                    Ok(()) => {
                        notify.success(format!("Created {} {new_name}", kind.noun()));
                        cache.changed(list);
                        if busy.try_get_untracked().is_some() {
                            close();
                            navigate(&page, Default::default());
                        }
                    }
                    Err(e) => notify.failed(&e, &format!("Failed to create {}", kind.noun())),
                }
                busy.set(false);
            });
        }
    };

    let type_select = move |id: &'static str, value: RwSignal<ColumnTypeName>| {
        view! {
            <select
                id=id
                class="uc-select"
                prop:value=move || wire_name(&value.get())
                on:change=move |e| {
                    let picked = event_target_value(&e);
                    if let Some(t) = SCALARS.iter().find(|t| wire_name(*t) == picked) {
                        value.set(*t);
                    }
                }
            >
                {SCALARS
                    .iter()
                    .map(|t| {
                        let name = wire_name(t);
                        let value = name.clone();
                        view! { <option value=value>{name}</option> }
                    })
                    .collect_view()}
            </select>
        }
    };

    view! {
        <Modal
            open
            title=move || format!("Create {}", kind.get().noun())
            ok_text="Create"
            busy
            on_cancel=move || close()
            on_ok
        >
            <div class="uc-field">
                <label class="uc-label" for="uc-new-kind">"Object type"</label>
                <select
                    id="uc-new-kind"
                    class="uc-select"
                    prop:value=move || kind.get().tab()
                    on:change=move |e| {
                        if let Some(k) = Kind::from_tab(&event_target_value(&e)) {
                            kind.set(k);
                        }
                    }
                >
                    <option value="tables">"External table"</option>
                    <option value="volumes">"Volume"</option>
                    <option value="functions">"Function"</option>
                    <option value="models">"Registered model"</option>
                </select>
            </div>
            <div class="uc-field">
                <label class="uc-label" for="uc-new-name">
                    <span class="req">"*"</span>
                    "Name"
                </label>
                <input
                    id="uc-new-name"
                    class="uc-input"
                    type="text"
                    required
                    prop:value=move || name.get()
                    on:input=move |e| name.set(event_target_value(&e))
                />
            </div>
            <Show when=move || kind.get() == Kind::Tables>
                <div class="uc-form-row">
                    <div class="uc-field">
                        <label class="uc-label" for="uc-new-column">"Column name"</label>
                        <input
                            id="uc-new-column"
                            class="uc-input"
                            type="text"
                            prop:value=move || column_name.get()
                            on:input=move |e| column_name.set(event_target_value(&e))
                        />
                    </div>
                    <div class="uc-field">
                        <label class="uc-label" for="uc-new-column-type">"Column type"</label>
                        {type_select("uc-new-column-type", column_type)}
                    </div>
                </div>
                <div class="uc-field">
                    <label class="uc-label" for="uc-new-format">"Data source format"</label>
                    <select
                        id="uc-new-format"
                        class="uc-select"
                        prop:value=move || wire_name(&format.get())
                        on:change=move |e| {
                            let picked = event_target_value(&e);
                            if let Some(f) = FORMATS.iter().find(|f| wire_name(*f) == picked) {
                                format.set(*f);
                            }
                        }
                    >
                        {FORMATS
                            .iter()
                            .map(|f| {
                                let name = wire_name(f);
                                let value = name.clone();
                                view! { <option value=value>{name}</option> }
                            })
                            .collect_view()}
                    </select>
                </div>
            </Show>
            <Show when=move || kind.get() == Kind::Volumes>
                <div class="uc-field">
                    <label class="uc-label" for="uc-new-volume-type">"Volume type"</label>
                    <select
                        id="uc-new-volume-type"
                        class="uc-select"
                        prop:value=move || wire_name(&volume_type.get())
                        on:change=move |e| {
                            volume_type
                                .set(
                                    match event_target_value(&e).as_str() {
                                        "EXTERNAL" => VolumeType::External,
                                        _ => VolumeType::Managed,
                                    },
                                )
                        }
                    >
                        <option value="MANAGED">"Managed"</option>
                        <option value="EXTERNAL">"External"</option>
                    </select>
                </div>
            </Show>
            <Show when=needs_location>
                <div class="uc-field">
                    <label class="uc-label" for="uc-new-location">
                        <span class="req">"*"</span>
                        "Storage location"
                    </label>
                    <input
                        id="uc-new-location"
                        class="uc-input"
                        type="text"
                        placeholder="s3://bucket/path"
                        prop:value=move || location.get()
                        on:input=move |e| location.set(event_target_value(&e))
                    />
                </div>
            </Show>
            <Show when=move || kind.get() == Kind::Functions>
                <div class="uc-form-row">
                    <div class="uc-field">
                        <label class="uc-label" for="uc-new-language">"Language"</label>
                        <select
                            id="uc-new-language"
                            class="uc-select"
                            on:change=move |e| {
                                language
                                    .set(
                                        match event_target_value(&e).as_str() {
                                            "sql" => Language::Sql,
                                            _ => Language::Python,
                                        },
                                    )
                            }
                        >
                            <option value="python">"Python"</option>
                            <option value="sql">"SQL"</option>
                        </select>
                    </div>
                    <div class="uc-field">
                        <label class="uc-label" for="uc-new-return-type">"Return type"</label>
                        {type_select("uc-new-return-type", return_type)}
                    </div>
                </div>
                <div class="uc-field">
                    <label class="uc-label" for="uc-new-definition">
                        <span class="req">"*"</span>
                        "Definition"
                    </label>
                    <textarea
                        id="uc-new-definition"
                        class="uc-textarea uc-textarea--code"
                        spellcheck="false"
                        prop:value=move || definition.get()
                        on:input=move |e| definition.set(event_target_value(&e))
                    />
                </div>
            </Show>
            <div class="uc-field">
                <label class="uc-label" for="uc-new-comment">"Comment"</label>
                <textarea
                    id="uc-new-comment"
                    class="uc-textarea"
                    prop:value=move || comment.get()
                    on:input=move |e| comment.set(event_target_value(&e))
                />
            </div>
        </Modal>
    }
}
