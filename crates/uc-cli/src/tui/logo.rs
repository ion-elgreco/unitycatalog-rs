//! The logo of the web UI as pixels, for the empty detail pane: Ferris
//! before the ring. Drawn from `docs/assets/logo-light.svg`, which is
//! upstream's Unity Catalog logo (Apache-2.0) with Ferris (CC0). Changed:
//! made into 36 × 24 pixels, with Ferris's lines and the ring's shapes
//! redrawn so that they show at that size.
//!
//! A cell holds two pixels, one above the other: the half block `▀` in the
//! color of the top pixel, on the color of the bottom one. This works in
//! every terminal, also with 256 colors.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::symbols::half_block;

/// The columns of the logo.
pub const WIDTH: u16 = 36;
/// The lines of the logo: two pixels each.
pub const HEIGHT: u16 = 12;

/// The pixels, a letter each: `.` for none, else a color of [`color`].
const PIXELS: [&str; 24] = [
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
fn color(letter: u8) -> Option<Color> {
    let (r, g, b) = match letter {
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
    };
    Some(Color::Rgb(r, g, b))
}

/// Draws the logo in the middle of `area`, over what is there: a cell
/// without pixels keeps its symbol and colors. Draws nothing when the logo
/// does not fit.
pub fn render(area: Rect, buf: &mut Buffer) {
    if area.width < WIDTH || area.height < HEIGHT {
        return;
    }
    let left = area.x + (area.width - WIDTH) / 2;
    let top = area.y + (area.height - HEIGHT) / 2;
    for (y, pair) in (top..).zip(PIXELS.chunks(2)) {
        let pixels = pair[0].bytes().zip(pair[1].bytes());
        for (x, (upper, lower)) in (left..).zip(pixels) {
            let cell = &mut buf[(x, y)];
            match (color(upper), color(lower)) {
                (Some(upper), Some(lower)) => {
                    cell.set_char(half_block::UPPER).set_fg(upper).set_bg(lower);
                }
                (Some(upper), None) => {
                    cell.set_char(half_block::UPPER).set_fg(upper);
                }
                (None, Some(lower)) => {
                    cell.set_char(half_block::LOWER).set_fg(lower);
                }
                (None, None) => {}
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The smallest box around the cells that are not blank.
    fn drawn(buf: &Buffer) -> Option<Rect> {
        let area = buf.area;
        let cells: Vec<(u16, u16)> = (area.top()..area.bottom())
            .flat_map(|y| (area.left()..area.right()).map(move |x| (x, y)))
            .filter(|&xy| buf[xy].symbol() != " ")
            .collect();
        let left = cells.iter().map(|&(x, _)| x).min()?;
        let right = cells.iter().map(|&(x, _)| x).max()?;
        let top = cells.iter().map(|&(_, y)| y).min()?;
        let bottom = cells.iter().map(|&(_, y)| y).max()?;
        Some(Rect::new(left, top, right - left + 1, bottom - top + 1))
    }

    #[test]
    fn each_pixel_has_a_color() {
        assert_eq!(PIXELS.len(), usize::from(2 * HEIGHT));
        for line in PIXELS {
            assert_eq!(line.len(), usize::from(WIDTH), "{line}");
            assert!(
                line.bytes().all(|p| p == b'.' || color(p).is_some()),
                "{line}"
            );
        }
    }

    #[test]
    fn the_logo_is_in_the_middle() {
        let area = Rect::new(0, 0, WIDTH + 4, HEIGHT + 2);
        let mut buf = Buffer::empty(area);
        render(area, &mut buf);
        assert_eq!(drawn(&buf), Some(Rect::new(2, 1, WIDTH, HEIGHT)));

        // A cell shows its top pixel in front and its bottom pixel behind:
        // Ferris's shell over the light in an eye, and a square's corner
        // under nothing.
        let orange = Color::Rgb(0xf7, 0x4c, 0x00);
        let white = Color::Rgb(0xff, 0xff, 0xff);
        let eye = &buf[(2 + 10, 1 + 8)];
        assert_eq!((eye.symbol(), eye.fg, eye.bg), ("▀", orange, white));
        let pink = Color::Rgb(0xf4, 0x3b, 0xa6);
        let corner = &buf[(2 + 20, 1)];
        assert_eq!(
            (corner.symbol(), corner.fg, corner.bg),
            ("▄", pink, Color::Reset)
        );
    }

    #[test]
    fn a_small_area_gets_no_logo() {
        for area in [
            Rect::new(0, 0, WIDTH - 1, HEIGHT),
            Rect::new(0, 0, WIDTH, HEIGHT - 1),
        ] {
            let mut buf = Buffer::empty(area);
            render(area, &mut buf);
            assert_eq!(drawn(&buf), None);
        }
    }
}
