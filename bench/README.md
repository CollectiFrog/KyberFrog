# Banc de latence (#28-4)

Outils du banc de mesure KyberFrog vs NDI, Phase A. Le plan, les portes et le
protocole sont dans [docs/dev/plan-bench-latency.md](../docs/dev/plan-bench-latency.md).

Python 3.12, bibliothèque standard seule, Windows.

| Script | Rôle |
|---|---|
| `inventory.py` | `env.json` du poste et du bundle Kyber mesuré ; `--diff A B` compare deux inventaires |
| `spout_probe.py` | liste les senders Spout, lit taille/format et débit d'un sender, sans SDK |
| `smoke.py` | kycontroller + `kyclient --spout-out` depuis un bundle, vérifie le débit du sender de sortie |
| `step1_gate.py` | porte de l'étape 1 : `kybench gen` → `probe` (100 % des IDs) et → `relay` → `probe` |
| `step2_gate.py` | porte de l'étape 2 (plancher de bruit) : F0 à vide, sous charge K, lecteur concurrent, étalon 50 ms / 7 ms, jitter du générateur |
| `step3_gate.py` | porte de l'étape 3 (codec d'ID sous compression) : générateur → chaîne K → sonde, débit par défaut, minimum et marge ; refuse de démarrer si une application Spout tourne (B11) |
| `kybench/` | l'instrument (Rust) : générateur Spout cadencé (timecode lisible au centre), sonde, relais-étalon, codec d'ID |

```powershell
python bench\inventory.py --bundle C:\Users\trist\KyberFrog-bench\bundle-643ee0e -o env.json
python bench\smoke.py --bundle C:\Users\trist\KyberFrog-bench\bundle-643ee0e

# kybench : build dans l'image MinGW (depuis bench\kybench), puis porte de l'étape 1
docker run --rm -v "${PWD}:/src" -v kybench-cargo-registry:/cargo/registry `
  -v kybench-target:/target -w /src kyber/debian-win64:local-0.27 bash build.sh
python bench\step1_gate.py --kybench bench\kybench\kybench.exe --out bench\runs\<date>-step1
python bench\step2_gate.py --kybench bench\kybench\kybench.exe --bundle C:\Users\trist\KyberFrog-bench\bundle-643ee0e --out bench\runs\<date>-step2
python bench\step3_gate.py --kybench bench\kybench\kybench.exe --bundle C:\Users\trist\KyberFrog-bench\bundle-643ee0e --out bench\runs\<date>-step3
```

`runs/<date>-<étape>/` garde les inventaires et résultats versionnés de chaque porte.
