# Passthrough Spout — publier tous les Spout (#27)

!!! warning "Conception arrêtée, pas encore implémentée"
    Cette page est la spec du passthrough Spout : **aucun code n'existe encore**.
    Il sortira **en bêta** — comportement, libellés et réglages pourront encore
    changer. État sur le [board](backlog.md#item-27).

## Principe

**Un interrupteur côté émission** — « Publier tous les Spout » :

```
☐ emission.spout_passthrough   « Publier tous les Spout »
    tout sender Spout local (Resolume Advanced Output, Spout Out TOP, OBS…)
    → 1 transmetteur chacun,  Source::Spout { sender }            [zero-copy]
```

Chaque sender Spout de la machine apparaît sur le LAN comme un transmetteur
individuel, annoncé en mDNS. Kyber s'utilise ainsi depuis Resolume,
TouchDesigner ou OBS comme on utiliserait NDI côté émetteur, sans ajouter une
seule copie au chemin natif.

Côté réception, l'opérateur crée ses viewers à la main : il choisit l'émetteur
dans « Émetteurs détectés » et coche `spout_out` pour ré-exposer le flux en
sender Spout local, vers Resolume (*Sources → Spout Servers*) ou TouchDesigner
(*Spout In TOP*).

## Bilan

**Pour**

- **Zéro latence ajoutée.** Resolume et TouchDesigner font déjà du Spout
  nativement, en GPU ; le passthrough publie le sender tel quel, sans relecture
  ni blit intermédiaire.
- **Aucun plugin à maintenir** côté hôte : ni FFGL, ni TouchDesigner, ni build
  MSVC en parallèle du fork MinGW.
- **Coût quasi nul au repos.** Un transmetteur n'encode qu'à l'ouverture d'une
  session : publier dix senders que personne ne regarde ne coûte rien.
- **Pas de boucle automatique.** Le passthrough ne crée aucun sender, il publie
  seulement ceux qui existent.
- **Découvrabilité** : un sender = une entrée mDNS, visible un par un dans
  « Émetteurs détectés ».
- **Réversible à l'identique** : les transmetteurs sont dérivés, jamais écrits
  dans la config ; éteindre la bascule restaure l'état d'avant.

**Contre**

- **Réception manuelle** : chaque flux à recevoir demande un viewer créé à la
  main.
- **Plafond ~9 transmetteurs** par machine (ports IPC kycontroller
  9091..9100).
- **Incompatible avec « Tout envoyer »** sur la même machine : ce sont deux
  stratégies d'émission concurrentes.
- **Transmetteurs dérivés non modifiables** : on les exclut, on ne les édite
  pas.
- Dans Resolume, le dropdown Advanced Output affiche toujours « Spout », jamais
  « Kyber ».

## Implémentation

- **Dérivé, jamais persisté.** Sur le modèle de `op_set_send_all`
  (`app.rs:349-383`), qui **ne mute jamais les listes**. Seuls le booléen et son
  exclude sont persistés ; les transmetteurs dérivés n'entrent **pas** dans
  `config.emission.transmitters`, sinon le TOML changerait à chaque sender qui
  apparaît. **Corollaire : non modifiable** — la réconciliation écraserait toute
  édition au tick suivant. L'échappatoire est **l'exclusion**.
- **Placement des champs — ⚠️ piège de sérialisation TOML.**
  `emission.spout_passthrough: bool` + `emission.spout_passthrough_exclude:
  Vec<String>` doivent être déclarés **avant** `defaults: toml::Table` et
  `transmitters` (*« TOML requires bare keys before `[table]` / `[[array]]`
  sections »*, `config.rs:211-213`). Tous en `#[serde(default)]`.
- **Réconciliation** (~2 s) sur les senders locaux (`spout.rs:25-124`),
  idempotente, via les `op_*` (verrou **config avant manager**,
  `app.rs:604-614`).
- **Exclusion avec `send_all`.** `send_all` = 1 kycontroller exposant toutes les
  sources (1 entrée mDNS `kind=all`) ; passthrough = N kycontroller, 1 par
  sender. L'endpoint renvoie une **erreur explicite**, jamais un succès silencieux
  (précédent : `op_add_spout`, `app.rs:168-171`).
- **Anti-flap.** Publier après **N ticks stables**, temporiser avant de démonter
  — sinon Resolume qui recharge une compo fait spawner/tuer des kycontroller en
  rafale, en conflit avec le backoff du superviseur.
- **Plafond** : plafonner, journaliser et afficher quand ça mord ; jamais
  échouer en silence.
- **Exclusion fichier-only** : `[emission] spout_passthrough_exclude = ["Preview",
  …]`. Elle survit à la réconciliation.
- **UI** : une bascule dans la section Émission, qui grise **`kind:"spout"`**
  dans la création de transmetteur (screen/camera restent offerts). Transmetteurs
  dérivés listés en lecture seule **avec le nom du sender**. Miroir dans le tray.
  Libellé « **Publier tous les Spout** » — distinct de « Tout envoyer », dont la
  mécanique est différente.
- **Fichiers :** `shared/src/config.rs` (champ + exclusion + tests round-trip) ·
  `kyberfrog/src/app.rs` (`op_set_spout_passthrough` + réconciliation) ·
  `src/web.rs` + UI · `src/tray/`.

## Boucles

Deux cas restent possibles :

| Cas | Mécanisme | Parade |
|---|---|---|
| **Viewer ré-exposé sur la même machine** | un viewer avec `spout_out` crée un sender local, que le passthrough republierait | exclure les senders qui correspondent au `spout_out` d'un viewer local — KyberFrog sait lesquels il a créés |
| **Boucle via l'app hôte** | B affiche un flux venu de A → Advanced Output → sender publié → A l'affiche → … | **indétectable** (noms légitimes) : responsabilité opérateur, comme une caméra filmant son propre écran |

**Tests unitaires** (`shared`, cible Linux) : un sender égal au `spout_out` d'un
viewer local n'est jamais publié ; un sender opérateur `Kyber Out A` **l'est** ;
l'exclusion survit à la réconciliation ; l'extinction restaure le TOML à
l'identique.

## Plan de validation bêta

Testable **sur la seule machine de dev** (Resolume Arena 7.22.9 et TouchDesigner
2025.32280 y sont installés ; loopback via `is_self`) :

- [ ] Spout Out TOP `Kyber Out A` dans TD → activer le passthrough → **1
  transmetteur** apparaît, annoncé en mDNS, lisible depuis un viewer.
- [ ] Deuxième sender → un deuxième transmetteur ; le fermer → le transmetteur
  est démonté après la temporisation, sans rafale de restarts.
- [ ] Recharger une compo Resolume → pas de flap visible.
- [ ] La bascule grise la création de transmetteur Spout, et `send_all` est
  refusé avec une erreur explicite tant que le passthrough est actif.
- [ ] Un sender listé dans `spout_passthrough_exclude` n'est jamais publié.
- [ ] Viewer local avec `spout_out` sur la même machine → son sender n'est
  **pas** republié.
- [ ] Au-delà de ~9 senders : plafond atteint, message explicite.
- [ ] Extinction → transmetteurs dérivés démontés, listes manuelles et TOML
  inchangés.

## Limites connues

- gel du transmetteur si la **résolution de la source change en cours de
  stream** (voir #8) — un changement de résolution de composition Resolume le
  déclenche ;
- avec `multi_client=false` (réglage basse latence), un 2e client sur le même
  flux reçoit un **409 Conflict** (voir #28) ;
- un sender Spout vivant sur un **autre adaptateur GPU** échoue à
  `OpenSharedResource()` (`iosys_spout.c:66-67`) — PC multi-GPU.
