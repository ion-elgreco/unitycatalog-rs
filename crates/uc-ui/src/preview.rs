//! What the file viewer shows of a file: its kind from its name, and the
//! text, JSON or table of its first bytes. Parquet, Avro and other binary
//! files have no preview.

/// How the viewer shows a file, from its name.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// An image, with its media type.
    Image(&'static str),
    /// JSON: a document, or one per line, as the files of a Delta log.
    Json,
    /// A table, with its separator.
    Table(u8),
    /// Text, when the bytes are text.
    Text,
}

impl Kind {
    pub fn of(name: &str) -> Self {
        let extension = name
            .rsplit_once('.')
            .map(|(_, e)| e.to_ascii_lowercase())
            .unwrap_or_default();
        match extension.as_str() {
            "png" => Kind::Image("image/png"),
            "jpg" | "jpeg" => Kind::Image("image/jpeg"),
            "gif" => Kind::Image("image/gif"),
            "webp" => Kind::Image("image/webp"),
            "svg" => Kind::Image("image/svg+xml"),
            "bmp" => Kind::Image("image/bmp"),
            "avif" => Kind::Image("image/avif"),
            "json" | "ndjson" | "jsonl" | "crc" => Kind::Json,
            "csv" => Kind::Table(b','),
            "tsv" => Kind::Table(b'\t'),
            _ => Kind::Text,
        }
    }
}

/// What the viewer shows of the first bytes of a file.
#[derive(Clone, Debug, PartialEq)]
pub enum Preview {
    Text(String),
    /// The rows of a table, the first one its header.
    Table(Vec<Vec<String>>),
    /// Not text: no preview.
    Binary,
}

/// The rows of a table that the viewer adds at a time.
pub const MAX_ROWS: usize = 200;

/// The preview of `bytes`, the first bytes of a file of `kind`; `whole`
/// when they are the whole file. A table has at most `max_rows` rows.
pub fn preview(kind: Kind, bytes: &[u8], whole: bool, max_rows: usize) -> Preview {
    let Some(text) = text(bytes, whole) else {
        return Preview::Binary;
    };
    match kind {
        Kind::Json => Preview::Text(pretty_json(&text, whole).unwrap_or(text)),
        Kind::Table(separator) => Preview::Table(rows(&text, separator, whole, max_rows)),
        Kind::Text | Kind::Image(_) => Preview::Text(text),
    }
}

/// `bytes` as text: none for bytes that are not UTF-8, or that hold a NUL,
/// as binary files do. A character cut off at the end of a part of a file
/// is left out.
fn text(bytes: &[u8], whole: bool) -> Option<String> {
    if bytes.contains(&0) {
        return None;
    }
    match std::str::from_utf8(bytes) {
        Ok(text) => Some(text.to_string()),
        // At most 3 bytes of a character are cut off at the end.
        Err(e) if !whole && e.error_len().is_none() && bytes.len() - e.valid_up_to() < 4 => {
            Some(String::from_utf8_lossy(&bytes[..e.valid_up_to()]).into_owned())
        }
        Err(_) => None,
    }
}

/// `text` pretty-printed as one JSON document, else as one per line; none
/// when it is not JSON. A part of a file can only be JSON per line: its
/// last line, which may be cut off, stays as it is.
fn pretty_json(text: &str, whole: bool) -> Option<String> {
    if whole && let Ok(value) = serde_json::from_str::<serde_json::Value>(text) {
        return serde_json::to_string_pretty(&value).ok();
    }
    let lines: Vec<&str> = text.lines().filter(|l| !l.trim().is_empty()).collect();
    let (last, complete) = match whole {
        true => (None, lines.as_slice()),
        false => match lines.split_last() {
            Some((last, complete)) => (Some(*last), complete),
            None => (None, lines.as_slice()),
        },
    };
    let mut documents = complete
        .iter()
        .map(|line| {
            let value = serde_json::from_str::<serde_json::Value>(line).ok()?;
            serde_json::to_string_pretty(&value).ok()
        })
        .collect::<Option<Vec<_>>>()?;
    if documents.is_empty() {
        return None;
    }
    documents.extend(last.map(str::to_string));
    Some(documents.join("\n"))
}

/// The first `max_rows` rows of a table, without a row that the end of a
/// part of a file cuts off.
fn rows(text: &str, separator: u8, whole: bool, max_rows: usize) -> Vec<Vec<String>> {
    let mut reader = csv::ReaderBuilder::new()
        .delimiter(separator)
        .has_headers(false)
        .flexible(true)
        .from_reader(text.as_bytes());
    let mut rows: Vec<Vec<String>> = reader
        .records()
        .map_while(Result::ok)
        .take(max_rows + 1)
        .map(|record| record.iter().map(str::to_string).collect())
        .collect();
    if !whole && rows.len() <= max_rows {
        rows.pop();
    }
    rows.truncate(max_rows);
    rows
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kinds() {
        assert_eq!(Kind::of("a.PNG"), Kind::Image("image/png"));
        assert_eq!(Kind::of("00000000000000000000.json"), Kind::Json);
        assert_eq!(Kind::of("data.tsv"), Kind::Table(b'\t'));
        assert_eq!(Kind::of("part-0.parquet"), Kind::Text);
        assert_eq!(Kind::of("README"), Kind::Text);
    }

    #[test]
    fn binary_files_have_no_preview() {
        assert_eq!(
            preview(Kind::Text, b"PAR1\0\0\x15", true, MAX_ROWS),
            Preview::Binary
        );
        assert_eq!(
            preview(Kind::Text, &[0xff, 0xfe, 0x41], true, MAX_ROWS),
            Preview::Binary
        );
        // A character cut off at the end of a part is left out.
        let cut = "naïve".as_bytes();
        let part = &cut[..cut.len() - 3];
        assert_eq!(
            preview(Kind::Text, part, false, MAX_ROWS),
            Preview::Text("na".into())
        );
    }

    #[test]
    fn json_documents_and_lines() {
        let one = preview(Kind::Json, br#"{"a":1}"#, true, MAX_ROWS);
        assert_eq!(one, Preview::Text("{\n  \"a\": 1\n}".into()));
        let log = "{\"add\":{\"path\":\"p\"}}\n{\"remove\":{}}\n";
        let Preview::Text(lines) = preview(Kind::Json, log.as_bytes(), true, MAX_ROWS) else {
            panic!("not text");
        };
        assert!(lines.contains("\"add\": {") && lines.contains("\"remove\": {}"));
        // A part keeps its cut-off last line as it is.
        let part = "{\"a\":1}\n{\"b\":";
        let Preview::Text(text) = preview(Kind::Json, part.as_bytes(), false, MAX_ROWS) else {
            panic!("not text");
        };
        assert!(text.ends_with("\n{\"b\":"), "{text}");
        // Not JSON: the text as it is.
        assert_eq!(
            preview(Kind::Json, b"not json", true, MAX_ROWS),
            Preview::Text("not json".into())
        );
    }

    #[test]
    fn tables() {
        let csv = "id,name\n1,\"a, b\"\n2,c\n";
        assert_eq!(
            preview(Kind::Table(b','), csv.as_bytes(), true, MAX_ROWS),
            Preview::Table(vec![
                vec!["id".into(), "name".into()],
                vec!["1".into(), "a, b".into()],
                vec!["2".into(), "c".into()],
            ])
        );
        // A part leaves out its last row, which may be cut off.
        let part = "id\tname\n1\ta\n2\tb";
        let Preview::Table(rows) = preview(Kind::Table(b'\t'), part.as_bytes(), false, MAX_ROWS)
        else {
            panic!("not a table");
        };
        assert_eq!(rows.len(), 2);
        let many = "x\n".repeat(MAX_ROWS + 50);
        let Preview::Table(rows) = preview(Kind::Table(b','), many.as_bytes(), true, MAX_ROWS)
        else {
            panic!("not a table");
        };
        assert_eq!(rows.len(), MAX_ROWS);
        // More rows when asked for more.
        let Preview::Table(rows) = preview(Kind::Table(b','), many.as_bytes(), true, 2 * MAX_ROWS)
        else {
            panic!("not a table");
        };
        assert_eq!(rows.len(), MAX_ROWS + 50);
    }
}
