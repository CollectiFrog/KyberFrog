"""K : effet du mouvement du fond sur la latence (même diagnostic que nn_scroll.py)."""
import csv, statistics, sys, time
from pathlib import Path

sys.path.insert(0, r"C:\Users\trist\Workspace\KyberFrog_WS2\kyberfrog\bench")
from step2_gate import K_OUT, SRC, KPipeline, dist, start

K = Path(r"C:\Users\trist\Workspace\KyberFrog_WS2\kyberfrog\bench\kybench\kybench.exe")
BUNDLE = Path(r"C:\Users\trist\KyberFrog-bench\bundle-643ee0e")
out = Path(sys.argv[1]); out.mkdir(parents=True, exist_ok=True)
secs, warm = 60, 15
CASES = {"aligned": {"scroll": 16, "scroll_offset": 0}, "misaligned": {"scroll": 16, "scroll_offset": 8}, "default": {}}

for name in sys.argv[2].split(","):
    tag = f"k-{name}"
    gen = start(K, "gen", out, f"{tag}-gen", name=SRC, duration=secs + warm + 45, csv=out / f"{tag}-gen.csv",
                **CASES[name])
    time.sleep(2)
    with KPipeline(BUNDLE, out / tag, SRC):
        time.sleep(warm)
        start(K, "probe", out, f"{tag}-probe", name=K_OUT, duration=secs, csv=out / f"{tag}-probe.csv").wait()
    gen.wait()
    g = {int(x["id"]): x for x in csv.DictReader(open(out / f"{tag}-gen.csv"))}
    seen, first = [], set()
    for x in csv.DictReader(open(out / f"{tag}-probe.csv")):
        if x["decoded"] == "1" and int(x["id_a"]) in g and int(x["id_a"]) not in first:
            first.add(int(x["id_a"]))
            seen.append(x)
    lat = [(int(x["t_out_us"]) - int(g[int(x["id_a"])]["t_pub_us"])) / 1000 for x in seen]
    ids = sorted(first)
    ev = statistics.median(l for x, l in zip(seen, lat) if int(x["id_a"]) % 2 == 0)
    od = statistics.median(l for x, l in zip(seen, lat) if int(x["id_a"]) % 2 == 1)
    print(f"== K {name}: {len(seen)} IDs, manquants {ids[-1] - ids[0] + 1 - len(ids)}\n  t_out-t_pub {dist(lat)}"
          f"\n  pair/impair {ev:.2f}/{od:.2f}\n  copie Spout {dist([int(x['copy_us']) / 1000 for x in g.values()])}", flush=True)
    time.sleep(3)
