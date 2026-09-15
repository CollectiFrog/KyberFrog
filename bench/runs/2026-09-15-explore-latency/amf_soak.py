"""Requalification AMF (#28-1) : chaîne K en h264_amf pendant N secondes, métriques incluses.

    python amf_soak.py <out> [secondes]

Même acquisition et même analyse que la porte de l'étape 4 ; seul `encoder`
change dans le kyber_config.toml généré.
"""
import json, sys, time, types
from pathlib import Path

BENCH = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(BENCH))
import step2_gate
import step4_gate

# Absolu : kycontroller lit KYBER_CONFIG_PATH depuis le dossier du bundle.
out = Path(sys.argv[1]).resolve(); out.mkdir(parents=True, exist_ok=True)
seconds = float(sys.argv[2]) if len(sys.argv) > 2 else 600
step2_gate.CONFIG = step2_gate.CONFIG.replace('encoder = "x264"', 'encoder = "amf"')
a = types.SimpleNamespace(kybench=BENCH / "kybench" / "kybench.exe",
                          bundle=Path(r"C:\Users\trist\KyberFrog-bench\bundle-643ee0e"))
f0 = step4_gate.f0_floor(BENCH / "runs" / "2026-09-13-step2")
work = out / "amf-soak"
t0 = time.time()
step4_gate.acquire(a, work, True, seconds, 15)
r = step4_gate.analyse_run(work, f0)
(out / "amf-soak.json").write_text(json.dumps(r, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
print(json.dumps({"elapsed_s": round(time.time() - t0), "latency_ms": r["latency_ms"], "probe": r["probe"],
                  "kyavserver": r["kyavserver"], "client_stop": r["meta"]["client_stop"],
                  "join_ambiguous": r["join"]["ambiguous"], "completeness": r["completeness"]["fraction"]},
                 indent=2, ensure_ascii=False))
