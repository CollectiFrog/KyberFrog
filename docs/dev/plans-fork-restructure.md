# Chaîne de forks — organisation et outillage (#24, #25)

*Comment la chaîne de forks est maintenue. Son contenu (repos, divergence
commit par commit) est dans [audit-fork-chain.md](audit-fork-chain.md).*

## Principe

**La structure de Kyber est conservée telle quelle, et son coût est outillé.**

- Clone récursif de `kyber-desktop` ; les **submodules sont le seul mécanisme de
  pin**. Aucun manifest parallèle, aucune arborescence propre à KyberFrog.
- Chaque repo forké reste **byte-identique à upstream hors commits de code**,
  sans réorganisation interne : un `git log <tag>..kyberfrog-dev` liste
  exactement la divergence, et un patch se rejoue ou se propose upstream sans
  conflit structurel.
- Branche fork unique **`kyberfrog-dev`** dans les 7 repos ; KyberFrog pinne un
  **SHA** de `kyber-desktop` dans `packaging/versions.sh`, jamais une branche.
- La cascade de bumps (`txproto` → `kymedia` → `kysdk` → `kyber-desktop`) est
  assumée et **scriptée**.

## Outillage

| Outil | Rôle |
|---|---|
| `packaging/fork-lint.sh` | Pré-vol contre `upload-pack: not our ref` : `.gitmodules` committé vs remotes, `fetch --prune` avant la vérification de reachability, worktrees propres |
| `packaging/rebase-fork.sh <version>` | Cascade complète de rebase sur une version upstream, en machine à états (`--dry-run` / `--continue` / `--abort`). Cibles résolues en lisant les gitlinks upstream (`ls-tree`, robuste aux renames), commits `deps…bump` écartés au replay puis régénérés en fin de cascade, travail sur des branches `rebase/<version>`, rien n'est poussé |
| skill `/rebase-fork` | Procédure et intention de chaque commit fork ; résout les conflits que le script ne peut pas trancher, lance les vérifications, met à jour `versions.sh` et produit un rapport |
| `bump-fork.sh` *(racine du workspace)* | Remonte les bumps de pointeurs d'un changement local : `./bump-fork.sh kyber-desktop kyberfrog-dev` |

## Bilan

**Pour**

- **Rebase upstream naturel** : `git rebase` classique dans chaque repo,
  outillé de bout en bout ; le rebase 0.27.1 est passé par ce chemin.
- **Contribution upstream directe** : une MR part du fork GitLab sans
  reformatage, puisque l'arbre ne diverge que par le code.
- **Aucun mécanisme en double** : les submodules font foi, un cloneur
  extérieur obtient un arbre cohérent sans script maison.
- **Coût de mise en place nul** : aucune migration, rien à reconstruire en CI.

**Contre**

- **La cascade de bumps demeure** : un fix d'une ligne en feuille de chaîne
  produit jusqu'à trois commits de plomberie en amont.
- **Sept repos à ouvrir** pour lire tout le code forké ; l'audit en tient
  l'inventaire.
- **Discipline requise** sur `.gitmodules` : ses changements vivent dans des
  commits `build(submodules)` dédiés, jamais dans un bump.

## Remontée amont (#25)

Seul levier qui réduit le coût **récurrent** de la chaîne : chaque commit accepté
upstream est un commit de moins à rejouer.

**Vague 1 — prête.** Branches bugfixes-only, cherry-picks propres sur les bases
0.27.x :

| Repo | Branche | Base upstream | Contenu |
|---|---|---|---|
| `txproto` | `fix/lavd-directshow-0.27` | `kyber-0.27.1` | enregistrement + série lavd |
| `kynput` | `fix/xy-scale-0.27` | `0.27.0` | scales X/Y séparés + tests |
| `kyber-desktop` | `fix/fractional-mouse-deltas-0.27` | `0.27.1` | accumulateur fractionnaire |
| `kyber-desktop` | `fix/unknown-source-dims-0.27` | `0.27.1` | sources 0×0 |

**Avant d'ouvrir les MRs :**

- [ ] build de validation sur ces bases (upstream a refactoré kyclient entre
  0.26 et 0.27.1 : un conflit *sémantique* reste possible) ;
- [ ] vérifier que les repos `kyber-frog/*` ont la **relation de fork GitLab**
  vers `kyber.stream/*`, sans laquelle une MR cross-projet est impossible ;
- [ ] **décision opérateur** : identité de contribution (compte perso ou
  `kyber-frog`) et politique de licence des contributions (Kyber est en
  AGPL/commercial dual).

Précédent favorable : upstream a déjà adopté le pattern d'override par variable
d'environnement (`KYBER_CONFIG`).
