# Réorganisation du backlog — le rendre prenable par quelqu'un d'extérieur

> **État : étude, rien n'est encore migré.** Écrite le 2026-08-25 sur
> `feat/backlog-reorg` (base `dev`). Objectif : qu'un contributeur qui découvre
> le projet puisse ouvrir **une** page et se dire « je prends celui-là », sans
> avoir à décoder cinq systèmes de numérotation ni à deviner s'il a le matériel
> pour le faire.

## 1. Ce qui existe aujourd'hui

Sept surfaces décrivent le travail à faire. Aucune ne dit laquelle fait foi.

| Surface | Taille | Rôle affiché | Public réel | État |
|---|---|---|---|---|
| `README.md` § TODO / Roadmap | 10 puces | vitrine | externe | périmé (§ 2.1) |
| `TODO.md` | 243 l. | plan de travail séquencé | interne | figé au 2026-07-18 |
| `IMPROVEMENTS.md` | 719 l. / 46 Ko | backlog numéroté `#N`, *what / why / how* | interne | 13 items actifs + 160 l. d'archive |
| `docs/dev/plan-*.md` (×5) | 200–400 l. | spec + arbitrages d'un chantier | celui qui code | **sain** |
| `docs/dev/todo-linux.md` | 278 l. | tableau de bord d'un chantier | celui qui code | **sain**, et c'est le bon format (§ 3) |
| `CHANGELOG.md` | par version | ce qui est livré | externe | sain |
| Tracker GitLab | — | — | — | inutilisé : aucun `.gitlab/issue_templates`, et `TODO.md` se termine par « créer les tâches dans le tracker » |

Le problème n'est pas le volume, c'est que **trois de ces surfaces décrivent les
mêmes items** avec des états différents. La dérive n'est pas un risque, elle est
déjà là et elle est mesurable.

## 2. Ce qui casse — constaté

### 2.1 Le repo se contredit sur ce qui est livré

- `d594e6c` (« zero-copy validé E2E et livré ») touche `IMPROVEMENTS.md` et
  `docs/dev/plan-spout-zerocopy.md` — **pas `TODO.md`**. Résultat sur `dev`
  aujourd'hui : `IMPROVEMENTS.md` § 28 dit « ✅ LIVRÉ le 2026-07-18, le blocage
  libVLC 4 était périmé », pendant que `TODO.md` liste toujours « levier 2 » à
  faire **et** range `#8 zero-copy GPU` dans « Déféré », motif « nécessite
  libVLC 4 côté fork ». Deux fichiers voisins, deux réponses opposées.
- `TODO.md` § Rebase laisse trois cases décochées dont au moins une est
  démontrablement faite : `packaging/versions.sh:8` pinne
  `KYBER_DESKTOP_REF="ac3d781…"` avec le commentaire « dev @ kyber 0.27.1
  rebase (2026-07-08/09) ». Le pin existe, donc le build et les pushes ont eu
  lieu.
- `TODO.md` sur `dev`/`main` envoie encore un contributeur vers la branche
  `feat/linux-arm-support` et vers « les tâches de Romain Henry ». Cette branche
  n'existe plus ; le chantier a été **repris de zéro en amd64 seul** en août
  (`feat/linux-support`, MR !10). La correction existe — mais seulement sur la
  branche non mergée. Sur la branche par défaut, le document **oriente
  activement vers du travail mort**.
- `README.md` § Roadmap annonce le wrapper Tauri comme à venir (livré en 0.5.0)
  et « Linux/ARM support (community contribution in progress) » (faux sur les
  deux moitiés).

### 2.2 Cinq systèmes d'identifiants, un seul documenté

`IMPROVEMENTS.md` pose la règle : les `#N` sont des **identifiants stables**,
référencés par `CLAUDE.md`, les commits et les MR. Dans les faits coexistent :

| Namespace | Exemple | Où | Documenté ? |
|---|---|---|---|
| `#N` | `#27` | IMPROVEMENTS | oui |
| lettres de sous-item | `#18-B`, `C`, `D`, `E`, `F` | IMPROVEMENTS § 18, CHANGELOG | non |
| bugs d'audit | `B1`, `B2`, `B5` | IMPROVEMENTS § 17 | non |
| options de phase | `P1-A`, `P1-B`, `P3b`, `P5` | IMPROVEMENTS § 17 | non |
| phases de chantier | `P0`…`P6` | `plan-linux-amd64.md`, `todo-linux.md` | oui, localement |
| série d'audit ancienne | `C2` | TODO § Déféré | **non, et sans source** |
| leviers | « levier 2 » | IMPROVEMENTS § 28, TODO | non |

`TODO.md` écrit « valider **Ctrl+Alt+F** sous keyboard grab actif (B5/#15) » :
deux namespaces en quatre caractères, et `#15` est archivé.

Conséquence directe et publiée : `docs/dev/contributing.md` — la page
« Contributing » du site — dit à un nouveau venu « Good targets for new tests
(`IMPROVEMENTS.md` #14) ». **`#14` est archivé Shipped** (`IMPROVEMENTS.md:617`,
tests unitaires en CI, livré). L'item réellement ouvert s'appelle `C2` et vit
dans `TODO.md`. Le nouveau venu suit la référence et tombe sur du fait.

### 2.3 Rien ne dit ce qu'il faut *avoir* pour prendre un item

C'est le vrai blocage de ce projet, et il n'est écrit nulle part de façon
lisible. Les items ouverts se répartissent en quatre catégories d'**accès**, pas
de difficulté :

- ceux qu'on fait sur un portable (`#2` SSE, `#22` hover, `C2` tests, polices) ;
- ceux qui exigent la **chaîne de forks** — 7 dépôts imbriqués, ~1 h 30 de build
  (`#17` ph. 2, `#26`, `#28` leviers 1/3, V4L2) ;
- ceux qui exigent du **matériel ou l'opérateur** — écran vertical, deuxième
  machine pour le mDNS, GPU AMD, VM Linux (`#19`, `#20`, `#17` ph. 2, VAAPI) ;
- ceux qui attendent une **décision produit** et où coder serait une faute
  (`#23`, `#24`, tray Linux, kyberfrog-cast).

Aujourd'hui les quatre sont mélangés dans la même liste à puces. Un extérieur ne
peut pas savoir que `#17 phase 2` lui demandera un écran pivoté et une chaîne de
build cross-repo, ni que `#2` se fait en une soirée sans rien installer de plus.

### 2.4 Le backlog n'est pas publié, et l'entrée est cassée

`IMPROVEMENTS.md` et `TODO.md` sont à la racine du dépôt, **absents de la nav
`mkdocs.yml`**. Sur le site publié, le développeur externe lit dans
« Contributing » qu'il doit « garder le backlog honnête » et voit des `#14`,
`#3`, `#24`, `#25` — sans aucune entrée de navigation vers un backlog. Le seul
chemin est un lien sortant vers 46 Ko de markdown brut.

Ce n'est pas rattrapable par un simple lien : `60569f1` (« fix cross-tree
IMPROVEMENTS.md link breaking strict build ») montre qu'un fichier racine **ne
peut pas** être lié depuis `docs/` sous `mkdocs build --strict`. Le board devra
donc vivre **dans `docs/`**.

Vérifié aussi au passage : `mkdocs.yml:6` déclare
`site_url: https://kyber-frog.gitlab.io/kyberfrog/`, qui répond **308** vers
`https://kyber-anysource-b41fc4.gitlab.io/kyberfrog/` (domaine unique GitLab
Pages). Le `README.md` pointe déjà le bon hôte ; c'est le `site_url` — donc les
URL canoniques et le `sitemap.xml` — qui est faux.

### 2.5 Une spec de 396 lignes déguisée en entrée de backlog

Répartition réelle d'`IMPROVEMENTS.md` :

| Item | Lignes | Part |
|---|---|---|
| `#27` passthrough Spout | **396** | **55 %** |
| `#17` remote desktop | 61 | 8 % |
| `#28` latence | 57 | 8 % |
| `#22` polish UI | 51 | 7 % |
| les 8 autres items actifs | 138 | 19 % |
| archive *Shipped* | ~160 | (hors actif) |

`#27` n'est pas un item de backlog : c'est un document d'architecture complet
(décision « deux interrupteurs », 5 justifications, 3 topologies de boucle,
filtre anti-boucle à 2 couches, plan de test E2E) — plus long que
`plan-tauri-shell.md`. Il est doublé par 87 lignes dans `TODO.md`, soit **36 %
de `TODO.md`**. La convention pour ça existe déjà et a été appliquée à Tauri, au
zero-copy, au fork chain et à Linux : `docs/dev/plan-*.md`. `#27`, `#28` et
`#17` sont les trois qui n'ont jamais été extraits.

### 2.6 Langue

Le site est `language: en`, le manuel utilisateur et l'architecture sont en
anglais. `TODO.md` est 100 % français ; `IMPROVEMENTS.md` a des titres anglais
et des corps français. Le contributeur qui arrive par le site bascule de langue
au moment précis où il cherche quoi faire.

## 3. Le bon format existe déjà dans le repo

Rien à inventer. `docs/dev/todo-linux.md`, écrit en août, est déjà très proche
de ce qu'on cherche : une légende explicite (`✅ fait · 🟡 partiel · ⬜ à faire ·
➖ sans objet`), des tableaux `Sujet | État | Ce qu'il reste`, un périmètre
arbitré en tête, un bandeau de point d'arrêt daté, et la séparation nette entre
« cœur v1 » et « à porter ensuite ».

Ce qui lui manque pour servir de board général : la colonne **accès** (§ 2.3),
et le fait qu'il ne couvre qu'un chantier.

La cible, c'est donc : **généraliser `todo-linux.md`, extraire les specs en
`plan-*.md`, et supprimer les deux fichiers racine.**

## 4. Cible

### 4.1 Une surface, un rôle

| Fichier | Répond à | Public |
|---|---|---|
| `docs/dev/backlog.md` *(nouveau, dans la nav)* | « qu'est-ce que je peux prendre, maintenant ? » | **externe** |
| `docs/dev/plan-*.md` *(existe)* | « comment on le fait, et pourquoi comme ça » | celui qui prend l'item |
| `docs/dev/todo-<chantier>.md` *(existe)* | « où en est ce chantier, ligne par ligne » | celui qui code |
| `CHANGELOG.md` *(existe)* | « qu'est-ce qui est livré » | tout le monde |
| `docs/dev/backlog-archive.md` *(nouveau)* | « à quoi correspond `#14` ? » | quiconque lit un vieux commit |

`TODO.md` et `IMPROVEMENTS.md` disparaissent. Les `#N` **ne sont jamais
renumérotés** : l'archive garde une ligne par numéro livré → version du
CHANGELOG + `plan-*.md`, ce qui préserve le contrat passé avec `CLAUDE.md`, les
commits et les MR.

### 4.2 Le board

Une table unique, triable à l'œil, où la colonne `État` **est** la colonne du
kanban :

```
| ID | Sujet | État | Accès | Fini quand | Détail |
```

- **État** — `🧊 à arbitrer` · `📋 prêt` · `🚧 en cours` · `⏳ bloqué` · `✅ livré`
- **Accès** — `💻 poste seul` · `🔧 chaîne fork (~1h30, 7 dépôts)` ·
  `🎛️ matériel / opérateur` · `🧭 décision produit`

`Accès` est l'ajout qui change tout : c'est la colonne qui permet à quelqu'un de
l'extérieur de filtrer en trois secondes ce qu'il peut réellement faire.

Sous la table, **une fiche courte par item `📋 prêt`** — et seulement ceux-là :

```markdown
#### #2 — Log streaming en SSE            📋 prêt · 💻 poste seul
**Pourquoi** le polling recharge les N dernières lignes toutes les X s ; ça
suffit, mais ça saute et ça repart de zéro à chaque tick.
**Où** `kyberfrog/src/web.rs` (route à ajouter), `ui/src/api.ts` (`setInterval`
+ `fetch` → `EventSource`). L'helper de lecture des logs est réutilisé tel quel.
**Fini quand** `GET /logs/stream` pousse les nouvelles lignes en
`text/event-stream`, l'UI n'a plus de `setInterval`, et le polling reste
fonctionnel en repli.
**Pour tester** rien de spécial : `cargo test` + l'app en local, aucun matériel.
```

Six lignes. C'est ce qui manque partout aujourd'hui : *pourquoi*, *où*,
*comment je sais que c'est fini*, *qu'est-ce qu'il me faut pour tester*.

Les items `🧊`, `⏳` et `🧭` n'ont **pas** de fiche — juste la ligne de table et
un lien. On ne rédige pas une fiche « prenable » pour quelque chose qui ne se
prend pas.

## 5. Le board, rempli avec les items réels

Établi en relisant le code, `packaging/versions.sh`, l'historique et
`feat/linux-support` — pas en recopiant les deux fichiers. C'est cette table qui
permet de juger si le format tient.

### Émission / réception, cœur produit

| ID | Sujet | État | Accès | Fini quand |
|---|---|---|---|---|
| #27 | Passthrough Spout, 2 sens indépendants | 📋 prêt | 💻 + 🎛️ *(Resolume et TD sont sur le poste de dev, loopback `is_self`)* | les 3 scénarios E2E passent, extinction restaure le TOML à l'identique |
| #19 | Retester écran-seul et « Tout envoyer » | 📋 prêt | 🎛️ poste de dev | validation visuelle des 2 scénarios |
| #20 | Découverte mDNS, validation 2 machines | 📋 prêt | 🎛️ **2 machines** + install fraîche | l'émetteur apparaît chez B, disparaît au goodbye, règle NSIS UDP 5353 suffisante |
| #18-D/F | Entrée / sortie SRT-RTSP | 📋 prêt | 🔧 chaîne fork | txproto accepte une URL `rtsp://`/`srt://`, variant `Source::Url` |
| #18-C/E | Entrée / sortie NDI | ⏳ bloqué | 🔧 + dépendance libndi propriétaire | — |
| #26 | Écran source figé côté émetteur | 🧊 à arbitrer | 🔧 | besoin non exprimé, variante gardée en réserve |

### Interface

| ID | Sujet | État | Accès | Fini quand |
|---|---|---|---|---|
| #22 | Hover cohérent sur tous les boutons | 📋 prêt | 💻 | archi arrêtée le 2026-07-14, `#21` livré ne bloque plus rien |
| — | Polices servies par Google Fonts | 📋 prêt | 💻 | `ui/index.html` + `ui/dist/index.html` n'appellent plus `fonts.googleapis.com` — un LAN sans internet garde ses polices *(trouvé via `lintian` sur Linux, mais **Windows est concerné aussi**)* |
| #2 | Streaming de logs en SSE | 📋 prêt | 💻 | cf. fiche § 4.2 |
| #3 | Credentials dans l'UI | 📋 prêt | 💻 | champs optionnels qui priment sur le défaut transparent |
| #23 | Drawers → modales | 🧭 décision | — | **ne pas coder avant d'avoir tranché** |

### Chaîne fork / latence *(priorité n°1 du projet)*

| ID | Sujet | État | Accès | Fini quand |
|---|---|---|---|---|
| #28-1 | Encodeur GPU : AMF/NVENC + `zerolatency` | 📋 prêt | 🔧 + 🎛️ GPU AMD | le défaut n'est plus x264 CPU ; le patch FFmpeg est déjà dans l'arbre |
| #28-4 | **Mesurer avant d'optimiser** | 📋 prêt | 🎛️ poste de dev, **zéro code** | comparaison `Spout In TOP` Kyber vs direct dans TD ; timestamps déjà exposés (`capi.rs:1639`) |
| #28-3 | `multi_client=false` (session unique) | 🧭 décision | — | tension avec #27 : un 2ᵉ client prend un 409 |
| #28-2 | Zero-copy D3D11 en réception | ✅ livré | — | 2026-07-18, `plan-spout-zerocopy.md` |
| #17-P2 | Rotation écran vertical (transpose GPU) | ⏳ bloqué | 🎛️ **écran vertical** + 🔧 ~1h30 | root cause tracée (`iosys_dxgi.c:855`, `encode.c:519`) |
| #17-P3 | Accélération pointeur, `Ctrl+Alt+F`, logs de resize | 📋 prêt | 🔧 | — |
| #1 | Choix du moniteur de sortie | ⏳ bloqué | amont kyclient/winit | `Fullscreen::Borderless(None)` |
| #24 | Sortir de la chaîne de forks imbriqués | 🧭 décision | — | **audit livré le 2026-07-07**, plans A/B/C écrits, séquence A→C→B? — arbitrage en attente depuis |
| #25 | Réduire la divergence, remonter en amont | 📋 prêt | 🔧 + relation amont | ~15 bugfixes purs identifiés pour la 1ʳᵉ vague |
| — | Migration `KYBER_CONFIG_PATH` → `KYBER_CONFIG` | 📋 prêt | 🔧 | amont 0.27 l'implémente ; droper les 2 shims kyctl/kymedia au rebase suivant |
| — | Image docker de build `local-0.27` | 📋 prêt | 🔧 | dérivé propre au lieu du patch meson par pip |

### Linux *(sur `feat/linux-support`, MR !10 — aucun de ces items n'a de `#N`)*

| ID | Sujet | État | Accès | Fini quand |
|---|---|---|---|---|
| — | `/tmp/kyber` codé en dur | ⏳ bloqué | amont **`kyutil`, pas un de nos forks** | `$XDG_RUNTIME_DIR/kyber` en amont ; lié #25 |
| — | `libpulse` `abort()` sans serveur audio | ⏳ bloqué | 🔧 | bloquant pour un boîtier headless muet |
| — | Énumération caméra V4L2 | 📋 prêt | 🔧 | `cameras.rs` + `EnumerateDisplays` côté fork |
| — | Encodeur VAAPI + `scale=w=1920` en dur | 📋 prêt | 🎛️ machine Linux | — |
| — | Orphelins à la mort du superviseur | 📋 prêt | 🎛️ VM Linux | vérifier qu'un kill ne laisse aucun enfant |
| — | Fenêtre native et tray sous Linux | 🧭 décision | — | wry/webkit2gtk + libappindicator, ou navigateur assumé |
| — | arm64 | 🧊 hors périmètre | — | § 7 du plan Linux |

### Transverse

| ID | Sujet | État | Accès | Fini quand |
|---|---|---|---|---|
| `C2` | Étoffer les tests unitaires | 📋 prêt | 💻 | **le meilleur premier item** : `shared` est pur, compile et tourne sur l'hôte Linux, et les cibles sont déjà listées dans `contributing.md` |
| — | kyberfrog-cast | 🧭 décision | — | définir les cas d'usage **avant** toute implémentation ; le cœur E2E est prouvé |

**24 items. 13 sont `📋 prêt`, dont 5 tenables sur un simple portable** — ce qui
n'était lisible nulle part avant cette table.

## 6. Migration proposée

Cinq phases, chacune livrable seule et vérifiable.

1. **État des lieux** — corriger les 9 dérives du § 2 (zero-copy, pin 0.27.1,
   section Linux morte, roadmap README, `#14` dans `contributing.md`,
   `site_url`). Aucune restructuration. *Vérif : plus une seule affirmation du
   repo contredite par le code.*
2. **Extraction des specs** — `#27` → `docs/dev/plan-spout-passthrough.md`,
   `#28` → `docs/dev/plan-latence.md`, l'audit `#17` →
   `docs/dev/plan-remote-desktop.md`. Déplacement **à l'identique**, zéro
   réécriture technique. *Vérif : `IMPROVEMENTS.md` perd ~500 lignes sans
   qu'une phrase technique disparaisse.*
3. **Le board** — `docs/dev/backlog.md` (table § 5 + fiches des `📋`),
   `docs/dev/backlog-archive.md` (une ligne par `#N` livré), suppression de
   `TODO.md` et `IMPROVEMENTS.md`. Attribution de `#29`…`#35` aux items Linux et
   au bug des polices. *Vérif : `mkdocs build --strict` passe.*
4. **Branchement** — entrée « Backlog » dans la nav `mkdocs.yml`, § Roadmap du
   `README.md` réduite à un lien, `contributing.md` pointant le board (et non
   plus `#14`), `CLAUDE.md` § 100 mis à jour. *Vérif : depuis la page d'accueil
   du site, trois clics suffisent pour arriver à un item prenable.*
5. **Tracker (optionnel)** — ouvrir une issue GitLab **uniquement** pour les
   items réellement pris ou offerts à quelqu'un, la lier depuis le board.
   Labels miroir des colonnes (`state::*`, `access::*`).

## 7. Non fait, et à confirmer

- **Rien n'est migré.** Cette étude ne touche aucun des fichiers concernés :
  elle ajoute un seul document. Les phases 1 à 5 attendent l'arbitrage.
- **Le tracker GitLab n'a pas pu être inspecté** — pas d'authentification API
  depuis cet environnement, et pas de `glab` sur le poste (`contributing.md` le
  note). Conclu structurellement : aucun `.gitlab/issue_templates`, aucune
  référence d'issue dans l'historique, et `TODO.md` demande encore de « créer
  les tâches dans le tracker ». **À confirmer :** existe-t-il déjà des issues
  ouvertes ?
- **Le rebase 0.27.1** — le pin `versions.sh:8` prouve le build et le push ; la
  case « validation smoke E2E » reste à confirmer par toi, elle ne laisse pas de
  trace dans le dépôt kyberfrog.
- **Mermaid** — `mkdocs.yml` a `pymdownx.superfences` mais **pas** de
  `custom_fences` mermaid : tout schéma mermaid ajouté dans `docs/` rendrait en
  bloc de code sur le site (GitLab, lui, le rend). D'où l'absence de schéma
  mermaid ici. À ajouter si on veut des diagrammes dans la doc publiée.

## 8. Questions ouvertes

1. **Séquencement avec la MR !10.** `feat/linux-support` réécrit lourdement
   `TODO.md` (+64/−71). Si la réorg supprime `TODO.md` sur une branche issue de
   `dev`, le merge de !10 part en conflit. *Reco : merger !10 d'abord, puis
   réorganiser par-dessus.*
2. **Tracker.** Markdown seul / issues GitLab + board / hybride.
   *Reco : hybride — le board markdown fait foi, une issue n'est ouverte que
   pour un item réellement pris.*
3. **Langue du board.** *Reco : anglais* — c'est la pièce qui s'adresse à
   l'extérieur, et le site est déjà `language: en`. Les `plan-*.md` restent en
   français.
4. **Sort de `IMPROVEMENTS.md`.** Suppression pure avec archive d'une ligne par
   `#N`, ou conservation du fichier comme archive figée ?
   *Reco : suppression + `backlog-archive.md`.*
5. **Périmètre de la passe.** Restructuration seule, ou restructuration **et**
   remise à l'état réel (les 9 corrections) ?
   *Reco : les deux d'un coup — corriger sans restructurer laisserait la
   duplication qui a produit la dérive.*
