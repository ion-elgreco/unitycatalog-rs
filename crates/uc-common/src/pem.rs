//! PEM files of several keys, such as `auth.signing_key_file` and
//! `secrets.key_file`.

/// Each block of `pem`, from its `-----BEGIN` line to its `-----END` line,
/// in order. Text between blocks is skipped.
pub fn blocks(pem: &str) -> impl Iterator<Item = &str> {
    const END: &str = "-----END ";
    pem.match_indices("-----BEGIN ").filter_map(|(start, _)| {
        let block = &pem[start..];
        let end = block.find(END)? + END.len();
        let close = block[end..].find("-----")? + "-----".len();
        Some(&block[..end + close])
    })
}

#[cfg(test)]
mod tests {
    #[test]
    fn splits_a_file_into_its_blocks() {
        let pem = "-----BEGIN A-----\nAAAA\n-----END A-----\nnote\n-----BEGIN B-----\nBB\n-----END B-----\n";
        let blocks: Vec<&str> = super::blocks(pem).collect();
        assert_eq!(
            blocks,
            [
                "-----BEGIN A-----\nAAAA\n-----END A-----",
                "-----BEGIN B-----\nBB\n-----END B-----",
            ]
        );
        assert_eq!(super::blocks("not a key").count(), 0);
    }
}
