use buffa::{EnumValue, Enumeration};
use js_sys::{Array, Date, Function, Intl, Object, Reflect};
use uc_common::EpochMillis;
use wasm_bindgen::JsValue;

thread_local! {
    /// Upstream's `Intl.DateTimeFormat`, made once.
    static FORMAT: Function = {
        let options = Object::new();
        for (key, value) in [
            ("year", "numeric"),
            ("month", "2-digit"),
            ("day", "2-digit"),
            ("hour", "2-digit"),
            ("minute", "2-digit"),
            ("second", "2-digit"),
        ] {
            let _ = Reflect::set(&options, &key.into(), &value.into());
        }
        let _ = Reflect::set(&options, &"hour12".into(), &false.into());
        let locales = Array::of1(&"en-US".into());
        Intl::DateTimeFormat::new(&locales, &options).format()
    };
}

/// Upstream's `formatTimestamp`: an epoch-millis time in the browser's
/// time zone, as `en-US` numbers, such as `10/15/2026, 17:27:41`. Empty for
/// 0, a time that the server did not send, as upstream's pages show a
/// missing value.
pub fn format_timestamp(ms: EpochMillis) -> String {
    if ms == 0 {
        return String::new();
    }
    let date = Date::new(&JsValue::from_f64(ms as f64));
    FORMAT
        .with(|format| format.call1(&JsValue::NULL, &date))
        .ok()
        .and_then(|text| text.as_string())
        .unwrap_or_default()
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
