# TODO — chantiers KyberFrog

Plan de travail issu de la session du **2026-06-19**. Les *chantiers* sont à
attaquer **un par un** (on décidera lequel en premier) ; on coche au fur et à
mesure. Le backlog canonique (le *quoi/pourquoi/comment* détaillé) reste
[`IMPROVEMENTS.md`](IMPROVEMENTS.md) ; les `#N` ci-dessous y renvoient.

## ⚡ Quick wins (rapides, indépendants)

- [x] **CI — timeout `build-fork` 3h → 1h30** ([.gitlab-ci.yml](.gitlab-ci.yml)).
  Le dernier run a fini en **1h06** ; `1h30m` garde une marge confortable. ✅ fait.

## 📚 Chantier A — Documentation

**Décisions actées :** User Manual = **MkDocs Material sur GitLab Pages** ;
doc technique = **in-repo `docs/`, même site, section "Dev"**. → un seul site,
deux sections.

- [x] **A1 — Clean du README.** Scindé en **User** (présentation, install depuis
  la Release, liens vers le site) et **Dev** (archi, build, fork, release, liens
  vers les docs dev). README = point d'entrée ; le détail est sur le site. ✅
- [x] **A2 — Setup du site MkDocs + Pages.** `mkdocs.yml` (thème Material, nav
  User/Dev) + job `pages` dans `.gitlab-ci.yml` (`mkdocs build --strict`,
  default branch, `needs: []`, output `public/`). Build validé localement
  (`squidfunk/mkdocs-material`). → #11 ✅
- [x] **A3 — User Manual** (`docs/user/`) : `index` (présentation + modèle
  mental), `installation`, `getting-started`, `troubleshooting`, `faq`. → #11 ✅
- [x] **A4 — Doc technique** (`docs/dev/`) : `index`, `architecture`, `building`
  (avec le **fork build model** migré depuis IMPROVEMENTS), `releasing` (CI +
  pipeline), `contributing` ; `docs/E2E-spout-output.md` rangé dans la nav. → #12 ✅
- [ ] **A5 (à décider) — langue.** Tout est en **anglais** (cohérent repo/OSS).
  Si User Manual souhaité en **français** (VJ francophones) → traduire (rapide).
- [ ] **A6 (config GitLab) — activer Pages** sur le projet (Settings → Pages) au
  premier run du job sur `main`. Rien à coder.

## 🖥️ Chantier B — Restructuration IHM Web (Claude design) → #13

- [ ] **B1 — Cadrage / maquette** de la nouvelle UI (design system, navigation
  Émission/Réception/Logs, responsive).
- [ ] **B2 — Implémentation** de la refonte (`kyberfrog/src/web/index.html` +
  `web.rs`). Décider : HTML/JS vanilla servi par axum, ou petit front buildé.
- [ ] **B3 — Remote-control viewer** (#10) **livré dans cette refonte** :
  checkbox "remote control" par viewer → kyclient *windowed* avec
  `--inputs true --keyboard-grab true`, exclusif de `spout_out`, escape =
  Ctrl+Alt+F. Vérifier côté serveur que le canal input est servi pour une
  source écran.
- [ ] **B4 (option)** — intégrer le **SSE log streaming** (#2) pendant la refonte.

## 🧪 Chantier C — Tests & CI → #14

- [x] **C1 — Job `test`** dans `.gitlab-ci.yml` (`cargo test --workspace --locked`
  dans `$WIN64_IMAGE`, sur MR + `main`, en `needs` d'`installer` → un test rouge
  bloque le package/release). ✅ fait — **validé localement dans l'image**
  (`9 passed; 0 failed`, `--locked` OK).
- [ ] **C2 — Étoffer les tests** : `app.rs` (`resolve_port`, `resolve_viewer_id`),
  `config.rs::kyclient_args`, cas limites de `gen.rs`. (9 tests existent déjà
  dans `shared/`.)

## 📋 Backlog non planifié (reste dans IMPROVEMENTS, pas pour ce tour)

- **#1** Ciblage de sortie par moniteur — *bloqué* par un changement kyclient upstream.
- **#3** Gestion des credentials dans l'UI.
- **#8** Raffinements Spout v1 (taille native, zero-copy GPU).
- **#15** Bug — sortie plein écran kyclient (Ctrl+Alt+F). ⚠️ vérifier d'abord que
  ce n'est pas juste le mauvais raccourci (l'opérateur tapait Alt+Maj+F).
- **#16** Menu clic-droit dans la fenêtre kyclient (façon NDI Studio Monitor) :
  fermer + (re)configurer la connexion (IP/port). Recoupe #10/#13.
- **Étape 3 du plan global** : app **Tauri** (à ne lancer qu'après #13).

## Ordre conseillé

1. **Quick win timeout** — 1 min, zéro risque.
2. **Chantier C (tests)** — petit, et sécurise les refactors suivants.
3. **Chantier A (doc)** — clarifie le projet ; utile avant d'ouvrir aux contributeurs.
4. **Chantier B (IHMWeb)** — le plus gros ; bénéficie d'avoir des tests en place
   et d'absorber #10 (remote desktop) + #2 (SSE).

*(Ordre indicatif — on tranche ensemble lequel attaquer.)*
