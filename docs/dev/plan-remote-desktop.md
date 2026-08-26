# Remote desktop — audit et phases 2–3 (#17)

> Spec extraite d'`IMPROVEMENTS.md` le 2026-08-25, **sans une ligne de
> réécriture technique** : le contenu est celui de l'audit du 2026-07-02. L'état
> d'avancement de l'item ne vit plus ici mais dans le board
> ([backlog.md](backlog.md)) — ce document dit le *pourquoi* et le *comment*.

- **What:** la feature "remote-control viewer" est codée côté KyberFrog (#10).
  **Phase 1 livrée + validée E2E paysage → paysage le 2026-07-05** (voir
  CHANGELOG.md 0.4.0) — utilisable en configuration paysage → paysage. Restent
  ouverts : écrans **verticaux** (B1, phase 2 — cassé tant que la rotation
  n'est pas gérée), `Ctrl+Alt+F` sous keyboard grab (B5, jamais testé),
  accélération pointeur Windows non compensée (phase 3).
- **Why important:** le remote desktop est une feature attendue et
  différenciante (remote desktop over QUIC, sans outil tiers).

**Audit fait (2026-07-02)** — chemin souris tracé de bout en bout, causes racines
identifiées :

- **B1 (🔴 root cause « inversion X/Y » écrans verticaux)** : rotation non
  appliquée. La capture DXGI livre une texture **non pivotée**
  (`iosys_dxgi.c:855`) et l'encodeur n'attache la rotation qu'en **metadata**
  (`encode.c:519`) ; le chemin desktop natif (kyclient/kyvlcplayer) **ignore
  totalement cette metadata** (seuls les backends web/ws la lisent). Pendant ce
  temps l'énumération renvoie les dims **pivotées** (`DesktopCoordinates`,
  portrait = 1080×1920). Le client projette donc le curseur dans un repère
  portrait sur une image affichée paysage → clics décalés « à 90° ».
- **B2 (🟠 scale souris incohérent, 3 causes)** : (a)
  `VideoLayout::local_to_host` calcule le scale **sur width seul** et
  l'applique à X et Y (`kynput/src/video_layout.rs:147`) — **corrigé phase 1**
  (scales X/Y séparés) ; (b) deltas relatifs tronqués `f64 → as i16`
  (`winit_handler/mod.rs:251`) — **corrigé phase 1** (accumulateur
  fractionnaire) ; (c) injection `MOUSEEVENTF_MOVE` relative → l'accélération
  pointeur Windows de l'hôte s'applique, aucune compensation de résolution —
  **reste ouvert, phase 3 (P3b)**.
- **B3/B4 (🟡 mineurs)** : arrondis entiers `inject_position` ; race
  `get_virtualscreen()` déjà commentée dans le code.
- **B5 (🟡)** : Ctrl+Alt+F — pas prouvé cassé, tester le bon combo d'abord.

**Plan d'implémentation (3 phases) :**

1. **Phase 1 — quick wins kynput/kyclient** (pas de rebuild txproto) : ✅
   **livrée, embarquée en 0.4.0** — scales X/Y séparés dans
   `local_to_host`/`host_to_local`, accumulateur fractionnaire des deltas côté
   kyclient, tests unitaires `VideoLayout`. Détail : CHANGELOG.md 0.4.0.
2. **Phase 2 — rotation (fix B1, le gros morceau)** :
   - P1-A (retenu) : transpose GPU D3D11 dans txproto avant encode quand
     `rotation != IDENTITY` — un seul endroit, tous les clients corrigés,
     l'énumération (dims pivotées) devient cohérente avec les pixels.
   - Alternative si coût GPU rédhibitoire : P1-B côté client (propager la
     metadata dans le path RTP natif + rendu pivoté kyvlcplayer + transform
     souris dans VideoLayout) — plus de code, 3 crates.
   - Build fork complet ~1h30 + **validation hardware écran vertical
     obligatoire**.
3. **Phase 3 — polish** :
   - P3b : neutraliser l'accélération Windows (documenter « désactiver
     Enhance pointer precision » ou convertir relatif→absolu server-side).
   - P5 : protocole de test Ctrl+Alt+F ; si cassé, traiter le combo dans le
     hook `WH_KEYBOARD_LL` avant le forward.
   - Diag : logs `host_size/video_size/scale` au resize + exposer `rotation`
     dans `/enumerate_displays`.

- **Scope :** fork-side (kynput + kyclient + txproto). Phase 1 = build léger ;
  Phase 2 = chaîne complète ~1h30.

