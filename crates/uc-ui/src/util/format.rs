use js_sys::Date;
use uc_common::EpochMillis;
use wasm_bindgen::JsValue;

/// The browser's language, for the dates.
fn language() -> String {
    web_sys::window()
        .and_then(|w| w.navigator().language())
        .unwrap_or_else(|| "en-US".into())
}

/// The console's `formatTimestamp`: `Date.toLocaleString()`, such as
/// `6/5/2026, 7:14:51 PM`. A dash for a missing value, as the console's
/// pages show it.
pub fn format_timestamp(value: Option<EpochMillis>) -> String {
    let Some(ms) = value else {
        return "—".into();
    };
    let date = Date::new(&JsValue::from_f64(ms as f64));
    String::from(date.to_locale_string(&language(), &JsValue::UNDEFINED))
}

/// `value`, else a dash.
pub fn or_dash(value: Option<String>) -> String {
    match value {
        Some(value) if !value.is_empty() => value,
        _ => "—".into(),
    }
}

/// An object as the server sent it, for the "Raw JSON" card.
pub fn pretty_json<T: serde::Serialize>(value: &T) -> String {
    serde_json::to_string_pretty(value).unwrap_or_default()
}
