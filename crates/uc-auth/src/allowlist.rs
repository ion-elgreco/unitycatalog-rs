use std::collections::HashSet;

/// Exact entries, and entries whose `*` matches one DNS label
/// (`https://*.dev.example.com`).
#[derive(Debug, Clone, Default)]
pub struct Allowlist {
    exact: HashSet<String>,
    wildcards: Vec<Vec<String>>,
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
                true => list
                    .wildcards
                    .push(entry.split('*').map(str::to_string).collect()),
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
        !value.trim().is_empty()
            && (self.exact.contains(value)
                || self.wildcards.iter().any(|parts| matches(parts, value)))
    }

    pub fn is_any_allowed(&self, values: &[String]) -> bool {
        values.iter().any(|v| self.is_allowed(v))
    }
}

/// Whether `value` is `parts` joined by runs of characters other than `.`
/// and `/` (`[^./]+` for each `*`).
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
        .take_while(|(_, c)| *c != '.' && *c != '/')
        .any(|(i, c)| matches(rest, &after[i + c.len_utf8()..]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allowlists() {
        let list = Allowlist::new(&["https://accounts.example.com", " https://*.dev.example.com"]);
        assert!(list.is_allowed("https://accounts.example.com"));
        assert!(list.is_allowed("https://acme.dev.example.com"));
        assert!(!list.is_allowed("https://a.b.dev.example.com"));
        assert!(!list.is_allowed("https://.dev.example.com"));
        assert!(!list.is_allowed("https://evil.com/x.dev.example.com"));
        assert!(!list.is_allowed(""));
        assert!(Allowlist::new(&[" ", ""]).is_empty());
        assert!(Allowlist::new(&["api://*"]).is_allowed("api://app"));
    }
}
