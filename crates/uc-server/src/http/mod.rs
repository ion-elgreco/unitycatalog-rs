//! Request and response helpers for every route: the error body, the
//! extractors and the header spelling.

pub mod error;
pub mod extract;
pub mod header_case;

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
