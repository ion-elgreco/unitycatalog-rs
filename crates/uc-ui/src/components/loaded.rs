//! A view's content once its data is loaded: before, a spinner; after a
//! failure, the server's message.

use leptos::prelude::*;

use super::spinner::LoadingBlock;
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

#[component]
pub fn LoadError(message: String) -> impl IntoView {
    view! { <div class="uc-load-error">{message}</div> }
}
