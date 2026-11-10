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

/// The server's URL for links in answers, without a trailing `/`:
/// `auth.public_url` when set, else the URL that the client called
/// (`X-Forwarded-Proto` and `X-Forwarded-Host` behind a proxy, else
/// `Host`). A client can send these headers itself, so only a scheme of
/// `http` or `https` and a host of a name or address with a port count:
/// no other text gets into the links.
pub fn public_url(public_url: Option<&str>, headers: &axum::http::HeaderMap) -> String {
    if let Some(url) = public_url {
        return url.to_string();
    }
    let first = |name: &str| {
        headers
            .get(name)
            .and_then(|h| h.to_str().ok())
            .and_then(|h| h.split(',').next())
            .map(str::trim)
            .filter(|h| !h.is_empty())
    };
    let is_host = |host: &&str| {
        host.chars()
            .all(|c| c.is_ascii_alphanumeric() || ".-:[]".contains(c))
    };
    let host = first("x-forwarded-host")
        .filter(is_host)
        .or_else(|| first(axum::http::header::HOST.as_str()).filter(is_host))
        .unwrap_or("localhost");
    let scheme = first("x-forwarded-proto")
        .filter(|scheme| ["http", "https"].contains(scheme))
        .unwrap_or("http");
    format!("{scheme}://{host}")
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
    fn the_public_url_is_the_setting_else_the_called_url() {
        use axum::http::{HeaderMap, HeaderValue};
        let mut headers = HeaderMap::new();
        assert_eq!(public_url(None, &headers), "http://localhost");
        headers.insert("host", HeaderValue::from_static("uc:8080"));
        assert_eq!(public_url(None, &headers), "http://uc:8080");
        headers.insert(
            "x-forwarded-host",
            HeaderValue::from_static("uc.example.com, proxy"),
        );
        headers.insert("x-forwarded-proto", HeaderValue::from_static("https"));
        assert_eq!(public_url(None, &headers), "https://uc.example.com");
        for (name, value) in [
            ("x-forwarded-host", "evil.example/path?x=\"y"),
            ("x-forwarded-proto", "javascript"),
        ] {
            headers.insert(name, HeaderValue::from_static(value));
        }
        assert_eq!(public_url(None, &headers), "http://uc:8080");
        assert_eq!(
            public_url(Some("https://data.example.com/uc"), &headers),
            "https://data.example.com/uc"
        );
    }

    #[test]
    fn form_encodes_as_java() {
        assert_eq!(form_encode("a b+c/d:e"), "a+b%2Bc%2Fd%3Ae");
        assert_eq!(form_encode("ns.x-y*z_1"), "ns.x-y*z_1");
    }
}
