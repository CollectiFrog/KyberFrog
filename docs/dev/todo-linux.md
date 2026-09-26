# Linux — état par fonctionnalité

*État réel du portage Linux amd64, fonctionnalité par fonctionnalité.
L'architecture et ses choix sont dans [plan-linux-amd64.md](plan-linux-amd64.md) ;
les items ouverts portent leur numéro du [board](backlog.md).*

Légende : ✅ fait · 🟡 fait, à valider · ⬜ à faire · ➖ sans objet sur Linux

## Cœur

| Sujet | État | Détail |
|---|---|---|
| Compilation de `kyberfrog` | ✅ | le job `test` compile le workspace sur l'hôte Linux ; les modules Win32 tombent sur leurs stubs |
| Compilation du fork | ✅ | bundle amd64 produit en CI et en local |
| Capture écran (`grab_backend`) | ✅ | écrit systématiquement ; `auto` résolu à chaque démarrage de transmetteur (session relue, systemd en repli) ; `xcb` validé. Override par `kyberfrog.toml` uniquement |
| Viewer (`kyclient`) | 🟡 | affiche un flux distant ; `--fullscreen` accepté par un bundle du fork compilé, mais passage réel en plein écran et `--display-idx` à vérifier (#41) |
| Supervision des enfants | ✅ | `LD_LIBRARY_PATH` + `PR_SET_PDEATHSIG` ; aucun orphelin après un `SIGKILL`, sous systemd comme lancé à la main (#40, Pi 5, 2026-09-23) |
| Chemins de config / logs | ✅ | `$XDG_CONFIG_HOME/kyberfrog`, `$XDG_STATE_HOME/kyberfrog` |
| Découverte mDNS | ✅ | annonce + découverte, sans Avahi ni règle pare-feu sur la VM ; cas général à documenter (#42) |
| Bureau à distance | ✅ | souris, clics, clavier ; `/dev/uinput` ouvert au groupe `input` par le `.deb` |
| Autostart (systemd user) | ✅ | `WantedBy=default.target` |
| Paquet `.deb` | ✅ | dépendances calculées, `lintian` propre, install → upgrade → purge validé |
| Chaîne CI Linux | ✅ | `build-fork-linux`, `deb`, `release-deb` ; le pipeline du tag v0.6.0 a attaché `kyberfrog_0.6.0_amd64.deb` à la release (#43) |
| Ouverture du dossier de logs | ✅ | `xdg-open` |
| IPC `/tmp/kyber` | ⬜ | chemin partagé codé en dur dans `kyutil` (`libkypc/.../unix.rs:38`) ; un résidu root bloque les autres utilisateurs. KyberFrog le détecte et l'explique ; correction amont `$XDG_RUNTIME_DIR/kyber` (#30) |
| Serveur audio | ⬜ | le fork ajoute toujours `pulse` à l'`api_list` : sans serveur PulseAudio/PipeWire, `libpulse` abort et tue kyavserver (#31) |
| Encodeur | ⬜ | x264 par défaut ; VAAPI et le `scale=w=1920` en dur (#33) |

## Au-delà du cœur

| Sujet | État | Détail |
|---|---|---|
| Caméra V4L2 | ✅ | pin `camera_device` côté fork en place ; énumération par `ffmpeg -sources v4l2` (nom de carte = ce que lavd hashe) ; `EnumerateDisplays` n'offre que la caméra épinglée (`kymedia` `1cd85a8`). Pin par chemin de nœud (`/dev/…`, lien suivi) et options d'ouverture (`camera_options`) pour les cartes à nœuds homonymes (Pi 5 `rp1-cfe`)  ; validé de bout en bout sur Pi 5 + C790 le 2026-09-26 (#32) — débit d'encodage : #49 |
| Fenêtre native | ⬜ | `shell/stub.rs` : dashboard au navigateur — décision #34 |
| Icône de barre des tâches | ⬜ | `tray/stub.rs` : pilotage par le web + systemd — décision #34 |
| Override du backend depuis l'UI | ⬜ | aujourd'hui `kyberfrog.toml` |
| Spout | ➖ | technologie Windows ; tuile masquée via `/status.platform` |
| « Tout envoyer » | ➖ | `all_sources` est `cfg(windows)` dans le fork ; bascule masquée |
| arm64 | ✅ | bundle fork aarch64 publié à chaque pin, `.deb` vérifié (`ARM aarch64`, rien au-dessus de `GLIBC_2.39`), installé et démarré sur un Pi 5 en Trixie (#35) ; performance x264 non mesurée (#46 S0) — [arm64](plan-linux-amd64.md#arm64) |
