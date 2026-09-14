//! kybench — latency bench instrument (#28-4, docs/dev/plan-bench-latency.md).
//!
//!   kybench gen   --name kybench-src --width 1920 --height 1080 --fps 60 --duration 60 --csv gen.csv
//!   kybench probe --name kybench-src --duration 60 --poll-us 50 --csv probe.csv
//!   kybench relay --from kybench-src --to kybench-relay --delay-ms 50 --duration 60 --csv relay.csv
//!   kybench list
//!
//! Every timestamp is QPC microseconds, the clock of txproto, kyproto and VLC.

mod idcode;

#[cfg(windows)]
mod args;
#[cfg(windows)]
mod clock;
#[cfg(windows)]
mod gen;
#[cfg(windows)]
mod hud;
#[cfg(windows)]
mod probe;
#[cfg(windows)]
mod relay;
#[cfg(windows)]
mod spout;

#[cfg(windows)]
fn main() {
    let mut argv = std::env::args().skip(1);
    let cmd = argv.next().unwrap_or_default();
    let result = args::Args::parse(argv).and_then(|a| match cmd.as_str() {
        "gen" => gen::run(&a),
        "probe" => probe::run(&a),
        "relay" => relay::run(&a),
        "list" => {
            spout::sender_names().iter().for_each(|n| println!("{n}"));
            Ok(())
        }
        _ => Err("usage: kybench <gen|probe|relay|list> [--key value]...".into()),
    });
    if let Err(e) = result {
        eprintln!("kybench {cmd}: {e}");
        std::process::exit(1);
    }
}

#[cfg(not(windows))]
fn main() {
    eprintln!("kybench only runs on Windows (Spout / D3D11)");
    std::process::exit(1);
}
