"""Banc de latence KyberFrog vs NDI — lancement manuel d'une configuration (#28-4).

    python bench\\latency_bench.py <config> --out bench\\runs\\<date>-<nom> [--seconds 180]

Configurations :

    f0       plancher du banc : `kybench gen` → `probe`, sans KyberFrog ni NDI.
             Ce que coûte l'instrument lui-même ; à retrancher mentalement.
    k-x264   chaîne KyberFrog complète, encodeur logiciel x264 (livré avant 0.6.0).
    k-amf    chaîne KyberFrog complète, encodeur GPU AMF (défaut depuis 0.6.0).
    ndi      NDI SDK : `kybench ndi-gen` → `ndi-probe`, même générateur, même ID.

Chaque configuration écrit `result.json` et un résumé en clair dans le dossier
`--out`. Les CSV bruts restent à côté pour recalcul.

Le contenu de l'image change le résultat en NDI (voir `--scroll`) : NDI compresse
image par image, donc le mouvement le fait varier. KyberFrog ne bouge pas.
Prérequis : aucune autre application Spout ouverte, Docker Desktop lancé si le
bundle en dépend, machine au repos.
"""

from __future__ import annotations

import argparse
import csv
import hashlib
import json
import os
import signal
import subprocess
import sys
import time
from pathlib import Path

import spout_probe

SRC, K_OUT, NDI_NAME = "kybench-src", "bench-out", "kybench-ndi"
FPS = 60
USER, PASSWORD = "vj", "kyberfrog"  # login transparent de KyberFrog (shared/src/lib.rs)
STARTUP_BUDGET_S = 45  # démarrage de KPipeline (≈ 17 s mesurés), marge incluse

CONFIG = """\
# Banc de latence (#28-4) — instance isolée, source Spout épinglée.
[kyavserver]
encoder = "{encoder}"
spout_sender = "{sender}"

[kycontroller]
port = {port}
tray = false

[[kycontroller.auth.basic.logins]]
username = "{user}"
password = "{hash}"
"""


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
    """kycontroller (source Spout `SRC`) + kyclient `--spout-out K_OUT`.

    kyclient tourne dans son propre groupe de processus et s'arrête par
    CTRL_BREAK (son handler `ctrlc` → déconnexion → sortie rc=0 en ~0,1 s) ; le
    kill forcé ne sert que de repli, consigné dans `client_stop`."""

    def __init__(self, bundle, workdir, encoder, port=9150, min_fps=50):
        self.bundle, self.work, self.encoder, self.port = bundle.resolve(), workdir, encoder, port
        self.min_fps = min_fps
        self.procs = []
        self.client_stop = None
        self.out_fps = None

    def __enter__(self):
        self.work.mkdir(parents=True, exist_ok=True)
        config = self.work / "kyber_config.toml"
        config.write_text(CONFIG.format(encoder=self.encoder, sender=SRC, port=self.port, user=USER,
                                        hash=hashlib.sha256(PASSWORD.encode()).hexdigest()),
                          encoding="utf-8")
        self.procs.append(subprocess.Popen(
            [str(self.bundle / "kycontroller.exe")], cwd=self.bundle,
            stdout=open(self.work / "kycontroller.log", "wb"), stderr=subprocess.STDOUT,
            env={**os.environ, "KYBER_CONFIG_PATH": str(config)}))  # chemin absolu obligatoire
        time.sleep(3)
        client = [str(self.bundle / "kyclient.exe"), "--port", str(self.port),
                  "--tls-skip-verification", "--auth-username", USER, "--auth-password", PASSWORD,
                  "--spout-out", K_OUT, "--inputs", "false", "--audio", "false",
                  "--keyboard-grab", "false", "--metrics", "false", "127.0.0.1"]
        self.procs.append(subprocess.Popen(client, cwd=self.work,
                                           stdout=open(self.work / "kyclient.stdout.log", "wb"),
                                           stderr=subprocess.STDOUT,
                                           creationflags=subprocess.CREATE_NEW_PROCESS_GROUP))
        try:
            if not wait_sender(K_OUT, 60):
                raise RuntimeError(f"sender « {K_OUT} » absent après 60 s")
            time.sleep(8)  # backlog de démarrage (4 à 6 s au smoke test)
            self.out_fps = sender_fps(K_OUT, 3)
            log(f"chaîne KyberFrog ({self.encoder}) : « {K_OUT} » à {self.out_fps} fps")
            if self.out_fps is None or self.out_fps < self.min_fps:
                raise RuntimeError(f"chaîne K trop lente ({self.out_fps} fps)")
        except Exception:
            self.__exit__()
            raise
        return self

    def stop_client(self, timeout=10):
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


def analyse(gen_csv, probe_csv):
    """Latence de chaque image, première vue seulement, plus pertes et pacing.

    `t_out` = l'image est lisible par le récepteur (sonde Spout, ou trame NDI
    reçue) ; `t_up` = elle est dans une texture GPU, ce que paie tout récepteur
    réel — seule la config `ndi` a cette colonne."""
    gen_rows = rows(gen_csv)
    t_pub = {int(r["id"]): int(r["t_pub_us"]) for r in gen_rows}
    seen = rows(probe_csv)
    decoded, first = [], set()
    for r in seen:
        i = int(r["id_a"])
        if r["decoded"] == "1" and i in t_pub and i not in first:
            first.add(i)
            decoded.append(r)
    ids = sorted(first)
    lat_out = [(int(r["t_out_us"]) - t_pub[int(r["id_a"])]) / 1000 for r in decoded]
    lat_up = [(int(r["t_up_us"]) - t_pub[int(r["id_a"])]) / 1000 for r in decoded if r.get("t_up_us")]
    span = ids[-1] - ids[0] + 1 if ids else 0
    dev = [(int(r["t_pub_us"]) - int(r["t_deadline_us"])) / 1000 for r in gen_rows]
    return {
        "frames_generated": len(gen_rows),
        "frames_seen": len(seen),
        "ids_decoded": len(ids),
        "decode_failures": sum(1 for r in seen if r["decoded"] != "1"),
        "ids_missing": span - len(ids),
        "loss_fraction": round((span - len(ids)) / span, 6) if span else None,
        "latency_ms": dist(lat_out),
        "latency_gpu_ms": dist(lat_up) or None,
        "generator_late_ms": dist(dev),
    }


def summary(config, res):
    """Résumé en clair : le p50 en millisecondes et en images à 60 Hz."""
    head = res["latency_gpu_ms"] or res["latency_ms"]
    frames = head["p50"] / (1000 / FPS)
    loss = res["loss_fraction"]
    return "\n".join([
        f"=== {config} — retard entre l'image émise et l'image reçue",
        f"  une image sur deux sous : {head['p50']:6.1f} ms   ({frames:.2f} image à 60 Hz)",
        f"  19 images sur 20 sous   : {head['p95']:6.1f} ms",
        f"  99 images sur 100 sous  : {head['p99']:6.1f} ms",
        f"  pire cas vu             : {head['max']:6.1f} ms",
        f"  images arrivées         : {res['ids_decoded']} sur {res['ids_decoded'] + res['ids_missing']}"
        + (f"  ({loss * 100:.3f} % perdues)" if loss is not None else ""),
    ])


# --- configurations ----------------------------------------------------------

def run_f0(a, out):
    gen = start(a.kybench, "gen", out, "gen", name=SRC, duration=a.seconds + 8,
                csv=out / "gen.csv", **a.content)
    time.sleep(2)
    start(a.kybench, "probe", out, "probe", name=SRC, duration=a.seconds, csv=out / "probe.csv").wait()
    gen.wait()
    return {}


def run_k(a, out, encoder):
    gen = start(a.kybench, "gen", out, "gen", name=SRC,
                duration=a.seconds + a.warmup + STARTUP_BUDGET_S, csv=out / "gen.csv", **a.content)
    time.sleep(2)
    with KPipeline(a.bundle, out, encoder) as pipe:
        time.sleep(a.warmup)
        start(a.kybench, "probe", out, "probe", name=K_OUT, duration=a.seconds,
              csv=out / "probe.csv").wait()
        meta = {"encoder": encoder, "out_fps": pipe.out_fps}
    meta["client_stop"] = pipe.client_stop
    gen.wait()
    return meta


def run_ndi(a, out):
    gen = start(a.kybench, "ndi-gen", out, "gen", name=NDI_NAME, duration=a.seconds + 12,
                csv=out / "gen.csv", ndi_dll=a.ndi_dll, **a.content)
    time.sleep(2)
    start(a.kybench, "ndi-probe", out, "probe", source=NDI_NAME, duration=a.seconds,
          csv=out / "probe.csv", ndi_dll=a.ndi_dll).wait()
    gen.wait()
    return {"ndi_dll": str(a.ndi_dll)}


CONFIGS = {
    "f0": run_f0,
    "k-x264": lambda a, out: run_k(a, out, "x264"),
    "k-amf": lambda a, out: run_k(a, out, "amf"),
    "ndi": run_ndi,
}


def main():
    p = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    p.add_argument("config", choices=sorted(CONFIGS))
    p.add_argument("--out", type=Path, required=True, help="dossier de résultats (créé)")
    p.add_argument("--seconds", type=float, default=180, help="durée de mesure (défaut 180)")
    p.add_argument("--warmup", type=float, default=15, help="chauffe avant mesure, configs k (défaut 15)")
    p.add_argument("--kybench", type=Path, default=Path(__file__).parent / "kybench" / "kybench.exe")
    p.add_argument("--bundle", type=Path, default=Path(r"C:\Users\trist\KyberFrog-bench\bundle-643ee0e"),
                   help="bundle KyberFrog mesuré, configs k")
    p.add_argument("--ndi-dll", type=Path, default=None, help="défaut : NDI_RUNTIME_DIR_V6")
    p.add_argument("--scroll", type=int, default=None,
                   help="pas de défilement du fond en px ; 16 = aligné sur la grille de compression")
    p.add_argument("--scroll-offset", type=int, default=None, help="décalage du fond en px")
    p.add_argument("--clip", type=Path, default=None,
                   help="images BGRA brutes rejouées en boucle au lieu du fond synthétique "
                        "(recette ffmpeg dans bench/README.md)")
    a = p.parse_args()

    if a.ndi_dll is None:
        root = os.environ.get("NDI_RUNTIME_DIR_V6")
        a.ndi_dll = Path(root) / "Processing.NDI.Lib.x64.dll" if root else None
    if a.config == "ndi" and (a.ndi_dll is None or not a.ndi_dll.exists()):
        sys.exit("runtime NDI introuvable : installe NDI 6 ou passe --ndi-dll")
    if a.config.startswith("k-") and not (a.bundle / "kycontroller.exe").exists():
        sys.exit(f"bundle introuvable : {a.bundle}")

    if a.clip is not None and not a.clip.exists():
        sys.exit(f"clip introuvable : {a.clip}")
    a.content = {k: v for k, v in (("scroll", a.scroll), ("scroll_offset", a.scroll_offset),
                                   ("clip", a.clip)) if v is not None}
    a.out.mkdir(parents=True, exist_ok=True)
    log(f"{a.config} : {a.seconds} s dans {a.out}")
    t0 = time.time()
    meta = CONFIGS[a.config](a, a.out)
    res = analyse(a.out / "gen.csv", a.out / "probe.csv")
    res["meta"] = {"config": a.config, "seconds": a.seconds,
                   "content": {k: str(v) for k, v in a.content.items()},
                   "elapsed_s": round(time.time() - t0), **meta}
    # newline="\n" : sans lui Python écrit du CRLF sur Windows, et les résultats
    # versionnés partiraient en diff de fin de ligne.
    (a.out / "result.json").write_text(json.dumps(res, indent=2, ensure_ascii=False) + "\n",
                                       encoding="utf-8", newline="\n")
    text = summary(a.config, res)
    (a.out / "summary.txt").write_text(text + "\n", encoding="utf-8", newline="\n")
    print(text)


if __name__ == "__main__":
    main()
