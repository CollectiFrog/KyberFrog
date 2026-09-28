"""Formats et tailles de source Spout à travers toute la chaîne KyberFrog (audit C1/C2).

    python bench\\format_check.py --bundle <dossier du bundle> --out bench\\runs\\<date>-formats
        [--encoder amf] [--formats bgra8,rgb10a2,rgba16f] [--size 1280x720]

Pour chaque format : `kybench pattern` publie des barres de couleur dans ce
format, la chaîne du banc latence (kycontroller épinglé sur ce sender, puis
`kyclient --spout-out`) les transporte, et `kybench check` compare ce qui
ressort aux barres attendues. Résultat : `results.md` et `results.json` dans
`--out`, avec les logs de chaque cas à côté.

Mêmes prérequis que le banc latence (voir README) ; une autre application
Spout ouverte ne gêne pas ce test-ci, qui ne mesure pas de temps.
"""
from __future__ import annotations

import argparse
import json
import re
import subprocess
import time
from pathlib import Path

from latency_bench import SRC, K_OUT, KPipeline, kill_tree, log, wait_sender

ALL_FORMATS = "bgra8,rgba8,bgra8srgb,rgba8srgb,bgrx8,rgb10a2,rgba16f,rgba16,rgba32f,r8,r16,r16f,r32f"
KYBENCH = Path(__file__).parent / "kybench" / "kybench.exe"
LOG_LINES = re.compile(r"Unsupported sender texture format: \d+|Bound sender texture [^\r\n]+"
                       r"|Spout: converting [^\r\n]+|Init\(\) failed[^\r\n]*")


def run_case(bundle, out, fmt, size, encoder):
    work = out / f"{encoder}-{fmt}-{size}"
    work.mkdir(parents=True, exist_ok=True)
    w, h = size.split("x")
    pattern = subprocess.Popen([str(KYBENCH), "pattern", "--name", SRC, "--format", fmt,
                                "--width", w, "--height", h, "--duration", "600"],
                               stdout=open(work / "pattern.log", "wb"), stderr=subprocess.STDOUT)
    result = {"format": fmt, "size": size, "encoder": encoder}
    try:
        # A name in the registry is not enough: it can be an orphan left by a
        # killed sender. The pattern process must be the one publishing it.
        if not wait_sender(SRC, 10) or pattern.poll() is not None:
            result.update(status="setup", detail=(work / "pattern.log").read_text(errors="replace").strip())
            return result
        try:
            with KPipeline(bundle, work, encoder, port=9160, min_fps=0) as k:
                check = subprocess.run([str(KYBENCH), "check", "--name", K_OUT, "--expect", fmt,
                                        "--wait", "10"], capture_output=True, text=True)
                (work / "check.log").write_text(check.stdout + check.stderr)
                status = ("pass" if check.returncode == 0 else
                          "no picture" if not k.out_fps else "wrong colours")
                result.update(status=status,
                              out_fps=k.out_fps, detail=check.stdout.strip() or check.stderr.strip())
        except RuntimeError as e:
            result.update(status="no picture", detail=str(e))
    finally:
        kill_tree(pattern)
        controller_log = work / "kycontroller.log"
        if controller_log.exists():
            seen = LOG_LINES.findall(controller_log.read_text(errors="replace"))
            result["log"] = sorted(set(seen), key=seen.index)[:4]
    log(f"{encoder} {fmt} {size}: {result['status']}")
    return result


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--bundle", type=Path, required=True)
    ap.add_argument("--out", type=Path, required=True)
    ap.add_argument("--encoder", default="amf")
    ap.add_argument("--formats", default=ALL_FORMATS)
    ap.add_argument("--size", default="1280x720")
    a = ap.parse_args()
    a.out = a.out.resolve()  # KYBER_CONFIG_PATH must be absolute
    a.out.mkdir(parents=True, exist_ok=True)

    results = [run_case(a.bundle, a.out, fmt, a.size, a.encoder) for fmt in a.formats.split(",")]
    (a.out / "results.json").write_text(json.dumps(results, indent=2, ensure_ascii=False))
    lines = [f"# Formats Spout — {a.encoder}, {a.size}", "",
             f"Bundle : `{a.bundle}` — {time.strftime('%Y-%m-%d %H:%M')}", "",
             "| Format | Résultat | Images/s en sortie | Log kycontroller |", "|---|---|---|---|"]
    for r in results:
        fps = f"{r['out_fps']:.0f}" if r.get("out_fps") else "—"
        logs = "<br>".join(f"`{l}`" for l in r.get("log", [])) or "—"
        lines.append(f"| {r['format']} | {r['status']} | {fps} | {logs} |")
    (a.out / "results.md").write_text("\n".join(lines) + "\n", encoding="utf-8")
    print("\n".join(lines))
    return 0 if all(r["status"] == "pass" for r in results) else 1


if __name__ == "__main__":
    raise SystemExit(main())
