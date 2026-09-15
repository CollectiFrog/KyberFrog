//! `kybench gen`: paced Spout generator. Each frame carries its ID (idcode) over
//! a moving pseudo-random background with a fixed seed, plus a human-readable
//! timecode (hud) for the operator's visual check; `t_pub` is taken once
//! the GPU has executed the copy, right before the frame counter is bumped.

use crate::args::Args;
use crate::clock::{now_us, Pacer};
use crate::{hud, idcode};
use crate::spout::{Device, Res, Sender};

/// Blocky pseudo-random background, scrolled horizontally: the encoder sees
/// real motion, identical from one run to the next for a given seed.
pub struct Background {
    tile: Vec<u8>,
    tile_w: usize,
}

const BLOCK: usize = 16;
const SCROLL_PX: usize = 8;
const WRAP_PX: usize = 1024;

impl Background {
    pub fn new(w: usize, h: usize, seed: u64) -> Self {
        let tile_w = w + WRAP_PX;
        let mut tile = vec![0u8; tile_w * h * 4];
        let mut s = seed | 1;
        let mut next = || {
            s ^= s << 13;
            s ^= s >> 7;
            s ^= s << 17;
            s
        };
        let blocks_x = tile_w.div_ceil(BLOCK);
        let colors: Vec<[u8; 4]> = (0..blocks_x * h.div_ceil(BLOCK))
            .map(|_| {
                let r = next();
                [r as u8, (r >> 8) as u8, (r >> 16) as u8, 255]
            })
            .collect();
        for y in 0..h {
            for x in 0..tile_w {
                let c = colors[(y / BLOCK) * blocks_x + x / BLOCK];
                tile[(y * tile_w + x) * 4..][..4].copy_from_slice(&c);
            }
        }
        Self { tile, tile_w }
    }

    pub fn compose(&self, n: u64, frame: &mut [u8], w: usize) {
        self.compose_scrolled(n, frame, w, SCROLL_PX, 0);
    }

    /// Scroll by `scroll_px` per frame from `offset_px`. With the default 8 px
    /// and 16 px blocks, every other frame has its blocks aligned on the 16×16
    /// macroblock grid (cheap for an intra codec) and the next one half a block
    /// off (costly) — visible in NDI as an even/odd latency pattern.
    pub fn compose_scrolled(&self, n: u64, frame: &mut [u8], w: usize, scroll_px: usize, offset_px: usize) {
        let off = (offset_px + n as usize * scroll_px) % WRAP_PX;
        for (y, row) in frame.chunks_exact_mut(w * 4).enumerate() {
            let start = (y * self.tile_w + off) * 4;
            row.copy_from_slice(&self.tile[start..start + w * 4]);
        }
    }
}

/// Draw frame `n` (background, both ID bands, HUD) into a BGRA buffer; returns its ID.
pub fn render(background: &Background, frame: &mut [u8], w: usize, h: usize, n: u64, fps: u64,
              origins: &[(usize, usize); 2]) -> u32 {
    let id = (n as u32) & idcode::ID_MASK;
    background.compose(n, frame, w);
    for &(x, y) in origins {
        idcode::encode(frame, w * 4, x, y, id);
    }
    hud::draw(frame, w * 4, w, h, n, fps);
    id
}

pub fn run(a: &Args) -> Res<()> {
    let name = a.str("name", "kybench-src");
    let (w, h) = (a.num("width", 1920usize)?, a.num("height", 1080usize)?);
    let fps = a.num("fps", 60u64)?;
    let duration_s = a.num("duration", 60.0f64)?;
    let seed = a.num("seed", 1u64)?;
    // Lead before the deadline: the GPU copy is done and waited for during it,
    // so `t_pub` lands on the deadline rather than a copy later.
    let lead_us = a.num("lead-us", 2_000i64)?;
    let csv = a.str("csv", "gen.csv");

    let dev = Device::new()?;
    let sender = Sender::new(&dev, &name, w as u32, h as u32)?;
    let private = dev.private_texture(w as u32, h as u32)?;
    let background = Background::new(w, h, seed);
    let mut frame = vec![0u8; w * h * 4];
    let origins = idcode::band_origins(w, h);
    let pacer = Pacer::new()?;

    let start = now_us() + 500_000;
    let end = start + (duration_s * 1e6) as i64;
    let mut rows = Vec::with_capacity((duration_s * fps as f64) as usize + 1);
    let mut busy = 0u64;
    eprintln!("gen: '{name}' {w}x{h}@{fps}, {duration_s} s, seed {seed}, lead {lead_us} µs");

    for n in 0u64.. {
        let deadline = start + (n * 1_000_000 / fps) as i64;
        if deadline >= end {
            break;
        }
        let id = render(&background, &mut frame, w, h, n, fps, &origins);
        dev.upload(&private, &frame, w * 4)?;

        // Prepare, wait, signal — with the duration of the copy into the
        // shared texture recorded: it is paid before `t_pub`, so it is the
        // part of a Spout publication the latency does not include.
        pacer.sleep_until(deadline - lead_us);
        let t_copy = now_us();
        let prepared = sender.prepare(&dev, &private)?;
        let copy_us = now_us() - t_copy;
        let published = if prepared {
            pacer.sleep_until(deadline);
            Some(sender.signal())
        } else {
            busy += 1;
            None
        };
        let t_pub = published.unwrap_or_else(now_us);
        rows.push(format!("{id},{deadline},{t_pub},{},{copy_us}", published.is_some() as u8));
    }

    let mut out = String::from("id,t_deadline_us,t_pub_us,published,copy_us\n");
    for r in &rows {
        out.push_str(r);
        out.push('\n');
    }
    std::fs::write(&csv, out)?;
    eprintln!("gen: {} frames, {busy} dropped on a busy mutex -> {csv}", rows.len());
    Ok(())
}
