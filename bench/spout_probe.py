"""Sonde Spout minimale — porte de l'étape 0 du banc de latence (#28-4).

Lit, sans SDK ni GPU, ce qu'un récepteur Spout voit d'un sender : sa présence
dans `SpoutSenderNames`, son bloc d'info (taille, format DXGI, share handle) et
son compteur de frames (`<nom>_Count_Semaphore`), d'où un débit mesuré. Même
lecture que le receveur C de txproto (`iosys_spout.c`).

    python bench/spout_probe.py                   # liste les senders
    python bench/spout_probe.py bench-out -s 10   # débit sur 10 s ; code 1 si rien ne passe
"""

from __future__ import annotations

import argparse
import ctypes
import ctypes.wintypes as wt
import struct
import sys
import time

k32 = ctypes.WinDLL("kernel32", use_last_error=True)
k32.OpenFileMappingA.restype = wt.HANDLE
k32.OpenFileMappingA.argtypes = [wt.DWORD, wt.BOOL, ctypes.c_char_p]
k32.MapViewOfFile.restype = ctypes.c_void_p
k32.MapViewOfFile.argtypes = [wt.HANDLE, wt.DWORD, wt.DWORD, wt.DWORD, ctypes.c_size_t]
k32.UnmapViewOfFile.argtypes = [ctypes.c_void_p]
k32.OpenSemaphoreA.restype = wt.HANDLE
k32.OpenSemaphoreA.argtypes = [wt.DWORD, wt.BOOL, ctypes.c_char_p]
k32.ReleaseSemaphore.argtypes = [wt.HANDLE, wt.LONG, ctypes.POINTER(wt.LONG)]
k32.WaitForSingleObject.argtypes = [wt.HANDLE, wt.DWORD]
k32.CloseHandle.argtypes = [wt.HANDLE]

FILE_MAP_READ = 0x0004
SYNCHRONIZE = 0x00100000
SEMAPHORE_MODIFY_STATE = 0x0002
NAME_LEN = 256
MAX_SENDERS = 64


def read_map(name: str, size: int) -> bytes | None:
    handle = k32.OpenFileMappingA(FILE_MAP_READ, False, name.encode())
    if not handle:
        return None
    try:
        view = k32.MapViewOfFile(handle, FILE_MAP_READ, 0, 0, size)
        if not view:
            return None
        try:
            return ctypes.string_at(view, size)
        finally:
            k32.UnmapViewOfFile(view)
    finally:
        k32.CloseHandle(handle)


def senders() -> list[str]:
    raw = read_map("SpoutSenderNames", NAME_LEN * MAX_SENDERS) or b""
    names = []
    for i in range(0, len(raw), NAME_LEN):
        slot = raw[i:i + NAME_LEN].split(b"\0", 1)[0]
        if not slot:
            break
        names.append(slot.decode(errors="replace"))
    return names


def sender_info(name: str) -> dict | None:
    # SharedTextureInfo : shareHandle, width, height, format, usage (uint32 chacun), puis la description.
    raw = read_map(name, 20)
    if raw is None:
        return None
    handle, width, height, fmt, usage = struct.unpack("<5I", raw)
    return {"share_handle": hex(handle), "width": width, "height": height,
            "dxgi_format": fmt, "usage": usage}


class FrameCounter:
    """Lit le compteur sans le modifier (motif du SDK : décrémenter à 0 ms, puis restaurer)."""

    def __init__(self, name: str):
        self.sem = k32.OpenSemaphoreA(SYNCHRONIZE | SEMAPHORE_MODIFY_STATE, False,
                                      f"{name}_Count_Semaphore".encode())

    def read(self) -> int | None:
        if not self.sem:
            return None
        if k32.WaitForSingleObject(self.sem, 0) != 0:  # WAIT_OBJECT_0
            return 0
        prev = wt.LONG()
        k32.ReleaseSemaphore(self.sem, 1, ctypes.byref(prev))
        return prev.value + 1

    def close(self):
        if self.sem:
            k32.CloseHandle(self.sem)


def main():
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("sender", nargs="?", help="nom du sender à sonder")
    parser.add_argument("-s", "--seconds", type=float, default=5.0)
    args = parser.parse_args()

    names = senders()
    if not args.sender:
        print("\n".join(names) if names else "(aucun sender Spout)")
        return 0

    listed = args.sender in names
    info = sender_info(args.sender)
    print(f"listé dans SpoutSenderNames : {'oui' if listed else 'non'}")
    print(f"bloc d'info : {info}")
    counter = FrameCounter(args.sender)
    first = counter.read()
    if first is None:
        print("compteur de frames : absent")
        return 0 if listed and info and info["width"] else 1
    start = time.perf_counter()
    time.sleep(args.seconds)
    last = counter.read()
    elapsed = time.perf_counter() - start
    counter.close()
    fps = (last - first) / elapsed
    print(f"frames : {last - first} en {elapsed:.2f} s -> {fps:.2f} fps")
    return 0 if listed and info and info["width"] and fps > 0 else 1


if __name__ == "__main__":
    sys.exit(main())
