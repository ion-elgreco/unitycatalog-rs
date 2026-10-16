use serde::{Deserialize, Deserializer, Serialize, de};
use utoipa::{IntoParams, ToSchema};

use crate::{Error, Result};

/// Opaque page token. The next page starts after the item it names.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(transparent)]
pub struct PageToken(pub String);

impl PageToken {
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Page size used when `max_results` is absent or 0, and the largest page a
/// client can ask for. Upstream `PagedListingHelper.DEFAULT_PAGE_SIZE`.
pub const DEFAULT_PAGE_SIZE: i64 = 100;

/// Standard list-endpoint pagination query params.
#[derive(Debug, Clone, Default, Deserialize, IntoParams)]
pub struct Pagination {
    /// Maximum number of items to return. Server may return fewer.
    #[serde(default, deserialize_with = "lenient_opt_i64")]
    #[param(value_type = Option<i32>)]
    pub max_results: Option<i64>,
    /// Opaque token from a previous `next_page_token`.
    #[serde(default)]
    pub page_token: Option<String>,
}

impl Pagination {
    pub fn new(max_results: Option<i64>, page_token: Option<String>) -> Self {
        Self {
            max_results,
            page_token,
        }
    }

    /// Upstream `PagedListingHelper.getPageSize`: `max_results` capped at
    /// [`DEFAULT_PAGE_SIZE`]; absent or 0 means the default. A negative value
    /// is an `INVALID_ARGUMENT`, as in upstream `listEntity`.
    pub fn page_size(&self) -> Result<i64> {
        match self.max_results {
            Some(n) if n < 0 => Err(Error::invalid_argument(
                "maxResults must be greater than or equal to 0",
            )),
            Some(n) if n > 0 => Ok(n.min(DEFAULT_PAGE_SIZE)),
            _ => Ok(DEFAULT_PAGE_SIZE),
        }
    }

    /// Upstream `PagedListingHelper.getNextPageToken`: when a page is full,
    /// the token is the key of its last item; otherwise there is no token.
    /// (A full last page therefore yields one more, empty, page.)
    pub fn next_token<T, K: ToString>(
        &self,
        rows: &[T],
        key_of: impl Fn(&T) -> K,
    ) -> Result<Option<String>> {
        let size = self.page_size()?;
        if rows.is_empty() || (rows.len() as i64) < size {
            return Ok(None);
        }
        Ok(rows.last().map(|r| key_of(r).to_string()))
    }
}

/// Accept an integer given as a JSON number or as a string. Query strings
/// carry every value as a string, and `#[serde(flatten)]` buffers values in
/// a way that loses serde_urlencoded's string-to-number coercion, so plain
/// `Option<i64>` fails there. An empty string counts as absent.
pub fn lenient_opt_i64<'de, D: Deserializer<'de>>(
    d: D,
) -> std::result::Result<Option<i64>, D::Error> {
    struct V;
    impl<'de> de::Visitor<'de> for V {
        type Value = Option<i64>;
        fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            f.write_str("an integer")
        }
        fn visit_none<E: de::Error>(self) -> std::result::Result<Self::Value, E> {
            Ok(None)
        }
        fn visit_unit<E: de::Error>(self) -> std::result::Result<Self::Value, E> {
            Ok(None)
        }
        fn visit_some<D2: Deserializer<'de>>(
            self,
            d: D2,
        ) -> std::result::Result<Self::Value, D2::Error> {
            d.deserialize_any(self)
        }
        fn visit_i64<E: de::Error>(self, v: i64) -> std::result::Result<Self::Value, E> {
            Ok(Some(v))
        }
        fn visit_u64<E: de::Error>(self, v: u64) -> std::result::Result<Self::Value, E> {
            i64::try_from(v)
                .map(Some)
                .map_err(|_| E::custom(format!("integer out of range: {v}")))
        }
        fn visit_str<E: de::Error>(self, s: &str) -> std::result::Result<Self::Value, E> {
            let s = s.trim();
            if s.is_empty() {
                return Ok(None);
            }
            s.parse::<i64>()
                .map(Some)
                .map_err(|_| E::custom(format!("invalid integer value: '{s}'")))
        }
    }
    d.deserialize_any(V)
}

/// Accept a boolean given as a JSON bool or as a string (`true`/`false`,
/// any case). Same reason as [`lenient_opt_i64`].
pub fn lenient_bool<'de, D: Deserializer<'de>>(d: D) -> std::result::Result<bool, D::Error> {
    struct V;
    impl<'de> de::Visitor<'de> for V {
        type Value = bool;
        fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            f.write_str("a boolean")
        }
        fn visit_bool<E: de::Error>(self, v: bool) -> std::result::Result<bool, E> {
            Ok(v)
        }
        fn visit_str<E: de::Error>(self, s: &str) -> std::result::Result<bool, E> {
            match s.trim().to_ascii_lowercase().as_str() {
                "true" => Ok(true),
                "false" | "" => Ok(false),
                other => Err(E::custom(format!("invalid boolean value: '{other}'"))),
            }
        }
    }
    d.deserialize_any(V)
}

/// `?force=true` query for cascading deletes.
#[derive(Debug, Default, Deserialize)]
pub struct DeleteParams {
    #[serde(default, deserialize_with = "lenient_bool")]
    pub force: bool,
}

/// Standard list-endpoint params for resources scoped to a schema:
/// `?catalog_name=X&schema_name=Y` and [`Pagination`].
/// Used by tables/volumes/functions/models list endpoints.
#[derive(Debug, Deserialize)]
pub struct SchemaScopedListParams {
    pub catalog_name: String,
    pub schema_name: String,
    #[serde(flatten)]
    pub page: Pagination,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn page(q: &str) -> std::result::Result<Pagination, serde_urlencoded::de::Error> {
        serde_urlencoded::from_str(q)
    }

    #[test]
    fn page_size_follows_upstream() {
        assert_eq!(Pagination::new(None, None).page_size().unwrap(), 100);
        assert_eq!(Pagination::new(Some(0), None).page_size().unwrap(), 100);
        assert_eq!(Pagination::new(Some(7), None).page_size().unwrap(), 7);
        assert_eq!(Pagination::new(Some(1000), None).page_size().unwrap(), 100);
        assert!(Pagination::new(Some(-1), None).page_size().is_err());
    }

    #[test]
    fn next_token_only_for_full_pages() {
        let p = Pagination::new(Some(2), None);
        assert_eq!(p.next_token(&["a"], |s| s.to_string()).unwrap(), None);
        assert_eq!(
            p.next_token(&["a", "b"], |s| s.to_string()).unwrap(),
            Some("b".into())
        );
        assert_eq!(
            p.next_token::<&str, String>(&[], |s| s.to_string())
                .unwrap(),
            None
        );
    }

    #[test]
    fn flattened_query_params_parse() {
        let parsed: SchemaScopedListParams =
            serde_urlencoded::from_str("catalog_name=c&schema_name=s&max_results=10&page_token=t")
                .unwrap();
        assert_eq!(parsed.page.max_results, Some(10));
        assert_eq!(parsed.page.page_token.as_deref(), Some("t"));
        assert_eq!(page("max_results=").unwrap().max_results, None);
        assert!(page("max_results=ten").is_err());
    }
}
