use leptos::prelude::*;

/// One row in the details column of a detail page, with its value as
/// formatted text.
#[derive(Clone)]
pub struct MetadataRow {
    pub label: &'static str,
    pub value: String,
    pub code: bool,
}

/// The rows with a value, as upstream's `MetadataList` skips the others.
#[component]
pub fn MetadataList(
    title: &'static str,
    #[prop(into)] rows: Signal<Vec<MetadataRow>>,
) -> impl IntoView {
    view! {
        <div class="uc-flex-col uc-gap-md">
            <h5 class="uc-title-5">{title}</h5>
            {move || {
                rows.get()
                    .into_iter()
                    .filter(|row| !row.value.is_empty())
                    .map(|MetadataRow { label, value, code }| {
                        let class = if code { "uc-text-code" } else { "" };
                        view! {
                            <div>
                                <span class="uc-text-strong">{label}": "</span>
                                <span class=class>{value}</span>
                            </div>
                        }
                    })
                    .collect_view()
            }}
        </div>
    }
}
