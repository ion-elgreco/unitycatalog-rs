use wasm_bindgen::JsCast;

/// Copy `text`: with the Clipboard API where the browser has it (https and
/// localhost), else with the `copy` command on the selected text.
pub async fn copy_text(text: &str) -> bool {
    let window = leptos::prelude::window();
    let clipboard = window.navigator().clipboard();
    if !clipboard.is_undefined() {
        let written = clipboard.write_text(text);
        return wasm_bindgen_futures::JsFuture::from(written).await.is_ok();
    }
    leptos::prelude::document()
        .dyn_into::<web_sys::HtmlDocument>()
        .ok()
        .and_then(|document| document.exec_command("copy").ok())
        .unwrap_or(false)
}
