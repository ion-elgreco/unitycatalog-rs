// @generated — do not edit by hand.
#![allow(unexpected_cfgs)]
#![allow(clippy::empty_docs)]
#![allow(clippy::useless_borrows_in_formatting)]
#![allow(non_camel_case_types)]
#![allow(clippy::derivable_impls)]
#![allow(clippy::match_single_binding)]
#![allow(clippy::uninlined_format_args)]
#![allow(clippy::doc_lazy_continuation)]

use std::collections::HashMap;

#[cfg(feature = "python")]
pub mod pyo3_impls;
pub use catalogs::v1::CatalogInfo;
pub use credentials::v1::CredentialInfo;
pub use external_locations::v1::ExternalLocationInfo;
pub use functions::v1::FunctionInfo;
pub use models::v1::ModelVersionInfo;
pub use models::v1::RegisteredModelInfo;
#[cfg(feature = "python")]
pub use pyo3_impls::*;
pub use recipients::v1::RecipientInfo;
pub use schemas::v1::SchemaInfo;
pub use shares::v1::ShareInfo;
pub use tables::v1::TableInfo;
pub use tokens::v1::PatInfo;
pub use volumes::v1::VolumeInfo;

pub type PropertyMap = HashMap<String, serde_json::Value>;
pub mod admin {
    pub mod v1 {
        include!("./unitycatalog.admin.v1.rs");
    }
}

pub mod audit {
    pub mod v1 {
        include!("./unitycatalog.audit.v1.rs");
    }
}

pub mod auth {
    pub mod v1 {
        include!("./unitycatalog.auth.v1.rs");
    }
}

pub mod catalogs {
    pub mod v1 {
        include!("./unitycatalog.catalogs.v1.rs");
    }
}

pub mod credentials {
    pub mod v1 {
        include!("./unitycatalog.credentials.v1.rs");
    }
}

pub mod delta {
    pub mod v1 {
        include!("./unitycatalog.delta.v1.rs");
    }
}

pub mod external_locations {
    pub mod v1 {
        include!("./unitycatalog.external_locations.v1.rs");
    }
}

pub mod files {
    pub mod v1 {
        include!("./unitycatalog.files.v1.rs");
    }
}

pub mod functions {
    pub mod v1 {
        include!("./unitycatalog.functions.v1.rs");
    }
}

pub mod health {
    pub mod v1 {
        include!("./unitycatalog.health.v1.rs");
    }
}

pub mod iceberg {
    pub mod v1 {
        include!("./unitycatalog.iceberg.v1.rs");
    }
}

pub mod metastore {
    pub mod v1 {
        include!("./unitycatalog.metastore.v1.rs");
    }
}

pub mod models {
    pub mod v1 {
        include!("./unitycatalog.models.v1.rs");
    }
}

pub mod permissions {
    pub mod v1 {
        include!("./unitycatalog.permissions.v1.rs");
    }
}

pub mod recipients {
    pub mod v1 {
        include!("./unitycatalog.recipients.v1.rs");
    }
}

pub mod schemas {
    pub mod v1 {
        include!("./unitycatalog.schemas.v1.rs");
    }
}

pub mod scim {
    pub mod v1 {
        include!("./unitycatalog.scim.v1.rs");
    }
}

pub mod search {
    pub mod v1 {
        include!("./unitycatalog.search.v1.rs");
    }
}

pub mod shares {
    pub mod v1 {
        include!("./unitycatalog.shares.v1.rs");
    }
}

pub mod sharing {
    pub mod v1 {
        include!("./unitycatalog.sharing.v1.rs");
    }
}

pub mod tables {
    pub mod v1 {
        include!("./unitycatalog.tables.v1.rs");
    }
}

pub mod tokens {
    pub mod v1 {
        include!("./unitycatalog.tokens.v1.rs");
    }
}

pub mod volumes {
    pub mod v1 {
        include!("./unitycatalog.volumes.v1.rs");
    }
}

/// Serde helpers that write 64-bit integers as JSON numbers, not as the quoted strings of
/// proto3 JSON. Reading accepts both.
pub mod json_int64 {
    /// `i64` as a JSON number.
    pub mod int64 {
        pub fn serialize<S: ::serde::Serializer>(
            value: &i64,
            s: S,
        ) -> ::core::result::Result<S::Ok, S::Error> {
            s.serialize_i64(*value)
        }

        pub fn deserialize<'de, D: ::serde::Deserializer<'de>>(
            d: D,
        ) -> ::core::result::Result<i64, D::Error> {
            ::buffa::json_helpers::int64::deserialize(d)
        }
    }

    /// `Option<i64>` as a JSON number or `null`.
    pub mod opt_int64 {
        pub fn serialize<S: ::serde::Serializer>(
            value: &::core::option::Option<i64>,
            s: S,
        ) -> ::core::result::Result<S::Ok, S::Error> {
            match value {
                ::core::option::Option::Some(v) => s.serialize_i64(*v),
                ::core::option::Option::None => s.serialize_none(),
            }
        }

        pub fn deserialize<'de, D: ::serde::Deserializer<'de>>(
            d: D,
        ) -> ::core::result::Result<::core::option::Option<i64>, D::Error> {
            ::buffa::json_helpers::opt_int64::deserialize(d)
        }
    }

    /// `u64` as a JSON number.
    pub mod uint64 {
        pub fn serialize<S: ::serde::Serializer>(
            value: &u64,
            s: S,
        ) -> ::core::result::Result<S::Ok, S::Error> {
            s.serialize_u64(*value)
        }

        pub fn deserialize<'de, D: ::serde::Deserializer<'de>>(
            d: D,
        ) -> ::core::result::Result<u64, D::Error> {
            ::buffa::json_helpers::uint64::deserialize(d)
        }
    }

    /// `Option<u64>` as a JSON number or `null`.
    pub mod opt_uint64 {
        pub fn serialize<S: ::serde::Serializer>(
            value: &::core::option::Option<u64>,
            s: S,
        ) -> ::core::result::Result<S::Ok, S::Error> {
            match value {
                ::core::option::Option::Some(v) => s.serialize_u64(*v),
                ::core::option::Option::None => s.serialize_none(),
            }
        }

        pub fn deserialize<'de, D: ::serde::Deserializer<'de>>(
            d: D,
        ) -> ::core::result::Result<::core::option::Option<u64>, D::Error> {
            ::buffa::json_helpers::opt_uint64::deserialize(d)
        }
    }

    /// A repeated `i64` or `u64` as an array of JSON numbers.
    pub mod repeated_int64 {
        pub use ::buffa::json_helpers::proto_seq::deserialize;

        pub fn serialize<T: ::serde::Serialize, S: ::serde::Serializer>(
            values: &[T],
            s: S,
        ) -> ::core::result::Result<S::Ok, S::Error> {
            s.collect_seq(values)
        }
    }

    /// A map with `i64` or `u64` values, the values as JSON numbers.
    pub mod map_int64 {
        pub use ::buffa::json_helpers::proto_map::deserialize;

        pub fn serialize<'a, M, K, V, S>(
            map: &'a M,
            s: S,
        ) -> ::core::result::Result<S::Ok, S::Error>
        where
            &'a M: ::core::iter::IntoIterator<Item = (&'a K, &'a V)>,
            K: ::core::fmt::Display + 'a,
            V: ::serde::Serialize + 'a,
            S: ::serde::Serializer,
        {
            s.collect_map(map.into_iter().map(|(k, v)| (k.to_string(), v)))
        }
    }
}

/// Serializers of the free-form JSON fields that write whole numbers as JSON integers
/// (`1`, not `1.0`). Reading is buffa's.
pub mod json_value {
    use ::buffa_types::google::protobuf::value::Kind;
    use ::buffa_types::google::protobuf::{ListValue, Struct, Value};
    use ::serde::Serialize;

    /// The largest whole number that an `f64` holds exactly: 2^53.
    const EXACT: f64 = 9_007_199_254_740_992.0;
    /// A free-form JSON type, serialized with whole numbers as integers.
    pub trait Exact {
        fn serialize_exact<S: ::serde::Serializer>(
            &self,
            s: S,
        ) -> ::core::result::Result<S::Ok, S::Error>;
    }

    impl Exact for Value {
        fn serialize_exact<S: ::serde::Serializer>(
            &self,
            s: S,
        ) -> ::core::result::Result<S::Ok, S::Error> {
            match &self.kind {
                ::core::option::Option::Some(Kind::NumberValue(n))
                    if n.fract() == 0.0 && n.abs() <= EXACT =>
                {
                    s.serialize_i64(*n as i64)
                }
                ::core::option::Option::Some(Kind::StructValue(st)) => st.serialize_exact(s),
                ::core::option::Option::Some(Kind::ListValue(list)) => list.serialize_exact(s),
                _ => self.serialize(s),
            }
        }
    }

    impl Exact for Struct {
        fn serialize_exact<S: ::serde::Serializer>(
            &self,
            s: S,
        ) -> ::core::result::Result<S::Ok, S::Error> {
            s.collect_map(self.fields.iter().map(|(k, v)| (k, Ser(v))))
        }
    }

    impl Exact for ListValue {
        fn serialize_exact<S: ::serde::Serializer>(
            &self,
            s: S,
        ) -> ::core::result::Result<S::Ok, S::Error> {
            s.collect_seq(self.values.iter().map(Ser))
        }
    }

    /// Serializes with [`Exact`].
    struct Ser<'a, T>(&'a T);
    impl<T: Exact> Serialize for Ser<'_, T> {
        fn serialize<S: ::serde::Serializer>(
            &self,
            s: S,
        ) -> ::core::result::Result<S::Ok, S::Error> {
            self.0.serialize_exact(s)
        }
    }

    /// A singular field.
    pub fn message_field<T: Exact + Default, S: ::serde::Serializer>(
        field: &::buffa::MessageField<T>,
        s: S,
    ) -> ::core::result::Result<S::Ok, S::Error> {
        match field.as_option() {
            ::core::option::Option::Some(value) => value.serialize_exact(s),
            ::core::option::Option::None => s.serialize_none(),
        }
    }

    /// A repeated field.
    pub fn repeated<T: Exact, S: ::serde::Serializer>(
        values: &[T],
        s: S,
    ) -> ::core::result::Result<S::Ok, S::Error> {
        s.collect_seq(values.iter().map(Ser))
    }

    /// A map field.
    pub fn map<T: Exact, S: ::serde::Serializer>(
        values: &::buffa::__private::HashMap<::buffa::alloc::string::String, T>,
        s: S,
    ) -> ::core::result::Result<S::Ok, S::Error> {
        s.collect_map(values.iter().map(|(k, v)| (k, Ser(v))))
    }
}
