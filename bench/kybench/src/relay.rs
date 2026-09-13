//! `kybench relay`: calibrated delay line. Reads a Spout sender, holds each
//! frame on the GPU and republishes it on another sender exactly `delay` after
//! it was detected — the accuracy standard of step 2.

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
    let poll_us = a.num("poll-us", 250i64)?;
    let csv = a.str("csv", "relay.csv");

    let dev = Device::new()?;
    let mut rx = Receiver::new(&from);
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
    eprintln!("relay: '{from}' -> '{to}' {w}x{h}, delay {delay_us} µs, {slots_needed} slots");

    let mut last = rx.frame_count().unwrap_or(0);
    let end = now_us() + (duration_s * 1e6) as i64;
    let (mut rows, mut overflow) = (Vec::new(), 0u64);

    while now_us() < end {
        if let Some(count) = rx.frame_count() {
            if count != last {
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
        while let Some((_, t_in, _)) = queue.front() {
            let due = t_in + delay_us;
            if now_us() + poll_us < due {
                break;
            }
            pacer.sleep_until(due);
            let (slot, t_in, count) = queue.pop_front().unwrap();
            let published = sender.publish(&dev, &slot)?;
            let t_pub = published.unwrap_or_else(now_us);
            rows.push(format!("{count},{t_in},{due},{t_pub},{}", published.is_some() as u8));
            free.push(slot);
        }
        let next = queue.front().map(|(_, t, _)| t + delay_us).unwrap_or(i64::MAX);
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
