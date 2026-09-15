"""Porte de l'étape 3 du banc de latence (#28-4) — codec d'ID sous compression.

Rejoue le générateur à travers la chaîne K (kyavserver x264 → kycontroller →
QUIC loopback → kyclient `--spout-out`) et ne compte que le décodage des IDs
sur le sender de sortie ; aucune latence n'est jugée ici.

    k-default   débit par défaut (kyclient ne demande rien : 20 Mbps)
    k-min       débit minimum configurable (`--bitrate` = `minimum_bitrate`, 5 Mbps)
    margin      exploration sous le plancher, hors porte (`minimum_bitrate` abaissé)

La sonde lit `bench-out` sans toucher au compteur (`NtQuerySemaphore`) ; rien ne
lit le sender source à part kyavserver (piste B11 : un lecteur Spout destructif
concurrent lui fait capturer des frames fantômes, visibles ici en doublons d'ID).

    python bench/step3_gate.py --kybench bench/kybench/kybench.exe \\
        --bundle C:/Users/trist/KyberFrog-bench/bundle-643ee0e --out bench/runs/<date>-step3 \\
        [--seconds 600] [--only k-default,k-min,margin] [--margin 2M,1M,500K] [--margin-seconds 120]

Seuils : docs/dev/plan-bench-latency.md § Étape 3. Un cas déjà présent dans
`--out` n'est pas rejoué sauf `--redo`.
"""

from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
import time
from collections import Counter
from pathlib import Path

import spout_probe
from step2_gate import FPS, K_OUT, SRC, KPipeline, dist, kill_tree, log, rows, start

THRESHOLDS = {
    "decode_rate_min": 0.9999,   # frames décodées / frames vues sur la sortie
    "false_accepts": 0,          # ID accepté mais incohérent (hors générateur, retour arrière, latence absurde)
    "k_fps_min": 59.0,           # débit du sender de sortie et IDs uniques par seconde
}
# Écart à la latence médiane du run au-delà duquel un ID « valide » ne peut pas
# être la frame qu'il prétend être (15 frames ; queue K observée ≤ 125 ms).
MAX_DEVIATION_MS = 250.0
GEN_CUT_PERIOD = 128  # kybench gen : WRAP_PX / SCROLL_PX
# Applications qui ouvrent des récepteurs Spout (lecteurs destructifs, B11).
SPOUT_APPS = ("TouchDesigner", "Resolume", "Arena", "Avenue", "SpoutToNDI", "NDItoSpout", "OBS", "obs64")


def parse_rate(text):
    m = re.fullmatch(r"(\d+)([KM]?)", text.strip().upper())
    if not m:
        raise ValueError(f"débit invalide : {text}")
    return int(m.group(1)) * {"": 1, "K": 1_000, "M": 1_000_000}[m.group(2)]


def spout_apps_running():
    out = subprocess.run(["tasklist", "/FO", "CSV", "/NH"], capture_output=True, text=True).stdout
    names = {line.split(",")[0].strip('"') for line in out.splitlines() if line}
    return sorted(n for n in names if any(n.lower().startswith(a.lower()) for a in SPOUT_APPS))


# --- analyse -----------------------------------------------------------------

def analyse(gen_csv, probe_csv):
    t_pub = {int(r["id"]): int(r["t_pub_us"]) for r in rows(gen_csv)}
    seen = rows(probe_csv)
    reasons = Counter()
    decoded, false_accepts = [], []
    last_id = None
    ok_lat = sorted((int(r["t_out_us"]) - t_pub[int(r["id_a"])]) / 1000 for r in seen
                    if r["decoded"] == "1" and int(r["id_a"]) in t_pub)
    median = ok_lat[len(ok_lat) // 2] if ok_lat else 0.0
    for r in seen:
        if r["decoded"] != "1":
            if not r["id_a"]:
                reasons["read_failed"] += 1       # copie de la ROI impossible (mutex Spout)
            elif r["id_a"].isdigit() and r["id_b"].isdigit():
                reasons["copies_disagree"] += 1   # deux CRC valides, IDs différents
            elif r["id_a"].isdigit() or r["id_b"].isdigit():
                reasons["one_copy_only"] += 1     # l'autre copie rejetée : jamais acceptée seule
            else:
                reasons[f"both_{r['id_a']}"] += 1  # both_BadCrc, both_NoContrast
            continue
        fid = int(r["id_a"])
        lat = (int(r["t_out_us"]) - t_pub[fid]) / 1000 if fid in t_pub else None
        why = None
        if lat is None:
            why = "id_not_generated"
        elif lat < 0 or abs(lat - median) > MAX_DEVIATION_MS:
            why = "implausible_latency"
        elif last_id is not None and fid < last_id:
            why = "id_went_backwards"
        if why:
            false_accepts.append({"frame_count": int(r["frame_count"]), "id": fid, "latency_ms": lat, "why": why})
        else:
            decoded.append((fid, lat, int(r["t_out_us"])))
            last_id = fid

    ids = [d[0] for d in decoded]
    counts = [int(r["frame_count"]) for r in seen]
    t_out = [int(r["t_out_us"]) for r in seen]
    span_s = (t_out[-1] - t_out[0]) / 1e6 if len(t_out) > 1 else 0
    unique = len(set(ids))
    n_decoded = sum(1 for r in seen if r["decoded"] == "1")
    return {
        "frames_seen": len(seen),
        "decoded_crc_ok": n_decoded,
        "decode_failures": len(seen) - n_decoded,
        "decode_rate": round(n_decoded / len(seen), 6) if seen else 0,
        "failure_reasons": dict(reasons),
        "false_accepts": false_accepts,
        # Débit réel de la chaîne : IDs distincts sortis par seconde, et cadence
        # du sender de sortie (compteur Spout) sur la même fenêtre.
        "unique_ids": unique,
        "unique_ids_per_s": round((unique - 1) / span_s, 3) if span_s else None,
        "out_fps": round((counts[-1] - counts[0]) / span_s, 3) if span_s else None,
        "counter_skips": sum(int(r["step"]) - 1 for r in seen[1:]),
        "id_duplicates": sum(1 for a, b in zip(ids, ids[1:]) if b == a),
        "id_gaps": sum(b - a - 1 for a, b in zip(ids, ids[1:]) if b > a + 1),
        # Le fond du générateur saute toutes les WRAP_PX / SCROLL_PX = 128 frames
        # (coupure de scène) : c'est là que la sortie perd des frames (étape 3).
        "missing_ids_on_cut": sum(1 for a, b in zip(ids, ids[1:]) for i in range(a + 1, b) if i % GEN_CUT_PERIOD == 0),
        "first_id": ids[0] if ids else None,
        "last_id": ids[-1] if ids else None,
        # Indicatif seulement (la latence n'est pas l'objet de l'étape 3).
        "latency_ms_indicative": dist([d[1] for d in decoded]),
    }


def kyavserver_log(path, since):
    """Débit réellement configuré dans x264 et incidents relevés par kyavserver.

    `since` (horodatage local `AAAA-MM-JJTHH:MM:SS`) sépare les frames jetées au
    démarrage (backlog avant que le client soit prêt) de celles du run."""
    text = path.read_text(encoding="utf-8", errors="replace")
    drops = re.findall(r"^(\S{19})\S*\s.*Dropping (?:filtered )?frame", text, re.M)
    requested = re.findall(r"Start Kymux Video: URI .*?bitrate: (\d+)", text)
    return {
        "client_requested_bps": [int(x) for x in requested],
        "encoder_b": [int(x) for x in re.findall(r"^b = (\d+)", text, re.M)],
        "x264_bitrate_kbps": [int(x) for x in re.findall(r"rc=\w+ .*?bitrate=(\d+)", text)],
        "bitrate_too_low_warnings": text.count("Bitrate too low"),
        "set_bitrate_events": re.findall(r"Set bitrate to (\d+)", text),
        "dropped_frames_startup": sum(1 for t in drops if t < since),
        "dropped_frames_during_run": sum(1 for t in drops if t >= since),
        "error_lines": len(re.findall(r" ERROR ", text)),
        "encoder_starts": text.count("Create encoder libx264"),
    }


# --- cas ---------------------------------------------------------------------

def k_case(a, out, tag, seconds, bitrate=None, minimum=None):
    """Générateur → K → sonde sur `bench-out` pendant `seconds`."""
    extra = f"minimum_bitrate = {minimum}\n" if minimum is not None else ""
    client = ["--bitrate", str(bitrate)] if bitrate is not None else []
    gen = start(a.kybench, "gen", out, f"{tag}-gen", name=SRC, duration=seconds + 60, csv=out / f"{tag}-gen.csv")
    work = out / f"k-{tag}"
    try:
        time.sleep(2)
        with KPipeline(a.bundle, work, SRC, kyavserver_extra=extra, client_args=client):
            log(f"{tag} : {seconds} s, débit demandé {bitrate or 'défaut'}, minimum_bitrate {minimum or 'défaut'}")
            since = time.strftime("%Y-%m-%dT%H:%M:%S")
            probe = start(a.kybench, "probe", out, f"{tag}-probe", name=K_OUT, duration=seconds,
                          csv=out / f"{tag}-probe.csv", finish_in_lock=str(a.finish_in_lock).lower())
            probe.wait()
        gen.wait()
    except BaseException:
        kill_tree(gen)
        raise
    res = {"seconds": seconds, "bitrate_arg": bitrate, "minimum_bitrate": minimum,
           **analyse(out / f"{tag}-gen.csv", out / f"{tag}-probe.csv"),
           "kyavserver": kyavserver_log(work / "kycontroller.log", since)}
    log(f"{tag} : {res['decoded_crc_ok']}/{res['frames_seen']} décodées ({res['decode_rate']:.4%}), "
        f"faux positifs {len(res['false_accepts'])}, sortie {res['out_fps']} fps, "
        f"IDs uniques {res['unique_ids_per_s']}/s, x264 b={res['kyavserver']['encoder_b']}")
    return res


def case_default(a, out):
    return k_case(a, out, "k-default", a.seconds)


def case_min(a, out):
    return k_case(a, out, "k-min", a.seconds, bitrate=5_000_000)


def case_margin(a, out):
    res = {}
    for text in a.margin.split(","):
        bps = parse_rate(text)
        res[text] = k_case(a, out, f"margin-{text}", a.margin_seconds, bitrate=bps, minimum=bps)
        time.sleep(2)
    return res


CASES = {"k-default": case_default, "k-min": case_min, "margin": case_margin}


# --- porte -------------------------------------------------------------------

def gate(res, expected_bps):
    T = THRESHOLDS
    g = {}
    for name in ("k-default", "k-min"):
        r = res.get(name)
        if not r:
            continue
        b = r["kyavserver"]["encoder_b"]
        g[f"{name}_decode_rate"] = {"value": r["decode_rate"], "frames_seen": r["frames_seen"],
                                    "threshold": T["decode_rate_min"],
                                    "passed": r["frames_seen"] > 0 and r["decode_rate"] >= T["decode_rate_min"]}
        g[f"{name}_false_accepts"] = {"value": len(r["false_accepts"]), "threshold": T["false_accepts"],
                                      "passed": len(r["false_accepts"]) <= T["false_accepts"]}
        # Conditions de validité du cas : débit x264 effectif, cadence réelle (B11).
        g[f"{name}_bitrate"] = {"value": b, "expected": expected_bps[name],
                                "passed": b == [expected_bps[name]] and not r["kyavserver"]["set_bitrate_events"]}
        fps = min(r["out_fps"] or 0, r["unique_ids_per_s"] or 0)
        g[f"{name}_fps"] = {"value": {"out_fps": r["out_fps"], "unique_ids_per_s": r["unique_ids_per_s"],
                                      "id_duplicates": r["id_duplicates"]},
                            "threshold": T["k_fps_min"], "passed": fps >= T["k_fps_min"]}
    return g


def main():
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--kybench", type=Path, required=True)
    ap.add_argument("--bundle", type=Path, required=True)
    ap.add_argument("--out", type=Path, required=True)
    ap.add_argument("--seconds", type=float, default=600)
    ap.add_argument("--margin", default="2M,1M,500K", help="débits d'exploration sous le plancher")
    ap.add_argument("--margin-seconds", type=float, default=120)
    ap.add_argument("--only", default=",".join(CASES))
    ap.add_argument("--redo", action="store_true")
    ap.add_argument("--finish-in-lock", action="store_true",
                    help="la sonde attend l'exécution GPU de sa copie sous le mutex Spout")
    a = ap.parse_args()
    sys.stdout.reconfigure(encoding="utf-8")
    a.kybench = a.kybench.resolve()
    out = a.out.resolve()
    out.mkdir(parents=True, exist_ok=True)

    busy = [s for s in (SRC, K_OUT) if s in spout_probe.senders()]
    apps = spout_apps_running()
    if busy or apps:
        print(f"senders déjà présents : {busy} ; applications Spout ouvertes : {apps} — les arrêter d'abord")
        return 2

    for name in a.only.split(","):
        path = out / f"{name}.json"
        if path.exists() and not a.redo:
            log(f"{name} : déjà fait ({path.name}), passé")
            continue
        log(f"=== {name} ===")
        result = CASES[name](a, out)
        path.write_text(json.dumps(result, indent=2) + "\n", encoding="utf-8")

    res = {n: json.loads((out / f"{n}.json").read_text(encoding="utf-8"))
           for n in CASES if (out / f"{n}.json").exists()}
    g = gate(res, {"k-default": 20_000_000, "k-min": 5_000_000})
    report = {"seconds": a.seconds, "thresholds": THRESHOLDS, "cases_done": sorted(res), "gate": g,
              "margin": {k: {f: v[f] for f in ("frames_seen", "decode_rate", "failure_reasons", "false_accepts",
                                                "out_fps", "unique_ids_per_s")} | {"encoder_b": v["kyavserver"]["encoder_b"]}
                         for k, v in res.get("margin", {}).items()},
              "passed": all(v["passed"] for v in g.values()) and {"k-default", "k-min"} <= set(res)}
    (out / "step3.json").write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    print(json.dumps(report["gate"], indent=2))
    print("PASSED" if report["passed"] else "NOT PASSED")
    return 0 if report["passed"] else 1


if __name__ == "__main__":
    sys.exit(main())
