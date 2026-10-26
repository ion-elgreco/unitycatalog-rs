//! A short text with a logo before it, such as a table's format: "DELTA"
//! with the Delta Lake logo. The header's badges, the Details grid and the
//! schema page's list show labels.

use buffa::EnumValue;
use leptos::prelude::*;
use uc_client::models::tables::v1::DataSourceFormat;

use crate::util::{enum_name, or_dash};

/// A text, with a logo or without.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Label {
    pub text: String,
    /// The logo's file in the UI's build, such as `/formats/delta.svg`.
    pub logo: Option<&'static str>,
}

impl Label {
    pub fn new(text: impl Into<String>, logo: Option<&'static str>) -> Self {
        Self {
            text: text.into(),
            logo,
        }
    }

    /// A table's format, with its logo; a dash for none.
    pub fn format(format: Option<EnumValue<DataSourceFormat>>) -> Self {
        Self::new(or_dash(format.map(enum_name)), format.and_then(format_logo))
    }
}

impl From<String> for Label {
    fn from(text: String) -> Self {
        Self::new(text, None)
    }
}

impl From<&str> for Label {
    fn from(text: &str) -> Self {
        Self::new(text, None)
    }
}

/// The logo of a table format, for the formats that have one.
pub fn format_logo(format: EnumValue<DataSourceFormat>) -> Option<&'static str> {
    match format.as_known()? {
        DataSourceFormat::Delta => Some("/formats/delta.svg"),
        DataSourceFormat::Iceberg => Some("/formats/iceberg.png"),
        DataSourceFormat::Parquet => Some("/formats/parquet.svg"),
        DataSourceFormat::Lance => Some("/formats/lance.png"),
        _ => None,
    }
}

/// The logo, then the text. The logo has no alt text: the text names it.
#[component]
pub fn LabelText(label: Label) -> impl IntoView {
    match label.logo {
        Some(logo) => view! {
            <span class="uc-logo-text">
                <img class="uc-logo-text__logo" src=logo alt=""/>
                {label.text}
            </span>
        }
        .into_any(),
        None => label.text.into_any(),
    }
}
