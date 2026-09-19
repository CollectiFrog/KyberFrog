"""Inventaire du poste de mesure — étape 0 du banc de latence (#28-4).

Produit un `env.json` qui fige tout ce qui peut faire bouger une mesure de
latence sur la machine : Windows, CPU/RAM, GPU et pilote, écrans, plan
d'alimentation, HAGS, Game Mode, résolution du timer, processus gênants,
runtime NDI et bundle Kyber mesuré (empreintes, révisions embarquées,
présence du zero-copy Spout).

Seul `generated_at` varie d'une exécution à l'autre : deux inventaires qui
diffèrent sur autre chose signalent un poste qui a bougé.

    python bench/inventory.py --bundle <dossier-kyber> -o env.json
    python bench/inventory.py --diff env-a.json env-b.json   # porte de l'étape 0

Windows uniquement, bibliothèque standard seule.
"""

from __future__ import annotations

import argparse
import ctypes
import ctypes.wintypes as wt
import csv
import datetime as dt
import hashlib
import io
import json
import os
import platform
import re
import subprocess
import sys
import winreg
from pathlib import Path

SCHEMA = 1

# Processus qui prennent du GPU, du CPU ou un sender Spout pendant un run.
INTERFERERS = [
    "kyberfrog", "kycontroller", "kyavserver", "kyclient",
    "arena", "avenue", "resolume", "touchdesigner", "obs64", "vmix",
    "application.ndi", "application.network", "ndi ", "spouttondi", "nditospout",
    "chrome", "msedge", "firefox", "discord", "steam", "teams",
]

# Binaires du bundle Kyber dont l'empreinte définit la config K.
BUNDLE_FILES = [
    "kycontroller.exe", "kyavserver.exe", "kyclient.exe", "kyclient.dll",
    "libtxproto-0.dll", "libvlc.dll", "libvlccore*.dll",
]


def reg(root, path, name):
    try:
        with winreg.OpenKey(root, path) as key:
            return winreg.QueryValueEx(key, name)[0]
    except OSError:
        return None


def run(cmd):
    out = subprocess.run(cmd, capture_output=True, check=False)
    return out.stdout.decode(errors="replace")


def powershell_json(script):
    text = run(["powershell", "-NoProfile", "-NonInteractive", "-Command",
                f"{script} | ConvertTo-Json -Depth 3"]).strip()
    if not text:
        return []
    data = json.loads(text)
    return data if isinstance(data, list) else [data]


def windows():
    nt = r"SOFTWARE\Microsoft\Windows NT\CurrentVersion"
    hklm = winreg.HKEY_LOCAL_MACHINE
    product = reg(hklm, nt, "ProductName") or ""
    # ProductName dit toujours « Windows 10 » sous Windows 11 : le build tranche.
    if int(reg(hklm, nt, "CurrentBuild") or 0) >= 22000:
        product = product.replace("Windows 10", "Windows 11")
    return {
        "product": product,
        "edition": reg(hklm, nt, "EditionID"),
        "display_version": reg(hklm, nt, "DisplayVersion"),
        "build": f"{reg(hklm, nt, 'CurrentBuild')}.{reg(hklm, nt, 'UBR')}",
    }


class MEMORYSTATUSEX(ctypes.Structure):
    _fields_ = [("dwLength", wt.DWORD), ("dwMemoryLoad", wt.DWORD)] + [
        (n, ctypes.c_ulonglong) for n in (
            "ullTotalPhys", "ullAvailPhys", "ullTotalPageFile", "ullAvailPageFile",
            "ullTotalVirtual", "ullAvailVirtual", "ullAvailExtendedVirtual")
    ]


def machine():
    mem = MEMORYSTATUSEX()
    mem.dwLength = ctypes.sizeof(mem)
    ctypes.windll.kernel32.GlobalMemoryStatusEx(ctypes.byref(mem))
    return {
        "cpu": (reg(winreg.HKEY_LOCAL_MACHINE,
                    r"HARDWARE\DESCRIPTION\System\CentralProcessor\0",
                    "ProcessorNameString") or "").strip(),
        "logical_cpus": os.cpu_count(),
        "ram_gib": round(mem.ullTotalPhys / 2**30),
    }


def gpus():
    return powershell_json(
        "Get-CimInstance Win32_VideoController | Sort-Object Name | Select-Object "
        "Name,DriverVersion,@{n='DriverDate';e={$_.DriverDate.ToString('yyyy-MM-dd')}}"
    )


class DISPLAY_DEVICEW(ctypes.Structure):
    _fields_ = [("cb", wt.DWORD), ("DeviceName", wt.WCHAR * 32),
                ("DeviceString", wt.WCHAR * 128), ("StateFlags", wt.DWORD),
                ("DeviceID", wt.WCHAR * 128), ("DeviceKey", wt.WCHAR * 128)]


class DEVMODEW(ctypes.Structure):
    _fields_ = [("dmDeviceName", wt.WCHAR * 32), ("dmSpecVersion", wt.WORD),
                ("dmDriverVersion", wt.WORD), ("dmSize", wt.WORD),
                ("dmDriverExtra", wt.WORD), ("dmFields", wt.DWORD),
                ("dmUnion", ctypes.c_byte * 16), ("dmColor", ctypes.c_short),
                ("dmDuplex", ctypes.c_short), ("dmYResolution", ctypes.c_short),
                ("dmTTOption", ctypes.c_short), ("dmCollate", ctypes.c_short),
                ("dmFormName", wt.WCHAR * 32), ("dmLogPixels", wt.WORD),
                ("dmBitsPerPel", wt.DWORD), ("dmPelsWidth", wt.DWORD),
                ("dmPelsHeight", wt.DWORD), ("dmDisplayFlags", wt.DWORD),
                ("dmDisplayFrequency", wt.DWORD)] + [
        (n, wt.DWORD) for n in ("dmICMMethod", "dmICMIntent", "dmMediaType",
                                "dmDitherType", "dmReserved1", "dmReserved2",
                                "dmPanningWidth", "dmPanningHeight")
    ]


def displays():
    user32 = ctypes.windll.user32
    attached, primary = 0x1, 0x4
    found, i = [], 0
    while True:
        dev = DISPLAY_DEVICEW()
        dev.cb = ctypes.sizeof(dev)
        if not user32.EnumDisplayDevicesW(None, i, ctypes.byref(dev), 0):
            break
        i += 1
        if not dev.StateFlags & attached:
            continue
        mode = DEVMODEW()
        mode.dmSize = ctypes.sizeof(mode)
        user32.EnumDisplaySettingsW(dev.DeviceName, -1, ctypes.byref(mode))  # ENUM_CURRENT_SETTINGS
        found.append({
            "device": dev.DeviceName, "adapter": dev.DeviceString,
            "primary": bool(dev.StateFlags & primary),
            "mode": f"{mode.dmPelsWidth}x{mode.dmPelsHeight}@{mode.dmDisplayFrequency}",
        })
    return found


def power_plan():
    text = run(["powercfg", "/getactivescheme"])
    m = re.search(r"([0-9a-f]{8}-[0-9a-f-]{27})\s+\((.*)\)", text)
    return {"guid": m.group(1), "name": m.group(2)} if m else {"raw": text.strip()}


def graphics_settings():
    hags = reg(winreg.HKEY_LOCAL_MACHINE,
               r"SYSTEM\CurrentControlSet\Control\GraphicsDrivers", "HwSchMode")
    game_mode = reg(winreg.HKEY_CURRENT_USER, r"Software\Microsoft\GameBar",
                    "AutoGameModeEnabled")
    return {
        "hags": {2: "on", 1: "off", None: "default"}.get(hags, f"HwSchMode={hags}"),
        "game_mode": {1: "on", 0: "off", None: "default (on)"}.get(game_mode, str(game_mode)),
        "directx_user_global": reg(winreg.HKEY_CURRENT_USER,
                                   r"Software\Microsoft\DirectX\UserGpuPreferences",
                                   "DirectXUserGlobalSettings"),
    }


def clocks():
    lo, hi, cur = wt.ULONG(), wt.ULONG(), wt.ULONG()
    # NtQueryTimerResolution(Maximum, Minimum, Current) — unités de 100 ns ;
    # « Maximum » est la période la plus longue, « Minimum » la plus fine.
    ctypes.windll.ntdll.NtQueryTimerResolution(ctypes.byref(lo), ctypes.byref(hi),
                                               ctypes.byref(cur))
    freq = ctypes.c_longlong()
    ctypes.windll.kernel32.QueryPerformanceFrequency(ctypes.byref(freq))
    return {
        "timer_resolution_ms": {"coarsest": lo.value / 1e4, "finest": hi.value / 1e4,
                                "current": cur.value / 1e4},
        "qpc_frequency_hz": freq.value,
    }


def interferers():
    names = {row[0].lower() for row in csv.reader(io.StringIO(
        run(["tasklist", "/fo", "csv", "/nh"]))) if row}
    return sorted(n for n in names if any(p in n for p in INTERFERERS))


def file_version(path):
    rows = powershell_json(f"(Get-Item -LiteralPath '{path}').VersionInfo | "
                           "Select-Object FileVersion")
    return rows[0]["FileVersion"] if rows else None


def ndi():
    runtime = os.environ.get("NDI_RUNTIME_DIR_V6")
    dll = Path(runtime, "Processing.NDI.Lib.x64.dll") if runtime else None
    return {
        "runtime_dir": runtime,
        "runtime_version": file_version(dll) if dll and dll.exists() else None,
    }


REVISION = re.compile(rb"\b\d+\.\d+\.\d+-\d+-g[0-9a-f]{7,}\b")


def bundle(root: Path | None):
    if root is None:
        return None
    files = {}
    for pattern in BUNDLE_FILES:
        matches = sorted(root.glob(pattern))
        if not matches:
            files[pattern] = None
            continue
        path = matches[0]
        data = path.read_bytes()
        files[path.name] = {
            "size": len(data),
            "sha256": hashlib.sha256(data).hexdigest(),
            "revisions": sorted({m.decode() for m in REVISION.findall(data)}),
        }
    client = root / "kyclient.dll"
    return {
        "dir": str(root.resolve()),
        "spout_zero_copy": client.exists() and b"D3D11 zero-copy" in client.read_bytes(),
        "files": files,
    }


def inventory(bundle_dir):
    return {
        "schema": SCHEMA,
        "generated_at": dt.datetime.now(dt.timezone.utc).isoformat(timespec="seconds"),
        "windows": windows(),
        "machine": machine(),
        "gpus": gpus(),
        "displays": displays(),
        "power_plan": power_plan(),
        "graphics": graphics_settings(),
        "clocks": clocks(),
        "interferers_running": interferers(),
        "ndi": ndi(),
        "kyber_bundle": bundle(bundle_dir),
        "python": platform.python_version(),
    }


def diff(a, b, path=""):
    if isinstance(a, dict) and isinstance(b, dict):
        for key in sorted(set(a) | set(b)):
            if not path and key == "generated_at":
                continue
            yield from diff(a.get(key), b.get(key), f"{path}.{key}" if path else key)
    elif a != b:
        yield f"{path}: {json.dumps(a, ensure_ascii=False)} -> {json.dumps(b, ensure_ascii=False)}"


def main():
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--bundle", type=Path, help="dossier du bundle Kyber mesuré")
    parser.add_argument("-o", "--output", type=Path, default=Path("env.json"))
    parser.add_argument("--diff", nargs=2, type=Path, metavar=("A", "B"),
                        help="compare deux inventaires (hors generated_at) ; code 1 si différents")
    args = parser.parse_args()

    if args.diff:
        a, b = (json.loads(p.read_text(encoding="utf-8")) for p in args.diff)
        changes = list(diff(a, b))
        print("\n".join(changes) if changes else "identiques (hors generated_at)")
        return 1 if changes else 0

    env = inventory(args.bundle)
    args.output.write_text(json.dumps(env, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    print(f"{args.output} écrit")
    return 0


if __name__ == "__main__":
    sys.exit(main())
