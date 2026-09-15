"""NN : effet du transport NDI (config par processus via NDI_CONFIG_DIR)."""
import csv, json, os, statistics, subprocess, sys, time
from pathlib import Path

sys.path.insert(0, r"C:\Users\trist\Workspace\KyberFrog_WS2\kyberfrog\bench")
from step2_gate import dist

K = r"C:\Users\trist\Workspace\KyberFrog_WS2\kyberfrog\bench\kybench\kybench.exe"
DLL = r"C:\Program Files\NDI\NDI 6 Runtime\v6\Processing.NDI.Lib.x64.dll"
out = Path(sys.argv[1]); out.mkdir(parents=True, exist_ok=True)
secs = float(sys.argv[3]) if len(sys.argv) > 3 else 15


def cfg(tcp, rudp, unicast, adapters=()):
    return {"ndi": {"groups": {"send": "Public,", "recv": "Public,"}, "machinename": "",
                    "tcp": {"recv": {"enable": tcp}}, "rudp": {"recv": {"enable": rudp}},
                    "unicast": {"recv": {"enable": unicast}},
                    "networks": {"ips": "", "discovery": ""}, "adapters": {"allowed": list(adapters)},
                    "multicast": {"send": {"enable": False}, "recv": {"enable": False}}}}


CONFIGS = {
    "default": None,
    "tcp": cfg(True, False, False),
    "udp": cfg(False, False, True),
    "rudp": cfg(False, True, False),
    "loopback": cfg(True, True, True, ["127.0.0.1"]),
}

for name in sys.argv[2].split(","):
    env = dict(os.environ)
    if CONFIGS[name] is not None:
        d = out / f"cfg-{name}"
        d.mkdir(exist_ok=True)
        (d / "ndi-config.v1.json").write_text(json.dumps(CONFIGS[name], indent=2))
        env["NDI_CONFIG_DIR"] = str(d)
    tag = f"tr-{name}"
    gen = subprocess.Popen([K, "ndi-gen", "--name", "kybench-ndi", "--duration", str(secs + 12), "--csv",
                            str(out / f"{tag}-gen.csv"), "--ndi-dll", DLL], env=env,
                           stderr=open(out / f"{tag}-gen.log", "wb"))
    time.sleep(2)
    r = subprocess.run([K, "ndi-probe", "--source", "kybench-ndi", "--duration", str(secs), "--csv",
                        str(out / f"{tag}-probe.csv"), "--ndi-dll", DLL], env=env,
                       stderr=open(out / f"{tag}-probe.log", "wb"))
    gen.wait()
    if r.returncode != 0 or not (out / f"{tag}-probe.csv").exists():
        print(f"== {name}: échec", (out / f"{tag}-probe.log").read_text(errors="replace")[-300:])
        continue
    g = {int(x["id"]): x for x in csv.DictReader(open(out / f"{tag}-gen.csv"))}
    ok = [x for x in csv.DictReader(open(out / f"{tag}-probe.csv")) if x["decoded"] == "1" and int(x["id_a"]) in g][120:]
    lat = [(int(x["t_up_us"]) - int(g[int(x["id_a"])]["t_pub_us"])) / 1000 for x in ok]
    ev = statistics.median(l for x, l in zip(ok, lat) if int(x["id_a"]) % 2 == 0)
    od = statistics.median(l for x, l in zip(ok, lat) if int(x["id_a"]) % 2 == 1)
    iv = [(int(b["t_out_us"]) - int(a["t_out_us"])) / 1000 for a, b in zip(ok, ok[1:])]
    print(f"== {name}: n={len(ok)} t_up-t_pub {dist(lat)} pair/impair {ev:.2f}/{od:.2f} intervalle {dist(iv)}")
    print("  ", (out / f"{tag}-probe.csv.json").read_text().strip())
    time.sleep(2)
