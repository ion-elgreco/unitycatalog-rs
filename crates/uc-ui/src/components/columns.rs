//! A table's columns, as upstream's console shows them: position, name,
//! type, nullability and comment, with a pencil that edits the comment.

use leptos::prelude::*;
use leptos::task::spawn_local;
use uc_api::{ColumnInfo, TableInfo, UpdateColumn, UpdateTable};
use uc_common::wire_name;

use super::card::Card;
use super::icons::Pencil;
use crate::api;
use crate::components::{Modal, use_notify};
use crate::load::{Key, Load, use_cache};

#[component]
pub fn ColumnsCard(
    info: RwSignal<Load<TableInfo>>,
    #[prop(into)] full_name: String,
) -> impl IntoView {
    let columns = Signal::derive(move || {
        info.with(|i| {
            i.ready()
                .and_then(|t| t.columns.clone())
                .unwrap_or_default()
        })
    });
    // The column whose comment is edited: its name and the text.
    let editing: RwSignal<Option<String>> = RwSignal::new(None);
    let text = RwSignal::new(String::new());
    let busy = RwSignal::new(false);
    let notify = use_notify();
    let cache = use_cache();
    let edit = move |column: &ColumnInfo| {
        text.set(column.comment.clone().unwrap_or_default());
        editing.set(column.name.clone());
    };
    let on_ok = {
        let full_name = full_name.clone();
        move || {
            let Some(name) = editing.get_untracked() else {
                return;
            };
            let full_name = full_name.clone();
            let request = UpdateTable {
                comment: None,
                columns: vec![UpdateColumn {
                    name: name.clone(),
                    comment: text.get_untracked(),
                }],
            };
            busy.set(true);
            spawn_local(async move {
                match api::client().update_table(&full_name, &request).await {
                    Ok(updated) => {
                        cache.set(Key::Table(full_name), updated);
                        notify.success(format!("Updated the comment of {name}"));
                        if busy.try_get_untracked().is_some() {
                            editing.set(None);
                        }
                    }
                    Err(e) => notify.failed(&e, "Failed to update the column"),
                }
                busy.set(false);
            });
        }
    };
    let rows = move || {
        let columns = columns.get();
        if columns.is_empty() {
            return view! { <p class="uc-empty">"No columns."</p> }.into_any();
        }
        view! {
            <div class="uc-table-wrap">
                <table class="uc-table">
                    <thead>
                        <tr>
                            <th>"#"</th>
                            <th>"Column"</th>
                            <th>"Type"</th>
                            <th>"Nullable"</th>
                            <th>"Comment"</th>
                            <th></th>
                        </tr>
                    </thead>
                    <tbody>
                        {columns
                            .into_iter()
                            .enumerate()
                            .map(|(i, column)| {
                                let position = column.position.map_or(i.to_string(), |p| p.to_string());
                                let type_text = column
                                    .type_text
                                    .clone()
                                    .or_else(|| column.type_name.as_ref().map(wire_name))
                                    .unwrap_or_else(|| "—".into());
                                let nullable = match column.nullable {
                                    Some(false) => "false",
                                    Some(true) => "true",
                                    None => "—",
                                };
                                let comment = column.comment.clone().filter(|c| !c.is_empty());
                                let name = column.name.clone().unwrap_or_default();
                                let title = format!("Edit the comment of {name}");
                                view! {
                                    <tr>
                                        <td class="uc-muted">{position}</td>
                                        <td class="uc-strong">{name}</td>
                                        <td class="uc-mono">{type_text}</td>
                                        <td class="uc-muted">{nullable}</td>
                                        <td class="uc-muted">{comment.unwrap_or_else(|| "—".into())}</td>
                                        <td class="uc-right">
                                            <button
                                                type="button"
                                                class="uc-btn uc-btn--outline uc-btn--icon uc-btn--xs"
                                                title=title.clone()
                                                aria-label=title
                                                on:click=move |_| edit(&column)
                                            >
                                                <Pencil/>
                                            </button>
                                        </td>
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
    view! {
        <Card title="Columns">{rows}</Card>
        <Modal
            open=Signal::derive(move || editing.try_with(Option::is_some).unwrap_or(false))
            title=move || format!("Edit the comment of {}", editing.get().unwrap_or_default())
            ok_text="Save"
            busy
            on_cancel=move || editing.set(None)
            on_ok
        >
            <div class="uc-field">
                <label class="uc-label" for="uc-column-comment">"Comment"</label>
                <textarea
                    id="uc-column-comment"
                    class="uc-textarea"
                    prop:value=move || text.get()
                    on:input=move |e| text.set(event_target_value(&e))
                />
            </div>
        </Modal>
    }
}
