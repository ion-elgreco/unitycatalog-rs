//! What a list says when it has nothing: a line in gray and, where a
//! first-time user lands, a hint under it that says what the objects are
//! and how to make one.

use leptos::prelude::*;

/// An empty list's text, with a hint or without.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Empty {
    pub text: String,
    pub hint: Option<&'static str>,
}

impl Empty {
    pub fn new(text: impl Into<String>, hint: &'static str) -> Self {
        Self {
            text: text.into(),
            hint: Some(hint),
        }
    }
}

impl From<&str> for Empty {
    fn from(text: &str) -> Self {
        Self {
            text: text.into(),
            hint: None,
        }
    }
}

/// The text, with the hint under it.
#[component]
pub fn EmptyState(#[prop(into)] empty: Empty) -> impl IntoView {
    view! {
        <div class="uc-empty">
            <p>{empty.text}</p>
            {empty.hint.map(|hint| view! { <p class="uc-empty__hint">{hint}</p> })}
        </div>
    }
}
