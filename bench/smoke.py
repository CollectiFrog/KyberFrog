"""Smoke test de la config K — porte de l'étape 0 du banc de latence (#28-4).

Lance, depuis un bundle Kyber, un kycontroller isolé (port dédié, config
jetable via `KYBER_CONFIG_PATH`, source = écran principal animé par une
fenêtre témoin, x264) puis un kyclient qui republie le flux en Spout, et vérifie avec `spout_probe` que le sender de
sortie existe, a une taille et avance. Tue tout en sortant.

    python bench/smoke.py --bundle <dossier-kyber> [--seconds 10] [--metrics]

Code 0 si le sender débite au moins `--min-fps`.
"""

from __future__ import annotations

import argparse
import hashlib
import os
import subprocess
import sys
import tempfile
import time
from pathlib import Path

import spout_probe

USER, PASSWORD = "vj", "kyberfrog"  # login transparent de KyberFrog (shared/src/lib.rs)

CONFIG = """\
# Smoke test du banc de latence (#28-4) — instance isolée, source écran.
[kyavserver]
encoder = "x264"

[kycontroller]
port = {port}
tray = false

[[kycontroller.auth.basic.logins]]
username = "{user}"
password = "{hash}"
"""


# La capture d'écran (DXGI Desktop Duplication) ne produit une frame que si
# l'écran change : un bureau immobile débite 0 fps. Une fenêtre qui change de
# couleur à chaque rafraîchissement garantit un flux continu.
ANIMATOR = """
import tkinter as tk
root = tk.Tk(); root.geometry("640x360+100+100"); root.attributes("-topmost", True)
label = tk.Label(root, font=("Consolas", 64)); label.pack(fill="both", expand=True)
n = 0
def tick():
    global n
    n += 1
    label.config(text=str(n), bg="#%06x" % ((n * 2654435761) & 0xFFFFFF))
    root.after(8, tick)
tick(); root.mainloop()
"""


def wait_sender(name, timeout):
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        if name in spout_probe.senders():
            return True
        time.sleep(0.5)
    return False


def main():
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--bundle", type=Path, required=True)
    parser.add_argument("--port", type=int, default=9150)
    parser.add_argument("--sender", default="bench-out")
    parser.add_argument("--seconds", type=float, default=10.0)
    parser.add_argument("--min-fps", type=float, default=50.0)
    parser.add_argument("--metrics", action="store_true",
                        help="kyclient --metrics true (metrics.json dans le dossier de travail)")
    parser.add_argument("--workdir", type=Path,
                        help="dossier de travail (config, logs, metrics.json) ; temporaire par défaut")
    args = parser.parse_args()

    if args.sender in spout_probe.senders():
        print(f"un sender « {args.sender} » existe déjà : arrêter son émetteur d'abord")
        return 2

    work = args.workdir or Path(tempfile.mkdtemp(prefix="kybench-smoke-"))
    work.mkdir(parents=True, exist_ok=True)
    config = work / "kyber_config.toml"
    config.write_text(CONFIG.format(port=args.port, user=USER,
                                    hash=hashlib.sha256(PASSWORD.encode()).hexdigest()),
                      encoding="utf-8")
    bundle = args.bundle.resolve()
    print(f"dossier de travail : {work}")

    procs = []
    try:
        procs.append(subprocess.Popen([sys.executable, "-c", ANIMATOR]))
        with open(work / "kycontroller.log", "wb") as log:
            procs.append(subprocess.Popen(
                [str(bundle / "kycontroller.exe")], cwd=bundle, stdout=log,
                stderr=subprocess.STDOUT, env={**os.environ, "KYBER_CONFIG_PATH": str(config)}))
        time.sleep(3)
        client = [str(bundle / "kyclient.exe"), "--port", str(args.port),
                  "--tls-skip-verification", "--auth-username", USER,
                  "--auth-password", PASSWORD, "--spout-out", args.sender,
                  "--inputs", "false", "--audio", "false", "--keyboard-grab", "false",
                  "--metrics", "true" if args.metrics else "false", "127.0.0.1"]
        with open(work / "kyclient.stdout.log", "wb") as log:
            procs.append(subprocess.Popen(client, cwd=work, stdout=log,
                                          stderr=subprocess.STDOUT))

        if not wait_sender(args.sender, 60):
            print(f"sender « {args.sender} » absent après 60 s")
            return 1
        # Laisse passer le backlog de démarrage (4 à 6 s au premier smoke test).
        time.sleep(8)
        info = spout_probe.sender_info(args.sender)
        counter = spout_probe.FrameCounter(args.sender)
        first = counter.read()
        start = time.perf_counter()
        time.sleep(args.seconds)
        last = counter.read()
        elapsed = time.perf_counter() - start
        counter.close()
        if first is None or last is None:
            print(f"sender « {args.sender} » : compteur de frames absent")
            return 1
        fps = (last - first) / elapsed
        print(f"sender « {args.sender} » : {info}")
        print(f"débit : {fps:.2f} fps sur {elapsed:.1f} s")
        return 0 if info and info["width"] and fps >= args.min_fps else 1
    finally:
        # Arbre entier : un kill simple de kycontroller laisse kyavserver orphelin.
        for proc in reversed(procs):
            subprocess.run(["taskkill", "/T", "/F", "/PID", str(proc.pid)],
                           capture_output=True, check=False)
            proc.wait()


if __name__ == "__main__":
    sys.exit(main())
