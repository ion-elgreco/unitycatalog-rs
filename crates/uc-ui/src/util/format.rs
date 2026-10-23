use js_sys::{Array, Date, Function, Intl, Object, Reflect};
use uc_common::EpochMillis;
use wasm_bindgen::JsValue;

thread_local! {
    /// The console's `Date.toLocaleString()`, as an `Intl.DateTimeFormat`
    /// made once: the browser's language, and the date and the time in
    /// numbers.
    static FORMAT: Function = {
        let options = Object::new();
        for key in ["year", "month", "day", "hour", "minute", "second"] {
            let _ = Reflect::set(&options, &key.into(), &"numeric".into());
        }
        let language = web_sys::window()
            .and_then(|w| w.navigator().language())
            .unwrap_or_else(|| "en-US".into());
        Intl::DateTimeFormat::new(&Array::of1(&language.into()), &options).format()
    };
}

/// The console's `formatTimestamp`: `Date.toLocaleString()`, such as
/// `6/5/2026, 7:14:51 PM`. A dash for a missing value, as the console's
/// pages show it.
pub fn format_timestamp(value: Option<EpochMillis>) -> String {
    let Some(ms) = value else {
        return "—".into();
    };
    let date = Date::new(&JsValue::from_f64(ms as f64));
    FORMAT
        .with(|format| format.call1(&JsValue::NULL, &date))
        .ok()
        .and_then(|text| text.as_string())
        .unwrap_or_default()
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
