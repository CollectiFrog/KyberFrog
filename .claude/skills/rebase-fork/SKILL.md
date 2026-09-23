---
name: rebase-fork
description: Rebase la chaîne de forks Kyber (kyber-desktop→kysdk→kyctl/kymedia/kynput→txproto/vlc-rs) sur une nouvelle version upstream kyber.stream, via kyberfrog/packaging/rebase-fork.sh + fork-lint.sh. Utiliser quand l'utilisateur demande un rebase de la chaîne, une MAJ upstream kyber ("rebase 0.27", "monter la chaîne sur 0.28"), ou après acceptation de MRs upstream pour resynchroniser les forks.
---

# Rebase de la chaîne de forks Kyber

Ce skill vit dans le repo `kyberfrog` : les commandes ci-dessous sont
relatives à sa racine. `rebase-fork.sh` prend comme `fork-root` le submodule
`vendor/kyber-desktop` s'il est initialisé (`./dev.sh setup --fork`), sinon un
`kyber-desktop` **frère** de `kyberfrog` (l'ancien layout) — sinon passer le
chemin en second argument.

## Environnement attendu

Le cas nominal est un clone vierge : `git clone -b dev …` puis
`./dev.sh setup --fork` (racine ≤ 80 caractères sous Windows). À ce stade :

- toute la chaîne est dans `vendor/kyber-desktop`, chaque sous-repo en
  **HEAD détachée** sur le pin. C'est normal : le script part de
  `origin/kyberfrog-dev`, travaille sur `rebase/<version>`, et `--abort`
  restaure la HEAD détachée d'origine. Aucune branche locale à créer ;
- `kyber/debian-win64:local-0.27` (meson ≥ 1.10) existe, dérivé par
  `setup --fork` ;
- les remotes `upstream` (kyber.stream) sont ajoutés par le script en
  **SSH** (`git@gitlab.com:…`), et le push des forks passe aussi par SSH.
  Il faut donc une clé SSH sur GitLab. Sans clé, `setup --fork` a cloné en
  HTTPS et affiché la ligne `git config --global url."https://gitlab.com/".insteadOf "git@gitlab.com:"`
  — la poser (avec l'accord de l'utilisateur, c'est de la config globale)
  avant le dry-run, sinon `fetch upstream` échoue ;
- le fichier d'état `.rebase-fork.state` est écrit à la racine du fork
  (`vendor/kyber-desktop`) : `kyberfrog` voit alors le submodule
  « modifié », c'est attendu jusqu'à la fin du run.

Côté `kyberfrog`, travailler sur une branche `feat/kyber-<version>` issue
de `dev` : c'est elle qui portera le nouveau pin (étape 6), en MR vers `dev`.

## Contexte (lire d'abord si session fraîche)

- `docs/dev/audit-fork-chain.md` — cartographie de la chaîne,
  **intention de chaque commit fork** (base de connaissance pour résoudre
  les conflits), points de douleur.
- `docs/dev/plans-fork-restructure.md` §« Décision finale » —
  pourquoi ce processus existe (statu quo outillé).

Branches fork : **`kyberfrog-dev` partout** (renommé le 2026-07-09 depuis
`dev`/`feat/remote-desktop-fix` sur kynput — nommage scopé pour ne jamais
confondre avec une branche upstream ni y baser/pousser une PR amont par
erreur). `main`/`master` = miroirs upstream purs, jamais touchés, jamais
rebasés. Le tag txproto upstream est préfixé (`kyber-0.27.1`) ; le script
essaie `<version>` puis `kyber-<version>`. Les cibles des sous-repos ne
sont PAS les derniers tags : ce sont **les gitlinks que l'upstream du
parent pinne** (le script les calcule par `ls-tree`, ne pas deviner à la
main).

## Procédure

1. **Pré-vol** : `bash packaging/fork-lint.sh`
   - FAIL URL/pin → corriger avant tout (committer `.gitmodules`, pousser
     le commit manquant). WARN « dirty » : `git stash -u` dans les repos
     concernés (saleté connue : `kyctl/Cargo.lock` régénéré par un build
     local) — le rebase refuse les vrais fichiers modifiés.
2. **Plan** : `bash packaging/rebase-fork.sh --dry-run <version>`
   - Vérifier le nombre de commits à rejouer par repo vs l'audit (§2).
     Un écart inattendu = investiguer avant de lancer.
3. **Run** : `bash packaging/rebase-fork.sh <version>`
   - Tout se fait sur des branches `rebase/<version>` ; les branches fork
     et le checkout d'origine sont restaurables par `--abort`.
   - Les commits `deps…bump` sont **droppés automatiquement** au replay
     (ils conflicteraient sur les gitlinks) puis régénérés en fin de
     cascade. Les commits `build(submodules)` (URLs `.gitmodules`) sont
     **conservés** — ils sont nécessaires au clone CI.
4. **Conflits** (le script s'arrête, exit 2) :
   - Lire l'intention du commit dans l'audit avant de résoudre.
   - Commit devenu vide/déjà upstream (MR acceptée — c'est le but du plan
     A) : `git -C <repo> rebase --skip`.
   - Candidats probables : `264e059` (fix contrib glslang/lua — peut être
     obsolète si upstream a corrigé son build, alors skip) ; les 2 commits
     hérités txproto (`6265fc9`, `3a91b1e`, auteur Anton Khirnov) — s'ils
     réapparaissent, ils sont probablement déjà dans l'historique cible → skip.
   - Puis : `bash packaging/rebase-fork.sh --continue`.
5. **Validation** (obligatoire avant tout push) :
   - Matérialiser d'abord les worktrees au nouveau pin (rapport du script,
     étape 2) : `git -C vendor/kyber-desktop submodule update --init --recursive`.
   - **Check Linux rapide** (quelques minutes, compile toute la chaîne Rust
     contre les vraies libs natives, sans stub pkg-config) :
     `packaging/linux/build-fork-local.sh -f -c` — `-f` repart propre, obligatoire
     après un rebase. Il faut l'image `kyber/debian-linux:local` : dans un env
     vierge, la tirer (`docker pull registry.gitlab.com/kyber-frog/kyberfrog/debian-linux:latest-amd64`
     puis `docker tag … kyber/debian-linux:local`), ou `-b` pour la
     reconstruire si la nouvelle version change les dépendances système.
   - **Version meson** : si la nouvelle version exige plus que ce que
     `kyber/debian-win64:local-0.27` fournit, `meson setup` échoue tôt dans
     `build-win32.sh` ; relever la contrainte dans `dev.sh` (`FORK_IMAGE`,
     et le `pip install "meson>=…"` qui la dérive), la documenter.
   - **Build Windows complet** (~1 h 30 from scratch, en arrière-plan) :

     ```sh
     MSYS_NO_PATHCONV=1 docker run --rm -v "$(cygpath -m "$PWD/vendor/kyber-desktop"):/work" \
       -w /work kyber/debian-win64:local-0.27 \
       bash -c "KYBER_STAGING_DIRECTORY=kyberfrog-fork-bundle ./build-win32.sh -p"
     ```

   - **Smoke E2E via KyberFrog**, pas seulement les binaires fork bruts — ça
     valide aussi la génération de config (`shared/gen.rs`) contre la
     nouvelle version : `./dev.sh installer -f vendor/kyber-desktop/kyberfrog-fork-bundle`,
     installer, puis un émetteur écran + un récepteur en loopback
     (`http://localhost:7700`). Pour ne pas toucher une install de prod,
     lancer l'exe avec `APPDATA` pointé ailleurs.
   - Linux amd64 : `packaging/linux/build-fork-local.sh` (sans `-c`, ~20 min)
     puis `./dev.sh deb -f <bundle>`, si une VM est disponible.
6. **Publication** — jamais sans validation ni accord utilisateur :
   - Le rapport final du script imprime les `git push --force-with-lease`
     par repo (l'utilisateur pousse, ou accord explicite). Les MR sont
     désactivées sur les forks : intégration par push direct sur
     `kyberfrog-dev`.
   - Pinner kyberfrog sur le nouveau SHA kyber-desktop, sur la branche
     `feat/kyber-<version>` : `vendor/kyber-desktop` est déjà sur
     `rebase/<version>`, donc `git add vendor/kyber-desktop` + commit
     `build(fork): pinner <sha> — rebase sur kyber <version>`. Le gitlink
     **est** le pin (`packaging/versions.sh` le lit, la CI aussi).
   - Re-lancer `fork-lint.sh` (les pins poussés doivent être reachable).
   - Pousser la branche, MR vers `dev` : le pipeline de la MR construit et
     publie les bundles win64 et linux-amd64 du nouveau SHA (~1 h 30).
   - **arm64 hors CI** : la chaîne arm64 reste rouge tant que le bundle
     n'est pas poussé — `packaging/linux/build-fork-local.sh -a arm64`
     (~4 h en émulation), à signaler à l'utilisateur plutôt qu'à lancer
     d'office.
7. **Clôture** : mettre à jour `audit-fork-chain.md` (nouvelles bases,
   commits fork restants), docs/dev/backlog.md, et la mémoire.

## Pièges connus

- **Upstream renomme parfois un chemin de submodule** (ex. 0.26→0.27.x :
  `kymedia/external/` → `kymedia/subprojects/`, découvert au dry-run
  2026-07-08). Le script s'en sort (résolution des gitlinks par nom, bump
  par `update-index`), mais attendre : (a) un conflit sur le commit qui
  édite `.gitmodules`/le gitlink à l'ancien chemin — résoudre en portant
  la redirection d'URL fork sur le nouveau chemin (commit dédié
  `build(submodules)`, **jamais** dans un commit `deps…bump`, qui sera
  droppé au prochain rebase) ; (b) après rebase, vérifier que
  `.gitmodules` du parent pointe les submodules forkés vers `kyber-frog/*`
  et les non-forkés vers l'URL upstream absolue (sinon CI `not our ref`) ;
  (c) matérialiser les nouveaux chemins avant build
  (`git -c protocol.file.allow=always submodule update --init`, en
  pointant temporairement l'URL locale du submodule vers l'ancien
  checkout pour éviter un re-clone réseau lourd, puis restaurer l'URL
  canonique) ; (d) **supprimer les anciens dossiers devenus orphelins**
  (plus dans `.gitmodules`) — sinon ils remontent en faux
  « modified: <parent> (untracked content) » en cascade jusqu'à la racine.
  Vérifier d'abord qu'ils sont bien un doublon du SHA nouvellement pinné
  (`git -C ancien-chemin log -1`) avant de les `rm -rf`.

- Ne jamais éditer les sources ffmpeg extraites (`kymedia/contrib/work/`) —
  re-extraites à chaque build ; patches via `contrib/ffmpeg/000N-*.patch`.
- `git status` des parents affiche des gitlinks « modified » tant que la
  cascade n'est pas terminée : normal, les bumps de fin la referment.
- Toute vérification de reachability sans `fetch --prune` frais ment.
- Un `docker run` tapé depuis git-bash : `MSYS_NO_PATHCONV=1` et
  `cygpath -m` sur le chemin monté (git-bash réécrit `-w /work`), ou passer
  par `./dev.sh`.
- Sous Windows, la chaîne imbrique ~175 caractères sous la racine du repo :
  un nouveau submodule upstream plus profond peut faire dépasser 260
  (`Filename too long`). `core.longpaths` est posé localement par
  `setup --fork` ; le helper HTTPS, lui, n'a pas de parade → SSH.
- Vérifier le default branch GitLab (`git remote set-head origin -a`) au
  lieu de le supposer — il est normalement déjà `main`/`master` (miroir
  upstream, protégé) et n'a pas besoin d'être changé après un rebase.
