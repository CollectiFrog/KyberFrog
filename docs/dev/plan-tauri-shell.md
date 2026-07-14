# Plan d'architecture — coquille Tauri autour du cockpit web (#21)

*Décisions arbitrées avec l'opérateur le 2026-07-09. Objectif : empaqueter
le cockpit web (React + Vite) dans une vraie appli Windows au lieu d'ouvrir
`http://localhost:7700` dans le navigateur, et supprimer la fenêtre console
visible au lancement.*

## Constats de départ (état du code)

- `kyberfrog.exe` est un **binaire console** (`#[tokio::main] async fn
  main()`, pas de `windows_subsystem`) → c'est *ça* la console visible au
  lancement, pas un choix explicite (`kyberfrog/src/main.rs`).
- Il n'y a **aucun webview embarqué** aujourd'hui : « Ouvrir dashboard »
  dans le tray fait un `ShellExecuteW("open", "http://localhost:7700")` qui
  ouvre le navigateur par défaut (`kyberfrog/src/tray/windows.rs`).
- Le **tray est écrit à la main en Win32 pur** (thread dédié,
  `Shell_NotifyIconW` + GUID fixe, gestion `TaskbarCreated`, menu `muda`).
  Fait notable : Tauri v2 utilise lui aussi `muda` en interne — les deux
  stacks sont proches.
- `web.rs` sert déjà le build React en statique (`ServeDir` sur `ui/dist` à
  côté de l'exe) **et** toute l'API JSON sur le même port 7700 →
  same-origin, pas de CORS.
- L'installeur NSIS bundle exe + `ui/dist` + binaires fork ; l'autostart
  passe par une tâche planifiée AtLogOn (`install-kyberfrog.ps1`).
- Un **proxy dev vite existe déjà** vers `localhost:7700`
  (`ui/vite.config.ts`) mais incomplet : il manque `/cameras`,
  `/emission/send-all`, `/setups*`, `/prefs`.

> Note de cohérence : `CLAUDE.md` conditionne encore #21 à « l'étape 2
> (Spout output) » ; or #8 est shipped et `IMPROVEMENTS.md` (plus à jour)
> le conditionne à **#17** (remote desktop). Le vrai garde-fou est #17 —
> corriger CLAUDE.md au lancement du chantier.

## Décision d'architecture centrale

**Tauri = coquille native autour du serveur existant — jamais un pipeline
d'assets.** La fenêtre Tauri pointe sur une URL externe dans *tous* les cas :

```rust
#[cfg(debug_assertions)]
let url = "http://localhost:5173".to_string();   // vite dev, HMR
#[cfg(not(debug_assertions))]
let url = format!("http://localhost:{web_port}/"); // axum, build statique
```

- **Zéro changement** dans `ui/src/api.ts` ni `web.rs` : same-origin
  conservé, l'UI reste servie par axum en prod.
- **Ne pas utiliser `devUrl`/`frontendDist`** de `tauri.conf.json` : ce
  mécanisme charge l'UI via le protocole interne (`tauri://localhost`) en
  prod → cross-origin avec l'API JSON, exactement le problème qu'on évite.
  Un seul principe qui tient en dev comme en prod : Tauri = fenêtre.
- Alternative écartée : assets via protocole Tauri + API par IPC `invoke` —
  réécrit `api.ts`, ajoute une surface de bugs, contredit « aucune
  réécriture d'UI ».

**Le tray existant reste tel quel en v1.** Ne pas migrer vers l'API tray de
Tauri : ~670 lignes déjà durcies (orphan cleanup GUID, `TaskbarCreated`…)
contre un gain cosmétique. Piste de simplification v2 (facilitée par le
`muda` commun), pas un prérequis de #21.

## Comportement fenêtre / cycle de vie (arbitré)

- **Au démarrage, la fenêtre s'affiche** (pas de start-hidden).
- **Fermer la fenêtre ne quitte pas l'app** : `CloseRequested` →
  `prevent_default()` + `hide()` ; le tray continue de tourner.
- **Seul « Quitter » du tray arrête l'application** — même séquence d'arrêt
  qu'aujourd'hui (`discovery.shutdown()`, `manager.shutdown_all()`,
  `tray_handle.shutdown()`).
- « Ouvrir dashboard » (tray) affiche/focus la fenêtre Tauri au lieu
  d'ouvrir le navigateur.

## Plan de développement (phasé)

### Phase 0 — Spike de faisabilité (0.5–1 j)

`tauri` + `tauri-build` sur branche jetable, `tauri.conf.json` minimal, une
seule `WebviewWindow` sur `http://localhost:7700`. Valider : WebView2
présent sur les machines cibles, dashboard OK dans la fenêtre, tray custom
fonctionne en parallèle sans collision de thread. **Dans la foulée :**
compléter le proxy vite (4 routes manquantes) + `beforeDevCommand`
optionnel (`npm run dev --prefix ui`) → mode dev Tauri avec HMR quasi
gratuit. Le workflow navigateur actuel (F5, CLAUDE.md) reste valable en
parallèle.

### Phase 1 — Restructuration du bootstrap (le vrai morceau)

- `main()` ne peut plus être `#[tokio::main]` : l'event loop Tauri (`tao`)
  doit tourner sur le **thread principal**. Construire un
  `tokio::runtime::Runtime` à part et y faire tourner *tel quel* tout le
  corps actuel de `main.rs` (Manager, transmetteurs, mDNS, `web::spawn`,
  tray) depuis le hook `.setup()` de Tauri.
- Le `loop { tokio::select! }` actuel (TrayCommand + Ctrl-C) est remplacé
  par les événements Tauri qui redéclenchent la même séquence d'arrêt —
  logique inchangée, déclencheur différent.
- `TrayCommand::OpenDashboard` remplace le `ShellExecuteW`.
- Suivre le pattern cross-platform établi (`tray/{mod,windows,stub}.rs`) :
  module `shell/{windows,stub}.rs` pour que `cargo check`/tests restent
  headless sur Linux.

### Phase 2 — Packaging

- `#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]` :
  console cachée en release, gardée en dev (le dual-sink `flexi_logger`
  couvre le manque).
- **Rester sur l'installeur NSIS actuel** (décision, cf. tableau ci-dessous) :
  Tauri = simple dépendance Cargo, `build-installer.sh` change à la marge.
- Ajouter une **vérif/install WebView2 dans le `.nsi` existant** (capture
  l'essentiel du gain de `tauri build` à moindre risque) + lien fallback
  dans `INSTALL.md`.
- Auto-updater Tauri : **non** en v1 (clés de signature + serveur de
  manifeste, disproportionné pour un outil VJ en LAN de confiance).

### Phase 3 — Validation E2E

Lancement via raccourci installé (pas de flash console) → tray → « Ouvrir
dashboard » → fenêtre native fonctionnelle (round-trip ajout/suppression
transmetteur/viewer) → fermer la fenêtre ne tue pas les enfants supervisés
(Job Object cascadé à la vraie fin de process, pas à la fermeture de
fenêtre — point de vigilance spécifique) → AtLogOn démarre l'app (fenêtre
visible) → désinstallation propre.

## NSIS actuel vs `tauri build` (arbitré : NSIS en v1)

| | **Garder NSIS actuel** | **`tauri build`** |
|---|---|---|
| Bootstrap WebView2 | À ajouter à la main (quelques lignes `.nsi`) | Inclus nativement (template NSIS tauri-bundler) |
| Staging bundle fork (kycontroller/kyavserver/kyclient + DLLs + plugins VLC) | Déjà écrit, validé release après release | À reconstruire via `resources`/`externalBin` — non trivial, et il faudra *quand même* des hooks NSIS custom |
| Tâche AtLogOn, PATH > 1024 | Déjà écrit | Pas géré nativement par Tauri → hooks/template override, pas de réduction nette du NSIS maison |
| CI (#7) | Inchangée, déjà verte | À réécrire |
| Risque au moment de #21 | Faible — pipeline de release intouchée | Tauri **et** nouvelle pipeline en même temps → deux risques cumulés sur un pipeline tout juste validé (0.4.0) |
| SSOT version/metadata | Déjà gérée (git describe → `KYBERFROG_VERSION` → makensis) | Unifiable proprement, léger gain de cohérence |

**Décision :** NSIS pour livrer #21. La bascule complète vers `tauri build`
est un chantier « v2 » à part, à ne considérer que si l'intégration fenêtre
a tourné sans souci en prod pendant un moment — jamais en même temps que
#21 lui-même.
