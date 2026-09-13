# Passthrough Spout — publier tous les Spout (#27)

!!! warning "Fonctionnalité en bêta test"
    Le passthrough Spout est **en bêta test** : son comportement, ses libellés et
    ses réglages peuvent encore changer. Les retours se font sur le
    [board](backlog.md) (#27) ou dans une issue du tracker.

> **Décision révisée : un seul interrupteur, côté émission.** La spec du
> 2026-07-17 prévoyait **deux** interrupteurs indépendants, un par sens
> (« Publier tous les Spout » en émission, « Recevoir tous les Kyber » en
> réception). **Seule l'émission est retenue** ; la moitié réception n'est pas
> implémentée (voir [Non retenu](#non-retenu-le-passthrough-en-reception)). Le
> texte d'origine, avec les deux interrupteurs, reste lisible dans l'historique
> git (`dbef98d`). L'état d'avancement vit dans le [board](backlog.md) ; ce
> document dit le *pourquoi* et le *comment*.

## What

**Un interrupteur côté émission** — « Publier tous les Spout » :

```
☐ emission.spout_passthrough   « Publier tous les Spout »
    tout sender Spout local (Resolume Advanced Output, Spout Out TOP, OBS…)
    → 1 transmetteur chacun,  Source::Spout { sender }            [zero-copy]
```

Rend Kyber utilisable depuis Resolume / TouchDesigner / OBS comme on utiliserait
NDI **côté émetteur**, sans ajouter une seule copie au chemin natif : chaque
sender apparaît sur le LAN comme un transmetteur individuel, annoncé en mDNS.

**Côté réception, rien d'automatique.** L'opérateur crée ses viewers comme
aujourd'hui — à la main, en choisissant l'émetteur dans « Émetteurs détectés »
(mDNS) et en cochant `spout_out` pour ré-exposer le flux en sender Spout local
vers Resolume (*Sources → Spout Servers*) ou TouchDesigner (*Spout In TOP*).

## Why

Ceci **remplace** la demande initiale « un plugin Resolume (FFGL) + un plugin
TouchDesigner ». Conclusion de l'exploration du 2026-07-17 : **aucun des deux ne
doit être écrit.** Les 4 raisons, pour ne pas les re-dériver :

1. **FFGL n'a que Source / Effect / Mixer** — aucun type « output device ». Un
   plugin **ne peut pas** ajouter « Kyber » au dropdown de l'Advanced Output ;
   seul Resolume peut le faire.
2. **Resolume fait déjà Spout in/out nativement** — les senders apparaissent
   sous *Sources → Spout Servers*, et l'Advanced Output sort en Spout **en
   utilisant le nom de l'écran comme sender name**. C'est le modèle NDI, déjà
   livré.
3. **TouchDesigner aussi** — `Spout In/Out TOP` natifs et GPU ; le `sendername`
   du Spout In TOP est **déjà un menu des senders vivants**. Un `CPlusPlus TOP`
   ne sort qu'en CPUMem/CUDA → strictement pire.
4. **Aucune API C n'offrirait un chemin plus court** — `kyclient.dll` n'a
   **aucun callback frame** (`capi.rs:821/895`, seulement `HWND` ou
   `spout_out`), le QUIC (quinn) a **zéro export C**, et tout le fork est
   **MinGW** quand FFGL/TD sont MSVC.

Un plugin ne ferait donc que **ré-lire le même Spout en ajoutant un blit** —
soit de la latence, contre la priorité n°1 du projet. Le manque réel n'est pas
dans les hôtes : c'est que l'opérateur **câble le pont à la main** dans l'UI.

**Pourquoi l'émission suffit à ce stade.** Un transmetteur au repos est gratuit
(kycontroller n'encode qu'à l'ouverture de session), alors qu'un viewer est actif
d'office (session distante + décodage permanents) : « publier tout » ne coûte
presque rien, « recevoir tout » coûterait un décodage par flux du LAN. Et en
émission seule, KyberFrog **ne crée aucun sender Spout de lui-même** — la boucle
automatique A↔B, qui justifiait l'essentiel de la complexité de la spec
d'origine, ne peut plus naître du passthrough (voir [Boucles](#boucles)).

## How

- **Pattern : dérivé, jamais persisté.** Reprendre `op_set_send_all`
  (`app.rs:349-383`), qui **ne mute jamais les listes** — l'extinction restaure
  l'état d'avant à l'identique. Seul le booléen (+ son exclude) est persisté ;
  les transmetteurs dérivés n'entrent **pas** dans `config.emission.transmitters`,
  sinon le TOML churnerait à chaque sender Spout qui apparaît et l'extinction
  laisserait des déchets. **Corollaire : non modifiable** (comme `send_all`) —
  la boucle de réconciliation écraserait toute édition au tick suivant.
  L'échappatoire est **l'exclusion**, pas l'édition.
- **Placement des champs — ⚠️ piège de sérialisation TOML.**
  `emission.spout_passthrough: bool` + `emission.spout_passthrough_exclude:
  Vec<String>` doivent être déclarés **avant** `defaults: toml::Table` et
  `transmitters`. Raison déjà documentée pour `send_all` (`config.rs:211-213`) :
  *« TOML requires bare keys before `[table]` / `[[array]]` sections »*. Un
  `Vec<String>` sérialise en tableau **inline** (clé nue) → OK, mais l'ordre
  reste impératif. Tous en `#[serde(default)]`.
- **Une boucle de réconciliation** (~2 s) sur les senders locaux (`spout.rs`,
  `spout.rs:25-124`). Idempotente, via les `op_*` (verrou **config avant
  manager**, `app.rs:604-614`).
- **`send_all` et le passthrough s'excluent.** Ce sont deux stratégies
  d'*émission* concurrentes : `send_all` = **1 kycontroller** exposant *toutes*
  les sources, le client choisit à la connexion → 1 seule entrée mDNS
  (`kind=all`) ; passthrough = **N kycontroller**, 1 par sender → **N entrées
  mDNS individuelles**, que l'opérateur voit une à une dans « Émetteurs
  détectés ». L'une privilégie le coût (1 instance), l'autre la découvrabilité
  (N instances, mais plafond ~9). Précédent : `op_add_spout` refuse déjà quand
  `send_all` est actif (`app.rs:168-171`) — l'endpoint doit renvoyer une
  **erreur explicite**, jamais un succès silencieux.
- **Anti-flap (stabilité).** Un sender qui apparaît/disparaît rapidement
  (Resolume qui recharge une compo, TD qui recook) ferait spawner/tuer des
  kycontroller en rafale et entrerait en conflit avec le backoff du superviseur.
  Debounce : publier après **N ticks stables**, temporiser avant de démonter.
- **Plafond** — les ports IPC de kycontroller s'auto-allouent en
  **9091..9100 → ~9 instances max**. Plafonner, journaliser et afficher quand ça
  mord ; jamais échouer en silence.
- **Exclusion, fichier-only** (cohérent avec « advanced settings are file-only
  by design ») : `[emission] spout_passthrough_exclude = ["Preview", …]`. Elle
  survit à la réconciliation, contrairement à une édition.
- **UI — une bascule dans la section Émission.** Elle grise **`kind:"spout"`**
  dans la création de transmetteur (screen/camera restent offerts). Lister les
  transmetteurs dérivés en lecture seule **avec le nom du sender** — c'est ce que
  l'opérateur cherchera dans Resolume/TD. Miroir de la bascule dans le tray.
- **⚠️ Libellé — ne pas réutiliser « Tout envoyer »**, déjà pris par `send_all`
  et de mécanique **différente** (1 transmetteur pour tout vs N transmetteurs).
  Libellé retenu dans la spec : « **Publier tous les Spout** » (FR/EN).
- **Fichiers :** `shared/src/config.rs` (champ + exclusion + tests round-trip) ·
  `kyberfrog/src/app.rs` (`op_set_spout_passthrough` sur le modèle de
  `op_set_send_all`, + réconciliation) · `src/web.rs` + UI · `src/tray/` (miroir
  de la bascule).

## Boucles

En émission seule, le passthrough **ne crée aucun sender** : il ne fait que
publier ce qui existe. Les boucles automatiques de la spec d'origine (topologies 1
et 2, jusqu'à l'explosion exponentielle A↔B) supposaient une réception
automatique qui fabrique des senders `Kyber - *` ; elles ne peuvent plus naître
du passthrough seul.

Deux cas restent possibles, et relèvent de la **validation bêta** :

| Cas | Mécanisme | Parade |
|---|---|---|
| **Viewer manuel ré-exposé** | un viewer créé à la main avec `spout_out` crée un sender local ; le passthrough de la **même** machine le republie en transmetteur | exclure les senders qui correspondent au `spout_out` d'un viewer local — KyberFrog *sait* lesquels il a créés. **À vérifier dans la bêta.** |
| **Boucle via l'app hôte** | B affiche un flux Kyber venu de A sur un layer → Advanced Output → sender publié → A le reçoit et l'affiche → … | ❌ **indétectable** : les noms sont 100 % légitimes. Même nature qu'une caméra filmant son propre écran — responsabilité opérateur, à documenter. |

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
- [ ] Recharger une compo Resolume → pas de flap visible (debounce).
- [ ] La bascule grise la création de transmetteur Spout, et `send_all` est
  refusé avec une erreur explicite tant que le passthrough est actif.
- [ ] Un sender listé dans `spout_passthrough_exclude` n'est jamais publié.
- [ ] Viewer local avec `spout_out` sur la même machine → son sender n'est
  **pas** republié.
- [ ] Au-delà de ~9 senders : plafond atteint, message explicite.
- [ ] Extinction → transmetteurs dérivés démontés, listes manuelles et TOML
  inchangés.

## Limites à documenter

- le dropdown Advanced Output de Resolume dira toujours « Spout », jamais
  « Kyber » (impossible sans modification de Resolume) ;
- **boucle via l'app hôte** — indétectable, responsabilité opérateur (voir
  [Boucles](#boucles)) ;
- **pas de réception automatique** : chaque flux à recevoir demande un viewer
  créé à la main ;
- gel du transmetteur si la **résolution de la source change en cours de
  stream** (voir #8, archive) — un changement de résolution de composition
  Resolume déclenche ça ;
- avec `multi_client=false` (le réglage **basse latence**), un 2e client sur le
  même flux reçoit un **409 Conflict** (voir #28) ;
- un sender Spout vivant sur un **autre adaptateur GPU** échoue à
  `OpenSharedResource()` (`iosys_spout.c:66-67`) — PC multi-GPU ;
- **plafond ~9 transmetteurs** par machine.

## Non retenu : le passthrough en réception

La spec d'origine prévoyait un second interrupteur, `reception.spout_passthrough`
« Recevoir tous les Kyber » : tout flux Kyber découvert en mDNS (hors `is_self`)
→ 1 viewer dérivé chacun, `{ fullscreen: false, spout_out: "Kyber - <tx>@<host>" }`.
**Il n'est pas implémenté.** C'est lui qui portait l'essentiel de la complexité :

- un **coût permanent** — un viewer décode en continu, même si personne ne
  consomme le sender ; « recevoir tout » coûtait un décodage par flux du LAN ;
- le **filtre anti-boucle à deux couches** (identité des viewers dérivés +
  préfixe réservé `Kyber - `), indispensable dès qu'une machine active les deux
  sens, sous peine de ping-pong A↔B exponentiel ;
- la question non tranchée d'un voisin en « Tout envoyer » (`kind=all` : N
  sources derrière une seule annonce).

Si le besoin revient, repartir du texte complet dans l'historique git (`dbef98d`)
plutôt que de le re-dériver.
