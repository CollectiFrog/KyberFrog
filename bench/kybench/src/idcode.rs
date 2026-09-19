//! Frame ID code, robust to compression (plan-bench-latency.md § 4.2).
//!
//! A luma-only band of 32×32 px cells aligned on the 16×16 macroblock grid:
//! 2 rows × 18 cells = 576×64 px. Each row starts with two reference cells
//! (black 16 / white 235, swapped on the second row) that give an adaptive
//! threshold; the 32 remaining cells carry a 24-bit counter + CRC-8. Only the
//! central 16×16 of each cell is sampled, so deblocking at cell edges is ignored.
//!
//! The band is drawn twice (top-left and bottom-right); a frame is accepted only
//! if both copies decode to the same ID with a valid CRC.

pub const CELL: usize = 32;
pub const COLS: usize = 18;
pub const ROWS: usize = 2;
pub const BAND_W: usize = CELL * COLS;
pub const BAND_H: usize = CELL * ROWS;
pub const ID_MASK: u32 = 0x00FF_FFFF;

const BLACK: u8 = 16;
const WHITE: u8 = 235;
const REFS: usize = 2;
const BITS_PER_ROW: usize = COLS - REFS;
/// Minimum white − black luma gap for a band to be read at all.
const MIN_CONTRAST: u32 = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecodeError {
    NoContrast,
    BadCrc,
}

/// Top-left corners of the two band copies for a `width`×`height` frame.
pub fn band_origins(width: usize, height: usize) -> [(usize, usize); 2] {
    let x = (width - BAND_W - CELL) / CELL * CELL;
    let y = (height - BAND_H - CELL) / CELL * CELL;
    [(CELL, CELL), (x, y)]
}

/// CRC-8, polynomial 0x07, init 0.
pub fn crc8(bytes: &[u8]) -> u8 {
    let mut crc = 0u8;
    for &b in bytes {
        crc ^= b;
        for _ in 0..8 {
            crc = if crc & 0x80 != 0 { (crc << 1) ^ 0x07 } else { crc << 1 };
        }
    }
    crc
}

fn word(id: u32) -> u32 {
    let id = id & ID_MASK;
    id | (crc8(&id.to_le_bytes()[..3]) as u32) << 24
}

fn cell_luma(row: usize, col: usize, word: u32) -> u8 {
    if col < REFS {
        // Row 0: black, white. Row 1: white, black.
        if (row + col) % 2 == 0 { BLACK } else { WHITE }
    } else if word >> (row * BITS_PER_ROW + col - REFS) & 1 == 1 {
        WHITE
    } else {
        BLACK
    }
}

/// Draw the band for `id` into a BGRA frame at (`x0`, `y0`).
pub fn encode(frame: &mut [u8], stride: usize, x0: usize, y0: usize, id: u32) {
    let w = word(id);
    for row in 0..ROWS {
        for col in 0..COLS {
            let v = cell_luma(row, col, w);
            for y in 0..CELL {
                let start = (y0 + row * CELL + y) * stride + (x0 + col * CELL) * 4;
                for px in frame[start..start + CELL * 4].chunks_exact_mut(4) {
                    px.copy_from_slice(&[v, v, v, 255]);
                }
            }
        }
    }
}

/// Mean BT.709 luma of the central 16×16 of a cell, from BGRA pixels.
fn sample(band: &[u8], stride: usize, row: usize, col: usize) -> u32 {
    let (q, n) = (CELL / 4, CELL / 2);
    let mut sum = 0u32;
    for y in 0..n {
        let start = (row * CELL + q + y) * stride + (col * CELL + q) * 4;
        for px in band[start..start + n * 4].chunks_exact(4) {
            sum += (px[2] as u32 * 54 + px[1] as u32 * 183 + px[0] as u32 * 19) >> 8;
        }
    }
    sum / (n * n) as u32
}

/// Decode one band whose top-left pixel is at offset 0 of `band`.
pub fn decode(band: &[u8], stride: usize) -> Result<u32, DecodeError> {
    let black = (sample(band, stride, 0, 0) + sample(band, stride, 1, 1)) / 2;
    let white = (sample(band, stride, 0, 1) + sample(band, stride, 1, 0)) / 2;
    if white < black + MIN_CONTRAST {
        return Err(DecodeError::NoContrast);
    }
    let threshold = (black + white) / 2;
    let mut w = 0u32;
    for row in 0..ROWS {
        for col in REFS..COLS {
            if sample(band, stride, row, col) > threshold {
                w |= 1 << (row * BITS_PER_ROW + col - REFS);
            }
        }
    }
    let id = w & ID_MASK;
    if crc8(&id.to_le_bytes()[..3]) == (w >> 24) as u8 {
        Ok(id)
    } else {
        Err(DecodeError::BadCrc)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn band_with(id: u32) -> Vec<u8> {
        let mut band = vec![0u8; BAND_W * BAND_H * 4];
        encode(&mut band, BAND_W * 4, 0, 0, id);
        band
    }

    #[test]
    fn round_trips() {
        for id in [0, 1, 2, 0x5A5A5A, 18_000, ID_MASK] {
            assert_eq!(decode(&band_with(id), BAND_W * 4), Ok(id));
        }
    }

    #[test]
    fn survives_noise_and_limited_range() {
        let mut band = band_with(123_456);
        let mut seed = 7u32;
        for (i, b) in band.iter_mut().enumerate() {
            if i % 4 == 3 {
                continue;
            }
            seed ^= seed << 13;
            seed ^= seed >> 17;
            seed ^= seed << 5;
            // Compress to 32..=200 then add ±24 noise.
            let v = 32 + (*b as i32 * 168 / 255) + (seed % 49) as i32 - 24;
            *b = v.clamp(0, 255) as u8;
        }
        assert_eq!(decode(&band, BAND_W * 4), Ok(123_456));
    }

    #[test]
    fn rejects_flat_and_corrupted() {
        assert_eq!(decode(&vec![128u8; BAND_W * BAND_H * 4], BAND_W * 4), Err(DecodeError::NoContrast));
        let mut band = band_with(42);
        // Flip one payload cell (row 0, first bit) to the opposite level.
        let flipped = if sample(&band, BAND_W * 4, 0, REFS) > 128 { BLACK } else { WHITE };
        for y in 0..CELL {
            for x in 0..CELL {
                let o = (y * BAND_W + REFS * CELL + x) * 4;
                band[o..o + 3].fill(flipped);
            }
        }
        assert_eq!(decode(&band, BAND_W * 4), Err(DecodeError::BadCrc));
    }

    #[test]
    fn origins_are_macroblock_aligned() {
        for (w, h) in [(1920, 1080), (2560, 1440), (1280, 720)] {
            for (x, y) in band_origins(w, h) {
                assert_eq!((x % 16, y % 16), (0, 0));
                assert!(x + BAND_W <= w && y + BAND_H <= h);
            }
        }
    }
}
