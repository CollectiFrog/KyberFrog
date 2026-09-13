"""Porte de l'étape 1 du banc de latence (#28-4).

1. `kybench gen` → `kybench probe` pendant `--seconds` : la sonde doit décoder
   100 % des frames, IDs consécutifs, sans trou ni doublon.
2. `kybench gen` → `kybench relay` (retard 50 ms) → `kybench probe` sur la sortie
   du relais : le relais republie et la sonde décode.

Les latences sont affichées à titre indicatif : les mesurer pour de vrai est
l'objet de l'étape 2.

    python bench/step1_gate.py --kybench bench/kybench/kybench.exe --out <dossier> [--seconds 60]
"""

from __future__ import annotations

import argparse
import csv
import json
import statistics
import subprocess
import sys
import time
from pathlib import Path


def start(exe, cmd, out, log, **kw):
    args = [str(exe), cmd] + [x for k, v in kw.items() for x in (f"--{k.replace('_', '-')}", str(v))]
    return subprocess.Popen(args, stderr=open(out / f"{log}.log", "wb"))


def rows(path):
    with open(path, newline="") as f:
        return list(csv.DictReader(f))


def pct(values, p):
    s = sorted(values)
    return s[min(len(s) - 1, int(p / 100 * len(s)))]


def analyse(gen_csv, probe_csv):
    t_pub = {int(r["id"]): int(r["t_pub_us"]) for r in rows(gen_csv)}
    seen = rows(probe_csv)
    decoded = [r for r in seen if r["decoded"] == "1"]
    ids = [int(r["id_a"]) for r in decoded]
    gaps = sum(b - a - 1 for a, b in zip(ids, ids[1:]) if b > a + 1)
    dups = sum(1 for a, b in zip(ids, ids[1:]) if b <= a)
    skipped_counts = sum(int(r["step"]) - 1 for r in seen[1:])
    lat = [(int(r["t_out_us"]) - t_pub[int(r["id_a"])]) / 1000 for r in decoded if int(r["id_a"]) in t_pub]
    read = [(int(r["t_read_us"]) - int(r["t_out_us"])) / 1000 for r in decoded]
    return {
        "frames_seen": len(seen),
        "decoded": len(decoded),
        "decode_failures": len(seen) - len(decoded),
        "id_gaps": gaps,
        "id_duplicates": dups,
        "counter_skips": skipped_counts,
        "first_id": ids[0] if ids else None,
        "last_id": ids[-1] if ids else None,
        "latency_ms": {"p50": round(pct(lat, 50), 3), "p99": round(pct(lat, 99), 3),
                       "min": round(min(lat), 3), "max": round(max(lat), 3)} if lat else None,
        "read_ms_p50": round(statistics.median(read), 3) if read else None,
    }


def main():
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--kybench", type=Path, required=True)
    ap.add_argument("--out", type=Path, required=True)
    ap.add_argument("--seconds", type=float, default=60)
    ap.add_argument("--relay-seconds", type=float, default=20)
    args = ap.parse_args()
    out = args.out
    out.mkdir(parents=True, exist_ok=True)
    exe = args.kybench.resolve()

    # 1. gen → probe
    gen = start(exe, "gen", out, "gen", name="kybench-src", duration=args.seconds + 6, csv=out / "gen.csv")
    time.sleep(2)
    probe = start(exe, "probe", out, "probe", name="kybench-src", duration=args.seconds, csv=out / "probe.csv")
    probe.wait()
    gen.wait()
    direct = analyse(out / "gen.csv", out / "probe.csv")

    # 2. gen → relay 50 ms → probe
    s = args.relay_seconds
    gen = start(exe, "gen", out, "gen-relay", name="kybench-src", duration=s + 10, csv=out / "gen-relay.csv")
    time.sleep(1)
    relay = start(exe, "relay", out, "relay", **{"from": "kybench-src"}, to="kybench-relay",
                  delay_ms=50, duration=s + 6, csv=out / "relay.csv")
    time.sleep(3)
    probe = start(exe, "probe", out, "probe-relay", name="kybench-relay", duration=s,
                  csv=out / "probe-relay.csv")
    for p in (probe, relay, gen):
        p.wait()
    relayed = analyse(out / "gen-relay.csv", out / "probe-relay.csv")

    gate = {
        "direct_100pct": direct["frames_seen"] > 0 and direct["decode_failures"] == 0
                         and direct["id_gaps"] == 0 and direct["id_duplicates"] == 0,
        "relay_republishes": relayed["decoded"] > 0,
    }
    report = {"seconds": args.seconds, "direct": direct, "relay_50ms": relayed, "gate": gate,
              "passed": all(gate.values())}
    (out / "step1.json").write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    print(json.dumps(report, indent=2))
    return 0 if report["passed"] else 1


if __name__ == "__main__":
    sys.exit(main())
