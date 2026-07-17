# Plans d'architecture — restructurer la chaîne de forks (#24/#25)

*Propositions issues de l'[audit de la chaîne](audit-fork-chain.md)
(2026-07-07). Objectif premier : **limiter les dépendances et leur
divergence de fork**, en gardant un ensemble maintenable et compréhensible.*

Constat directeur de l'audit : la douleur ne vient pas du *volume* de code
forké (~30 commits, bien identifiés) mais de son **emballage** — 4 niveaux
de submodules imbriqués qui transforment chaque fix d'une ligne en cascade
de bumps, et 7 repos à ouvrir pour comprendre ce que le fork change.

Deux axes orthogonaux, combinables :

- **Réduire la divergence** (le *contenu*) → Plan A.
- **Changer l'emballage** (le *contenant*) → Plans B, C, (D).

---

## Plan A — Upstream-first : réduire la divergence à la source (#25)

**Quoi.** Proposer upstream (kyber.stream) les commits classés dans
l'audit, par paquets indépendants et dans cet ordre de facilité :

1. **Bugfixes purs, triviaux** : BGRA chroma (`kyctl`), scales X/Y
   (`kynput`), accumulateur fractionnaire + sources 0×0 (`kyber-desktop`).
   Chacun tient dans une MR d'une page.
2. **Série lavd/DirectShow** (`txproto`, 7 fixes) : l'iosys lavd upstream
   n'a jamais fonctionné sur hardware — série de patches à forte valeur,
   facile à défendre.
3. **Features génériques petites** : `KYBER_CONFIG_PATH` (2 commits),
   `--fullscreen` (1 commit).
4. **Features génériques grosses** (backend Spout capture, pinning/scoping,
   webcam, sortie Spout + smem vlc-rs) : à proposer une fois le contact
   établi et la confiance construite par 1–3.

**Effet.** Chaque MR acceptée retire définitivement du poids du fork. Si
1–3 passent, la divergence tombe de ~30 à ~15 commits concentrés sur 3
repos (`txproto`, `kymedia`, `kyctl`) ; si 4 passe, le fork tend vers zéro
et #24 se résout presque tout seul.

**Coût.** Faible par MR (les patches existent), mais latence externe
(réactivité des mainteneurs kyber.stream) et travail de mise en forme
(rebaser chaque patch sur upstream HEAD, tests). Aucune garantie
d'acceptation — le plan B/C reste nécessaire pour le résidu.

**Verdict.** À lancer **quoi qu'il arrive**, indépendant du choix
B/C. C'est la seule action qui réduit le coût *récurrent* de rebase.

**État (2026-07-08) — vague 1 préparée.** Comparaison faite avec upstream
0.27.x : aucun des fixes n'a été corrigé upstream entre-temps
(`iosys_lavd.c`/`iosys_common.c` intouchés depuis kyber-0.17.0,
`video_layout.rs` intouché depuis 0.27.0). Le fix BGRA `53df4ad` a été
**retiré de la vague 1** : il corrige du code ajouté par la feature
Spout-out (#8), pas du code upstream. Branches rebased créées et poussées
sur les forks kyber-frog (cherry-picks propres, zéro conflit) :

| Repo | Branche | Base upstream | Contenu |
|---|---|---|---|
| `txproto` | `fix/lavd-directshow-0.27` | `kyber-0.27.1` | 8 commits (registration + 7 fixes lavd) |
| `kynput` | `fix/xy-scale-0.27` | `0.27.0` | scales X/Y séparés + tests |
| `kyber-desktop` | `fix/fractional-mouse-deltas-0.27` | `0.27.1` | accumulateur fractionnaire |
| `kyber-desktop` | `fix/unknown-source-dims-0.27` | `0.27.1` | sources 0×0 |

Restes avant MR : (1) **build de validation** sur ces bases 0.27.x (les
cherry-picks sont propres mais upstream a refactoré kyclient entre 0.26.0
et 0.27.1 — 8 commits sur les fichiers concernés — un conflit *sémantique*
n'est pas exclu) ; (2) vérifier que les repos `kyber-frog/*` ont bien la
**relation de fork GitLab** vers `kyber.stream/*` (sinon pas de MR
cross-projet possible — il faudrait re-forker via l'UI GitLab) ; (3) MRs à
ouvrir par l'opérateur. Signal encourageant : upstream a déjà accepté le
pattern env-override (`KYBER_CONFIG` dans kynputservice 0.27.0), bon
précédent pour la vague 3 (`KYBER_CONFIG_PATH`).

---

## Plan B — Mono-repo : aplatir les 7 forks en un seul repo *(cible recommandée)*

**Quoi.** Créer un repo unique `kyber-frog/kyberfrog-fork` qui contient,
en sous-répertoires (import `git subtree`, historique préservé), tout ce
qui est réellement forké :

```
kyberfrog-fork/
├── manifest.toml         # pins des deps upstream PURES (tags exacts)
├── build/                # build-win32.sh adapté + scripts contrib
├── kyclient-app/         # ex kyber-desktop/kyclient (l'app winit)
├── kyctl/
├── kymedia/              # contrib/ inclus ; external/* → voir manifest
├── kynput/
├── txproto/
└── vlc-rs/
```

Les dépendances **upstream pures** (`kymux`, `kyutil`, `winit`, `vlc`,
`libvlcjni`, externals kynput, SDL) ne sont **plus des submodules** : le
script de checkout les clone à plat depuis `manifest.toml` (URL + tag) au
moment du build, comme `contrib/` télécharge déjà ses tarballs.
La table `[patch.crates-io]` de kysdk devient le `.cargo/config.toml` du
mono-repo (même mécanisme, un niveau au lieu de trois). `kysdk` et
`kyber-desktop` disparaissent en tant que repos.

**Règle de maintenabilité** : chaque sous-répertoire reste
**byte-identique à l'arbre upstream correspondant** hors commits kyberfrog
— pas de réorganisation interne — pour que `git log -- txproto/` liste
exactement la divergence et qu'un `git diff` contre un tag upstream reste
propre.

**Effets.**

- Cascade de bumps : **morte**. Un fix txproto = 1 commit, 1 push, 1 CI.
  `bump-fork.sh` supprimé.
- Fragilité `.gitmodules`/URLs relatives : **morte** (plus aucun submodule).
- Compréhension : `git log` du mono-repo = l'histoire complète du fork ;
  plus de bruit de bumps.
- Rebase 0.27.0 : `git subtree pull --prefix=<comp> <upstream> <tag>` par
  composant, conflits localisés ; extraction de patches pour le plan A :
  `git format-patch -- <comp>/`.
- Branches : une seule (`main`), plus de triplet main/kyberfrog-main/dev.
- CI kyberfrog : inchangée dans son principe — `KYBER_DESKTOP_REF`
  devient `KYBERFROG_FORK_REF`, le cache par SHA fonctionne pareil (un
  seul SHA couvre désormais *tout* le fork, fin des pins croisés
  désynchronisés).

**Coûts / risques.**

- Migration one-shot : import subtree ×6, adaptation des chemins de
  `build-win32.sh` et du `.cargo/config.toml`, reconstruction complète de
  validation — estimation 2 à 4 jours avec builds.
- À séquencer **après** le merge de `feat/linux-arm-support` (Romain
  Henry) pour ne pas casser une branche en vol ; son travail CI
  (matrice amd64/arm64) se transpose tel quel.
- Le rebase upstream par subtree est moins « naturel » qu'un
  `git rebase` de fork classique — discipline requise (toujours passer
  par `subtree pull`, jamais de commits mélangeant deux composants).

**Verdict.** Meilleur état final pour les critères énoncés
(compréhension, maintenance, dépendances minimales). Coût de migration
équivalent à quelques mois de friction de la cascade actuelle.

---

## Plan C — Manifest plat : garder les repos, tuer l'imbrication *(alternative légère)*

**Quoi.** Ne pas fusionner les repos, mais supprimer la *hiérarchie* :
étendre `packaging/versions.sh` (côté kyberfrog) en un manifest unique
listant **chaque** repo forké avec sa ref :

```toml
# packaging/fork-manifest.toml
[repos]
kyctl   = { url = "kyber-frog/kyctl",   ref = "<sha>" }
kymedia = { url = "kyber-frog/kymedia", ref = "<sha>" }
txproto = { url = "kyber-frog/txproto", ref = "<sha>" }
# … + deps upstream pures aux tags exacts
```

Un script de checkout (façon `repo`/`west`, ~100 lignes) clone tout **à
plat** dans `work/` ; la table `[patch.crates-io]` de kysdk est déplacée
dans kyberfrog/packaging et générée au checkout. `kysdk` disparaît ;
`kyber-desktop` se réduit à l'app kyclient (ou migre dans kyberfrog).

**Effets.**

- Cascade de bumps : morte aussi — bump = éditer 1 ligne du manifest,
  1 commit dans kyberfrog. Plus aucun commit de plomberie dans les forks.
- Submodules : morts également (le manifest remplace `.gitmodules`).
- Les repos forkés gardent leur forme de fork GitLab classique →
  **rebase upstream naturel** (`git rebase`), MRs upstream directes
  depuis le fork (synergie maximale avec le plan A).
- Compréhension : meilleure qu'aujourd'hui (un manifest lisible, plus de
  poupées russes) mais toujours 6 repos à ouvrir pour lire le code.

**Coûts / risques.**

- Script de checkout à écrire et maintenir (gestion d'erreurs, CI,
  reproductibilité) ; `build-win32.sh` à adapter aux nouveaux chemins.
- La cohérence inter-repos reste manuelle : le manifest pin des SHAs
  compatibles entre eux, c'est le même problème que les pins croisés —
  mais visible en un seul fichier au lieu de 3 niveaux.
- Estimation 1 à 2 jours.

**Verdict.** Bon rapport coût/bénéfice, migration douce, et **chemin
naturel vers B** (une fois à plat, fusionner en mono-repo est trivial).
Choisir C si l'appétit de migration est limité maintenant.

---

> **Mise à jour 2026-07-08** : une cinquième voie — rupture greenfield
> (intégration directe des dépendances bas niveau, suppression de la
> chaîne de forks) — est analysée dans
> [archi-greenfield.md](archi-greenfield.md). Si elle est retenue, le
> plan B est à **geler** (voir « Conséquences » dans ce doc).

## Plan B′ — kyberfrog absorbe le rôle de kyber-desktop *(raffinement de B, 2026-07-08)*

Question opérateur : « le cockpit peut-il fonctionner sans kyber-desktop
pour compiler les dépendances ? peut-on intégrer dans kyberfrog ce que
kyber-desktop intègre ? ». Réponse : oui — kyber-desktop n'apporte que
**trois choses** : l'app viewer `kyclient` (7 406 lignes Rust), le script
`build-win32.sh` (235 lignes) et des pins. Le cockpit, au runtime, ne
connaît que 3 exe sur le PATH. D'où deux variantes :

**B′-light — absorber l'orchestration seulement.** kyberfrog héberge :

- `packaging/fork-manifest.toml` : les pins de **tous** les repos (forks
  kyctl/kymedia/kynput/txproto/vlc-rs/kyber-desktop + upstream purs
  kymux/kyutil/winit/vlc…) dans un seul fichier ;
- un script de checkout (~100 lignes) + le build script adapté de
  `build-win32.sh` ;
- l'app viewer reste un fork git *rebasable* (kyber-desktop conservé).

*Précision d'architecture (objection opérateur 2026-07-08, retenue) :
« pins à plat » ne veut **pas** dire « arborescence à plat ».* Maintenir
une arbo différente de celle de Kyber serait un passif, pas un atout :
toute divergence *structurelle* (supprimer des submodules, déplacer des
dossiers, éditer `.gitmodules` ou les chemins `[patch.crates-io]` des
`Cargo.toml`) conflicte à chaque rebase et pollue chaque MR upstream.
Donc :

- le script de checkout matérialise **exactement l'arborescence
  upstream** (`kyber-desktop/kysdk/kymedia/external/txproto/…`) en
  clonant chaque repo à l'emplacement attendu, au SHA du manifest ;
- les `.gitmodules` et gitlinks des repos restent **intacts mais
  inertes** : jamais résolus (`git submodule update` n'est plus utilisé),
  jamais bumpés. Les repos forkés restent byte-identiques à upstream
  hors commits de code — le budget de divergence est réservé au code ;
- corollaire : les commits fork de plomberie (fixes d'URLs `c41829a`,
  `20c336c`, `27f48e8`) deviennent inutiles à terme, et le plan B
  mono-repo est définitivement écarté (divergence structurelle maximale) ;
- coût assumé : dans un tree de dev, les gitlinks des parents apparaissent
  « modifiés » (ils dérivent derrière le manifest) — cosmétique ; option :
  re-bump paresseux des gitlinks à chaque release pour garder les forks
  auto-cohérents pour un cloneur extérieur.

La cascade de bumps et les pannes `.gitmodules` meurent quand même
(les gitlinks ne pilotent plus la résolution) ; `kysdk` reste cloné
(il porte la table `[patch.crates-io]` committée) mais n'est plus jamais
committé. Le bundle binaire reste identique (171 MB — on ne touche pas au
moteur). Effort ~2–3 jours.

**B′-full — absorber aussi l'app viewer.** L'app `kyclient` devient un
crate kyberfrog (hors workspace par défaut, buildé par l'étage fork).
Séduisant : les **5 commits code du fork kyber-desktop vivent tous dans
cette app** (`--fullscreen`, `--spout-out`, fix 403, dims 0×0, deltas
fractionnaires) → ils deviennent du code kyberfrog normal et le fork
kyber-desktop disparaît entièrement. **Contrepartie** : upstream fait
évoluer cette app (8 commits sur les fichiers concernés rien qu'entre
0.26.0 et 0.27.1) — on transforme un fork rebasable en **copie divergente
resynchronisée à la main** (7,4 k lignes). À réserver au cas où le viewer
kyberfrog est assumé comme produit propre (affichage passif fullscreen,
Spout-out, remote desktop VJ) qui ne suit plus l'app desktop générique.

**Verdict.** B′-light ≥ B dans tous les scénarios (mêmes bénéfices que le
mono-repo sur la cascade et les submodules, sans créer de nouveau repo,
et le moteur Kyber reste non-redondant — voir la mise en garde γ/δ dans
[archi-greenfield.md](archi-greenfield.md)).

**Arbitré 2026-07-08 : B′-full écarté.** La dynamique opérateur est de
*garder/utiliser* le viewer, pas de le recoder — absorber l'app en copie
divergente y contrevient. Précision de découpage qui a motivé la
décision : kyber-desktop ne porte que la **coquille** du viewer (fenêtre,
event loop, input) ; le **moteur** (réception, décodage, rendu) vit dans
kyctl (`kyclient.dll`, `kyvlcplayer`) + libVLC — absorber la coquille
n'aurait donc même pas fait disparaître la dépendance au moteur. Le
viewer reste un fork git rebasable, piné à plat dans le manifest
B′-light ; sa divergence app (5 commits) peut encore fondre via le plan A
(2 des 5 sont déjà sur les branches upstream-ready du 2026-07-08).

## Plan D — Série de patches (écarté)

Garder les repos upstream vierges et porter la divergence en répertoire
de `.patch` appliqués au build (modèle distro / `contrib/ffmpeg`).
**Écarté** comme modèle principal : la divergence vit dans des crates
Rust multi-fichiers en évolution active (#17 phase 2 arrive) — le cycle
« éditer → régénérer le patch → re-tester l'application » dégrade
fortement la boucle de dev, et les conflits se résolvent sans l'aide de
git. Reste pertinent ponctuellement pour du C stable (c'est déjà le
mécanisme de `contrib/ffmpeg`).

---

## Synthèse & recommandation

| Critère | A upstream | B mono-repo | C manifest | D patches |
|---|---|---|---|---|
| Tue la cascade de bumps | — | ✅ | ✅ | ✅ |
| Tue la fragilité submodules | — | ✅ | ✅ | ✅ |
| Réduit la divergence elle-même | ✅✅ | — | — | — |
| Compréhension du code fork | ✅ (moins de code) | ✅✅ (1 repo, 1 log) | ✅ (1 manifest, 6 repos) | ❌ |
| Facilité rebase upstream 0.27.0 | ✅ (moins à rebaser) | ✅ (subtree pull) | ✅✅ (rebase natif) | ⚠️ ré-application |
| Facilité contribution upstream (#25) | n/a | ✅ (format-patch) | ✅✅ (MR directe du fork) | ✅ |
| Coût de migration | faible/MR, latence externe | 2–4 j | 1–2 j | élevé |

**Arbitrage opérateur (2026-07-08)** : **plan B direct** (le plan C
intermédiaire est sauté), avec le **plan A mené en parallèle et au plus
vite**. Séquence retenue :

1. **Maintenant — Plan A vague 1** : branches bugfixes-only rebased sur
   0.27.x, prêtes (voir « État » dans le plan A ci-dessus) ; MRs upstream
   à ouvrir par l'opérateur.
2. **Après merge linux-arm — Plan B′-light** (a remplacé la fusion
   mono-repo, écartée le 2026-07-08 comme divergence structurelle
   maximale — voir plan B′) : manifest + checkout + build dans
   kyberfrog/packaging, en y intégrant le rebase 0.27.x (chantier TODO
   existant) — pinner chaque composant *déjà rebasé* sur son tag 0.27.x.
3. Les sources du fork ne sont **pas** des submodules de kyberfrog :
   kyberfrog les référence par SHA dans `packaging/fork-manifest.toml`
   (voir décision ci-dessous).

**Décision finale (2026-07-08) — statu quo outillé.** Après examen de
B′-light, l'opérateur écarte aussi la dissociation gitlinks/manifest
(« pins inertes ») : deux mécanismes de gestion coexistants = divergence
de méthode. **Retenu : la structure actuelle telle quelle** (clone
récursif de kyber-desktop, submodules = seul mécanisme de pin), avec la
cascade assumée et **outillée** :

- `packaging/fork-lint.sh` — pré-vol anti-`not our ref` : `.gitmodules`
  committé vs remotes, `fetch --prune` avant vérif de reachability,
  worktrees propres. **Implémenté et validé le 2026-07-08** (0 FAIL sur
  la chaîne réelle) ;
- `packaging/rebase-fork.sh <version>` — cascade complète avec state
  machine (`--dry-run`/`--continue`/`--abort`) : cibles résolues par
  lecture des gitlinks upstream (`ls-tree`, robuste aux renames de
  chemins), commits `deps…bump` droppés au replay puis régénérés en fin
  de cascade, branches de travail `rebase/<version>` (branches fork
  intactes), rien n'est poussé. **Implémenté, dry-run 0.27.1 validé**
  (5+1+6+8+2+10+2 commits à rejouer, cohérent avec l'audit) ;
- skill Claude `/rebase-fork` (workspace `.claude/skills/`) — procédure +
  intention de chaque commit fork (l'audit sert de base) ; résout les
  conflits que le script ne peut pas, lance les vérifs, met à jour
  `versions.sh`, produit un rapport.

Découverte du dry-run : **upstream 0.27.x a renommé `kymedia/external/`
en `kymedia/subprojects/`** — le rebase réel 0.27.x aura des conflits de
migration de chemins sur kymedia (gérés par la procédure du skill).

Le plan A reste le vrai réducteur de cascade (moins de commits fork =
rebases plus courts) ; preuve de faisabilité : les cherry-picks vague 1
sur 0.27.x sont passés sans aucun conflit. Mesures d'accompagnement :
simplifier le layout de branches (une branche fork par repo) et pinner
un SHA (pas `dev`) dans `versions.sh` pour les releases.

Décisions restantes :

- [x] Ordre : **B direct** (C sauté) + A immédiat — arbitré 2026-07-08.
  Raffiné le même jour : **B′-light** (kyberfrog porte manifest + build,
  voir plan B′) est la forme recommandée de B ; B′-full (absorption de
  l'app viewer) et γ/δ (greenfield) restent des options ouvertes
  documentées.
- [x] kyberfrog-fork en submodule de kyberfrog ? **Non** — dépendance
  build-time binaire uniquement (kyberfrog ne linke aucun crate du fork) ;
  un pin par ref/SHA dans `packaging/` donne la même reproductibilité
  sans réintroduire la fragilité submodule ni coupler les cadences de
  release des deux repos.
- [ ] Plan A : identité de contribution (compte perso ? kyber-frog ?) et
  politique de licence des contributions (AGPL/commercial dual chez Kyber).
- [ ] Sort de `kyber-desktop` : réduit à l'app kyclient, ou app absorbée
  dans le mono-repo/kyberfrog ?
