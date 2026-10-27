//! "New" on a schema's page, as the console's `CreateSchemaObjectAction`:
//! a table, a volume, a SQL function or a registered model, from one
//! dialog with a type picker. Beyond the console's, a table can be managed,
//! have any number of columns and be in Iceberg format, which the Iceberg
//! REST catalog creates. After the call, a toast, and the list that gained
//! the object loads again.

use buffa::Enumeration;
use buffa_types::google::protobuf::Value;
use leptos::prelude::*;
use leptos::task::spawn_local;
use uc_client::models::functions::v1::{
    CreateFunction, ParameterStyle, RoutineBody, SecurityType, SqlDataAccess,
};
use uc_client::models::iceberg::v1::{IcebergSchema, IcebergStructField};
use uc_client::models::tables::v1::{ColumnInfo, ColumnTypeName, DataSourceFormat, TableType};
use uc_client::models::volumes::v1::VolumeType;

use super::field::{Select, TextArea, TextField};
use super::icons::{Plus, X};
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

const FORMATS: [DataSourceFormat; 9] = [
    DataSourceFormat::Delta,
    DataSourceFormat::Iceberg,
    DataSourceFormat::Csv,
    DataSourceFormat::Json,
    DataSourceFormat::Avro,
    DataSourceFormat::Parquet,
    DataSourceFormat::Orc,
    DataSourceFormat::Text,
    DataSourceFormat::Lance,
];

/// The property that names the id of a managed Delta table: the id of
/// its staging table.
const TABLE_ID: &str = "io.unitycatalog.tableId";

/// Managed, at a location the server picks under the storage root, or
/// external, at a location of yours.
const TABLE_TYPES: [TableType; 2] = [TableType::Managed, TableType::External];

const VOLUME_TYPES: [VolumeType; 2] = [VolumeType::Managed, VolumeType::External];

/// A column of the new table, as the form holds it.
#[derive(Clone, Copy)]
struct NewColumn {
    id: u64,
    name: RwSignal<String>,
    kind: RwSignal<ColumnTypeName>,
}

/// An enum value's name as the server sends it, such as `MANAGED`.
fn wire_name<E: Enumeration>(value: &E) -> String {
    value.proto_name().to_string()
}

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

/// A scalar type's name in Iceberg's schema.
fn iceberg_type(t: ColumnTypeName) -> &'static str {
    match t {
        ColumnTypeName::Boolean => "boolean",
        ColumnTypeName::Int => "int",
        ColumnTypeName::Long => "long",
        ColumnTypeName::Double => "double",
        ColumnTypeName::Date => "date",
        ColumnTypeName::Timestamp => "timestamptz",
        ColumnTypeName::Binary => "binary",
        ColumnTypeName::Variant => "variant",
        _ => "string",
    }
}

/// The columns as an Iceberg schema. The server assigns the field ids
/// again.
fn iceberg_schema(columns: &[(String, ColumnTypeName)]) -> IcebergSchema {
    let fields = columns
        .iter()
        .zip(1..)
        .map(|((name, t), id)| IcebergStructField {
            id,
            name: name.clone(),
            field_type: Value::from(iceberg_type(*t)).into(),
            ..Default::default()
        })
        .collect();
    IcebergSchema {
        schema_type: "struct".into(),
        fields,
        ..Default::default()
    }
}

/// A column of a scalar type at `position`, as the API wants it.
fn column(name: &str, t: ColumnTypeName, position: i32) -> ColumnInfo {
    let type_json = serde_json::json!({
        "name": name,
        "type": delta_type(t),
        "nullable": true,
        "metadata": {},
    });
    ColumnInfo {
        name: name.to_string(),
        type_text: Some(wire_name(&t).to_lowercase()),
        type_json: type_json.to_string(),
        type_name: Some(t.into()),
        position: Some(position),
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
    let table_type = RwSignal::new(TableType::Managed);
    let format = RwSignal::new(DataSourceFormat::Delta);
    let columns: RwSignal<Vec<NewColumn>> = RwSignal::new(Vec::new());
    let volume_type = RwSignal::new(VolumeType::Managed);
    let return_type = RwSignal::new(ColumnTypeName::String);
    let definition = RwSignal::new(String::new());
    let busy = RwSignal::new(false);
    let notify = use_notify();
    let cache = use_cache();
    // A column row's signals belong to the dialog, not to the click that
    // adds the row.
    let owner = StoredValue::new(Owner::current().expect("a component has an owner"));
    let next_column = StoredValue::new(0u64);
    let new_column = move |name: &str| {
        next_column.update_value(|n| *n += 1);
        let id = next_column.get_value();
        owner.with_value(|owner| {
            owner.with(|| NewColumn {
                id,
                name: RwSignal::new(name.to_string()),
                kind: RwSignal::new(ColumnTypeName::String),
            })
        })
    };
    let reset_columns = move || columns.set(vec![new_column("value")]);
    reset_columns();
    let close = move || {
        open.set(false);
        name.set(String::new());
        comment.set(String::new());
        location.set(String::new());
        definition.set(String::new());
        reset_columns();
    };
    let needs_location = move || {
        (kind.get() == Kind::Tables && table_type.get() == TableType::External)
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
        let columns: Vec<(String, ColumnTypeName)> = columns.with_untracked(|columns| {
            columns
                .iter()
                .map(|c| {
                    (
                        c.name.get_untracked().trim().to_string(),
                        c.kind.get_untracked(),
                    )
                })
                .collect()
        });
        if kind == Kind::Tables && columns.iter().any(|(name, _)| name.is_empty()) {
            notify.error("Every column needs a name");
            return;
        }
        let source_format = format.get_untracked();
        let iceberg = source_format == DataSourceFormat::Iceberg;
        if kind == Kind::Tables
            && table_type.get_untracked() == TableType::Managed
            && !iceberg
            && source_format != DataSourceFormat::Delta
        {
            notify.error("A managed table is in Delta or Iceberg format");
            return;
        }
        if kind == Kind::Tables
            && iceberg
            && columns.iter().any(|(_, t)| *t == ColumnTypeName::Variant)
        {
            notify.error("An Iceberg table cannot have a VARIANT column");
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
                // The Iceberg REST catalog writes the first metadata file:
                // without a location the table is managed. Iceberg engines
                // read the comment from the `comment` property, Unity
                // Catalog from the table's own field.
                Kind::Tables if iceberg => {
                    async {
                        let external = table_type.get_untracked() == TableType::External;
                        client
                            .create_iceberg_table(
                                &catalog,
                                &schema,
                                &new_name,
                                iceberg_schema(&columns),
                            )
                            .with_location(external.then_some(path))
                            .with_properties(comment.clone().map(|c| ("comment", c)))
                            .await?;
                        if let Some(comment) = comment {
                            client
                                .table(format!("{catalog}.{schema}.{new_name}"))
                                .update()
                                .with_comment(comment)
                                .await?;
                        }
                        Ok(())
                    }
                    .await
                }
                Kind::Tables => {
                    async {
                        let table_type = table_type.get_untracked();
                        let columns = columns
                            .iter()
                            .zip(0..)
                            .map(|((name, t), i)| column(name, *t, i))
                            .collect::<Vec<ColumnInfo>>();
                        let create = client
                            .create_table(&new_name, &catalog, &schema, table_type)
                            .with_columns(columns)
                            .with_data_source_format(source_format)
                            .with_comment(comment);
                        // A managed table takes over a staging table, as
                        // engines make one: its id and its location under the
                        // storage root.
                        let create = if table_type == TableType::Managed {
                            let staging = client
                                .create_staging_table(new_name, catalog, schema)
                                .await?;
                            create
                                .with_storage_location(staging.staging_location)
                                .with_properties([(TABLE_ID, staging.id)])
                        } else {
                            create.with_storage_location(path)
                        };
                        create.await.map(|_| ())
                    }
                    .await
                }
                Kind::Volumes => {
                    let volume_type = volume_type.get_untracked();
                    client
                        .create_volume(catalog, schema, new_name, volume_type)
                        .with_comment(comment)
                        .with_storage_location(
                            (volume_type == VolumeType::External).then_some(path),
                        )
                        .await
                        .map(|_| ())
                }
                Kind::Functions => {
                    // A SQL function, as the console makes it.
                    let return_type = return_type.get_untracked();
                    let function = CreateFunction {
                        name: new_name.clone(),
                        catalog_name: catalog,
                        schema_name: schema,
                        data_type: Some(return_type.into()),
                        full_data_type: Some(wire_name(&return_type)),
                        routine_body: RoutineBody::Sql.into(),
                        routine_definition: Some(routine),
                        parameter_style: ParameterStyle::S.into(),
                        is_deterministic: Some(true),
                        sql_data_access: Some(SqlDataAccess::NoSql.into()),
                        is_null_call: Some(false),
                        security_type: SecurityType::Definer.into(),
                        specific_name: Some(new_name),
                        comment,
                        ..Default::default()
                    };
                    client.create_function(function).await.map(|_| ())
                }
                Kind::Models => client
                    .create_registered_model(new_name, catalog, schema)
                    .with_comment(comment)
                    .await
                    .map(|_| ()),
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
                <Select
                    id="uc-new-table-type"
                    label="Table type"
                    choices=&TABLE_TYPES
                    value=table_type
                    name=wire_name
                    text=|t: &TableType| {
                        match *t {
                            TableType::External => "External",
                            _ => "Managed",
                        }
                            .to_string()
                    }
                />
                <Select
                    id="uc-new-format"
                    label="Data source format"
                    choices=&FORMATS
                    value=format
                    name=wire_name
                />
                <div class="uc-field">
                    <span class="uc-label">"Columns"</span>
                    <For
                        each=move || columns.get()
                        key=|c| c.id
                        children=move |c| {
                            view! {
                                <div class="uc-column-row">
                                    <TextField
                                        id=format!("uc-new-column-{}", c.id)
                                        label="Name"
                                        value=c.name
                                        required=true
                                    />
                                    <Select
                                        id=format!("uc-new-column-type-{}", c.id)
                                        label="Type"
                                        choices=&SCALARS
                                        value=c.kind
                                        name=wire_name
                                        fixed=true
                                    />
                                    <button
                                        type="button"
                                        class="uc-btn uc-btn--ghost uc-btn--icon uc-btn--sm"
                                        title="Remove column"
                                        aria-label="Remove column"
                                        disabled=move || columns.with(|columns| columns.len() == 1)
                                        on:click=move |_| {
                                            columns.update(|columns| columns.retain(|other| other.id != c.id))
                                        }
                                    >
                                        <X/>
                                    </button>
                                </div>
                            }
                        }
                    />
                    <button
                        type="button"
                        class="uc-btn uc-btn--outline uc-btn--sm uc-self-start"
                        on:click=move |_| columns.update(|columns| columns.push(new_column("")))
                    >
                        <Plus/>
                        "Add column"
                    </button>
                </div>
            </Show>
            <Show when=move || kind.get() == Kind::Volumes>
                <Select
                    id="uc-new-volume-type"
                    label="Volume type"
                    choices=&VOLUME_TYPES
                    value=volume_type
                    name=wire_name
                    text=|t: &VolumeType| {
                        match *t {
                            VolumeType::External => "External",
                            _ => "Managed",
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
