//! The Details tab, as the console's: a grid of labels and values
//! (`MetaGrid`), the properties (`PropertiesCard`) and the object as the
//! server sent it (the "Raw JSON" card).

use leptos::prelude::*;
use uc_common::SecurablePropertiesMap;

use super::card::Card;

/// The Details tab: the grid, the properties of an object that has them,
/// and the raw JSON.
#[component]
pub fn DetailsTab(
    #[prop(into)] rows: Signal<Vec<MetaRow>>,
    #[prop(into, optional)] properties: Option<Signal<Vec<(String, String)>>>,
    #[prop(into)] json: Signal<String>,
) -> impl IntoView {
    view! {
        <Card>
            <MetaGrid rows/>
        </Card>
        {properties.map(|properties| view! { <PropertiesCard properties/> })}
        <RawJsonCard json/>
    }
}

/// One row of the grid.
#[derive(Clone, PartialEq)]
pub struct MetaRow {
    pub label: &'static str,
    pub value: String,
    /// In the code font: ids and locations.
    pub mono: bool,
}

impl MetaRow {
    pub fn new(label: &'static str, value: impl Into<String>) -> Self {
        Self {
            label,
            value: value.into(),
            mono: false,
        }
    }

    pub fn mono(label: &'static str, value: impl Into<String>) -> Self {
        Self {
            label,
            value: value.into(),
            mono: true,
        }
    }
}

#[component]
pub fn MetaGrid(#[prop(into)] rows: Signal<Vec<MetaRow>>) -> impl IntoView {
    view! {
        <dl class="uc-meta">
            {move || {
                rows.get()
                    .into_iter()
                    .map(|row| {
                        let class = if row.mono { "uc-mono" } else { "" };
                        view! {
                            <dt>{row.label}</dt>
                            <dd class=class>{row.value}</dd>
                        }
                    })
                    .collect_view()
            }}
        </dl>
    }
}

/// A properties map as the card's rows, sorted by key.
pub fn properties(map: Option<&SecurablePropertiesMap>) -> Vec<(String, String)> {
    let mut rows: Vec<(String, String)> = map
        .map(|map| map.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
        .unwrap_or_default();
    rows.sort();
    rows
}

#[component]
pub fn PropertiesCard(#[prop(into)] properties: Signal<Vec<(String, String)>>) -> impl IntoView {
    view! {
        <Card title="Properties">
            {move || {
                let rows = properties.get();
                if rows.is_empty() {
                    return view! { <p class="uc-empty">"No properties."</p> }.into_any();
                }
                view! {
                    <div class="uc-table-wrap">
                        <table class="uc-table">
                            <thead>
                                <tr>
                                    <th>"Key"</th>
                                    <th>"Value"</th>
                                </tr>
                            </thead>
                            <tbody>
                                {rows
                                    .into_iter()
                                    .map(|(key, value)| {
                                        view! {
                                            <tr>
                                                <td class="uc-mono">{key}</td>
                                                <td class="uc-mono">{value}</td>
                                            </tr>
                                        }
                                    })
                                    .collect_view()}
                            </tbody>
                        </table>
                    </div>
                }
                    .into_any()
            }}
        </Card>
    }
}

#[component]
pub fn RawJsonCard(#[prop(into)] json: Signal<String>) -> impl IntoView {
    view! {
        <Card title="Raw JSON">
            <pre class="uc-code">{move || json.get()}</pre>
        </Card>
    }
}
