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
/// a missing value, as upstream's pages show it.
pub fn format_timestamp(value: Option<EpochMillis>) -> String {
    let Some(ms) = value else {
        return String::new();
    };
    let date = Date::new(&JsValue::from_f64(ms as f64));
    FORMAT
        .with(|format| format.call1(&JsValue::NULL, &date))
        .ok()
        .and_then(|text| text.as_string())
        .unwrap_or_default()
}
