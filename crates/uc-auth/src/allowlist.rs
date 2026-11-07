use std::collections::HashSet;

/// Exact entries, and entries whose `*` matches one DNS label
/// (`https://*.dev.example.com`).
#[derive(Debug, Clone, Default)]
pub struct Allowlist {
    exact: HashSet<String>,
    /// Each pattern, and its parts around the `*`s.
    wildcards: Vec<(String, Vec<String>)>,
}

impl Allowlist {
    /// The non-blank `entries`, trimmed.
    pub fn new(entries: &[impl AsRef<str>]) -> Self {
        let mut list = Self::default();
        for entry in entries
            .iter()
            .map(|e| e.as_ref().trim())
            .filter(|e| !e.is_empty())
        {
            match entry.contains('*') {
                true => list.wildcards.push((
                    entry.to_string(),
                    entry.split('*').map(str::to_string).collect(),
                )),
                false => {
                    list.exact.insert(entry.to_string());
                }
            }
        }
        list
    }

    pub fn is_empty(&self) -> bool {
        self.exact.is_empty() && self.wildcards.is_empty()
    }

    pub fn is_allowed(&self, value: &str) -> bool {
        self.matching(value).is_some()
    }

    pub fn is_any_allowed(&self, values: &[String]) -> bool {
        values.iter().any(|v| self.is_allowed(v))
    }

    /// How `value` is allowed: `Some(None)` as an exact entry, `Some(Some(pattern))` by the
    /// first pattern with `*` that matches it, `None` when it is not.
    pub fn matching(&self, value: &str) -> Option<Option<&str>> {
        if value.trim().is_empty() {
            return None;
        }
        if self.exact.contains(value) {
            return Some(None);
        }
        (self.wildcards.iter())
            .find(|(_, parts)| matches(parts, value))
            .map(|(pattern, _)| Some(pattern.as_str()))
    }
}

/// A character of one DNS label: a letter, a digit or `-`.
fn label_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '-'
}

/// Whether `value` is `parts` joined by runs of label characters
/// (`[A-Za-z0-9-]+` for each `*`). No `.`, `/`, `@`, `?`, `#` or `\`, so a
/// `*` never moves the host of a URL.
fn matches(parts: &[String], value: &str) -> bool {
    let Some((first, rest)) = parts.split_first() else {
        return value.is_empty();
    };
    let Some(after) = value.strip_prefix(first.as_str()) else {
        return false;
    };
    if rest.is_empty() {
        return after.is_empty();
    }
    // One or more label characters, then the remaining parts.
    after
        .char_indices()
        .take_while(|(_, c)| label_char(*c))
        .any(|(i, c)| matches(rest, &after[i + c.len_utf8()..]))
}

/// Why the URL pattern `pattern` cannot be an issuer or an endpoint, if it
/// cannot: it must be an `http` or `https` URL, and a `*` in its host must
/// be followed by two fixed labels at least (`https://*.example.com`), so
/// that it stays under one domain.
pub fn url_pattern_error(pattern: &str) -> Option<String> {
    let Some(rest) = (pattern.strip_prefix("https://")).or_else(|| pattern.strip_prefix("http://"))
    else {
        return Some("Expected an http or https URL.".into());
    };
    let authority = rest.split('/').next().unwrap_or_default();
    let host = authority
        .rsplit_once(':')
        .map_or(authority, |(host, _)| host);
    if host.is_empty() || authority.contains(['@', '?', '#', '\\']) {
        return Some("Expected the host of the URL.".into());
    }
    let labels: Vec<&str> = host.split('.').collect();
    let fixed = labels.iter().rev().take_while(|l| !l.contains('*')).count();
    if host.contains('*') && fixed < 2 {
        return Some(
            "A `*` in the host must be followed by two fixed labels at least, such as \
             https://*.example.com."
                .into(),
        );
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allowlists() {
        let list = Allowlist::new(&["https://accounts.example.com", " https://*.dev.example.com"]);
        assert!(list.is_allowed("https://accounts.example.com"));
        assert!(list.is_allowed("https://acme.dev.example.com"));
        assert_eq!(
            list.matching("https://acme.dev.example.com"),
            Some(Some("https://*.dev.example.com"))
        );
        assert_eq!(list.matching("https://accounts.example.com"), Some(None));
        assert!(!list.is_allowed("https://a.b.dev.example.com"));
        assert!(!list.is_allowed("https://.dev.example.com"));
        assert!(!list.is_allowed("https://evil.com/x.dev.example.com"));
        assert!(!list.is_allowed(""));
        assert!(Allowlist::new(&[" ", ""]).is_empty());
        assert!(Allowlist::new(&["api://*"]).is_allowed("api://app"));
    }

    #[test]
    fn a_star_never_moves_the_host() {
        let list = Allowlist::new(&["https://*.dev.example.com"]);
        for value in [
            "https://16909060?.dev.example.com",
            "https://evil.com#.dev.example.com",
            "https://evil.com@x.dev.example.com",
            "https://evil.com\\.dev.example.com",
            "https://evil.com:443.dev.example.com",
        ] {
            assert!(!list.is_allowed(value), "{value}");
        }
        let tenants = Allowlist::new(&["https://login.example.com/*/v2.0"]);
        assert!(tenants.is_allowed("https://login.example.com/72f988bf-86f1/v2.0"));
        assert!(!tenants.is_allowed("https://login.example.com/a?b/v2.0"));
    }

    #[test]
    fn url_patterns() {
        assert_eq!(url_pattern_error("https://*.dev.example.com"), None);
        assert_eq!(url_pattern_error("https://login.example.com/*/v2.0"), None);
        assert_eq!(url_pattern_error("http://127.0.0.1:9000"), None);
        assert_eq!(url_pattern_error("https://sts.*.amazonaws.com"), None);
        for pattern in [
            "https://*",
            "http://*",
            "https://*.com",
            "https://example.*",
            "https://*.*.com",
            "ftp://example.com",
            "example.com",
            "https://",
            "https://a@example.com",
        ] {
            assert!(url_pattern_error(pattern).is_some(), "{pattern}");
        }
    }
}
