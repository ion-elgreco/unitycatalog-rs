use buffa::{EnumValue, Enumeration};
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

/// A size in bytes, as `512 B`, `1.5 KB` or `12.3 MB`: kilo is 1024.
pub fn format_size(bytes: i64) -> String {
    const UNITS: [&str; 5] = ["KB", "MB", "GB", "TB", "PB"];
    if bytes < 1024 {
        return format!("{bytes} B");
    }
    let mut size = bytes as f64 / 1024.0;
    let mut unit = 0;
    while size >= 1024.0 && unit < UNITS.len() - 1 {
        size /= 1024.0;
        unit += 1;
    }
    format!("{size:.1} {}", UNITS[unit])
}

/// The console's `formatTimestamp`: `Date.toLocaleString()`, such as
/// `6/5/2026, 7:14:51 PM`. A dash for 0, a time that the server did not
/// send, as the console's pages show a missing value.
pub fn format_timestamp(ms: EpochMillis) -> String {
    if ms == 0 {
        return "—".into();
    }
    let date = Date::new(&JsValue::from_f64(ms as f64));
    FORMAT
        .with(|format| format.call1(&JsValue::NULL, &date))
        .ok()
        .and_then(|text| text.as_string())
        .unwrap_or_default()
}

/// `value`, else a dash: also for an empty one, which the server did not
/// send.
pub fn or_dash(value: impl Into<Option<String>>) -> String {
    match value.into() {
        Some(value) if !value.is_empty() => value,
        _ => "—".into(),
    }
}

/// An object as the server sent it, for the "Raw JSON" card.
pub fn pretty_json<T: serde::Serialize>(value: &T) -> String {
    serde_json::to_string_pretty(value).unwrap_or_default()
}

/// The name of an enum value as the server sends it, such as `MANAGED`.
/// Empty for 0 (`…_UNSPECIFIED`), a value that the server did not send,
/// and for a value that the UI does not know.
pub fn enum_name<E: Enumeration>(value: EnumValue<E>) -> String {
    match value.as_known() {
        Some(known) if known.to_i32() != 0 => known.proto_name().to_string(),
        _ => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::format_size;

    #[test]
    fn sizes() {
        assert_eq!(format_size(0), "0 B");
        assert_eq!(format_size(1023), "1023 B");
        assert_eq!(format_size(1536), "1.5 KB");
        assert_eq!(format_size(5 * 1024 * 1024), "5.0 MB");
        assert_eq!(format_size(3 * 1024_i64.pow(4)), "3.0 TB");
    }
}
