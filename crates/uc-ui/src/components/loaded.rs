//! A view's content once its data is loaded: before, a skeleton, as the
//! console's `QueryState`; after a failure, the server's message.

use leptos::prelude::*;

use super::empty::{Empty, EmptyState};
use crate::load::Load;

/// How far a load is, without its value.
#[derive(Clone, PartialEq)]
pub enum Stage {
    Loading,
    Ready,
    Failed(String),
}

impl Stage {
    pub fn of<T>(load: &Load<T>) -> Self {
        match load {
            Load::Loading => Stage::Loading,
            Load::Ready(_) => Stage::Ready,
            Load::Failed(message) => Stage::Failed(message.clone()),
        }
    }
}

/// `children` once `state` is ready. They read the value from `state`, so
/// a new value (such as an edited comment) does not build them again.
#[component]
pub fn Loaded<T>(state: RwSignal<Load<T>>, children: ChildrenFn) -> impl IntoView
where
    T: Send + Sync + 'static,
{
    let stage = Memo::new(move |_| state.with(Stage::of));
    move || match stage.get() {
        Stage::Loading => view! { <LoadingBlock/> }.into_any(),
        Stage::Failed(message) => view! { <LoadError message/> }.into_any(),
        Stage::Ready => children().into_any(),
    }
}

/// A loaded list, as a view: the skeleton while it loads, the failure,
/// `empty` for none, else `rows` of the items.
pub fn loaded_list<T>(
    load: Load<Vec<T>>,
    empty: impl Into<Empty>,
    rows: impl FnOnce(Vec<T>) -> AnyView,
) -> AnyView {
    match load {
        Load::Loading => view! { <LoadingBlock/> }.into_any(),
        Load::Failed(message) => view! { <LoadError message/> }.into_any(),
        Load::Ready(list) if list.is_empty() => view! { <EmptyState empty/> }.into_any(),
        Load::Ready(list) => rows(list),
    }
}

/// A card's title with the count of a loaded list, as the console's:
/// "Schemas (3)", and 0 until the list loads.
pub fn counted<T: Send + Sync + 'static>(
    label: &'static str,
    list: RwSignal<Load<Vec<T>>>,
) -> impl Fn() -> String + Copy {
    move || format!("{label} ({})", list.with(|l| l.ready().map_or(0, Vec::len)))
}

/// The console's skeleton: a short bar and a block that pulse.
#[component]
pub fn LoadingBlock() -> impl IntoView {
    view! {
        <div class="uc-skeleton" role="status" aria-label="Loading">
            <div class="uc-skeleton__bar"></div>
            <div class="uc-skeleton__bar uc-skeleton__bar--block"></div>
        </div>
    }
}

#[component]
pub fn LoadError(message: String) -> impl IntoView {
    view! { <p class="uc-error" role="alert">{message}</p> }
}
