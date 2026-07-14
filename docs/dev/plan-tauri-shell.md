# Plan d'architecture — coquille Tauri autour du cockpit web (#21)

*Décisions arbitrées avec l'opérateur le 2026-07-09. Objectif : empaqueter
le cockpit web (React + Vite) dans une vraie appli Windows au lieu d'ouvrir
`http://localhost:7700` dans le navigateur, et supprimer la fenêtre console
visible au lancement.*

## État d'avancement — ✅ livré (2026-07-15, toutes phases)

Phases 0+1 (2026-07-14) : `kyberfrog/src/shell/{mod,windows,stub}.rs`, main
sync + runtime tokio manuel, fenêtre WebView2, close = hide, clic gauche
tray = dashboard. Phase 2 (2026-07-15) : bootstrap WebView2 dans le `.nsi` +
`WebView2Loader.dll` livrée. Phase 3 : install réelle testée (bug loader
trouvé/corrigé) puis **checklist E2E validée par l'opérateur le 2026-07-15**.
Mergé `feat/tauri` → `dev`. Restent les pistes v2 non planifiées (bascule
`tauri build`, migration tray).

**Déviations vs le plan initial (assumées) :**

- La fenêtre vise le vite dev server via l'**env `KYBERFROG_UI_URL`** (défaut :
  toujours `http://localhost:<web_port>/`), pas un câblage `cfg(debug)` → 5173
  en dur — un build debug doit marcher sans vite lancé.
- `windows_subsystem = "windows"` **dans tous les builds** (pas seulement
  release) : l'opérateur ne veut jamais de console ; les logs vivent dans le
  fichier + le drawer de l'UI.
- **`CREATE_NO_WINDOW` sur tous les enfants spawnés** (supervisor + énumération
  ffmpeg) — découvert en test : une fois kyberfrog GUI, chaque enfant console
  pop-ait sa propre console vide (avant, ils héritaient silencieusement de
  celle de kyberfrog). La console invisible se propage aux petits-enfants
  (kyavserver vérifié).
- **Clic gauche tray = ouvrir/focus le dashboard** (simple ou double), menu au
  clic droit — « double-clic seulement » est infaisable proprement avec un
  menu modal sur le simple clic (le 2ᵉ clic referme le menu avant que
  `WM_LBUTTONDBLCLK` n'arrive).
- `generate_context!` exige une **icône de fenêtre PNG** : `icons/icon.png` =
  le bloc PNG 256×256 extrait tel quel de `assets/kyberfrog.ico` (toutes ses
  entrées sont PNG-compressées — ne pas réencoder via System.Drawing, il
  corrompt l'alpha). `tauri-build` pointe sur l'ico existant
  (`WindowsAttributes::window_icon_path`).

**Gotchas de build :** une instance `kyberfrog.exe` ouverte verrouille
`target/` à travers le mount docker → `tauri-build` échoue en « Permission
denied (os error 13) » ; fermer l'app avant de builder. Le spike a validé la
cross-compilation MinGW windows-gnu de toute la pile (tauri 2.11 / wry 0.55 /
tao 0.35) et la cohabitation au link de `winresource` avec la ressource
tauri-winres.

**Gotcha déploiement (trouvé au test installeur, 2026-07-15) :**
`WebView2Loader.dll` **doit être livrée à côté de `kyberfrog.exe`**. Sur la
cible windows-**gnu** le loader WebView2 ne peut pas être linké statiquement
(la lib statique est MSVC-only) : `webview2-com` la charge à l'exécution.
Symptôme sinon : boîte « WebView2Loader.dll est introuvable » au lancement,
process zombie (le runtime tokio du bootstrap tourne — logs mDNS — pendant
que le thread principal est bloqué sur la boîte de dialogue, et ni fenêtre ni
web UI). Le build dev marche « par accident » : le build script de
`webview2-com-sys` copie la DLL dans le dossier target à côté de l'exe.
`build-installer.sh` la stage désormais explicitement.

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
d'assets.** La fenêtre Tauri pointe sur une URL externe dans *tous* les cas
(implémenté dans `shell/windows.rs::dashboard_url`) :

```rust
// Défaut : l'axum embarqué. KYBERFROG_UI_URL (env) la pointe sur le vite
// dev server (http://localhost:5173/) pour l'HMR.
let url = std::env::var("KYBERFROG_UI_URL")
    .unwrap_or_else(|_| format!("http://localhost:{web_port}/"));
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

### Phase 0 — Spike de faisabilité ✅ (2026-07-14)

`tauri` + `tauri-build` sur branche jetable, `tauri.conf.json` minimal, une
seule `WebviewWindow` sur `http://localhost:7700`. Valider : WebView2
présent sur les machines cibles, dashboard OK dans la fenêtre, tray custom
fonctionne en parallèle sans collision de thread. **Dans la foulée :**
compléter le proxy vite (4 routes manquantes) + `beforeDevCommand`
optionnel (`npm run dev --prefix ui`) → mode dev Tauri avec HMR quasi
gratuit. Le workflow navigateur actuel (F5, CLAUDE.md) reste valable en
parallèle.

### Phase 1 — Restructuration du bootstrap ✅ (2026-07-14)

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

- ~~Console cachée en release seulement~~ → **fait en phase 1, et dans tous
  les builds** (voir Déviations) ; complété par `CREATE_NO_WINDOW` sur les
  enfants.
- **Rester sur l'installeur NSIS actuel** (décision, cf. tableau ci-dessous) :
  Tauri = simple dépendance Cargo, `build-installer.sh` change à la marge.
- Ajouter une **vérif/install WebView2 dans le `.nsi` existant** (capture
  l'essentiel du gain de `tauri build` à moindre risque) + lien fallback
  dans `INSTALL.md`.
- Auto-updater Tauri : **non** en v1 (clés de signature + serveur de
  manifeste, disproportionné pour un outil VJ en LAN de confiance).

### Phase 3 — Validation E2E (partiellement faite en smoke)

Déjà validé en smoke local (2026-07-14, build debug) : fenêtre native
fonctionnelle, `/status` 200, enfants sans console jusqu'à kyavserver,
cascade Job Object au kill. Reste (checklist TODO.md) : lancement via
raccourci installé (pas de flash console) → close = hide / clic gauche
tray / « Quitter » → AtLogOn démarre l'app (fenêtre visible) → viewer
remote-control sous shell GUI → désinstallation propre.

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
