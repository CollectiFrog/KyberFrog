//! `kybench probe`: watches a Spout sender's frame counter (non-destructive
//! read, strictly increasing), copies the two ID bands of each new frame to
//! staging textures, decodes them and timestamps the detection (`t_out`) and
//! the end of the CPU read.

use crate::args::Args;
use crate::clock::{now_us, Pacer};
use crate::idcode::{self, BAND_H, BAND_W};
use crate::spout::{Device, Receiver, Res};

fn fmt(r: &Result<u32, idcode::DecodeError>) -> String {
    match r {
        Ok(id) => id.to_string(),
        Err(e) => format!("{e:?}"),
    }
}

pub fn run(a: &Args) -> Res<()> {
    let name = a.str("name", "kybench-src");
    let duration_s = a.num("duration", 60.0f64)?;
    let poll_us = a.num("poll-us", 50i64)?;
    let wait_s = a.num("wait", 30.0f64)?;
    let csv = a.str("csv", "probe.csv");

    let dev = Device::new()?;
    let sdk_poll = a.str("sdk-poll", "false") == "true";
    let mut rx = Receiver::new(&name, sdk_poll);
    let pacer = Pacer::new()?;

    let give_up = now_us() + (wait_s * 1e6) as i64;
    while !rx.refresh(&dev)? || rx.frame_count().is_none() {
        if now_us() > give_up {
            return Err(format!("sender '{name}' not found (or has no frame counter)").into());
        }
        pacer.sleep_until(now_us() + 100_000);
    }
    let bands = [dev.staging_texture(BAND_W as u32, BAND_H as u32)?,
                 dev.staging_texture(BAND_W as u32, BAND_H as u32)?];
    eprintln!("probe: '{name}' {}x{}, {duration_s} s, poll {poll_us} µs", rx.width, rx.height);

    let mut last = rx.settled_frame_count(&pacer).unwrap_or(0);
    let end = now_us() + (duration_s * 1e6) as i64;
    let mut rows = Vec::new();
    let mut next_refresh = 0i64;

    while now_us() < end {
        let now = now_us();
        if now >= next_refresh {
            rx.refresh(&dev)?;
            next_refresh = now + 500_000;
        }
        let Some(count) = rx.frame_count() else { break };
        // Strictly increasing only: another receiver's Spout-SDK-style poll
        // (wait then release) shows up as a transient count − 1 — not a frame.
        if count <= last {
            pacer.sleep_until(now + poll_us);
            continue;
        }
        let t_out = now_us();
        let step = count - last;
        last = count;

        let (w, h) = (rx.width as usize, rx.height as usize);
        let Some(src) = rx.texture() else { continue };
        let copied = rx.locked(|| -> Res<()> {
            for (band, (x, y)) in bands.iter().zip(idcode::band_origins(w, h)) {
                dev.copy_region(band, src, x as u32, y as u32, BAND_W as u32, BAND_H as u32)?;
            }
            Ok(())
        });
        let (a_id, b_id) = match copied {
            Some(Ok(())) => (dev.read(&bands[0], idcode::decode)?, dev.read(&bands[1], idcode::decode)?),
            _ => {
                rows.push(format!("{count},{step},{t_out},{},,,0", now_us()));
                continue;
            }
        };
        let t_read = now_us();
        let ok = matches!((a_id, b_id), (Ok(x), Ok(y)) if x == y);
        rows.push(format!("{count},{step},{t_out},{t_read},{},{},{}", fmt(&a_id), fmt(&b_id), ok as u8));
    }

    let mut out = String::from("frame_count,step,t_out_us,t_read_us,id_a,id_b,decoded\n");
    let decoded = rows.iter().filter(|r| r.ends_with(",1")).count();
    for r in &rows {
        out.push_str(r);
        out.push('\n');
    }
    std::fs::write(&csv, out)?;
    eprintln!("probe: {} frames seen, {decoded} decoded -> {csv}", rows.len());
    Ok(())
}
