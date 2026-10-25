//! Request and response helpers for every route: the error body, the
//! extractors and the header spelling.

pub mod error;
pub mod extract;
pub mod header_case;
pub mod outbound;

/// `{}`: the answer of a call that upstream's spec answers with an empty JSON
/// object, such as a delete. Upstream's Java client reads every answer as
/// JSON and fails on an empty body.
#[derive(Debug, Clone, Copy)]
pub struct EmptyObject;

impl axum::response::IntoResponse for EmptyObject {
    fn into_response(self) -> axum::response::Response {
        axum::Json(serde_json::Map::new()).into_response()
    }
}

/// `application/x-www-form-urlencoded` for one value, as Java's
/// `URLEncoder.encode`: letters, digits and `.-*_` stay, a space is `+`.
pub fn form_encode(value: &str) -> String {
    serde_urlencoded::to_string([("", value)])
        .unwrap_or_default()
        .trim_start_matches('=')
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn form_encodes_as_java() {
        assert_eq!(form_encode("a b+c/d:e"), "a+b%2Bc%2Fd%3Ae");
        assert_eq!(form_encode("ns.x-y*z_1"), "ns.x-y*z_1");
    }
}
