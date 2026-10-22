//! The logo of the web UI (`uc_common::logo`) in the empty detail pane.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Color;
use uc_common::logo::{Cell, HEIGHT, WIDTH};

/// Draws the logo in the middle of `area`, over what is there: a cell
/// without pixels keeps its symbol and colors. Draws nothing when the logo
/// does not fit.
pub fn render(area: Rect, buf: &mut Buffer) {
    if area.width < WIDTH || area.height < HEIGHT {
        return;
    }
    let left = area.x + (area.width - WIDTH) / 2;
    let top = area.y + (area.height - HEIGHT) / 2;
    let color = |(r, g, b)| Color::Rgb(r, g, b);
    for (y, cells) in (top..).zip(uc_common::logo::rows()) {
        for (x, cell) in (left..).zip(cells) {
            let Some(Cell { symbol, fg, bg }) = cell else {
                continue;
            };
            let target = buf[(x, y)].set_char(symbol).set_fg(color(fg));
            if let Some(bg) = bg {
                target.set_bg(color(bg));
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
