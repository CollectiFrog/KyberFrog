"""NN : effet de l'alignement du fond sur la grille 16×16 (hypothèse contenu)."""
import csv, statistics, subprocess, sys, time
from pathlib import Path

sys.path.insert(0, r"C:\Users\trist\Workspace\KyberFrog_WS2\kyberfrog\bench")
from step2_gate import dist

K = r"C:\Users\trist\Workspace\KyberFrog_WS2\kyberfrog\bench\kybench\kybench.exe"
DLL = r"C:\Program Files\NDI\NDI 6 Runtime\v6\Processing.NDI.Lib.x64.dll"
out = Path(sys.argv[1]); out.mkdir(parents=True, exist_ok=True)
secs = 15
CASES = {"aligned": ["--scroll", "16", "--scroll-offset", "0"],
         "misaligned": ["--scroll", "16", "--scroll-offset", "8"],
         "default": []}

for name in sys.argv[2].split(","):
    tag = f"sc-{name}"
    gen = subprocess.Popen([K, "ndi-gen", "--name", "kybench-ndi", "--duration", str(secs + 12), "--csv",
                            str(out / f"{tag}-gen.csv"), "--ndi-dll", DLL, *CASES[name]],
                           stderr=open(out / f"{tag}-gen.log", "wb"))
    time.sleep(2)
    subprocess.run([K, "ndi-probe", "--source", "kybench-ndi", "--duration", str(secs), "--csv",
                    str(out / f"{tag}-probe.csv"), "--ndi-dll", DLL], stderr=open(out / f"{tag}-probe.log", "wb"))
    gen.wait()
    g = {int(x["id"]): x for x in csv.DictReader(open(out / f"{tag}-gen.csv"))}
    ok = [x for x in csv.DictReader(open(out / f"{tag}-probe.csv")) if x["decoded"] == "1" and int(x["id_a"]) in g][120:]
    lat = [(int(x["t_up_us"]) - int(g[int(x["id_a"])]["t_pub_us"])) / 1000 for x in ok]
    out_lat = [(int(x["t_out_us"]) - int(g[int(x["id_a"])]["t_pub_us"])) / 1000 for x in ok]
    ev = statistics.median(l for x, l in zip(ok, lat) if int(x["id_a"]) % 2 == 0)
    od = statistics.median(l for x, l in zip(ok, lat) if int(x["id_a"]) % 2 == 1)
    gg = [g[int(x["id_a"])] for x in ok]
    send = dist([(int(x["t_sent_us"]) - int(x["t_read_us"])) / 1000 for x in gg])
    late = dist([(int(x["t_pub_us"]) - int(x["t_deadline_us"])) / 1000 for x in gg])
    print(f"== {name}: n={len(ok)} décodées\n  t_out-t_pub {dist(out_lat)}\n  t_up-t_pub {dist(lat)}"
          f"\n  pair/impair {ev:.2f}/{od:.2f}\n  send {send}\n  retard gen {late}")
    time.sleep(2)
