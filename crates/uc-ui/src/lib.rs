//! The web UI, in Leptos 0.7, after upstream's new console (`ui-new`,
//! React with shadcn/ui).
//!
//! It has the layout, pages and behavior of
//! https://github.com/unitycatalog/unitycatalog/tree/main/ui-new, calls the
//! server with this workspace's `uc-client`, and adds what that console
//! leaves out: grants, temporary credentials, column comments and access
//! tokens.

pub mod api;
pub mod app;
pub mod auth;
pub mod components;
pub mod contents;
pub mod kind;
pub mod layout;
pub mod load;
pub mod pages;
pub mod preview;
pub mod search;
pub mod theme;
pub mod util;

use leptos::mount::mount_to_body;
use wasm_bindgen::prelude::*;

/// The entry point. `public/index.html` imports the loader (`uc_ui.js`),
/// which calls `start` once the wasm module is instantiated.
#[wasm_bindgen(start)]
pub fn start() {
    console_error_panic_hook::set_once();
    mount_to_body(app::App);
}
