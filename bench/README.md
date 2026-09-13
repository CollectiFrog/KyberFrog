# Banc de latence (#28-4)

Outils du banc de mesure KyberFrog vs NDI, Phase A. Le plan, les portes et le
protocole sont dans [docs/dev/plan-bench-latency.md](../docs/dev/plan-bench-latency.md).

Python 3.12, bibliothèque standard seule, Windows.

| Script | Rôle |
|---|---|
| `inventory.py` | `env.json` du poste et du bundle Kyber mesuré ; `--diff A B` compare deux inventaires |
| `spout_probe.py` | liste les senders Spout, lit taille/format et débit d'un sender, sans SDK |
| `smoke.py` | kycontroller + `kyclient --spout-out` depuis un bundle, vérifie le débit du sender de sortie |

```powershell
python bench\inventory.py --bundle C:\Users\trist\KyberFrog-bench\bundle-643ee0e -o env.json
python bench\smoke.py --bundle C:\Users\trist\KyberFrog-bench\bundle-643ee0e
```

`runs/<date>-<étape>/` garde les inventaires et résultats versionnés de chaque porte.
