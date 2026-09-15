"""Exploration hors protocole : leviers de latence K sans code (threads x264, AMF)."""
import json, sys, types
from pathlib import Path

sys.path.insert(0, r"C:\Users\trist\Workspace\KyberFrog_WS2\kyberfrog\bench")
import step2_gate
import step4_gate

BASE = step2_gate.CONFIG
X264 = """
[kyavserver.video_encoder_config]
b = 20000000
preset = "ultrafast"
tune = "zerolatency"
flags = "+global_header"
x264-params = "force-cfr=0"
threads = {threads}
"""
CASES = {
    "x264-t2": ("x264", ""),
    "x264-t6": ("x264", X264.format(threads=6)),
    "x264-t12": ("x264", X264.format(threads=12)),
    "amf": ("amf", ""),
}

out = Path(sys.argv[1]); out.mkdir(parents=True, exist_ok=True)
a = types.SimpleNamespace(kybench=Path(r"C:\Users\trist\Workspace\KyberFrog_WS2\kyberfrog\bench\kybench\kybench.exe"),
                          bundle=Path(r"C:\Users\trist\KyberFrog-bench\bundle-643ee0e"))
f0 = step4_gate.f0_floor(Path(r"C:\Users\trist\Workspace\KyberFrog_WS2\kyberfrog\bench\runs\2026-09-13-step2"))

for name in sys.argv[2].split(","):
    encoder, extra = CASES[name]
    # Le dictionnaire remplace tout : ajouté après [kyavserver], avant [kycontroller].
    step2_gate.CONFIG = BASE.replace('encoder = "x264"', f'encoder = "{encoder}"').replace("{extra}", extra.replace("{", "{{").replace("}", "}}"))
    step2_gate.log(f"=== {name} ===")
    work = out / name
    step4_gate.acquire(a, work, True, 60, 15)
    r = step4_gate.analyse_run(work, f0)
    (out / f"{name}.json").write_text(json.dumps(r, indent=2, ensure_ascii=False), encoding="utf-8")
    d = r["decomposition_ms"]
    print(f"== {name}: tête p50 {r['latency_ms']['p50']} p99 {r['latency_ms']['p99']}, IDs {r['probe']['unique_ids']}/{r['probe']['window_frames']}, "
          f"jointure ambiguë {r['join']['ambiguous']}", flush=True)
    for k in ("t_pub→acquired", "acquired→encoding", "encoding→encoded", "sent→received", "decoding→decoded",
              "decoded→prepared", "displayed→t_out"):
        print(f"   {k:22s} p50 {d[k]['p50']:7.3f}  p99 {d[k]['p99']:7.3f}")
    kav = [p for p in r["cpu"]["processes"] if p["name"] == "kyavserver.exe" and p["cores"] > 0.01]
    print("   cpu kyavserver", [(p["cores"], [t["cores"] for t in p["top_threads"][:3]]) for p in kav], flush=True)
