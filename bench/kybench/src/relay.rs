//! `kybench relay`: calibrated delay line. Reads a Spout sender, holds each
//! frame on the GPU and republishes it on another sender exactly `delay` after
//! it was detected — the accuracy standard of step 2. Publishing is split in
//! two (prepare `lead` before due, signal at due) so the input keeps being
//! polled in between.

use std::collections::VecDeque;

use crate::args::Args;
use crate::clock::{now_us, Pacer};
use crate::spout::{Device, Receiver, Res, Sender};

pub fn run(a: &Args) -> Res<()> {
    let from = a.str("from", "kybench-src");
    let to = a.str("to", "kybench-relay");
    let fps = a.num("fps", 60.0f64)?;
    let delay_us = match a.opt("delay-frames") {
        Some(k) => (k.parse::<f64>()? * 1e6 / fps).round() as i64,
        None => (a.num("delay-ms", 50.0f64)? * 1e3).round() as i64,
    };
    let duration_s = a.num("duration", 60.0f64)?;
    let poll_us = a.num("poll-us", 50i64)?;
    // See gen: GPU copy paid before the due time, `t_pub` on it.
    let lead_us = a.num("lead-us", 2_000i64)?;
    let csv = a.str("csv", "relay.csv");

    let dev = Device::new()?;
    let sdk_poll = a.str("sdk-poll", "false") == "true";
    let mut rx = Receiver::new(&from, sdk_poll);
    let pacer = Pacer::new()?;
    let give_up = now_us() + 30_000_000;
    while !rx.refresh(&dev)? || rx.frame_count().is_none() {
        if now_us() > give_up {
            return Err(format!("sender '{from}' not found").into());
        }
        pacer.sleep_until(now_us() + 100_000);
    }
    let (w, h) = (rx.width, rx.height);
    let sender = Sender::new(&dev, &to, w, h)?;
    let slots_needed = (delay_us as f64 * fps / 1e6).ceil() as usize + 8;
    let mut free: Vec<_> = (0..slots_needed).map(|_| dev.private_texture(w, h)).collect::<Res<_>>()?;
    let mut queue: VecDeque<(windows::Win32::Graphics::Direct3D11::ID3D11Texture2D, i64, i64)> =
        VecDeque::new();
    eprintln!("relay: '{from}' -> '{to}' {w}x{h}, delay {delay_us} µs, lead {lead_us} µs, {slots_needed} slots");

    let mut last = rx.settled_frame_count(&pacer).unwrap_or(0);
    let end = now_us() + (duration_s * 1e6) as i64;
    let (mut rows, mut overflow) = (Vec::new(), 0u64);
    // Frame whose copy is done and whose sender mutex is held, waiting for its due time.
    let mut armed: Option<(windows::Win32::Graphics::Direct3D11::ID3D11Texture2D, i64, i64, i64)> = None;

    while now_us() < end {
        // Input: never blocked for long, so a frame arriving while another one
        // is armed (3-frame delay = exactly one period) is still seen on time.
        if let Some(count) = rx.frame_count() {
            if count > last {
                let t_in = now_us();
                last = count;
                match (free.pop(), rx.texture()) {
                    (Some(slot), Some(src)) => {
                        if rx.locked(|| dev.copy(&slot, src)).is_some() {
                            dev.flush();
                            queue.push_back((slot, t_in, count));
                        } else {
                            free.push(slot);
                        }
                    }
                    (slot, _) => {
                        overflow += 1;
                        free.extend(slot);
                    }
                }
            }
        }
        // Output, two stages: arm (GPU copy paid, mutex held) `lead` before due,
        // then signal exactly at due.
        if armed.is_none() {
            if let Some((_, t_in, _)) = queue.front() {
                let due = t_in + delay_us;
                if now_us() + poll_us >= due - lead_us {
                    let (slot, t_in, count) = queue.pop_front().unwrap();
                    if sender.prepare(&dev, &slot)? {
                        armed = Some((slot, t_in, count, due));
                    } else {
                        rows.push(format!("{count},{t_in},{due},{},0", now_us()));
                        free.push(slot);
                    }
                }
            }
        }
        if let Some((_, _, _, due)) = &armed {
            if now_us() + poll_us >= *due {
                let (slot, t_in, count, due) = armed.take().unwrap();
                pacer.sleep_until(due);
                let t_pub = sender.signal();
                rows.push(format!("{count},{t_in},{due},{t_pub},1"));
                free.push(slot);
            }
        }
        let next = match (&armed, queue.front()) {
            (Some((_, _, _, due)), _) => *due,
            (None, Some((_, t_in, _))) => t_in + delay_us - lead_us,
            (None, None) => i64::MAX,
        };
        pacer.sleep_until((now_us() + poll_us).min(next));
    }

    let mut out = String::from("frame_count,t_in_us,t_due_us,t_pub_us,published\n");
    for r in &rows {
        out.push_str(r);
        out.push('\n');
    }
    std::fs::write(&csv, out)?;
    eprintln!("relay: {} frames republished, {overflow} dropped (no free slot) -> {csv}", rows.len());
    Ok(())
}
