# Coquille native Tauri autour du cockpit (#21)

Sous Windows, le cockpit web (React + Vite) s'ouvre dans une **vraie fenêtre
d'application** (Tauri / WebView2), et aucune console n'apparaît jamais.

## Principe

**Tauri est une fenêtre autour du serveur existant, jamais un pipeline
d'assets.** La fenêtre pointe sur une URL externe dans tous les cas
(`shell/windows.rs::dashboard_url`) :

```rust
// Défaut : l'axum embarqué. KYBERFROG_UI_URL (env) la pointe sur le vite
// dev server (http://localhost:5173/) pour l'HMR.
let url = std::env::var("KYBERFROG_UI_URL")
    .unwrap_or_else(|_| format!("http://localhost:{web_port}/"));
```

- L'UI reste servie par axum, **same-origin** avec l'API JSON : zéro changement
  dans `ui/src/api.ts` ni `web.rs`.
- `devUrl` / `frontendDist` de `tauri.conf.json` ne sont **pas** utilisés : ils
  chargeraient l'UI via `tauri://localhost`, cross-origin avec l'API.
- Un build debug fonctionne sans vite lancé ; `KYBERFROG_UI_URL` active l'HMR à
  la demande.

## Cycle de vie

- **Au démarrage, la fenêtre s'affiche.**
- **Fermer la fenêtre ne quitte pas l'app** : `CloseRequested` →
  `prevent_default()` + `hide()`, le tray continue de tourner.
- **Seul « Quitter » du tray arrête l'application** (ou Ctrl-C), par la séquence
  commune `shell::shutdown` : `discovery.shutdown()`,
  `manager.shutdown_all()`, `tray_handle.shutdown()`.
- **Clic gauche sur le tray** (simple ou double) : ouvre ou met au premier plan
  la fenêtre. Menu au clic droit.

## Bootstrap

- `main()` est **synchrone** : l'event loop Tauri (`tao`) doit posséder le
  thread principal. Un `tokio::runtime::Runtime` construit à la main exécute
  `bootstrap()` (Manager, transmetteurs, viewers, mDNS, `web::spawn`, tray), puis
  la main passe à `shell::run`.
- Module `shell/{mod,windows,stub}.rs` sur le pattern cross-platform du tray :
  hors Windows, le stub fait tourner la boucle de commandes en headless.
- `TrayCommand::OpenDashboard` affiche la fenêtre.
- **`windows_subsystem = "windows"` dans tous les builds** : aucune console, les
  logs vivent dans le fichier et dans le drawer de l'UI.
- **`CREATE_NO_WINDOW` sur tous les enfants** (supervisor + énumération ffmpeg) :
  sans ce flag, chaque enfant console ouvrirait sa propre console vide sous un
  parent GUI. La console invisible se propage aux petits-enfants (kyavserver).
- **Le tray Win32 existant est conservé** (thread dédié, `Shell_NotifyIconW` +
  GUID fixe, `TaskbarCreated`, menu `muda`).

## Packaging

- **Installeur NSIS maison**, Tauri n'étant qu'une dépendance Cargo.
- Le `.nsi` vérifie et installe **WebView2** si absent.
- **`WebView2Loader.dll` est livrée à côté de `kyberfrog.exe`** : sur la cible
  windows-**gnu**, le loader ne se linke pas statiquement (lib MSVC-only),
  `webview2-com` le charge à l'exécution. `build-installer.sh` la stage
  explicitement.
- Pas d'auto-updater Tauri (clés de signature + serveur de manifeste,
  disproportionné pour un outil en LAN de confiance).

## Bilan

**Pour**

- **Aucune réécriture d'UI** et un seul chemin de chargement en dev comme en
  prod.
- **Même UI partout** : la fenêtre native et un navigateur distant sur `:7700`
  affichent exactement la même page.
- **Pipeline de release inchangé** : le staging du bundle fork, la tâche
  AtLogOn, le PATH > 1024 et la CI restent ceux de l'installeur NSIS.
- **Tray éprouvé conservé** (orphan cleanup GUID, `TaskbarCreated`…).
- **Aucune console**, ni pour KyberFrog ni pour ses enfants.

**Contre**

- **Deux stacks de fenêtrage** cohabitent : Tauri pour la fenêtre, Win32 pour le
  tray (proches, les deux utilisent `muda`).
- **WebView2 requis** sur la machine cible, et une DLL de plus à livrer.
- **Windows seulement** : Linux garde le navigateur (#34).
- Pas de mise à jour automatique.

## Pièges de build

- Une instance `kyberfrog.exe` ouverte verrouille `target/` à travers le mount
  docker : `tauri-build` échoue en « Permission denied (os error 13) ». Fermer
  l'app avant de builder.
- `generate_context!` exige une **icône PNG** : `icons/icon.png` est le bloc PNG
  256×256 extrait tel quel de `assets/kyberfrog.ico` (toutes ses entrées sont
  PNG-compressées). Ne pas le réencoder via System.Drawing, qui corrompt l'alpha.
  `tauri-build` pointe sur l'ico (`WindowsAttributes::window_icon_path`).
- Sans `WebView2Loader.dll` à côté de l'exe : boîte « WebView2Loader.dll est
  introuvable » et process zombie (le runtime tokio tourne, mDNS compris, pendant
  que le thread principal attend la boîte de dialogue). Un build dev fonctionne
  parce que le build script de `webview2-com-sys` copie la DLL dans `target/`.
- La pile tauri 2.11 / wry 0.55 / tao 0.35 cross-compile en MinGW windows-gnu,
  et `winresource` cohabite au link avec la ressource tauri-winres.

## Évolutions possibles

- **`tauri build`** à la place du NSIS maison : bootstrap WebView2 et SSOT de
  version natifs, au prix de reconstruire le staging du bundle fork
  (`resources` / `externalBin`), la tâche AtLogOn et le PATH en hooks NSIS, et
  de réécrire la CI.
- **Migration du tray vers l'API Tauri**, facilitée par le `muda` commun.
