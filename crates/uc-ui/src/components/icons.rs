//! Lucide's icons (https://lucide.dev, ISC), as inline SVG: the ones that
//! upstream's console uses, under their lucide names. Each is a 24 by 24
//! outline in the text's color.

use leptos::prelude::*;

/// The contents of an icon's `<svg>`.
struct Glyph(&'static str);

/// `glyph`, `1em` square, in the text's color. `class` goes on the
/// wrapper, such as `uc-icon--table` for a kind's color.
fn icon(glyph: &Glyph, class: &'static str) -> impl IntoView {
    let class = if class.is_empty() {
        "uc-icon".to_string()
    } else {
        format!("uc-icon {class}")
    };
    view! {
        <span class=class aria-hidden="true">
            <svg
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                stroke-width="2"
                stroke-linecap="round"
                stroke-linejoin="round"
                inner_html=glyph.0
            ></svg>
        </span>
    }
}

macro_rules! lucide {
    ($(#[$doc:meta])* $name:ident, $body:literal) => {
        $(#[$doc])*
        #[component]
        pub fn $name(#[prop(optional)] class: &'static str) -> impl IntoView {
            const GLYPH: Glyph = Glyph($body);
            icon(&GLYPH, class)
        }
    };
}

lucide!(/// `chevron-right`.
    ChevronRight, r#"<path d="m9 18 6-6-6-6"/>"#);
lucide!(/// `chevron-down`.
    ChevronDown, r#"<path d="m6 9 6 6 6-6"/>"#);
lucide!(/// `chevron-left`.
    ChevronLeft, r#"<path d="m15 18-6-6 6-6"/>"#);
lucide!(/// `chevrons-left`.
    ChevronsLeft, r#"<path d="m11 17-5-5 5-5"/><path d="m18 17-5-5 5-5"/>"#);
lucide!(/// `chevrons-right`.
    ChevronsRight, r#"<path d="m6 17 5-5-5-5"/><path d="m13 17 5-5-5-5"/>"#);
lucide!(/// `x`.
    X, r#"<path d="M18 6 6 18"/><path d="m6 6 12 12"/>"#);
lucide!(/// `plus`.
    Plus, r#"<path d="M5 12h14"/><path d="M12 5v14"/>"#);
lucide!(/// `check`.
    Check, r#"<path d="M20 6 9 17l-5-5"/>"#);
lucide!(/// `search`.
    Search, r#"<circle cx="11" cy="11" r="8"/><path d="m21 21-4.3-4.3"/>"#);
lucide!(/// `sun`: the light theme.
    Sun, r#"<circle cx="12" cy="12" r="4"/><path d="M12 2v2"/><path d="M12 20v2"/><path d="m4.93 4.93 1.41 1.41"/><path d="m17.66 17.66 1.41 1.41"/><path d="M2 12h2"/><path d="M20 12h2"/><path d="m6.34 17.66-1.41 1.41"/><path d="m19.07 4.93-1.41 1.41"/>"#);
lucide!(/// `moon`: the dark theme.
    Moon, r#"<path d="M12 3a6 6 0 0 0 9 9 9 9 0 1 1-9-9Z"/>"#);
lucide!(/// `monitor`: the system's theme.
    Monitor, r#"<rect width="20" height="14" x="2" y="3" rx="2"/><line x1="8" x2="16" y1="21" y2="21"/><line x1="12" x2="12" y1="17" y2="21"/>"#);
lucide!(/// `user`.
    User, r#"<path d="M19 21v-2a4 4 0 0 0-4-4H9a4 4 0 0 0-4 4v2"/><circle cx="12" cy="7" r="4"/>"#);
lucide!(/// `log-out`.
    LogOut, r#"<path d="M9 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h4"/><polyline points="16 17 21 12 16 7"/><line x1="21" x2="9" y1="12" y2="12"/>"#);
lucide!(/// `panel-left-close`.
    PanelLeftClose, r#"<rect width="18" height="18" x="3" y="3" rx="2"/><path d="M9 3v18"/><path d="m16 15-3-3 3-3"/>"#);
lucide!(/// `panel-left-open`.
    PanelLeftOpen, r#"<rect width="18" height="18" x="3" y="3" rx="2"/><path d="M9 3v18"/><path d="m14 9 3 3-3 3"/>"#);
lucide!(/// `notebook`: a catalog.
    Notebook, r#"<path d="M2 6h4"/><path d="M2 10h4"/><path d="M2 14h4"/><path d="M2 18h4"/><rect width="16" height="20" x="4" y="2" rx="2"/><path d="M16 2v20"/>"#);
lucide!(/// `database`: a schema.
    Database, r#"<ellipse cx="12" cy="5" rx="9" ry="3"/><path d="M3 5V19A9 3 0 0 0 21 19V5"/><path d="M3 12A9 3 0 0 0 21 12"/>"#);
lucide!(/// `table`.
    Table, r#"<path d="M12 3v18"/><rect width="18" height="18" x="3" y="3" rx="2"/><path d="M3 9h18"/><path d="M3 15h18"/>"#);
lucide!(/// `hard-drive`: a volume.
    HardDrive, r#"<line x1="22" x2="2" y1="12" y2="12"/><path d="M5.45 5.11 2 12v6a2 2 0 0 0 2 2h16a2 2 0 0 0 2-2v-6l-3.45-6.89A2 2 0 0 0 16.76 4H7.24a2 2 0 0 0-1.79 1.11z"/><line x1="6" x2="6.01" y1="16" y2="16"/><line x1="10" x2="10.01" y1="16" y2="16"/>"#);
lucide!(/// `function-square`.
    FunctionSquare, r#"<rect width="18" height="18" x="3" y="3" rx="2"/><path d="M9 17c2 0 2.8-1 2.8-2.8V10c0-2 1-3.3 3.2-3"/><path d="M9 11.2h5.7"/>"#);
lucide!(/// `box`: a registered model.
    Box, r#"<path d="M21 8a2 2 0 0 0-1-1.73l-7-4a2 2 0 0 0-2 0l-7 4A2 2 0 0 0 3 8v8a2 2 0 0 0 1 1.73l7 4a2 2 0 0 0 2 0l7-4A2 2 0 0 0 21 16Z"/><path d="m3.3 7 8.7 5 8.7-5"/><path d="M12 22V12"/>"#);
lucide!(/// `pencil`.
    Pencil, r#"<path d="M21.174 6.812a1 1 0 0 0-3.986-3.987L3.842 16.174a2 2 0 0 0-.5.83l-1.321 4.352a.5.5 0 0 0 .623.622l4.353-1.32a2 2 0 0 0 .83-.497z"/><path d="m15 5 4 4"/>"#);
lucide!(/// `trash-2`.
    Trash2, r#"<path d="M3 6h18"/><path d="M19 6v14c0 1-1 2-2 2H7c-1 0-2-1-2-2V6"/><path d="M8 6V4c0-1 1-2 2-2h4c1 0 2 1 2 2v2"/><line x1="10" x2="10" y1="11" y2="17"/><line x1="14" x2="14" y1="11" y2="17"/>"#);
lucide!(/// `refresh-cw`.
    RefreshCw, r#"<path d="M3 12a9 9 0 0 1 9-9 9.75 9.75 0 0 1 6.74 2.74L21 8"/><path d="M21 3v5h-5"/><path d="M21 12a9 9 0 0 1-9 9 9.75 9.75 0 0 1-6.74-2.74L3 16"/><path d="M8 16H3v5"/>"#);
lucide!(/// `shield-check`: permissions.
    ShieldCheck, r#"<path d="M20 13c0 5-3.5 7.5-7.66 8.95a1 1 0 0 1-.67-.01C7.5 20.5 4 18 4 13V6a1 1 0 0 1 1-1c2 0 4.5-1.2 6.24-2.72a1.17 1.17 0 0 1 1.52 0C14.51 3.81 17 5 19 5a1 1 0 0 1 1 1z"/><path d="m9 12 2 2 4-4"/>"#);
lucide!(/// `copy`.
    Copy, r#"<rect width="14" height="14" x="8" y="8" rx="2" ry="2"/><path d="M4 16c-1.1 0-2-.9-2-2V4c0-1.1.9-2 2-2h10c1.1 0 2 .9 2 2"/>"#);
lucide!(/// `key-round`: tokens and credentials.
    KeyRound, r#"<path d="M2.586 17.414A2 2 0 0 0 2 18.828V21a1 1 0 0 0 1 1h3a1 1 0 0 0 1-1v-1a1 1 0 0 1 1-1h1a1 1 0 0 0 1-1v-1a1 1 0 0 1 1-1h.172a2 2 0 0 0 1.414-.586l.814-.814a6.5 6.5 0 1 0-4-4z"/><circle cx="16.5" cy="7.5" r=".5" fill="currentColor"/>"#);
lucide!(/// `external-link`.
    ExternalLink, r#"<path d="M15 3h6v6"/><path d="M10 14 21 3"/><path d="M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6"/>"#);
lucide!(/// `circle-check`.
    CircleCheck, r#"<circle cx="12" cy="12" r="10"/><path d="m9 12 2 2 4-4"/>"#);
lucide!(/// `circle-x`.
    CircleX, r#"<circle cx="12" cy="12" r="10"/><path d="m15 9-6 6"/><path d="m9 9 6 6"/>"#);
lucide!(/// `circle-minus`.
    CircleMinus, r#"<circle cx="12" cy="12" r="10"/><path d="M8 12h8"/>"#);
lucide!(/// `loader-circle`: a call runs.
    LoaderCircle, r#"<path d="M21 12a9 9 0 1 1-6.219-8.56"/>"#);
lucide!(/// `triangle-alert`.
    TriangleAlert, r#"<path d="m21.73 18-8-14a2 2 0 0 0-3.48 0l-8 14A2 2 0 0 0 4 21h16a2 2 0 0 0 1.73-3"/><path d="M12 9v4"/><path d="M12 17h.01"/>"#);
lucide!(/// `eye`: show a secret.
    Eye, r#"<path d="M2.062 12.348a1 1 0 0 1 0-.696 10.75 10.75 0 0 1 19.876 0 1 1 0 0 1 0 .696 10.75 10.75 0 0 1-19.876 0"/><circle cx="12" cy="12" r="3"/>"#);
lucide!(/// `eye-off`: hide a secret.
    EyeOff, r#"<path d="M10.733 5.076a10.744 10.744 0 0 1 11.205 6.575 1 1 0 0 1 0 .696 10.747 10.747 0 0 1-1.444 2.49"/><path d="M14.084 14.158a3 3 0 0 1-4.242-4.242"/><path d="M17.479 17.499a10.75 10.75 0 0 1-15.417-5.151 1 1 0 0 1 0-.696 10.75 10.75 0 0 1 4.446-5.143"/><path d="m2 2 20 20"/>"#);
lucide!(/// `info`.
    Info, r#"<circle cx="12" cy="12" r="10"/><path d="M12 16v-4"/><path d="M12 8h.01"/>"#);
