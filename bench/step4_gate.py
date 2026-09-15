"""Porte de l'étape 4 du banc de latence (#28-4) — chaîne K instrumentée.

Générateur → chaîne K (kyavserver x264 → kycontroller → QUIC loopback → kyclient
`--spout-out`) → sonde sur `bench-out`, avec et sans `kyclient --metrics true`.

    k-metrics-{1,2,3}   métriques internes : horloges, jointure ID ↔ PTS,
                        recoupement, complétude, décomposition, queue p99
    k-plain-{1,2,3}     même run sans métriques : overhead de `--metrics`
    cpu-idle            générateur à 1 fps : CPU de kyavserver quand la source
                        Spout ne publie presque rien (scrutation `av_usleep`)
    cpu-screen          contre-épreuve : même chaîne en source écran, sans Spout

Ordre entrelacé m, p, p, m, m, p (dérive lente compensée). Chaque run : processus
frais, préchauffage écarté, puis sonde pendant `--seconds`. kyclient est arrêté
par CTRL_BREAK (`metrics.json` complet), jamais tué sauf repli consigné.

    python bench/step4_gate.py --kybench bench/kybench/kybench.exe \\
        --bundle C:/Users/trist/KyberFrog-bench/bundle-643ee0e --out bench/runs/<date>-step4 \\
        --f0 bench/runs/2026-09-13-step2 [--seconds 300] [--warmup 60] [--only ...] [--redo]

Seuils : docs/dev/plan-bench-latency.md § Étape 4. Un run déjà acquis n'est pas
rejoué sauf `--redo` ; l'analyse est toujours refaite depuis les fichiers.
"""

from __future__ import annotations

import argparse
import bisect
import csv
import ctypes
import gzip
import json
import shutil
import statistics
import subprocess
import sys
import time
from collections import Counter
from ctypes import wintypes
from pathlib import Path

import spout_probe
from step2_gate import FPS, K_OUT, SRC, KPipeline, dist, kill_tree, log, pct, start
from step3_gate import GEN_CUT_PERIOD, kyavserver_log, spout_apps_running

THRESHOLDS = {
    "offset_abs_ms": 1.0,            # |network_ping.offset_micros|
    "join_ambiguous": 0,             # appariements ID ↔ PTS ambigus
    "join_coverage_min": 0.99,       # frames du générateur appariées / frames de la fenêtre
    "crosscheck_tolerance_ms": 1.0,  # p50 et p99 de t_out − displayed contre le plancher F0
    "overhead_ms": 0.5,              # |médiane des p50 avec − sans métriques|
    "completeness_min": 0.95,        # non bloquant : publié avec la décomposition
    "k_fps_min": 59.0,
}
RUNS = ["k-metrics-1", "k-plain-1", "k-plain-2", "k-metrics-2", "k-metrics-3", "k-plain-3"]
PERIOD_US = 1_000_000 / FPS
EVENTS = ("acquired", "encoding", "encoded", "sent", "received", "decoding", "decoded", "prepared", "displayed")
SEGMENTS = [("t_pub", "acquired")] + list(zip(EVENTS, EVENTS[1:])) + [("displayed", "t_out")]
STARTUP_BUDGET_S = 40  # démarrage de KPipeline (≈ 17 s mesurés) : durée du générateur
LOG_DROPS = ("message dropped", "Metric discarded", "Failed to send Metrics", "Channel full")


# --- fichiers ----------------------------------------------------------------

def open_text(path):
    path = Path(path)
    gz = path.with_name(path.name + ".gz")
    if not path.exists() and gz.exists():
        return gzip.open(gz, "rt", newline="", encoding="utf-8")
    return open(path, newline="", encoding="utf-8")


def rows(path):
    with open_text(path) as f:
        return list(csv.DictReader(f))


def gzip_file(path):
    if path.exists():
        with open(path, "rb") as src, gzip.open(path.with_name(path.name + ".gz"), "wb") as dst:
            shutil.copyfileobj(src, dst)
        path.unlink()


# --- CPU par thread (GetThreadTimes) -----------------------------------------

k32 = ctypes.WinDLL("kernel32", use_last_error=True)
k32.CreateToolhelp32Snapshot.restype = wintypes.HANDLE
k32.OpenThread.restype = wintypes.HANDLE
k32.OpenProcess.restype = wintypes.HANDLE


class THREADENTRY32(ctypes.Structure):
    _fields_ = [("dwSize", wintypes.DWORD), ("cntUsage", wintypes.DWORD), ("th32ThreadID", wintypes.DWORD),
                ("th32OwnerProcessID", wintypes.DWORD), ("tpBasePri", wintypes.LONG),
                ("tpDeltaPri", wintypes.LONG), ("dwFlags", wintypes.DWORD)]


def _filetime(ft):
    return (ft.dwHighDateTime << 32) | ft.dwLowDateTime


def _times(fn, handle):
    c, e, k, u = (wintypes.FILETIME() for _ in range(4))
    if not fn(handle, ctypes.byref(c), ctypes.byref(e), ctypes.byref(k), ctypes.byref(u)):
        return None
    return _filetime(k) + _filetime(u)


def _thread_name(handle):
    buf = wintypes.LPWSTR()
    try:
        if k32.GetThreadDescription(handle, ctypes.byref(buf)) >= 0 and buf.value:
            name = buf.value
            k32.LocalFree(buf)
            return name
    except (AttributeError, OSError):
        pass
    return ""


def pids_by_name(name):
    out = subprocess.run(["tasklist", "/FO", "CSV", "/NH", "/FI", f"IMAGENAME eq {name}"],
                         capture_output=True, text=True).stdout
    return [int(line.split(",")[1].strip('"')) for line in out.splitlines() if line.startswith('"')]


def cpu_snapshot(names=("kyavserver.exe", "kycontroller.exe", "kyclient.exe")):
    """Temps CPU (100 ns) des processus nommés et de chacun de leurs threads."""
    procs = {}
    for name in names:
        for pid in pids_by_name(name):
            h = k32.OpenProcess(0x1000, False, pid)  # PROCESS_QUERY_LIMITED_INFORMATION
            total = _times(k32.GetProcessTimes, h) if h else None
            if h:
                k32.CloseHandle(h)
            procs[pid] = {"name": name, "total": total, "threads": {}}
    snap = k32.CreateToolhelp32Snapshot(4, 0)  # TH32CS_SNAPTHREAD
    te = THREADENTRY32()
    te.dwSize = ctypes.sizeof(te)
    ok = k32.Thread32First(snap, ctypes.byref(te))
    while ok:
        if te.th32OwnerProcessID in procs:
            h = k32.OpenThread(0x0800, False, te.th32ThreadID)  # THREAD_QUERY_LIMITED_INFORMATION
            if h:
                t = _times(k32.GetThreadTimes, h)
                if t is not None:
                    procs[te.th32OwnerProcessID]["threads"][te.th32ThreadID] = (t, _thread_name(h))
                k32.CloseHandle(h)
        ok = k32.Thread32Next(snap, ctypes.byref(te))
    k32.CloseHandle(snap)
    return {"wall": time.perf_counter(), "procs": procs}


def cpu_delta(s0, s1, top=6):
    """Cœurs consommés entre deux instantanés, par processus et par thread (top N)."""
    wall = (s1["wall"] - s0["wall"]) * 1e7
    res = {"seconds": round(wall / 1e7, 1), "logical_cores": __import__("os").cpu_count(), "processes": []}
    for pid, p1 in s1["procs"].items():
        p0 = s0["procs"].get(pid)
        if not p0 or p0["total"] is None or p1["total"] is None:
            continue
        threads = []
        for tid, (t1, name) in p1["threads"].items():
            t0 = p0["threads"].get(tid, (None, ""))[0]
            if t0 is not None:
                threads.append({"tid": tid, "name": name, "cores": round((t1 - t0) / wall, 3)})
        threads.sort(key=lambda t: -t["cores"])
        res["processes"].append({"name": p1["name"], "pid": pid, "cores": round((p1["total"] - p0["total"]) / wall, 3),
                                 "threads": len(p1["threads"]), "top_threads": threads[:top]})
    return res


# --- acquisition -------------------------------------------------------------

def acquire(a, work, metrics, seconds, warmup, fps=FPS, probe=True, screen=False):
    """Un run K frais ; écrit `run.json` (conditions) à côté des CSV et logs.
    `screen` : source écran au lieu du générateur Spout (sans générateur ni sonde)."""
    work.mkdir(parents=True, exist_ok=True)
    for f in work.glob("*"):
        f.unlink()
    gen_dur = STARTUP_BUDGET_S + warmup + seconds + 10
    gen_end = time.monotonic() + 0.5 + gen_dur
    gen = None if screen else start(a.kybench, "gen", work, "gen", name=SRC, fps=fps, duration=gen_dur,
                                    csv=work / "gen.csv")
    meta = {"metrics": metrics, "fps": fps, "warmup_s": warmup, "source": "screen" if screen else SRC}
    try:
        time.sleep(2)
        k = KPipeline(a.bundle, work, None if screen else SRC, metrics=metrics,
                      min_fps=50 if fps == FPS and not screen else 0)
        t0 = time.monotonic()
        with k:
            meta["startup_s"] = round(time.monotonic() - t0, 1)
            time.sleep(warmup)
            meta["since"] = time.strftime("%Y-%m-%dT%H:%M:%S")
            measured = seconds if screen else min(seconds, gen_end - time.monotonic() - 5)
            if measured < seconds:
                log(f"démarrage lent : mesure réduite à {measured:.0f} s")
            meta["measured_s"] = round(measured, 1)
            c0 = cpu_snapshot()
            if probe:
                p = start(a.kybench, "probe", work, "probe", name=K_OUT, duration=measured, csv=work / "probe.csv")
                p.wait()
            else:
                time.sleep(measured)
            meta["cpu"] = cpu_delta(c0, cpu_snapshot())
            time.sleep(1)  # métriques des dernières frames sondées
        meta["client_stop"] = k.client_stop
        if gen:
            gen.wait()
    except BaseException:
        if gen:
            kill_tree(gen)
        raise
    (work / "run.json").write_text(json.dumps(meta, indent=2) + "\n", encoding="utf-8")
    for name in ("gen.csv", "probe.csv", "metrics.json"):
        gzip_file(work / name)


# --- analyse -----------------------------------------------------------------

def load_metrics(work):
    """`metrics.json` → {pts: {événement: ts}}, enregistrements réseau datés par
    le plus grand `ts` vidéo déjà écrit (ils n'ont pas d'horodatage propre)."""
    video, net = {}, []
    bad = duplicates = 0
    clock = None
    with open_text(work / "metrics.json") as f:
        for line in f:
            try:
                o = json.loads(line)
            except ValueError:
                bad += 1
                continue
            if o.get("type") == "video":
                ev = video.setdefault(o["pts"], {})
                duplicates += o["event"] in ev
                ev[o["event"]] = o["ts"]
                clock = o["ts"] if clock is None else max(clock, o["ts"])
            else:
                net.append((clock, o))
    return video, net, {"invalid_lines": bad, "duplicate_events": duplicates}


def probe_sightings(t_pub, probe_rows):
    """Premier et dernier passage de chaque ID décodé sur la sortie."""
    first, last, count = {}, {}, Counter()
    failures = unknown = 0
    for r in probe_rows:
        if r["decoded"] != "1":
            failures += 1
            continue
        fid, t = int(r["id_a"]), int(r["t_out_us"])
        if fid not in t_pub:
            unknown += 1
            continue
        count[fid] += 1
        first.setdefault(fid, t)
        last[fid] = t
    return first, last, count, failures, unknown


def join(pubs, video, lo, hi):
    """Appariement monotone : chaque `acquired` va à la dernière frame publiée
    avant lui. Ambigu si la capture tombe à plus d'une demi-période de la
    publication, si un ID est pris deux fois, si l'ordre des PTS contredit celui
    des IDs ou si le pas des PTS diffère de plus d'une demi-période de celui des
    `t_pub` (et non de `ΔID × période` : le générateur publie parfois en retard)."""
    pub_t = [t for t, _ in pubs]
    acq = sorted((ev["acquired"], pts) for pts, ev in video.items() if "acquired" in ev)
    pts_of, reasons = {}, Counter()
    ambiguous, prev = [], None
    for ts, pts in acq:
        k = bisect.bisect_right(pub_t, ts) - 1
        if k < 0:
            continue
        t, fid = pubs[k]
        why = []
        if ts - t >= PERIOD_US / 2:
            why.append("capture_late")
        if fid in pts_of:
            why.append("id_taken_twice")
        if prev:
            pfid, ppts, pt = prev
            if fid <= pfid or pts <= ppts:
                why.append("non_monotone")
            elif abs((pts - ppts) - (t - pt)) > PERIOD_US / 2:
                why.append("pts_step_mismatch")
        prev = (fid, pts, t)
        if lo <= fid <= hi:
            reasons.update(why)
            if why:
                ambiguous.append({"id": fid, "pts": pts, "why": why})
        # Signalée ou non, la frame est appariée : le contrôle pixel tranche.
        pts_of.setdefault(fid, pts)
    return pts_of, ambiguous, reasons


def pixel_check(sightings, video, pts_of, ids):
    """Contrôle indépendant de l'horloge : l'ID lu sur la sortie au plus près de
    `displayed(pts)` doit être l'ID apparié à cette PTS. Exception cohérente
    (B12) : la texture montre déjà n + 1, *déjà décodée* à l'instant de la
    lecture. Renvoie le verdict par ID et, pour B12, `t_out − decoded(n + 1)`."""
    tout = [t for t, _ in sightings]
    out, b12_lead = {}, []
    for fid in ids:
        ev = video[pts_of[fid]]
        if "displayed" not in ev:
            out[fid] = "not_displayed"
            continue
        # Lecture la plus proche de `displayed` (une frame en rattrapage peut
        # s'afficher ~2 ms avant la suivante).
        d = ev["displayed"]
        k = bisect.bisect_left(tout, d)
        cand = [j for j in (k - 1, k) if 0 <= j < len(tout)]
        j = min(cand, key=lambda j: abs(tout[j] - d)) if cand else None
        if j is None or abs(tout[j] - d) > 5000:
            out[fid] = "no_sighting"
            continue
        pid = sightings[j][1]
        nxt = video[pts_of[fid + 1]] if fid + 1 in pts_of else {}
        if pid == fid:
            out[fid] = "confirmed"
        elif pid == fid + 1 and "decoded" in nxt and nxt["decoded"] <= tout[j]:
            out[fid] = "b12_next_frame"
            b12_lead.append(((tout[j] - nxt["decoded"]) / 1000, (ev["displayed"] - nxt["decoded"]) / 1000,
                             (ev["prepared"] - nxt["decoded"]) / 1000 if "prepared" in ev else None,
                             (nxt["decoded"] - nxt["decoding"]) / 1000 if "decoding" in nxt else None))
        else:
            out[fid] = "contradicted"
    return out, b12_lead


def corr(x, y):
    try:
        return {"pearson": round(statistics.correlation(x, y), 3),
                "spearman": round(statistics.correlation(x, y, method="ranked"), 3), "n": len(x)}
    except statistics.StatisticsError:
        return {"pearson": None, "spearman": None, "n": len(x)}


def analyse_run(work, f0):
    meta = json.loads((work / "run.json").read_text(encoding="utf-8"))
    gen_rows = rows(work / "gen.csv")
    t_pub = {int(r["id"]): int(r["t_pub_us"]) for r in gen_rows if r["published"] == "1"}
    late_pub = {int(r["id"]) for r in gen_rows if int(r["t_pub_us"]) - int(r["t_deadline_us"]) > 1000}
    probe_rows = rows(work / "probe.csv")
    first, last, count, failures, unknown = probe_sightings(t_pub, probe_rows)
    ids = sorted(first)
    lo, hi = ids[0], ids[-1]
    window = [i for i in range(lo, hi + 1) if i in t_pub]
    missing = [i for i in window if i not in first]
    span = (int(probe_rows[-1]["t_out_us"]) - int(probe_rows[0]["t_out_us"])) / 1e6
    e2e = {i: (first[i] - t_pub[i]) / 1000 for i in ids}
    res = {
        "meta": {k: v for k, v in meta.items() if k != "cpu"},
        "cpu": meta["cpu"],
        "probe": {
            "frames_seen": len(probe_rows), "decode_failures": failures, "ids_not_generated": unknown,
            "out_fps": round((int(probe_rows[-1]["frame_count"]) - int(probe_rows[0]["frame_count"])) / span, 3),
            "window_frames": len(window), "unique_ids": len(ids),
            "missing_ids": len(missing),
            "missing_on_cut": sum(1 for i in missing if i % GEN_CUT_PERIOD == 0),
            "ids_seen_twice": sum(1 for c in count.values() if c > 1),
            "gen_late_publications": sum(1 for i in window if i in late_pub),
        },
        # Chiffre de tête : première apparition de chaque ID sur la sortie.
        "latency_ms": dist(list(e2e.values())),
        "latency_last_sighting_ms": dist([(last[i] - t_pub[i]) / 1000 for i in ids]),
        "kyavserver": kyavserver_log(work / "kycontroller.log", meta["since"]),
    }
    logs = {}
    for name in ("kyclient.stdout.log", "kycontroller.log"):
        text = (work / name).read_text(encoding="utf-8", errors="replace")
        logs[name] = {p: text.count(p) for p in LOG_DROPS}
    res["log_drops"] = logs
    if not meta["metrics"]:
        return res

    video, net, mstats = load_metrics(work)
    pubs = sorted((t, i) for i, t in t_pub.items())
    pts_of, ambiguous, reasons = join(pubs, video, lo, hi)
    joined = [i for i in window if i in pts_of]
    ev = {i: video[pts_of[i]] for i in joined}
    sightings = sorted((int(r["t_out_us"]), int(r["id_a"])) for r in probe_rows if r["decoded"] == "1")
    pixel, b12_lead = pixel_check(sightings, video, pts_of, joined)
    unresolved = [x for x in ambiguous if pixel.get(x["id"]) not in ("confirmed", "b12_next_frame")]

    offsets = [o["offset_micros"] for _, o in net if o.get("type") == "network_ping"]
    res["clock"] = {"offset_micros": offsets, "max_abs_offset_ms": max(abs(x) for x in offsets) / 1000,
                    "acquired_minus_t_pub_ms": dist([(ev[i]["acquired"] - t_pub[i]) / 1000 for i in joined])}
    # pts − t_pub : si la PTS vient de l'horloge de capture, sa dispersion est
    # celle de la détection — contrôle indépendant de l'appariement.
    off = [pts_of[i] - t_pub[i] for i in joined]
    med = statistics.median(off)
    res["join"] = {
        "window_frames": len(window), "joined": len(joined), "coverage": round(len(joined) / len(window), 6),
        # Signalement temporel pré-enregistré (capture à plus d'une demi-période,
        # pas des PTS ≠ pas des t_pub), puis verdict des pixels.
        "timing_flagged": len(ambiguous), "reasons": dict(reasons),
        "timing_flagged_examples": [{**x, "pixel": pixel.get(x["id"])} for x in ambiguous[:10]],
        "pixel_check": dict(Counter(pixel.values())),
        # B12 : délai entre le décodage de n + 1 et sa lecture sous le compteur de n.
        "b12_sighting_minus_next_decoded_ms": dist([x[0] for x in b12_lead]),
        "b12_displayed_minus_next_decoded_ms": dist([x[1] for x in b12_lead]),
        "b12_prepared_minus_next_decoded_ms": dist([x[2] for x in b12_lead if x[2] is not None]),
        "b12_next_decode_duration_ms": dist([x[3] for x in b12_lead if x[3] is not None]),
        "ambiguous": len({x["id"] for x in unresolved} | {i for i, v in pixel.items() if v == "contradicted"}),
        "pts_minus_t_pub_dispersion_ms": dist([(x - med) / 1000 for x in off]),
        "metrics_file": mstats,
    }

    seen = [i for i in joined if i in first and "displayed" in ev[i]]
    res["crosscheck"] = {
        "last_sighting_minus_displayed_ms": dist([(last[i] - ev[i]["displayed"]) / 1000 for i in seen]),
        "first_sighting_minus_displayed_ms": dist([(first[i] - ev[i]["displayed"]) / 1000 for i in seen]),
        "f0_ms": f0,
    }

    complete = [i for i in joined if all(e in ev[i] for e in EVENTS)]
    res["completeness"] = {
        "window_frames": len(window), "complete": len(complete),
        "fraction": round(len(complete) / len(window), 6),
        "skipped": sum(1 for i in joined if "skipped" in ev[i]),
        "by_event": {e: sum(1 for i in joined if e in ev[i]) for e in EVENTS},
    }

    def seg(i, a, b):
        ta = t_pub[i] if a == "t_pub" else ev[i][a]
        tb = last[i] if b == "t_out" else ev[i][b]
        return (tb - ta) / 1000

    full = [i for i in complete if i in first]
    segs = {f"{a}→{b}": [seg(i, a, b) for i in full] for a, b in SEGMENTS}
    res["decomposition_ms"] = {k: dist(v) for k, v in segs.items()}
    res["decomposition_ms"]["acquired→displayed"] = dist([seg(i, "acquired", "displayed") for i in full])
    res["decomposition_ms"]["t_pub→t_out (tête)"] = dist([e2e[i] for i in full])

    # --- queue p99 -----------------------------------------------------------
    p99 = pct(list(e2e.values()), 99)
    medians = {k: statistics.median(v) for k, v in segs.items()}
    tail = [i for i in full if e2e[i] >= p99]
    dominant, excess_sum = Counter(), Counter()
    for i in tail:
        excess = {f"{a}→{b}": seg(i, a, b) - medians[f"{a}→{b}"] for a, b in SEGMENTS}
        dominant[max(excess, key=excess.get)] += 1
        excess_sum.update({k: max(0.0, v) for k, v in excess.items()})

    def near_cut(i):
        return (i % GEN_CUT_PERIOD) in (GEN_CUT_PERIOD - 1, 0, 1, 2)

    # La frame de coupure (fond qui saute, i % 128 == 0), la suivante, et les autres.
    groups = {"cut": [i for i in full if i % GEN_CUT_PERIOD == 0],
              "after_cut": [i for i in full if i % GEN_CUT_PERIOD == 1],
              "other": [i for i in full if not near_cut(i)]}
    by_group = {g: {k: round(statistics.median(seg(i, *k.split("→")) for i in ids_), 3)
                    for k in ("encoding→encoded", "sent→received", "decoding→decoded", "decoded→prepared")}
                | {"t_pub→t_out": round(statistics.median(e2e[i] for i in ids_), 3), "n": len(ids_)}
                for g, ids_ in groups.items() if ids_}

    thr = {k: pct(segs[k], 99) for k in ("sent→received", "received→decoding")}
    remote = [(t, o["packets_lost"]) for t, o in net if o.get("type") == "network_remote" and t is not None]
    local = [(t, o["packets_lost"]) for t, o in net if o.get("type") == "network_local" and t is not None]
    windows = []
    for (t0, l0), (t1, l1) in zip(remote, remote[1:]):
        inside = [i for i in full if t0 <= ev[i]["sent"] < t1]
        if len(inside) < 60 or l1 < l0:
            continue
        windows.append({
            "lost": l1 - l0,
            "frames": len(inside),
            "sent_received_spikes": sum(1 for i in inside if seg(i, "sent", "received") >= thr["sent→received"]),
            "received_decoding_spikes": sum(1 for i in inside
                                            if seg(i, "received", "decoding") >= thr["received→decoding"]),
            "e2e_p99_ms": round(pct([e2e[i] for i in inside], 99), 3),
            "e2e_tail_frames": sum(1 for i in inside if e2e[i] >= p99),
        })
    lost = [w["lost"] for w in windows]
    res["tail"] = {
        "e2e_p99_ms": round(p99, 3), "tail_frames": len(tail),
        "dominant_segment": dict(dominant.most_common()),
        "excess_ms_by_segment": {k: round(v, 1) for k, v in excess_sum.most_common() if v >= 0.5},
        "median_by_cut_group_ms": by_group,
        "e2e_p99_excluding_near_cut_ms": round(pct([e2e[i] for i in ids if not near_cut(i)], 99), 3),
        "near_cut_fraction": round(sum(1 for i in tail if near_cut(i)) / len(tail), 3) if tail else None,
        "near_cut_base_rate": round(4 / GEN_CUT_PERIOD, 3),
        "after_late_publication": sum(1 for i in tail if any(j in late_pub for j in range(i - 3, i + 1))),
        "segment_p99_thresholds_ms": {k: round(v, 3) for k, v in thr.items()},
        "packets_lost_remote_per_s": round((remote[-1][1] - remote[0][1]) / ((remote[-1][0] - remote[0][0]) / 1e6), 1)
        if len(remote) > 1 and remote[-1][0] > remote[0][0] else None,
        "packets_lost_local_total": (local[-1][1] - local[0][1]) if len(local) > 1 else None,
        "windows_5s": len(windows),
        "corr_lost_vs_sent_received_spikes": corr(lost, [w["sent_received_spikes"] for w in windows]),
        "corr_lost_vs_received_decoding_spikes": corr(lost, [w["received_decoding_spikes"] for w in windows]),
        "corr_lost_vs_e2e_p99": corr(lost, [w["e2e_p99_ms"] for w in windows]),
        "corr_lost_vs_tail_frames": corr(lost, [w["e2e_tail_frames"] for w in windows]),
        "lost_per_window": dist(lost),
    }
    return res


# --- porte -------------------------------------------------------------------

def f0_floor(path):
    load = json.loads((path / "load.json").read_text(encoding="utf-8"))["runs"]
    return {"p50": max(r["latency_ms"]["p50"] for r in load), "p99": max(r["latency_ms"]["p99"] for r in load),
            "source": f"{path.name}/load.json (F0 pendant qu'une chaîne K tourne, pire run)"}


def gate(res, expected_frames):
    T = THRESHOLDS
    g = {}
    m = [res[n] for n in RUNS if n.startswith("k-metrics") and n in res]
    p = [res[n] for n in RUNS if n.startswith("k-plain") and n in res]
    valid = {}
    for n in RUNS:
        r = res.get(n)
        if r:
            valid[n] = {"encoder_b": r["kyavserver"]["encoder_b"], "drops": r["kyavserver"]["dropped_frames_during_run"],
                        "out_fps": r["probe"]["out_fps"], "client_stop": r["meta"]["client_stop"],
                        "window_frames": r["probe"]["window_frames"]}
    g["run_validity"] = {"value": valid, "passed": len(valid) == len(RUNS) and all(
        v["encoder_b"] == [20_000_000] and v["drops"] == 0 and v["out_fps"] >= T["k_fps_min"]
        and str(v["client_stop"]).startswith("ctrl_break") and v["window_frames"] >= expected_frames * 0.99
        for v in valid.values())}
    if m:
        off = max(r["clock"]["max_abs_offset_ms"] for r in m)
        g["offset_ms"] = {"value": off, "threshold": T["offset_abs_ms"], "passed": off <= T["offset_abs_ms"]}
        acq = [r["clock"]["acquired_minus_t_pub_ms"] for r in m]
        g["acquired_after_t_pub"] = {"value": [{k: d[k] for k in ("min", "p50", "p99")} for d in acq],
                                     "passed": all(d["min"] >= 0 and d["p99"] < PERIOD_US / 2000 for d in acq)}
        g["join"] = {"value": [{k: r["join"][k] for k in ("joined", "coverage", "ambiguous")} for r in m],
                     "threshold": {"ambiguous": T["join_ambiguous"], "coverage": T["join_coverage_min"]},
                     "passed": all(r["join"]["ambiguous"] <= T["join_ambiguous"]
                                   and r["join"]["coverage"] >= T["join_coverage_min"] for r in m)}
        cc = [r["crosscheck"]["last_sighting_minus_displayed_ms"] for r in m]
        f0 = m[0]["crosscheck"]["f0_ms"]
        g["crosscheck_ms"] = {"value": [{k: d[k] for k in ("p50", "p99")} for d in cc], "f0": f0,
                              "threshold": T["crosscheck_tolerance_ms"],
                              "passed": all(abs(d["p50"] - f0["p50"]) <= T["crosscheck_tolerance_ms"]
                                            and abs(d["p99"] - f0["p99"]) <= T["crosscheck_tolerance_ms"] for d in cc)}
        frac = min(r["completeness"]["fraction"] for r in m)
        g["completeness"] = {"value": [r["completeness"]["fraction"] for r in m], "threshold": T["completeness_min"],
                             "blocking": False, "passed": frac >= T["completeness_min"]}
    if m and p:
        mp50 = statistics.median(r["latency_ms"]["p50"] for r in m)
        pp50 = statistics.median(r["latency_ms"]["p50"] for r in p)
        g["overhead_ms"] = {"value": round(mp50 - pp50, 3), "metrics_p50": [r["latency_ms"]["p50"] for r in m],
                            "plain_p50": [r["latency_ms"]["p50"] for r in p],
                            "metrics_p99": [r["latency_ms"]["p99"] for r in m],
                            "plain_p99": [r["latency_ms"]["p99"] for r in p],
                            "threshold": T["overhead_ms"], "passed": abs(mp50 - pp50) <= T["overhead_ms"]}
    return g


def main():
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--kybench", type=Path, required=True)
    ap.add_argument("--bundle", type=Path, required=True)
    ap.add_argument("--out", type=Path, required=True)
    ap.add_argument("--f0", type=Path, required=True, help="dossier de runs de l'étape 2 (plancher)")
    ap.add_argument("--seconds", type=float, default=300)
    ap.add_argument("--warmup", type=float, default=60)
    ap.add_argument("--idle-seconds", type=float, default=60)
    ap.add_argument("--only", default=",".join(RUNS + ["cpu-idle", "cpu-screen"]))
    ap.add_argument("--redo", action="store_true", help="réacquérir les runs déjà présents")
    ap.add_argument("--analyse-only", action="store_true")
    a = ap.parse_args()
    sys.stdout.reconfigure(encoding="utf-8")
    a.kybench = a.kybench.resolve()
    out = a.out.resolve()
    out.mkdir(parents=True, exist_ok=True)
    f0 = f0_floor(a.f0.resolve())

    todo = [n for n in a.only.split(",") if a.redo or not (out / n / "run.json").exists()]
    if todo and not a.analyse_only:
        busy = [s for s in (SRC, K_OUT) if s in spout_probe.senders()]
        apps = spout_apps_running()
        stray = [n for n in ("kyclient.exe", "kycontroller.exe", "kyavserver.exe", "KyberFrog.exe") if pids_by_name(n)]
        if busy or apps or stray:
            print(f"senders : {busy} ; applications Spout : {apps} ; processus Kyber : {stray} — les arrêter d'abord")
            return 2
        for name in todo:
            log(f"=== {name} ===")
            if name == "cpu-idle":
                acquire(a, out / name, False, a.idle_seconds, 10, fps=1, probe=False)
            elif name == "cpu-screen":
                acquire(a, out / name, False, a.idle_seconds, 10, probe=False, screen=True)
            else:
                acquire(a, out / name, name.startswith("k-metrics"), a.seconds, a.warmup)
            time.sleep(3)

    res = {}
    for name in RUNS:
        if (out / name / "run.json").exists():
            res[name] = analyse_run(out / name, f0)
            (out / f"{name}.json").write_text(json.dumps(res[name], indent=2, ensure_ascii=False) + "\n",
                                              encoding="utf-8")
            r = res[name]
            log(f"{name} : tête p50 {r['latency_ms']['p50']} p99 {r['latency_ms']['p99']} ms, "
                f"{r['probe']['unique_ids']}/{r['probe']['window_frames']} IDs, stop {r['meta']['client_stop']}")
    extra_cpu = {n: json.loads((out / n / "run.json").read_text(encoding="utf-8"))["cpu"]
                 for n in ("cpu-idle", "cpu-screen") if (out / n / "run.json").exists()}
    g = gate(res, int(a.seconds * FPS))
    report = {"seconds": a.seconds, "warmup": a.warmup, "thresholds": THRESHOLDS, "runs_done": sorted(res),
              "gate": g,
              "cpu": {"k_runs": {n: r["cpu"] for n, r in res.items()}, **extra_cpu},
              "tail": {n: r["tail"] for n, r in res.items() if "tail" in r},
              "passed": set(RUNS) <= set(res) and all(v["passed"] for v in g.values() if v.get("blocking", True))}
    (out / "step4.json").write_text(json.dumps(report, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    print(json.dumps(report["gate"], indent=2, ensure_ascii=False))
    print("PASSED" if report["passed"] else "NOT PASSED")
    return 0 if report["passed"] else 1


if __name__ == "__main__":
    sys.exit(main())
