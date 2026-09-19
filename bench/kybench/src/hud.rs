//! Human-readable overlay for the operator's visual check: the frame ID as a
//! timecode `MM:SS:FF` (60 fps) and as a decimal counter, drawn with
//! seven-segment glyphs in the middle of the frame — away from the ID bands.
//! Luma-only, fixed size and position: the same in every run.

const SEGMENTS: [u8; 10] = [0x3F, 0x06, 0x5B, 0x4F, 0x66, 0x6D, 0x7D, 0x07, 0x7F, 0x6F];
const INK: u8 = 235;
const PAPER: u8 = 16;

struct Canvas<'a> {
    frame: &'a mut [u8],
    stride: usize,
}

impl Canvas<'_> {
    fn rect(&mut self, x: usize, y: usize, w: usize, h: usize, v: u8) {
        for row in y..y + h {
            let start = row * self.stride + x * 4;
            for px in self.frame[start..start + w * 4].chunks_exact_mut(4) {
                px.copy_from_slice(&[v, v, v, 255]);
            }
        }
    }

    /// One glyph of a `unit`-based seven-segment font: 4 units wide, 7 tall,
    /// strokes 1 unit thick. `ch` is a digit or ':'.
    fn glyph(&mut self, x: usize, y: usize, unit: usize, ch: u8) {
        let (w, h, t) = (4 * unit, 7 * unit, unit);
        if ch == b':' {
            self.rect(x + w / 2 - t / 2, y + 2 * unit - t / 2, t, t, INK);
            self.rect(x + w / 2 - t / 2, y + 5 * unit - t / 2, t, t, INK);
            return;
        }
        let seg = SEGMENTS[(ch - b'0') as usize];
        let on = |bit: u8| seg & bit != 0;
        if on(0x01) { self.rect(x, y, w, t, INK); }                       // a: top
        if on(0x02) { self.rect(x + w - t, y, t, h / 2, INK); }           // b: top right
        if on(0x04) { self.rect(x + w - t, y + h / 2, t, h / 2, INK); }   // c: bottom right
        if on(0x08) { self.rect(x, y + h - t, w, t, INK); }               // d: bottom
        if on(0x10) { self.rect(x, y + h / 2, t, h / 2, INK); }           // e: bottom left
        if on(0x20) { self.rect(x, y, t, h / 2, INK); }                   // f: top left
        if on(0x40) { self.rect(x, y + h / 2 - t / 2, w, t, INK); }       // g: middle
    }

    fn text(&mut self, x: usize, y: usize, unit: usize, s: &[u8]) {
        for (i, &ch) in s.iter().enumerate() {
            self.glyph(x + i * 5 * unit, y, unit, ch);
        }
    }
}

/// Width in pixels of `n` glyphs at `unit` (4 units per glyph, 1 unit gap).
fn text_w(n: usize, unit: usize) -> usize {
    n * 5 * unit - unit
}

/// Draw the overlay for frame `n` at `fps` into a `w`×`h` BGRA frame.
/// Does nothing when the frame is too small for it.
pub fn draw(frame: &mut [u8], stride: usize, w: usize, h: usize, n: u64, fps: u64) {
    let (big, small, pad) = (16usize, 8usize, 24usize);
    let tc = format!("{:02}:{:02}:{:02}", n / fps / 60 % 100, n / fps % 60, n % fps);
    let id = format!("{:07}", n & crate::idcode::ID_MASK as u64);
    let box_w = text_w(tc.len(), big) + 2 * pad;
    let box_h = 7 * big + 2 * small + 7 * small + 2 * pad;
    // Stay clear of the two ID bands (top-left and bottom-right corners).
    let margin = 2 * crate::idcode::CELL + crate::idcode::BAND_H;
    if box_w > w || box_h + 2 * margin > h {
        return;
    }
    let (x0, y0) = ((w - box_w) / 2, (h - box_h) / 2);
    let mut c = Canvas { frame, stride };
    c.rect(x0, y0, box_w, box_h, PAPER);
    c.text(x0 + pad, y0 + pad, big, tc.as_bytes());
    c.text(x0 + (box_w - text_w(id.len(), small)) / 2, y0 + pad + 7 * big + 2 * small, small, id.as_bytes());
}
