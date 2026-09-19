# Remote desktop — causes racines et correctifs (#17)

Le viewer « remote control » (#10) prend la main sur la machine distante :
clavier et souris sont forwardés sur QUIC. **La configuration paysage → paysage
fonctionne.** Ce document trace le chemin souris et les défauts restants.

Périmètre : **fork** (kynput + kyclient + txproto).

## Causes racines

| ID | Gravité | Défaut | État |
|---|---|---|---|
| **B1** | 🔴 | **Écrans verticaux : clics décalés « à 90° ».** La capture DXGI livre une texture **non pivotée** (`iosys_dxgi.c:855`) et l'encodeur n'attache la rotation qu'en **metadata** (`encode.c:519`) ; le chemin desktop natif (kyclient/kyvlcplayer) ignore cette metadata, que seuls les backends web/ws lisent. L'énumération renvoie pourtant les dimensions **pivotées** (`DesktopCoordinates`, portrait = 1080×1920) : le client projette le curseur dans un repère portrait sur une image affichée en paysage. | phase 2 |
| **B2a** | 🟠 | Scale souris calculé sur la largeur seule, appliqué à X et Y (`kynput/src/video_layout.rs:147`). | ✅ scales X/Y séparés |
| **B2b** | 🟠 | Deltas relatifs tronqués `f64 → as i16` (`winit_handler/mod.rs:251`). | ✅ accumulateur fractionnaire |
| **B2c** | 🟠 | Injection `MOUSEEVENTF_MOVE` relative : l'accélération pointeur Windows de l'hôte s'applique, sans compensation de résolution. | phase 3 |
| **B3/B4** | 🟡 | Arrondis entiers dans `inject_position` ; race `get_virtualscreen()` déjà commentée dans le code. | mineurs |
| **B5** | 🟡 | `Ctrl+Alt+F` sous keyboard grab — jamais constaté cassé. | à tester |

Les correctifs B2a/B2b sont couverts par les tests unitaires de `VideoLayout`.

## Phase 2 — rotation (B1)

**Transpose GPU D3D11 dans txproto, avant l'encodage**, quand
`rotation != IDENTITY`.

**Pour**

- **Un seul endroit** : tous les clients sont corrigés d'un coup, natif comme
  web.
- **L'énumération devient cohérente avec les pixels** : les dimensions pivotées
  annoncées correspondent enfin à l'image transmise, sans transformation côté
  client.
- **Reste sur le GPU**, sans download.

**Contre**

- Une passe GPU par frame sur les écrans pivotés, à mesurer. Si elle s'avère
  rédhibitoire, la correction se fait côté client, au prix de 3 crates
  (propagation de la metadata dans le chemin RTP natif, rendu pivoté
  kyvlcplayer, transformation souris dans `VideoLayout`).
- Build de la chaîne complète (~1 h 30) et **validation obligatoire sur un écran
  vertical réel**.

## Phase 3 — polish

- **B2c** : neutraliser l'accélération Windows — documenter « désactiver
  *Enhance pointer precision* », ou convertir relatif → absolu côté serveur.
- **B5** : protocole de test `Ctrl+Alt+F` ; si le combo est cassé, le traiter
  dans le hook `WH_KEYBOARD_LL` avant le forward.
- **Diagnostic** : logs `host_size` / `video_size` / `scale` au resize, et
  `rotation` exposée dans `/enumerate_displays`.
