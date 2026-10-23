//! "New" on a schema's page, as the console's `CreateSchemaObjectAction`:
//! an external table, a volume, a SQL function or a registered model, from
//! one dialog with a type picker. After the call, a toast, and the list
//! that gained the object loads again.

use leptos::prelude::*;
use leptos::task::spawn_local;
use uc_api::{
    ColumnInfo, ColumnTypeName, CreateFunction, CreateFunctionRequest, CreateRegisteredModel,
    CreateTable, CreateVolumeRequestContent, DataSourceFormat, ParameterStyle, RoutineBody,
    SecurityType, SqlDataAccess, TableType, VolumeType,
};
use uc_common::wire_name;

use super::field::{Select, TextArea, TextField};
use crate::api;
use crate::components::{Modal, use_notify};
use crate::kind::Kind;
use crate::load::{Key, use_cache};

/// The column types the dialog offers: the scalar ones, in the console's
/// order.
const SCALARS: [ColumnTypeName; 9] = [
    ColumnTypeName::Boolean,
    ColumnTypeName::Int,
    ColumnTypeName::Long,
    ColumnTypeName::Double,
    ColumnTypeName::Date,
    ColumnTypeName::Timestamp,
    ColumnTypeName::String,
    ColumnTypeName::Binary,
    ColumnTypeName::Variant,
];

const FORMATS: [DataSourceFormat; 7] = [
    DataSourceFormat::Delta,
    DataSourceFormat::Csv,
    DataSourceFormat::Json,
    DataSourceFormat::Avro,
    DataSourceFormat::Parquet,
    DataSourceFormat::Orc,
    DataSourceFormat::Text,
];

const VOLUME_TYPES: [VolumeType; 2] = [VolumeType::Managed, VolumeType::External];

/// A scalar type's name in Delta's schema, for `type_json`.
fn delta_type(t: ColumnTypeName) -> &'static str {
    match t {
        ColumnTypeName::Boolean => "boolean",
        ColumnTypeName::Int => "integer",
        ColumnTypeName::Long => "long",
        ColumnTypeName::Double => "double",
        ColumnTypeName::Date => "date",
        ColumnTypeName::Timestamp => "timestamp",
        ColumnTypeName::Binary => "binary",
        ColumnTypeName::Variant => "variant",
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
    let return_type = RwSignal::new(ColumnTypeName::String);
    let definition = RwSignal::new(String::new());
    let busy = RwSignal::new(false);
    let notify = use_notify();
    let cache = use_cache();
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
    let on_ok = move || {
        let kind = kind.get_untracked();
        let new_name = name.get_untracked().trim().to_string();
        if new_name.is_empty() {
            notify.error("Name is required");
            return;
        }
        let text = comment.get_untracked();
        let comment = (!text.is_empty()).then_some(text);
        let column_name = column_name.get_untracked().trim().to_string();
        if kind == Kind::Tables && column_name.is_empty() {
            notify.error("Column name is required");
            return;
        }
        let path = location.get_untracked().trim().to_string();
        if untrack(needs_location) && path.is_empty() {
            notify.error("Storage location is required");
            return;
        }
        let routine = definition.get_untracked();
        if kind == Kind::Functions && routine.trim().is_empty() {
            notify.error("Routine definition is required");
            return;
        }
        let catalog = catalog.clone();
        let schema = schema.clone();
        let list = Key::Entries(catalog.clone(), schema.clone(), kind);
        busy.set(true);
        spawn_local(async move {
            let client = api::client();
            let answer = match kind {
                Kind::Tables => {
                    let request = CreateTable {
                        name: new_name,
                        catalog_name: catalog,
                        schema_name: schema,
                        table_type: TableType::External,
                        columns: vec![column(&column_name, column_type.get_untracked())],
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
                        name: new_name,
                        volume_type,
                        comment,
                        storage_location: (volume_type == VolumeType::External).then_some(path),
                    };
                    client.create_volume(&request).await.map(|_| ())
                }
                Kind::Functions => {
                    // A SQL function, as the console makes it.
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
                            routine_body: RoutineBody::Sql,
                            routine_definition: Some(routine),
                            routine_dependencies: None,
                            parameter_style: ParameterStyle::S,
                            is_deterministic: Some(true),
                            sql_data_access: Some(SqlDataAccess::NoSql),
                            is_null_call: Some(false),
                            security_type: SecurityType::Definer,
                            specific_name: Some(new_name),
                            comment,
                            properties: None,
                            external_language: None,
                        },
                    };
                    client.create_function(&request).await.map(|_| ())
                }
                Kind::Models => {
                    let request = CreateRegisteredModel {
                        name: new_name,
                        catalog_name: catalog,
                        schema_name: schema,
                        comment,
                    };
                    client.create_registered_model(&request).await.map(|_| ())
                }
            };
            match answer {
                Ok(()) => {
                    notify.success(format!("Create {} complete", kind.noun()));
                    cache.changed(list);
                    if busy.try_get_untracked().is_some() {
                        close();
                    }
                }
                Err(e) => notify.failed(&e, &format!("Create {} failed", kind.noun())),
            }
            busy.set(false);
        });
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
            <Select
                id="uc-new-kind"
                label="Object type"
                choices=&Kind::ALL
                value=kind
                name=|kind: &Kind| kind.tab().to_string()
                text=|kind: &Kind| kind.new_label().to_string()
            />
            <TextField id="uc-new-name" label="Name" value=name required=true/>
            <Show when=move || kind.get() == Kind::Tables>
                <TextField id="uc-new-column" label="Column name" value=column_name required=true/>
                <Select
                    id="uc-new-column-type"
                    label="Column type"
                    choices=&SCALARS
                    value=column_type
                    name=wire_name
                />
                <Select
                    id="uc-new-format"
                    label="Data source format"
                    choices=&FORMATS
                    value=format
                    name=wire_name
                />
            </Show>
            <Show when=move || kind.get() == Kind::Volumes>
                <Select
                    id="uc-new-volume-type"
                    label="Volume type"
                    choices=&VOLUME_TYPES
                    value=volume_type
                    name=wire_name
                    text=|t: &VolumeType| {
                        match t {
                            VolumeType::Managed => "Managed",
                            VolumeType::External => "External",
                        }
                            .to_string()
                    }
                />
            </Show>
            <Show when=needs_location>
                <TextField
                    id="uc-new-location"
                    label="Storage location"
                    value=location
                    required=true
                    placeholder="s3://bucket/path"
                />
            </Show>
            <Show when=move || kind.get() == Kind::Functions>
                <TextArea
                    id="uc-new-definition"
                    label="Routine definition"
                    value=definition
                    required=true
                    code=true
                />
                <Select
                    id="uc-new-return-type"
                    label="Return type"
                    choices=&SCALARS
                    value=return_type
                    name=wire_name
                />
            </Show>
            <TextField id="uc-new-comment" label="Comment" value=comment/>
        </Modal>
    }
}
