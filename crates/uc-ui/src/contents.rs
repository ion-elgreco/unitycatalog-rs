//! What a catalog, schema or registered model holds: what a delete with
//! `force` removes along with it, as the server cascades it. The UI counts
//! it with the API's lists ([`uc_client::Contents`]) before an admin
//! confirms such a delete.

pub use uc_client::Contents;

use crate::api;
use crate::kind::Kind;
use crate::load::Key;

/// An object that the server deletes only empty, unless the call sets
/// `force`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Holder {
    Catalog(String),
    /// A schema, by its catalog and its name.
    Schema(String, String),
    /// A registered model, by its full name.
    Model(String),
}

impl Holder {
    /// The text of the box that sets `force`.
    pub fn force_label(&self) -> &'static str {
        match self {
            Holder::Catalog(_) => {
                "Also delete its schemas, and their tables, volumes, functions and models"
            }
            Holder::Schema(..) => "Also delete its tables, volumes, functions and models",
            Holder::Model(_) => "Also delete its versions",
        }
    }

    /// The object, as the dialogs name it, such as "the catalog".
    pub fn noun(&self) -> &'static str {
        match self {
            Holder::Catalog(_) => "the catalog",
            Holder::Schema(..) => "the schema",
            Holder::Model(_) => "the model",
        }
    }

    /// What goes with the data of the objects it holds.
    pub fn data_note(&self) -> &'static str {
        match self {
            Holder::Catalog(_) | Holder::Schema(..) => {
                "The data of managed tables and volumes, and of models, goes too."
            }
            Holder::Model(_) => "The data of its versions goes too.",
        }
    }

    /// The lists of what it holds, which the UI forgets after the delete.
    pub fn lists(&self) -> Vec<Key> {
        match self {
            Holder::Catalog(catalog) => vec![Key::Schemas(catalog.clone())],
            Holder::Schema(catalog, schema) => Kind::ALL
                .into_iter()
                .map(|kind| Key::Entries(catalog.clone(), schema.clone(), kind))
                .collect(),
            Holder::Model(full_name) => vec![Key::Versions(full_name.clone())],
        }
    }

    /// Count what it holds.
    pub async fn count(self) -> api::Result<Contents> {
        let client = api::client();
        match self {
            Holder::Catalog(catalog) => client.catalog_contents(&catalog).await,
            Holder::Schema(catalog, schema) => client.schema_contents(&catalog, &schema).await,
            Holder::Model(full_name) => client.model_contents(&full_name).await,
        }
    }
}
