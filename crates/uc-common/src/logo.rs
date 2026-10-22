//! The logo of the web UI as pixels, for terminals: Ferris before the ring.
//! Drawn from `docs/assets/logo-light.svg`, which is upstream's Unity
//! Catalog logo (Apache-2.0) with Ferris (CC0). Changed: made into 36 × 24
//! pixels, with Ferris's lines and the ring's shapes redrawn so that they
//! show at that size.
//!
//! A cell holds two pixels, one above the other: the half block `▀` in the
//! color of the top pixel, on the color of the bottom one.

/// The columns of the logo.
pub const WIDTH: u16 = 36;
/// The lines of the logo: two pixels each.
pub const HEIGHT: u16 = 12;

/// The pixels, a letter each: `.` for none, else a color of [`rgb`].
pub const PIXELS: [&str; 24] = [
    ".......................yyyy.........",
    "....................pp..yy..pp......",
    "..................ppppp....ppppp....",
    "..................ppppp....ppppp....",
    "...................ppppp..ppppp.....",
    "...............yy..ppp......ppp..yy.",
    "..............yyyy..............yyyy",
    "....................................",
    "........................nn..........",
    "..............ppppp....nnnn....ppppp",
    "................ppp...nnnnnn...ppppp",
    "........o.o..o.o......nnnnnn...ppppp",
    ".....o.oooooooooo.o...nnnnnn...ppppp",
    "..o..oooooooooooooo....nnnn....ppppp",
    "o.o.oooooooooooooooo....nn..........",
    "ooo.oooooooooooooooo................",
    ".oooooooooooooooooooo...........yyyy",
    "...ooooookwookwoooooo.......ppp..yy.",
    "...ooooookkookkoooooo.....ppppp.....",
    "..oooooookkookkooooooo.....ppppp....",
    "..oooooooookkooooooooo.....ppppp....",
    "...orrrrrrrroooooorr.o..yy..pp......",
    "....o.r.......ooo.r.o..yyyy.........",
    ".....o......ooo....o................",
];

/// The SVG's color of a letter of [`PIXELS`]; none for `.`.
pub fn rgb(letter: u8) -> Option<(u8, u8, u8)> {
    Some(match letter {
        // The ring: the hexagon, the triangles and the squares.
        b'n' => (0x16, 0x33, 0x57),
        b'y' => (0xff, 0xbc, 0x00),
        b'p' => (0xf4, 0x3b, 0xa6),
        // Ferris: the shell, the underside, the eyes and the light in them.
        b'o' => (0xf7, 0x4c, 0x00),
        b'r' => (0xa5, 0x2b, 0x00),
        b'k' => (0x00, 0x00, 0x00),
        b'w' => (0xff, 0xff, 0xff),
        _ => return None,
    })
}

/// The logo in 24-bit ANSI colors, a line of text per line of cells, with
/// the lines of `beside` on its right, in the middle of its height.
pub fn ansi(beside: &[&str]) -> String {
    let first = (usize::from(HEIGHT) - beside.len().min(usize::from(HEIGHT))) / 2;
    let mut out = String::new();
    for (row, pair) in PIXELS.chunks(2).enumerate() {
        for (upper, lower) in pair[0].bytes().zip(pair[1].bytes()) {
            match (rgb(upper), rgb(lower)) {
                (Some((r, g, b)), Some((br, bg, bb))) => out.push_str(&format!(
                    "\x1b[38;2;{r};{g};{b};48;2;{br};{bg};{bb}m▀\x1b[0m"
                )),
                (Some((r, g, b)), None) => out.push_str(&format!("\x1b[38;2;{r};{g};{b}m▀\x1b[0m")),
                (None, Some((r, g, b))) => out.push_str(&format!("\x1b[38;2;{r};{g};{b}m▄\x1b[0m")),
                (None, None) => out.push(' '),
            }
        }
        match row.checked_sub(first).and_then(|i| beside.get(i)) {
            Some(text) => {
                out.push_str("      ");
                out.push_str(text);
            }
            None => out.truncate(out.trim_end_matches(' ').len()),
        }
        out.push('\n');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_pixel_has_a_color() {
        assert_eq!(PIXELS.len(), usize::from(2 * HEIGHT));
        for line in PIXELS {
            assert_eq!(line.len(), usize::from(WIDTH), "{line}");
            assert!(
                line.bytes().all(|p| p == b'.' || rgb(p).is_some()),
                "{line}"
            );
        }
    }

    #[test]
    fn ansi_has_a_line_per_cell_row() {
        let text = ansi(&["name", "url"]);
        assert_eq!(text.lines().count(), usize::from(HEIGHT));
        let lines: Vec<&str> = text.lines().collect();
        assert!(lines[5].ends_with("      name"), "{}", lines[5]);
        assert!(lines[6].ends_with("      url"), "{}", lines[6]);
        // Ferris's shell over the light in an eye.
        assert!(text.contains("\x1b[38;2;247;76;0;48;2;255;255;255m▀"));
    }
}
