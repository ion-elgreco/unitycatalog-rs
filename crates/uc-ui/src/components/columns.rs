//! A table's columns, as upstream's console shows them: position, name,
//! type, nullability and comment, with a pencil that edits the comment.

use leptos::prelude::*;
use leptos::task::spawn_local;
use uc_api::{TableInfo, UpdateColumn, UpdateTable};
use uc_common::wire_name;

use super::card::Card;
use super::field::TextArea;
use super::icons::Pencil;
use crate::api;
use crate::components::{Modal, use_notify};
use crate::load::{Key, Load, use_cache};
use crate::util::or_dash;

/// A nested type over several lines, as the console shows it: each `<`
/// and `,` ends a line, and what follows is indented by its depth.
fn pretty_type(text: &str) -> String {
    if !text.contains('<') {
        return text.to_string();
    }
    let mut out = String::new();
    let mut depth = 0usize;
    // A space after a break is the source's, not the layout's.
    let mut at_line_start = false;
    let line_break = |out: &mut String, depth: usize| {
        out.push('\n');
        out.push_str(&"  ".repeat(depth));
    };
    for c in text.chars() {
        match c {
            '<' => {
                depth += 1;
                out.push('<');
                line_break(&mut out, depth);
                at_line_start = true;
            }
            '>' => {
                depth = depth.saturating_sub(1);
                line_break(&mut out, depth);
                out.push('>');
                at_line_start = false;
            }
            ',' => {
                out.push(',');
                line_break(&mut out, depth);
                at_line_start = true;
            }
            ' ' if at_line_start => {}
            c => {
                out.push(c);
                at_line_start = false;
            }
        }
    }
    out
}

#[component]
pub fn ColumnsCard(
    info: RwSignal<Load<TableInfo>>,
    #[prop(into)] full_name: String,
) -> impl IntoView {
    // The column whose comment is edited: its name and the text.
    let editing: RwSignal<Option<String>> = RwSignal::new(None);
    let text = RwSignal::new(String::new());
    let busy = RwSignal::new(false);
    let notify = use_notify();
    let cache = use_cache();
    let on_ok = move || {
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
    };
    let rows = move || {
        let columns = info.with(|i| {
            i.ready()
                .and_then(|t| t.columns.clone())
                .unwrap_or_default()
        });
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
                                let type_text = pretty_type(&or_dash(
                                    column.type_text.or_else(|| column.type_name.as_ref().map(wire_name)),
                                ));
                                let nullable = match column.nullable {
                                    Some(false) => "false",
                                    _ => "true",
                                };
                                let name = column.name.unwrap_or_default();
                                let title = format!("Edit the comment of {name}");
                                // What the pencil edits: the name and the comment.
                                let target = (name.clone(), column.comment.clone().unwrap_or_default());
                                view! {
                                    <tr>
                                        <td class="uc-muted">{position}</td>
                                        <td class="uc-strong">{name}</td>
                                        <td class="uc-mono uc-prewrap">{type_text}</td>
                                        <td class="uc-muted">{nullable}</td>
                                        <td class="uc-muted">{or_dash(column.comment)}</td>
                                        <td class="uc-right">
                                            <button
                                                type="button"
                                                class="uc-btn uc-btn--outline uc-btn--icon uc-btn--xs"
                                                title=title.clone()
                                                aria-label=title
                                                on:click=move |_| {
                                                    let (name, comment) = target.clone();
                                                    text.set(comment);
                                                    editing.set(Some(name));
                                                }
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
            <TextArea id="uc-column-comment" label="Comment" value=text/>
        </Modal>
    }
}
