"""Porte de l'étape 2 du banc de latence (#28-4) — plancher de bruit.

Cinq sous-tests, chacun dans son JSON, puis la porte agrégée dans `step2.json` :

    idle    F0 à vide : `kybench gen` → `probe`, N runs × S secondes
    load    F0 pendant qu'une chaîne K (kyavserver → kycontroller → kyclient
            `--spout-out`) tourne sur un *autre* sender kybench
    reader  F0 pendant que kyavserver lit le *même* sender que la sonde
    relay   étalon d'exactitude : relais 3 frames (50,0 ms) puis 7 ms
    pacing  jitter de `t_pub` par rapport à la grille 60 Hz (à partir des runs idle)

Seuils : docs/dev/plan-bench-latency.md § Étape 2.

    python bench/step2_gate.py --kybench bench/kybench/kybench.exe \\
        --bundle C:/Users/trist/KyberFrog-bench/bundle-643ee0e --out bench/runs/<date>-step2 \\
        [--seconds 300] [--runs 3] [--relay-seconds 60] [--only idle,relay]

Un sous-test déjà présent dans `--out` n'est pas rejoué sauf `--redo`.
"""

from __future__ import annotations

import argparse
import csv
import hashlib
import json
import os
import signal
import statistics
import subprocess
import sys
import time
from pathlib import Path

import spout_probe

SRC, LOAD_SRC, K_OUT, RELAY = "kybench-src", "kybench-load", "bench-out", "kybench-relay"
FPS = 60
USER, PASSWORD = "vj", "kyberfrog"  # login transparent de KyberFrog (shared/src/lib.rs)

CONFIG = """\
# Étape 2 du banc de latence (#28-4) — instance isolée, source Spout épinglée.
[kyavserver]
encoder = "x264"
spout_sender = "{sender}"
{extra}
[kycontroller]
port = {port}
tray = false

[[kycontroller.auth.basic.logins]]
username = "{user}"
password = "{hash}"
"""

THRESHOLDS = {
    "f0_p50_ms": 1.5,
    "f0_p99_load_ms": 3.0,
    "p50_spread_ms": 0.3,
    "relay_tolerance_ms": 0.5,
    "relay_p99_minus_p50_ms": 1.0,
    "pacing_p99_ms": 1.0,
    "reader_degradation_ms": 0.3,
}


# --- processus ---------------------------------------------------------------

def log(msg):
    print(time.strftime("%H:%M:%S"), msg, flush=True)


def start(exe, cmd, out, tag, **kw):
    args = [str(exe), cmd] + [x for k, v in kw.items() for x in (f"--{k.replace('_', '-')}", str(v))]
    return subprocess.Popen(args, stderr=open(out / f"{tag}.log", "wb"))


def kill_tree(proc):
    if proc.poll() is None:
        subprocess.run(["taskkill", "/T", "/F", "/PID", str(proc.pid)], capture_output=True, check=False)
    proc.wait()


def wait_sender(name, timeout):
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        if name in spout_probe.senders():
            return True
        time.sleep(0.5)
    return False


def sender_fps(name, seconds):
    counter = spout_probe.FrameCounter(name)
    first = counter.read()
    t0 = time.perf_counter()
    time.sleep(seconds)
    last = counter.read()
    counter.close()
    if first is None or last is None:
        return None
    return (last - first) / (time.perf_counter() - t0)


class KPipeline:
    """kycontroller (source Spout `source`) + kyclient `--spout-out K_OUT`.

    `kyavserver_extra` : lignes TOML ajoutées à `[kyavserver]` ;
    `client_args` : options kyclient en plus (avant l'adresse du serveur) ;
    `metrics` : `kyclient --metrics true`, `metrics.json` écrit dans `workdir` ;
    `min_fps` : débit minimal du sender de sortie exigé au démarrage.

    kyclient tourne dans son propre groupe de processus et s'arrête par
    CTRL_BREAK (son handler `ctrlc` → déconnexion → `metrics.json` vidé) ; le
    kill forcé ne sert que de repli, consigné dans `client_stop`."""

    def __init__(self, bundle, workdir, source, port=9150, kyavserver_extra="", client_args=(),
                 metrics=False, min_fps=50):
        self.bundle, self.work, self.source, self.port = bundle.resolve(), workdir, source, port
        self.extra, self.client_args = kyavserver_extra, list(client_args)
        self.metrics, self.min_fps = metrics, min_fps
        self.procs = []
        self.client_stop = None

    def __enter__(self):
        self.work.mkdir(parents=True, exist_ok=True)
        config = self.work / "kyber_config.toml"
        config.write_text(CONFIG.format(sender=self.source, port=self.port, user=USER, extra=self.extra,
                                        hash=hashlib.sha256(PASSWORD.encode()).hexdigest()),
                          encoding="utf-8")
        self.procs.append(subprocess.Popen(
            [str(self.bundle / "kycontroller.exe")], cwd=self.bundle,
            stdout=open(self.work / "kycontroller.log", "wb"), stderr=subprocess.STDOUT,
            env={**os.environ, "KYBER_CONFIG_PATH": str(config)}))
        time.sleep(3)
        client = [str(self.bundle / "kyclient.exe"), "--port", str(self.port),
                  "--tls-skip-verification", "--auth-username", USER, "--auth-password", PASSWORD,
                  "--spout-out", K_OUT, "--inputs", "false", "--audio", "false",
                  "--keyboard-grab", "false", "--metrics", str(self.metrics).lower(),
                  *self.client_args, "127.0.0.1"]
        self.procs.append(subprocess.Popen(client, cwd=self.work,
                                           stdout=open(self.work / "kyclient.stdout.log", "wb"),
                                           stderr=subprocess.STDOUT,
                                           creationflags=subprocess.CREATE_NEW_PROCESS_GROUP))
        try:
            if not wait_sender(K_OUT, 60):
                raise RuntimeError(f"sender « {K_OUT} » absent après 60 s")
            time.sleep(8)  # backlog de démarrage (4 à 6 s au smoke test)
            fps = sender_fps(K_OUT, 3)
            log(f"chaîne K sur « {self.source} » : « {K_OUT} » à {fps} fps")
            if fps is None or fps < self.min_fps:
                raise RuntimeError(f"chaîne K trop lente ({fps} fps)")
        except Exception:
            self.__exit__()
            raise
        return self

    def stop_client(self, timeout=10):
        """CTRL_BREAK à kyclient (sortie propre en ~0,1 s), kill forcé en repli."""
        client = self.procs[1] if len(self.procs) > 1 else None
        if client is None or client.poll() is not None:
            return
        os.kill(client.pid, signal.CTRL_BREAK_EVENT)
        try:
            client.wait(timeout)
            self.client_stop = f"ctrl_break rc={client.returncode}"
        except subprocess.TimeoutExpired:
            kill_tree(client)
            self.client_stop = "forced"

    def __exit__(self, *exc):
        self.stop_client()
        for proc in reversed(self.procs):
            kill_tree(proc)
        self.procs.clear()
        # kyavserver survit quelques secondes au kill de son kycontroller.
        deadline = time.monotonic() + 15
        while time.monotonic() < deadline and K_OUT in spout_probe.senders():
            time.sleep(0.5)


# --- analyse -----------------------------------------------------------------

def rows(path):
    with open(path, newline="") as f:
        return list(csv.DictReader(f))


def pct(values, p):
    s = sorted(values)
    return s[min(len(s) - 1, int(p / 100 * len(s)))]


def dist(values):
    if not values:
        return None
    return {"n": len(values), "p50": round(pct(values, 50), 3), "p95": round(pct(values, 95), 3),
            "p99": round(pct(values, 99), 3), "min": round(min(values), 3), "max": round(max(values), 3)}


def gen_pacing(gen_rows):
    """Écart de `t_pub` à la grille 60 Hz (ms) et intervalle entre frames (ms)."""
    dev = [(int(r["t_pub_us"]) - int(r["t_deadline_us"])) / 1000 for r in gen_rows]
    pubs = [int(r["t_pub_us"]) for r in gen_rows]
    return {
        "frames": len(gen_rows),
        "dropped_busy_mutex": sum(1 for r in gen_rows if r["published"] != "1"),
        "t_pub_minus_deadline_ms": dist(dev),
        "abs_deviation_ms": dist([abs(d) for d in dev]),
        "interval_ms": dist([(b - a) / 1000 for a, b in zip(pubs, pubs[1:])]),
    }


def analyse(gen_csv, probe_csv):
    """Latence `t_out − t_pub` de chaque frame décodée, pertes, doublons."""
    gen_rows = rows(gen_csv)
    t_pub = {int(r["id"]): int(r["t_pub_us"]) for r in gen_rows}
    seen = rows(probe_csv)
    decoded = [r for r in seen if r["decoded"] == "1"]
    ids = [int(r["id_a"]) for r in decoded]
    lat = [(int(r["t_out_us"]) - t_pub[int(r["id_a"])]) / 1000 for r in decoded if int(r["id_a"]) in t_pub]
    read = [(int(r["t_read_us"]) - int(r["t_out_us"])) / 1000 for r in decoded]
    return {
        "frames_seen": len(seen),
        "decoded": len(decoded),
        "decode_failures": len(seen) - len(decoded),
        "id_gaps": sum(b - a - 1 for a, b in zip(ids, ids[1:]) if b > a + 1),
        "id_duplicates": sum(1 for a, b in zip(ids, ids[1:]) if b <= a),
        "counter_skips": sum(int(r["step"]) - 1 for r in seen[1:]),
        "first_id": ids[0] if ids else None,
        "last_id": ids[-1] if ids else None,
        "latency_ms": dist(lat),
        "read_ms": dist(read),
        "gen": gen_pacing(gen_rows),
    }


def clean(run):
    return run["frames_seen"] > 0 and run["decode_failures"] == 0 and run["id_gaps"] == 0 \
        and run["id_duplicates"] == 0


# --- sous-tests --------------------------------------------------------------

def f0_run(exe, out, tag, seconds, gen=True):
    """`gen` (SRC) → `probe` pendant `seconds`. Le générateur peut être externe."""
    g = start(exe, "gen", out, f"{tag}-gen", name=SRC, duration=seconds + 8,
              csv=out / f"{tag}-gen.csv") if gen else None
    if g:
        time.sleep(2)
    p = start(exe, "probe", out, f"{tag}-probe", name=SRC, duration=seconds, csv=out / f"{tag}-probe.csv")
    p.wait()
    if g:
        g.wait()
    return p.returncode


def subtest_idle(a, out):
    runs = []
    for i in range(a.runs):
        tag = f"idle-{i + 1}"
        log(f"{tag} : {a.seconds} s")
        f0_run(a.kybench, out, tag, a.seconds)
        runs.append(analyse(out / f"{tag}-gen.csv", out / f"{tag}-probe.csv"))
        log(f"{tag} : {runs[-1]['decoded']} décodées, p50 {runs[-1]['latency_ms']['p50']} ms")
        time.sleep(2)
    return {"runs": runs}


def subtest_load(a, out):
    total = a.runs * (a.seconds + 14) + 90
    load_gen = start(a.kybench, "gen", out, "load-gen", name=LOAD_SRC, duration=total,
                     csv=out / "load-gen.csv")
    runs, k_fps = [], []
    try:
        time.sleep(2)
        with KPipeline(a.bundle, out / "k-load", LOAD_SRC):
            for i in range(a.runs):
                tag = f"load-{i + 1}"
                log(f"{tag} : {a.seconds} s, chaîne K sur « {LOAD_SRC} »")
                counter = spout_probe.FrameCounter(K_OUT)
                c0, t0 = counter.read(), time.perf_counter()
                f0_run(a.kybench, out, tag, a.seconds)
                c1, t1 = counter.read(), time.perf_counter()
                counter.close()
                k_fps.append(round((c1 - c0) / (t1 - t0), 2) if None not in (c0, c1) else None)
                runs.append(analyse(out / f"{tag}-gen.csv", out / f"{tag}-probe.csv"))
                log(f"{tag} : {runs[-1]['decoded']} décodées, p50 {runs[-1]['latency_ms']['p50']} ms, "
                    f"K {k_fps[-1]} fps")
                time.sleep(2)
    finally:
        kill_tree(load_gen)
    return {"runs": runs, "k_out_fps": k_fps}


def subtest_reader(a, out):
    runs, k_fps = [], []
    for i in range(a.reader_runs):
        tag = f"reader-{i + 1}"
        # Le générateur écrit son CSV en sortant : durée finie (démarrage de la
        # chaîne K ≈ 20 s), attendu, jamais tué.
        gen = start(a.kybench, "gen", out, f"{tag}-gen", name=SRC, duration=a.seconds + 40,
                    csv=out / f"{tag}-gen.csv")
        try:
            time.sleep(2)
            with KPipeline(a.bundle, out / f"k-{tag}", SRC):
                log(f"{tag} : {a.seconds} s, kyavserver lit « {SRC} » en même temps que la sonde")
                counter = spout_probe.FrameCounter(K_OUT)
                c0, t0 = counter.read(), time.perf_counter()
                f0_run(a.kybench, out, tag, a.seconds, gen=False)
                c1, t1 = counter.read(), time.perf_counter()
                counter.close()
                k_fps.append(round((c1 - c0) / (t1 - t0), 2) if None not in (c0, c1) else None)
            gen.wait()
        except BaseException:
            kill_tree(gen)
            raise
        runs.append(analyse(out / f"{tag}-gen.csv", out / f"{tag}-probe.csv"))
        log(f"{tag} : {runs[-1]['decoded']} décodées, p50 {runs[-1]['latency_ms']['p50']} ms, K {k_fps[-1]} fps")
    return {"runs": runs, "k_out_fps": k_fps}


def relay_case(a, out, tag, seconds, **delay):
    gen = start(a.kybench, "gen", out, f"{tag}-gen", name=SRC, duration=seconds + 10, csv=out / f"{tag}-gen.csv")
    time.sleep(1)
    relay = start(a.kybench, "relay", out, f"{tag}-relay", **{"from": SRC}, to=RELAY, duration=seconds + 6,
                  csv=out / f"{tag}-relay.csv", **delay)
    time.sleep(3)
    probe = start(a.kybench, "probe", out, f"{tag}-probe", name=RELAY, duration=seconds,
                  csv=out / f"{tag}-probe.csv")
    for p in (probe, relay, gen):
        p.wait()
    res = analyse(out / f"{tag}-gen.csv", out / f"{tag}-probe.csv")
    # Le compteur du sender source vaut id + 1 : jointure relais ↔ générateur.
    t_pub = {int(r["id"]): int(r["t_pub_us"]) for r in rows(out / f"{tag}-gen.csv")}
    rl = rows(out / f"{tag}-relay.csv")
    res["relay"] = {
        "republished": len(rl),
        "dropped_busy_mutex": sum(1 for r in rl if r["published"] != "1"),
        "detect_in_ms": dist([(int(r["t_in_us"]) - t_pub[int(r["frame_count"]) - 1]) / 1000
                              for r in rl if int(r["frame_count"]) - 1 in t_pub]),
        "t_pub_minus_due_ms": dist([(int(r["t_pub_us"]) - int(r["t_due_us"])) / 1000 for r in rl]),
    }
    return res


def subtest_relay(a, out):
    s = a.relay_seconds
    log(f"relay 3 frames (50,0 ms) : {s} s")
    r50 = relay_case(a, out, "relay-50ms", s, delay_frames=3)
    log(f"relay-50ms : p50 {r50['latency_ms']['p50']} ms, p99 {r50['latency_ms']['p99']} ms")
    time.sleep(2)
    log(f"relay 7 ms : {s} s")
    r7 = relay_case(a, out, "relay-7ms", s, delay_ms=7)
    log(f"relay-7ms : p50 {r7['latency_ms']['p50']} ms, p99 {r7['latency_ms']['p99']} ms")
    return {"50ms": {"setpoint_ms": 3000 / FPS, **r50}, "7ms": {"setpoint_ms": 7.0, **r7}}


def subtest_pacing(a, out):
    """Jitter de `t_pub` sur les générateurs des runs idle (et load s'ils existent)."""
    dev, interval = [], []
    files = sorted(out.glob("idle-*-gen.csv")) + sorted(out.glob("load-*-gen.csv"))
    for f in files:
        r = rows(f)
        dev += [(int(x["t_pub_us"]) - int(x["t_deadline_us"])) / 1000 for x in r]
        pubs = [int(x["t_pub_us"]) for x in r]
        interval += [(b - q) / 1000 for q, b in zip(pubs, pubs[1:])]
    return {"files": [f.name for f in files], "t_pub_minus_deadline_ms": dist(dev),
            "abs_deviation_ms": dist([abs(d) for d in dev]), "interval_ms": dist(interval)}


SUBTESTS = {"idle": subtest_idle, "load": subtest_load, "reader": subtest_reader,
            "relay": subtest_relay, "pacing": subtest_pacing}


# --- porte -------------------------------------------------------------------

def gate(res, expected_frames):
    T = THRESHOLDS
    g = {}
    idle, load, reader = (res.get(k, {}).get("runs", []) for k in ("idle", "load", "reader"))
    every = idle + load + reader

    def p50s(runs):
        return [r["latency_ms"]["p50"] for r in runs if r["latency_ms"]]

    def p99s(runs):
        return [r["latency_ms"]["p99"] for r in runs if r["latency_ms"]]

    def spread(runs):
        return round(max(p50s(runs)) - min(p50s(runs)), 3)

    if every:
        g["no_loss_no_decode_error"] = {
            "value": {"runs": len(every), "frames": sum(r["decoded"] for r in every),
                      "failures": sum(r["decode_failures"] for r in every),
                      "gaps": sum(r["id_gaps"] for r in every),
                      "duplicates": sum(r["id_duplicates"] for r in every)},
            "passed": all(clean(r) and r["decoded"] >= expected_frames - 2 for r in every)}
    if idle:
        g["f0_p50_idle_ms"] = {"value": p50s(idle), "threshold": T["f0_p50_ms"],
                               "passed": max(p50s(idle)) <= T["f0_p50_ms"]}
        g["p50_spread_idle_ms"] = {"value": spread(idle), "threshold": T["p50_spread_ms"],
                                   "passed": spread(idle) <= T["p50_spread_ms"]}
    if load:
        g["f0_p50_load_ms"] = {"value": p50s(load), "threshold": T["f0_p50_ms"],
                               "passed": max(p50s(load)) <= T["f0_p50_ms"]}
        g["f0_p99_load_ms"] = {"value": p99s(load), "threshold": T["f0_p99_load_ms"],
                               "passed": max(p99s(load)) <= T["f0_p99_load_ms"]}
        g["p50_spread_load_ms"] = {"value": spread(load), "threshold": T["p50_spread_ms"],
                                   "passed": spread(load) <= T["p50_spread_ms"]}
    if "relay" in res:
        for k, r in res["relay"].items():
            lat, sp = r["latency_ms"], r["setpoint_ms"]
            g[f"relay_{k}"] = {
                "value": {"setpoint": sp, "p50": lat["p50"], "p99_minus_p50": round(lat["p99"] - lat["p50"], 3),
                          "clean": clean(r)},
                "threshold": {"tolerance": T["relay_tolerance_ms"], "p99_minus_p50": T["relay_p99_minus_p50_ms"]},
                "passed": abs(lat["p50"] - sp) <= T["relay_tolerance_ms"]
                and lat["p99"] - lat["p50"] <= T["relay_p99_minus_p50_ms"] and clean(r)}
    if "pacing" in res and res["pacing"]["abs_deviation_ms"]:
        p99 = res["pacing"]["abs_deviation_ms"]["p99"]
        g["t_pub_jitter_p99_ms"] = {"value": p99, "threshold": T["pacing_p99_ms"], "passed": p99 <= T["pacing_p99_ms"]}
    if reader and idle:
        d = round(statistics.median(p50s(reader)) - statistics.median(p50s(idle)), 3)
        g["reader_degradation_ms"] = {"value": d, "threshold": T["reader_degradation_ms"],
                                      "passed": d <= T["reader_degradation_ms"]}
    return g


def main():
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--kybench", type=Path, required=True)
    ap.add_argument("--bundle", type=Path, required=True)
    ap.add_argument("--out", type=Path, required=True)
    ap.add_argument("--seconds", type=float, default=300)
    ap.add_argument("--runs", type=int, default=3)
    ap.add_argument("--reader-runs", type=int, default=1)
    ap.add_argument("--relay-seconds", type=float, default=60)
    ap.add_argument("--only", default=",".join(SUBTESTS), help="sous-tests, séparés par des virgules")
    ap.add_argument("--redo", action="store_true", help="rejouer les sous-tests déjà présents")
    a = ap.parse_args()
    sys.stdout.reconfigure(encoding="utf-8")
    a.kybench = a.kybench.resolve()
    # Absolu : kycontroller lit KYBER_CONFIG_PATH depuis le dossier du bundle.
    out = a.out.resolve()
    out.mkdir(parents=True, exist_ok=True)

    busy = [s for s in (SRC, LOAD_SRC, K_OUT, RELAY) if s in spout_probe.senders()]
    if busy:
        print(f"senders déjà présents : {busy} — arrêter leurs émetteurs d'abord")
        return 2

    for name in a.only.split(","):
        path = out / f"{name}.json"
        if path.exists() and not a.redo:
            log(f"{name} : déjà fait ({path.name}), passé")
            continue
        log(f"=== {name} ===")
        result = SUBTESTS[name](a, out)
        path.write_text(json.dumps(result, indent=2) + "\n", encoding="utf-8")

    res = {n: json.loads((out / f"{n}.json").read_text(encoding="utf-8"))
           for n in SUBTESTS if (out / f"{n}.json").exists()}
    g = gate(res, int(a.seconds * FPS))
    report = {"seconds": a.seconds, "runs": a.runs, "relay_seconds": a.relay_seconds,
              "thresholds": THRESHOLDS, "subtests_done": sorted(res), "gate": g,
              "passed": all(v["passed"] for v in g.values()) and set(res) == set(SUBTESTS)}
    (out / "step2.json").write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    print(json.dumps(report["gate"], indent=2))
    print("PASSED" if report["passed"] else "NOT PASSED")
    return 0 if report["passed"] else 1


if __name__ == "__main__":
    sys.exit(main())
