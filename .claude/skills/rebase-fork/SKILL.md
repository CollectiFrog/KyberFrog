---
name: rebase-fork
description: Rebase la chaîne de forks Kyber (kyber-desktop→kysdk→kyctl/kymedia/kynput→txproto/vlc-rs) sur une nouvelle version upstream kyber.stream, via kyberfrog/packaging/rebase-fork.sh + fork-lint.sh. Utiliser quand l'utilisateur demande un rebase de la chaîne, une MAJ upstream kyber ("rebase 0.27", "monter la chaîne sur 0.28"), ou après acceptation de MRs upstream pour resynchroniser les forks.
---

# Rebase de la chaîne de forks Kyber

Ce skill vit dans le repo `kyberfrog` : les commandes ci-dessous sont
relatives à sa racine. Il suppose le layout `fork-root` par défaut de
`rebase-fork.sh` (`packaging/rebase-fork.sh`'s `ROOT_DEFAULT` :
`../../kyber-desktop` depuis `packaging/`, donc `kyberfrog` et
`kyber-desktop` doivent être **checkoutés côte à côte** sous un même
dossier parent) — sinon passer le chemin en second argument.

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
   - Check rapide hors Win32 : `docker run --rm -v "${PWD}:/work" -w /work
     kyber/debian-win64:local cargo check` dans les workspaces Rust touchés.
     Sans le bon prefix pkg-config ça échoue sur les libs natives (vlc-rs,
     kynput-sys, txproto-sys) — stubber des `.pc` minimaux ou pointer
     `PKG_CONFIG_LIBDIR` sur un rootfs déjà buildé pour un check propre.
   - **Version meson** : kymedia ≥ 0.27 exige meson ≥ 1.10 ; l'image
     `kyber/debian-win64:local` n'a que 1.7 → `meson setup` échoue tôt
     dans `build-win32.sh`. Dériver une image le temps que l'image ops
     (pinnée dans `.gitlab-ci.yml`, registry non récupérable) soit à jour :
     `docker build -t kyber/debian-win64:local-0.27 - <<< 'FROM
     kyber/debian-win64:local
     RUN apt-get update && apt-get install -y python3-pip &&
     python3 -m pip install --break-system-packages "meson>=1.10"'`
   - Build complet : `KYBER_STAGING_DIRECTORY=kyberfrog-fork-bundle
     ./build-win32.sh -p` depuis kyber-desktop, avec l'image ci-dessus
     (~1h30 from-scratch, lancer en arrière-plan) ; puis smoke E2E réel
     (voir mémoire kyberfrog-test-env) — idéalement via `kyberfrog.exe`
     lui-même (APPDATA overridé pour isoler de l'install de prod), pas
     seulement les binaires fork bruts : ça valide aussi la génération de
     config (`shared/gen.rs`) contre la nouvelle version.
6. **Publication** — jamais sans validation ni accord utilisateur :
   - Le rapport final du script imprime les `git push --force-with-lease`
     par repo (l'utilisateur pousse, ou accord explicite).
   - Pinner le SHA kyber-desktop résolu dans `packaging/versions.sh`
     (jamais un nom de branche flottant — le job CI `build-fork` résout
     `KYBER_DESKTOP_REF` littéralement).
   - Re-lancer `fork-lint.sh` (les pins poussés doivent être reachable).
7. **Clôture** : mettre à jour `audit-fork-chain.md` (nouvelles bases,
   commits fork restants), TODO.md (chantier rebase), et la mémoire.

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
- PowerShell pour les montages Docker (git-bash réécrit `-w /work`).
- Vérifier le default branch GitLab (`git remote set-head origin -a`) au
  lieu de le supposer — il est normalement déjà `main`/`master` (miroir
  upstream, protégé) et n'a pas besoin d'être changé après un rebase.
