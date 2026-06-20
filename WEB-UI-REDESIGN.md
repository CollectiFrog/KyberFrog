# Cadrage — Refonte de l'IHM Web (#13)

> Document de cadrage (chantier **B1** du [`TODO.md`](TODO.md)). Le *quoi/pourquoi*
> de fond reste dans [`IMPROVEMENTS.md`](IMPROVEMENTS.md) (#13, #10, #2). Ce
> fichier fixe les **décisions de conception** et le **découpage** de
> l'implémentation (**B2**). Il est volontairement hors de `docs/` (site mkdocs)
> tant que la refonte n'est pas livrée.

## 1. Objectif & périmètre

Remplacer le POC mono-fichier (`kyberfrog/src/web/index.html`, ~388 lignes inline)
par une vraie IHM, et y intégrer trois choses :

- **#13** — refonte visuelle + structurelle, design system propre, layout lisible.
- **#10** — *remote-control viewer* : case par viewer → `kyclient` **fenêtré**
  qui **forwarde clavier+souris** (grab clavier), pour piloter une régie distante
  qui émet une **capture d'écran**. Mutuellement exclusif avec `spout_out`.
- **#2** — **SSE log streaming** : logs temps réel (remplace le polling).
- Item annexe de #13 : **titre d'onglet `KyberFrog — [Hostname]`**.

Hors périmètre (restent dans le backlog) : #1 ciblage moniteur (bloqué upstream),
#3 gestion credentials dans l'UI (auth reste transparente, file-only), #16 menu
clic-droit natif kyclient.

## 2. Décisions actées (réponses opérateur, 2026-06-19)

| Sujet | Décision |
|-------|----------|
| **Stack front** | **React + Vite + React Router**, projet buildé puis **embarqué** dans `kyberfrog.exe`. |
| **Layout** | **Page unique non-scrollable** (cockpit plein écran) : **Émission à gauche, Réception à droite**. |
| **Périmètre B** | Refonte **+ #10 remote-control + #2 SSE**. |
| **Langue UI** | Français (inchangé). |

## 3. Contraintes héritées (du code / build / produit)

- **Pas de toolchain natif sur l'hôte** : tout passe par l'image Docker
  `kyber/debian-win64:local` (mingw), qui **n'a pas Node**. → le front a besoin
  d'une **étape de build Node distincte** (cf. §5).
- **Cible `x86_64-pc-windows-gnu`** ; l'exe doit rester **un seul fichier** (le
  front est donc **embarqué dans le binaire**, pas servi depuis le disque).
- **API à conserver** (réutilisée telle quelle, voir §7) : `GET /status`,
  `GET /transmitters` (**endpoint de découverte stable, ne pas casser**),
  `GET /spout-senders`, les `POST/DELETE` transmitters/viewers, `GET /logs/*`.
- **Réglages avancés = file-only par design** (auth, encoder, install dir,
  base_port, flags inputs/audio/keyboard/TLS). L'UI **n'édite que** transmitters
  et viewers. La refonte **ne change pas ce contrat** (sauf le sous-ensemble
  exposé par #10, voir §7.2).
- **Modèle existant** : `Transmitter { name, port, source: Spout|Screen }`,
  `Viewer { id, server, port, fullscreen, spout_out?, enabled }` + `status`
  (`starting|running|restarting|stopped|unknown`). Une machine peut être
  émetteur seul, récepteur seul, ou les deux.
- **`kyclient` : ordre d'arguments strict** (`[OPTIONS] [--] [STREAMER_IP]`) — la
  construction de la commande reste dans `shared::Globals::kyclient_args()`, pas
  dans le front.

## 4. Architecture de l'information & layout

Cockpit **100vh, sans scroll de page** ; chaque pane scrolle **indépendamment**.

```
┌──────────────────────────────────────────────────────────────────────┐
│ 🐸 KyberFrog — PCRegie        192.168.1.10 · ● en ligne        ⓘ      │  ← TopBar
├──────────────────────────────────┬───────────────────────────────────┤
│ ÉMISSION                    [+ ▾] │ RÉCEPTION                    [+]   │  ← en-têtes de pane
│ ┌──────────────────────────────┐ │ ┌───────────────────────────────┐ │
│ │ ● arena-totem  Spout  :9000  │ │ │ ● viewer-1  192.168.1.20:9000 │ │
│ │   running          [↻] [🗑]   │ │ │   running  plein écran        │ │
│ ├──────────────────────────────┤ │ │            [✎][▶][■][↻][🗑]    │ │
│ │ ◐ screen       Écran  :9001  │ │ ├───────────────────────────────┤ │
│ │   restarting       [↻] [🗑]   │ │ │ ● relay  →Spout "KyberFrog"   │ │
│ │                              │ │ │   running  [✎][▶][■][↻][🗑]    │ │
│ │  (scroll interne au pane)    │ │ │ ◐ régie-2  🎮 remote-control  │ │  ← #10
│ └──────────────────────────────┘ │ └───────────────────────────────┘ │
├──────────────────────────────────┴───────────────────────────────────┤
│ LOGS  [KyberFrog ▾]   ● live (SSE)  [⏸] [vider]                  [⌃]  │  ← tiroir logs
│ 12:01:03  INFO  viewer-1 connected …                                   │     (repliable)
└──────────────────────────────────────────────────────────────────────┘
```

- **TopBar** (hauteur fixe) : marque + **hostname** (alimente aussi
  `document.title = "KyberFrog — <hostname>"`), IP LAN, pastille en ligne/perdu,
  bouton ⓘ (à propos / version / liens — pas de réglages, file-only).
- **Deux panes** (CSS grid `1fr 1fr`, gap) : **Émission** (gauche) / **Réception**
  (droite). Chaque pane = en-tête (titre + bouton d'ajout) + **liste scrollable**
  de cartes. État vide = *empty state* explicite (« Aucun transmetteur… »).
- **Tiroir Logs** (bas, pleine largeur, **repliable**) : conserve la page
  non-scrollable tout en gardant les logs accessibles ; sélecteur de source
  (`app` / par transmitter / par viewer), toggle **live SSE**, pause, console
  scrollable (auto-scroll si déjà en bas).
- **Ajout** :
  - Émission : menu `[+ ▾]` → liste des **senders Spout détectés** (`/spout-senders`,
    `(actif)` mis en avant) + « Capture d'écran » ; port optionnel (auto sinon).
  - Réception : `[+]` → drawer de création viewer (cf. §6).

**Responsive** (la cible primaire = poste opérateur large) : en dessous de
~1100px, les deux panes **s'empilent** et la page redevient scrollable ; le
tiroir logs passe en pleine hauteur via sa route dédiée (cf. §6). Le « non-
scrollable » est un objectif **desktop**, pas une contrainte mobile.

## 5. Stack technique & pipeline de build

### Projet front
- Nouveau dossier **`apps/KyberFrog/ui/`** : projet **Vite + React + TypeScript**,
  **React Router** pour les vues secondaires (§6), **TanStack Query** pour le
  fetch/poll/mutations, **EventSource** natif pour les logs SSE.
- Sortie : `ui/dist/` (assets hashés).

### Embed dans l'exe
- Le crate `kyberfrog` embarque `ui/dist/` via **`rust-embed`** (embed au build
  release, lecture-disque en debug — pratique pour itérer). Axum sert :
  - les **routes API** (déclarées explicitement → priorité),
  - puis les **assets statiques**,
  - puis un **fallback SPA** → `index.html` (pour que les deep-links React Router
    fonctionnent au refresh).
- `GET /transmitters` **reste à la racine** (découverte) ; pas de namespace `/api`
  (évite de casser les autres instances). Pas de collision : les routes API sont
  spécifiques et enregistrées avant le fallback.

### Build (le point dur : Node absent de l'image mingw)
Deux étapes, **front d'abord puis exe** :

1. **Stage Node** (`node:20-alpine` en CI ; conteneur node en local) :
   `cd ui && npm ci && npm run build` → artefact `ui/dist/`.
2. **Stage mingw** (image actuelle) : `cargo build --release …` embarque le
   `ui/dist/` produit à l'étape 1.

- **CI** (`.gitlab-ci.yml`) : ajouter un job `ui-build` (stage `build`, image
  `node`, artefact `ui/dist/`) ; le job `installer` le déclare en `needs`
  (artifacts: true) pour disposer du `dist/` avant `cargo build`. Le `test`
  workspace reste inchangé.
- **Local** (CLAUDE.md « dev loop ») : documenter la commande à deux temps
  (`docker run … node …` puis `docker run … cargo …`). Pour itérer sur l'UI
  seule : **vite dev server** (`npm run dev`, port 5173) + backend kyberfrog
  (7700), avec un **proxy Vite** renvoyant `/status`, `/transmitters`,
  `/viewers`, `/logs`, `/spout-senders` vers `127.0.0.1:7700`.

## 6. Routing (React Router dans un cockpit mono-écran)

Le cockpit `/` reste **monté en permanence** ; React Router pilote des **vues
secondaires en surcouche** (deep-linkables, bouton Précédent OK) :

- `/` — cockpit (2 panes + tiroir logs).
- `/reception/new` — drawer création viewer.
- `/reception/:id` — drawer édition viewer (nom, IP:port, plein écran, **Spout
  out**, **remote control** #10 ; Appliquer/Lancer/Stop/Redémarrer/Supprimer).
- `/emission/new` — drawer ajout transmitter (Spout / Écran).
- `/logs` — tiroir logs en plein écran (utile responsive).
- `/about` — modal version/hostname/liens.

Les drawers/modals rendent dans un `<Outlet>` au-dessus du cockpit ; fermer =
revenir à `/`.

## 7. Données & API

### 7.1 Réutilisé tel quel
`GET /status` (hostname, ips, transmitters[], viewers[]), `GET /transmitters`,
`GET /spout-senders`, `POST /transmitters`, `POST /transmitters/:name/restart`,
`DELETE /transmitters/:name`, `POST /viewers`, `POST /viewers/:id`,
`POST /viewers/:id/{start,stop,restart}`, `DELETE /viewers/:id`.

### 7.2 Changements pour #10 (remote-control)
Un seul booléen, le plus simple pour l'UI :

- **`shared`** : `Viewer { …, remote_control: bool (#[serde(default)]) }`.
- **`shared::Globals::kyclient_args()`** : si `remote_control` →
  `--inputs true --keyboard-grab true` et **pas** `--fullscreen` (fenêtré, pour
  garder Ctrl+Alt+F et le chrome de fenêtre). **Mutuellement exclusif avec
  `spout_out`** : on normalise (remote_control ⇒ `spout_out = None`) ; l'UI
  empêche les deux.
- **`app.rs`** : `op_add_viewer` / `op_update_viewer` prennent `remote_control` ;
  `ViewerView` le sérialise ; `ViewerForm` (web.rs) le reçoit.
- **Tests** (chantier C2) : `kyclient_args` avec `remote_control` (force inputs+grab,
  pas de fullscreen) et l'exclusion mutuelle avec `spout_out`.
- ⚠️ **Dépendance fork (à vérifier AVANT de promettre l'E2E)** : côté émetteur,
  confirmer que `kycontroller`/`kyavserver` **servent le canal input** pour une
  source **écran** (le `--inputs` existe côté kyclient ; reste à confirmer que
  l'hôte l'accepte en retour). Risque bloquant — cf. §10.

### 7.3 Changements pour #2 (SSE)
- Nouveaux endpoints `GET /logs/stream/app`,
  `GET /logs/stream/transmitter/:name`, `GET /logs/stream/viewer/:id`
  (`text/event-stream`). Implémentation simple et cross-platform : envoyer
  d'abord les **N dernières lignes** (backfill), puis **tailer** le fichier
  (poll de la taille toutes ~500 ms, push des octets ajoutés) via
  `axum::response::sse`. Les fichiers de log sont déjà alimentés par la
  redirection stdio des enfants — la source est la même que le polling, donc
  **purement additif** (IMPROVEMENTS #2).
- Front : `EventSource` + toggle **live/pause**. On **garde** les `GET /logs/*`
  (tail one-shot) en repli (navigateur sans SSE / debug).

## 8. Design system

Reprend la palette sombre actuelle, formalisée en **tokens** (`ui/src/tokens.css`
ou thème) :

| Token | Valeur | Usage |
|-------|--------|-------|
| `--bg` | `#14161a` | fond app |
| `--panel` | `#1d2026` | cartes / panes |
| `--line` | `#2b2f37` | bordures |
| `--field` | `#0f1115` | inputs / console |
| `--fg` / `--muted` | `#e6e8ec` / `#8a909b` | texte |
| `--accent` | `#4da3ff` | actions primaires, ports |
| `--ok`/`--warn`/`--bad` | `#3ad16f`/`#f2b134`/`#ec5b5b` | états |

**États** (parité avec les glyphes du tray) : `●` running (ok), `◐` starting/
restarting (warn), `✗` stopped (bad), `?` unknown (muted).

**Composants** : `AppShell`, `TopBar`, `Pane`, `PaneHeader`, `AddMenu`,
`TransmitterCard`, `ViewerCard`, `ViewerForm` (drawer), `StatusBadge`, `Button`
(primary/ghost/danger), `TextField`/`NumberField`/`Select`/`Toggle`,
`LogDrawer`/`LogConsole`, `EmptyState`, `ConfirmDialog`, `Toast` (erreurs API).

## 9. Gestion d'état (front)

- **TanStack Query** : `useQuery(['status'])` avec `refetchInterval: 2000` ;
  `useQuery(['senders'])` à 5000 ; mutations (add/remove/restart/start/stop/
  update) → `invalidateQueries(['status'])`. Les handlers renvoient déjà le
  `StatusPayload` complet → on peut aussi `setQueryData` (mise à jour immédiate).
- **Logs** hors Query : `EventSource` par source, géré dans le `LogDrawer`.
- **Saisie préservée** : ne pas réécraser un champ en cours d'édition lors d'un
  refetch (comportement déjà présent dans le POC — à conserver via formulaires
  contrôlés isolés du flux `status`).

## 10. Risques & questions ouvertes (à trancher avant/pendant B2)

1. **#10 côté fork — canal input pour source écran** : *risque bloquant* pour le
   remote-control E2E. **Action** : vérifier `kycontroller`/`kyavserver` (repos
   fork) avant d'annoncer la fonctionnalité ; si absent → #10 devient un lot
   fork séparé et la refonte livre l'UI + le câblage prêt mais désactivé.
2. **Image de build Node** : job CI `node:20-alpine` séparé (recommandé) **ou**
   ajouter Node à l'image mingw partagée. Le job séparé évite de toucher une
   image utilisée par tout Kyber. → **à confirmer.**
3. **Embed** : `rust-embed` (recommandé) vs `include_dir`. → défaut `rust-embed`.
4. **Poids/Tauri** : garder l'UI encapsulable pour l'étape 3 (Tauri) — le même
   `ui/` se réutilisera. Aucune dépendance à un serveur axum dans les composants
   (tout passe par des fonctions d'accès API isolées).
5. **#2 tail Windows** : poll de taille de fichier (robuste, pas d'inotify) vs
   `notify` — défaut **poll 500 ms** (simple, suffisant pour des logs).

## 11. Découpage de l'implémentation (B2)

1. **Échafaudage** `ui/` (Vite+React+TS+Router+Query) + proxy dev + page cockpit
   statique branchée sur `/status` (lecture seule).
2. **Embed + service** : `rust-embed` dans `kyberfrog`, fallback SPA dans
   `web.rs`, route statique ; build release qui embarque `ui/dist/`.
3. **CI** : job `ui-build` (Node) + `needs` dans `installer` ; doc dev loop.
4. **Parité fonctionnelle** : Émission (add Spout/écran, restart, remove),
   Réception (CRUD viewer, start/stop/restart, rename), logs (tiroir).
5. **#2 SSE** : endpoints `/logs/stream/*` + `EventSource` + toggle live.
6. **#10 remote-control** : champ `remote_control` (shared/app/web) + UI
   (exclusif `spout_out`) + tests C2 ; **après** vérif fork (§10.1).
7. **Finitions** : titre d'onglet `KyberFrog — [Hostname]`, empty states, toasts
   d'erreur, responsive, à-propos.

> Lots **1→4** = la refonte livrable et testable seule ; **5** et **6** sont
> additifs et peuvent suivre. **6** est gated par le risque fork (§10.1).
