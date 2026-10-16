//! Breadcrumbs and the title at the top; below, the content beside an
//! aside with the details: two thirds and one third, as antd's columns of
//! 16 and 8; in a narrow window, the aside above the content. Equivalent to
//! upstream `components/layouts/DetailsLayout.tsx`.

use leptos::prelude::*;

/// One breadcrumb entry. `href` is `None` for the current/final crumb.
pub struct Breadcrumb {
    pub label: String,
    pub href: Option<String>,
}

impl Breadcrumb {
    pub fn link(label: impl Into<String>, href: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            href: Some(href.into()),
        }
    }

    pub fn current(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            href: None,
        }
    }
}

/// The layout of a detail page. Its children are the content and an
/// `<aside>`. A page shows one object, and builds this layout again for
/// another, so the breadcrumbs are plain values.
#[component]
pub fn DetailsLayout(
    title: AnyView,
    breadcrumbs: Vec<Breadcrumb>,
    children: Children,
) -> impl IntoView {
    let crumbs = breadcrumbs
        .into_iter()
        .enumerate()
        .map(|(i, crumb)| {
            let separator = (i > 0).then(|| {
                view! { <li class="uc-breadcrumbs__sep" aria-hidden="true">"/"</li> }
            });
            let item = match crumb.href {
                Some(href) => view! {
                    <li>
                        <a class="uc-breadcrumbs__link" href=href>{crumb.label}</a>
                    </li>
                }
                .into_any(),
                None => view! {
                    <li class="uc-breadcrumbs__current" aria-current="page">{crumb.label}</li>
                }
                .into_any(),
            };
            view! { {separator}{item} }
        })
        .collect_view();

    view! {
        <div class="uc-flex-col uc-gap-md uc-grow">
            <div>
                <nav class="uc-breadcrumbs" aria-label="Breadcrumb">
                    <ol>{crumbs}</ol>
                </nav>
                {title}
            </div>
            <div class="uc-details-grid">{children()}</div>
        </div>
    }
}
