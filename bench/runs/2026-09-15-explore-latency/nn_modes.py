"""Compare les modes d'envoi NDI de kybench ndi-gen (sync/async × clock)."""
import csv, statistics, subprocess, sys, time
from pathlib import Path

sys.path.insert(0, r"C:\Users\trist\Workspace\KyberFrog_WS2\kyberfrog\bench")
from step2_gate import dist

K = r"C:\Users\trist\Workspace\KyberFrog_WS2\kyberfrog\bench\kybench\kybench.exe"
DLL = r"C:\Program Files\NDI\NDI 6 Runtime\v6\Processing.NDI.Lib.x64.dll"
out = Path(sys.argv[1]); out.mkdir(parents=True, exist_ok=True)
modes = [m.split(":") for m in sys.argv[2].split(",")]  # async:clock:color
secs = float(sys.argv[3]) if len(sys.argv) > 3 else 15

for asy, clk, color in modes:
    tag = f"async{asy}-clock{clk}-{color}"
    gen = subprocess.Popen([K, "ndi-gen", "--name", "kybench-ndi", "--duration", str(secs + 10), "--csv",
                            str(out / f"{tag}-gen.csv"), "--ndi-dll", DLL, "--async", asy, "--clock", clk],
                           stderr=open(out / f"{tag}-gen.log", "wb"))
    time.sleep(2)
    subprocess.run([K, "ndi-probe", "--source", "kybench-ndi", "--duration", str(secs), "--csv",
                    str(out / f"{tag}-probe.csv"), "--ndi-dll", DLL, "--color", color],
                   stderr=open(out / f"{tag}-probe.log", "wb"))
    gen.wait()
    g = {int(r["id"]): r for r in csv.DictReader(open(out / f"{tag}-gen.csv"))}
    p = [r for r in csv.DictReader(open(out / f"{tag}-probe.csv"))]
    ok = [r for r in p if r["decoded"] == "1" and int(r["id_a"]) in g][120:]
    ids = [int(r["id_a"]) for r in ok]
    lat = lambda k: dist([(int(r[k]) - int(g[int(r["id_a"])]["t_pub_us"])) / 1000 for r in ok])
    gg = [g[i] for i in ids]
    iv = [(int(b["t_out_us"]) - int(a["t_out_us"])) / 1000 for a, b in zip(ok, ok[1:])]
    even = [(int(r["t_up_us"]) - int(g[int(r["id_a"])]["t_pub_us"])) / 1000 for r in ok if int(r["id_a"]) % 2 == 0]
    odd = [(int(r["t_up_us"]) - int(g[int(r["id_a"])]["t_pub_us"])) / 1000 for r in ok if int(r["id_a"]) % 2 == 1]
    print(f"== {tag}: {len(p)} reçues, décodées {sum(r['decoded'] == '1' for r in p)}, "
          f"trous {sum(b - a - 1 for a, b in zip(ids, ids[1:]) if b > a + 1)}")
    print("  t_up-t_pub", lat("t_up_us"))
    print("  pair/impair p50", round(statistics.median(even), 2), round(statistics.median(odd), 2))
    print("  gen late", dist([(int(x["t_pub_us"]) - int(x["t_deadline_us"])) / 1000 for x in gg]))
    print("  send call", dist([(int(x["t_sent_us"]) - int(x["t_read_us"])) / 1000 for x in gg]))
    print("  recv interval", dist(iv))
    print("  ", open(out / f"{tag}-probe.csv.json").read().strip())
    time.sleep(2)
