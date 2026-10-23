//! The fields of the dialogs, as the console's forms: a label over a line
//! of text, a text area or a choice, in a `uc-field` block.

use leptos::prelude::*;

/// The label of a field; `required` marks it with a star.
fn field_label(id: &'static str, text: &'static str, required: bool) -> impl IntoView {
    view! {
        <label class="uc-label" for=id>
            {required.then(|| view! { <span class="req">"*"</span> })}
            {text}
        </label>
    }
}

/// One line of text.
#[component]
pub fn TextField(
    id: &'static str,
    label: &'static str,
    value: RwSignal<String>,
    #[prop(optional)] required: bool,
    #[prop(optional)] placeholder: Option<&'static str>,
    #[prop(optional)] autocomplete: Option<&'static str>,
) -> impl IntoView {
    view! {
        <div class="uc-field">
            {field_label(id, label, required)}
            <input
                id=id
                class="uc-input"
                type="text"
                required=required
                placeholder=placeholder
                autocomplete=autocomplete
                prop:value=move || value.get()
                on:input=move |e| value.set(event_target_value(&e))
            />
        </div>
    }
}

/// Several lines of text; `code` for a definition, in the code font.
#[component]
pub fn TextArea(
    id: &'static str,
    label: &'static str,
    value: RwSignal<String>,
    #[prop(optional)] required: bool,
    #[prop(optional)] code: bool,
) -> impl IntoView {
    let class = if code {
        "uc-textarea uc-textarea--code"
    } else {
        "uc-textarea"
    };
    view! {
        <div class="uc-field">
            {field_label(id, label, required)}
            <textarea
                id=id
                class=class
                spellcheck=code.then_some("false")
                prop:value=move || value.get()
                on:input=move |e| value.set(event_target_value(&e))
            />
        </div>
    }
}

/// A choice among `choices`: `name` is a choice's value in the `<select>`,
/// and `text`, else `name`, what it shows.
#[component]
pub fn Select<T>(
    id: &'static str,
    label: &'static str,
    choices: &'static [T],
    value: RwSignal<T>,
    name: fn(&T) -> String,
    #[prop(optional)] text: Option<fn(&T) -> String>,
    /// As wide as its text, in a row of fields.
    #[prop(optional)]
    fixed: bool,
) -> impl IntoView
where
    T: Copy + PartialEq + Send + Sync + 'static,
{
    let text = text.unwrap_or(name);
    let class = if fixed {
        "uc-field uc-field--fixed"
    } else {
        "uc-field"
    };
    // The choice is marked on its option: a `value` set on the `<select>`
    // itself would come before its options exist.
    view! {
        <div class=class>
            {field_label(id, label, false)}
            <select
                id=id
                class="uc-select"
                on:change=move |e| {
                    let picked = event_target_value(&e);
                    if let Some(choice) = choices.iter().find(|choice| name(choice) == picked) {
                        value.set(*choice);
                    }
                }
            >
                {choices
                    .iter()
                    .map(|choice| {
                        view! {
                            <option value=name(choice) prop:selected=move || value.get() == *choice>
                                {text(choice)}
                            </option>
                        }
                    })
                    .collect_view()}
            </select>
        </div>
    }
}
