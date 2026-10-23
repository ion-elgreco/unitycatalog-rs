//! Light, dark, or the OS's choice, as upstream's console (`lib/theme.tsx`):
//! kept in `localStorage` under `uc-ui-color-mode` and set as
//! `data-color-mode` on `<html>`, which `styles.css` reads. `index.html`
//! sets it before the first paint, so the page does not flash.

use leptos::prelude::*;

const STORAGE_KEY: &str = "uc-ui-color-mode";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ColorMode {
    Light,
    Dark,
    /// What the OS prefers.
    Auto,
}

impl ColorMode {
    pub const ALL: [ColorMode; 3] = [ColorMode::Light, ColorMode::Dark, ColorMode::Auto];

    fn as_str(self) -> &'static str {
        match self {
            ColorMode::Light => "light",
            ColorMode::Dark => "dark",
            ColorMode::Auto => "auto",
        }
    }

    fn parse(value: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|mode| mode.as_str() == value)
    }

    pub fn label(self) -> &'static str {
        match self {
            ColorMode::Light => "Light",
            ColorMode::Dark => "Dark",
            ColorMode::Auto => "System",
        }
    }
}

#[derive(Clone, Copy)]
pub struct Theme {
    pub mode: RwSignal<ColorMode>,
}

impl Theme {
    /// A new mode: on `<html>` and in `localStorage`.
    pub fn set(&self, mode: ColorMode) {
        self.mode.set(mode);
        if let Some(html) = document().document_element() {
            let _ = html.set_attribute("data-color-mode", mode.as_str());
        }
        if let Some(storage) = window().local_storage().ok().flatten() {
            let _ = storage.set_item(STORAGE_KEY, mode.as_str());
        }
    }
}

/// The kept mode, else `Auto`, which `index.html` has set on `<html>`.
pub fn provide_theme() {
    let kept = window()
        .local_storage()
        .ok()
        .flatten()
        .and_then(|s| s.get_item(STORAGE_KEY).ok().flatten())
        .and_then(|value| ColorMode::parse(&value))
        .unwrap_or(ColorMode::Auto);
    provide_context(Theme {
        mode: RwSignal::new(kept),
    });
}

pub fn use_theme() -> Theme {
    expect_context::<Theme>()
}
