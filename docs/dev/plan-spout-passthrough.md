# Passthrough Spout — deux interrupteurs indépendants (#27)

> Spec extraite d'`IMPROVEMENTS.md` le 2026-08-25, **sans une ligne de
> réécriture technique** : le contenu est celui arrêté le 2026-07-17. L'état
> d'avancement de l'item ne vit plus ici mais dans le board
> ([backlog.md](backlog.md)) — ce document dit le *pourquoi* et le *comment*.

- **What:** **deux interrupteurs indépendants**, un par sens — décision
  opérateur du 2026-07-17, cf. « Pourquoi deux interrupteurs » ci-dessous :

  ```
  ☐ emission.spout_passthrough   « Publier tous les Spout »
      tout sender Spout local (Resolume Advanced Output, Spout Out TOP, OBS…)
      → 1 transmetteur chacun,  Source::Spout { sender }            [zero-copy]

  ☐ reception.spout_passthrough  « Recevoir tous les Kyber »
      tout flux Kyber découvert (mDNS, hors is_self)
      → 1 viewer chacun, { fullscreen: false, spout_out: "Kyber - <tx>@<host>" }
        → Resolume > Sources > Spout Servers · TD > Spout In TOP
  ```

  Rend Kyber utilisable depuis Resolume / TouchDesigner / OBS comme on
  utiliserait NDI, **sans ajouter une seule copie** au chemin natif.

- **Pourquoi deux interrupteurs et pas un seul mode (important) :**
  1. **Ça colle à l'infra réelle.** Les machines ont des rôles **asymétriques** :
     une machine envoie ses flux TD, une autre reçoit tous les flux TD pour les
     injecter dans Resolume Arena. Le ping-pong est l'exception, pas la norme.
  2. **Ça supprime la boucle *par construction*, pas par filtrage.** Une boucle
     exige qu'une **même machine** crée des senders `Kyber - *` (réception)
     **et** publie des senders (émission). En sens unique, il n'y a **rien à
     re-publier** → topologies 1 et 2 **structurellement impossibles**. Le
     filtre anti-boucle ne reste nécessaire que pour la machine qui active les
     **deux** sens — cas légitime (envoyer son TD *et* recevoir les autres),
     mais explicite et minoritaire. Défense en profondeur : archi **puis**
     runtime.
  3. **Ça laisse éviter le côté cher.** Un transmetteur au repos est gratuit
     (kycontroller n'encode qu'à l'ouverture de session) ; **un viewer est actif
     d'office** (session distante + décodage permanents). En un seul mode,
     c'était tout ou rien ; séparés, l'opérateur prend le côté gratuit sans le
     côté coûteux.
  4. **Ça épouse le schéma existant.** La config a **déjà** `[emission]` et
     `[reception]` : un champ par moitié, le placement se déduit du modèle.
  5. **Ça réduit le conflit avec « Tout envoyer ».** `send_all` est un mode
     d'**émission** : seul `emission.spout_passthrough` l'exclut. La réception
     devient orthogonale.

- **Why:** ceci **remplace** la demande initiale « un plugin Resolume (FFGL) +
  un plugin TouchDesigner ». Conclusion de l'exploration du 2026-07-17 :
  **aucun des deux ne doit être écrit.** Les 4 raisons, pour ne pas les
  re-dériver :
  1. **FFGL n'a que Source / Effect / Mixer** — aucun type « output device ».
     Un plugin **ne peut pas** ajouter « Kyber » au dropdown de l'Advanced
     Output ; seul Resolume peut le faire.
  2. **Resolume fait déjà Spout in/out nativement** — les senders apparaissent
     sous *Sources → Spout Servers*, et l'Advanced Output sort en Spout **en
     utilisant le nom de l'écran comme sender name**. C'est le modèle NDI, déjà
     livré.
  3. **TouchDesigner aussi** — `Spout In/Out TOP` natifs et GPU ; le
     `sendername` du Spout In TOP est **déjà un menu des senders vivants**. Un
     `CPlusPlus TOP` ne sort qu'en CPUMem/CUDA → strictement pire.
  4. **Aucune API C n'offrirait un chemin plus court** — `kyclient.dll` n'a
     **aucun callback frame** (`capi.rs:821/895`, seulement `HWND` ou
     `spout_out`), le QUIC (quinn) a **zéro export C**, et tout le fork est
     **MinGW** quand FFGL/TD sont MSVC.

  Un plugin ne ferait donc que **ré-lire le même Spout en ajoutant un blit** —
  soit de la latence, contre la priorité n°1 du projet. Le manque réel n'est pas
  dans les hôtes : c'est que l'opérateur **câble le pont à la main** dans l'UI.

- **How:**
  - **Pattern : dérivé, jamais persisté.** Reprendre `op_set_send_all`
    (`app.rs:349-383`), qui **ne mute jamais les listes** — l'extinction restaure
    l'état d'avant à l'identique. Seuls les 2 booléens (+ leurs excludes) sont
    persistés ; les ressources dérivées n'entrent **pas** dans
    `config.emission.transmitters` / `reception.viewers`, sinon le TOML
    churnerait à chaque sender Spout qui apparaît et l'extinction laisserait des
    déchets. **Corollaire : non modifiable** (comme `send_all`) — la boucle de
    réconciliation écraserait toute édition au tick suivant. L'échappatoire est
    **l'exclusion**, pas l'édition.
  - **Placement des champs — ⚠️ piège de sérialisation TOML.**
    `emission.spout_passthrough: bool` + `emission.spout_passthrough_exclude:
    Vec<String>` doivent être déclarés **avant** `defaults: toml::Table` et
    `transmitters` ; `reception.spout_passthrough` + son exclude **avant**
    `viewers`. Raison déjà documentée pour `send_all` (`config.rs:211-213`) :
    *« TOML requires bare keys before `[table]` / `[[array]]` sections »*. Un
    `Vec<String>` sérialise en tableau **inline** (clé nue) → OK, mais l'ordre
    reste impératif. Tous en `#[serde(default)]`.
  - **Deux boucles de réconciliation indépendantes** (~2 s), une par sens :
    émission ← `spout.rs` (senders locaux, `spout.rs:25-124`) · réception ←
    `discovery.rs` (mDNS, `discovery.rs:32-51`). Idempotentes, via les `op_*`
    (verrou **config avant manager**, `app.rs:604-614`).
  - **Combinaisons — les 2 passthrough sont INDÉPENDANTS et CUMULABLES.**
    Émission et réception ne s'excluent **jamais** l'un l'autre : les 4
    combinaisons sont valides et supportées.

    | `emission.spout_passthrough` | `reception.spout_passthrough` | Rôle | Boucle possible ? |
    |---|---|---|---|
    | ☐ | ☐ | manuel (défaut) | non |
    | ☑ | ☐ | **machine émettrice** (ex. envoie ses flux TD) | **non — par construction** |
    | ☐ | ☑ | **machine réceptrice** (ex. reçoit tout pour Resolume) | **non — par construction** |
    | ☑ | ☑ | bidirectionnelle | oui → **avertir**, filtrer, **ne jamais bloquer** |

    **Le cas ☑/☑ est légitime et doit rester possible** (une machine peut
    vouloir envoyer son TD *et* recevoir les autres). L'UI **avertit**
    (« les 2 sens actifs — risque de boucle, filtre anti-boucle actif ») ;
    elle **n'interdit pas**, ne désactive pas l'autre bascule, et ne demande pas
    de confirmation. Seule combinaison réellement exclue : **`send_all` ⟷
    `emission.spout_passthrough`** — deux stratégies d'*émission* concurrentes
    (cf. ci-dessous). `send_all` n'a **aucun** effet sur la réception.
  - **`send_all` vs `emission.spout_passthrough` — pourquoi ils s'excluent.**
    `send_all` = **1 kycontroller** exposant *toutes* les sources, le client
    choisit à la connexion → 1 seule entrée mDNS (`kind=all`). Passthrough =
    **N kycontroller**, 1 par sender → **N entrées mDNS individuelles**. Or
    `reception.spout_passthrough` **a besoin d'annonces individuelles** pour
    énumérer et s'abonner flux par flux. Les deux stratégies sont donc
    alternatives : l'une privilégie le coût (1 instance), l'autre la
    découvrabilité (N instances, mais plafond ~9).
  - **❓ À trancher — interop avec un voisin en « Tout envoyer ».** Que fait
    `reception.spout_passthrough` face à un transmetteur découvert `kind=all` ?
    Il expose N sources derrière **une** annonce ; s'y abonner « une fois »
    donne une source ambiguë. Proposition : **ignorer `kind=all` en
    réception-passthrough** et le journaliser (l'opérateur crée un viewer
    manuel s'il le veut), plutôt que de deviner. Le TXT `kind` est déjà là
    (`discovery.rs:76-94`).
  - **⚠️ ANTI-BOUCLE — nécessaire uniquement quand les DEUX sens sont actifs
    sur la même machine.**
    Le découpage en 2 interrupteurs rend les topologies 1 et 2 **impossibles par
    construction en sens unique** (rien de créé localement à re-publier). Ce qui
    suit ne s'applique donc qu'à la machine « bidirectionnelle » — cas
    légitime mais explicite. **L'UI doit avertir quand les 2 sens sont
    activés** (« risque de boucle, filtre actif »). Les 3 topologies :

    | # | Topologie | Effet sans parade | Parade |
    |---|---|---|---|
    | 1 | **Machine seule bidirectionnelle** — viewer crée `Kyber - x@A`, l'émission le republie | 1 transmetteur parasite, pollue le LAN. `is_self` empêche le ré-abonnement local, donc **pas** d'explosion | filtre publication |
    | 2 | **Ping-pong A↔B** — exige que **les deux** machines soient bidirectionnelles | **explosion exponentielle** : chaque tour ajoute 1 tx + 1 viewer **par machine**, jusqu'au plafond ~9, sur toute la flotte | **sens unique** (archi) **+** filtre publication |
    | 3 | **Boucle via l'app hôte** — B affiche `Kyber - x@A` sur un layer → Advanced Output → `Kyber Out B` → publié → A s'y abonne → l'affiche → … | boucle réelle, **mais avec des noms 100 % légitimes** | ❌ **indétectable** — voir plus bas |

    **Filtre de publication, en deux couches (les deux, pas l'une ou l'autre) :**
    1. **Identité (primaire, exact).** Exclure les senders dont le nom est
       **exactement** l'un des `spout_out` des viewers dérivés vivants —
       KyberFrog *sait* lesquels il a créés. Exact, donc **zéro faux positif**.
    2. **Préfixe réservé `Kyber - ` (secondaire, défense en profondeur).**
       Rattrape ce que l'identité rate : sender **orphelin d'un kyclient
       crashé** (l'entrée survit en mémoire partagée Spout alors qu'aucun viewer
       ne la revendique plus) et sender d'un **KyberFrog voisin** d'une version
       sans filtre. Sans cette 2ᵉ couche, un crash rouvre la topologie 2.

    **Corollaire : `Kyber - ` devient un préfixe RÉSERVÉ** pour les noms de
    senders Spout locaux. Un écran Resolume nommé `Kyber - Main` serait
    silencieusement non publié → **avertir dans l'UI** quand un sender local
    matche le préfixe sans correspondre à un viewer dérivé, au lieu de l'ignorer
    en silence. (`Kyber Out A` ne matche **pas** `Kyber - ` — les noms de la doc
    opérateur sont sûrs, mais la marge est mince : le dire explicitement.)

    **Défense en profondeur côté réception :** journaliser (et proposer de
    filtrer) les transmetteurs découverts dont le TXT `tx` commence par
    `kyber-` — c'est la signature d'un relais republié, le slugifieur
    (`app.rs:640-648`) transformant `Kyber - main@regie` en
    `kyber-main-regie`. À logger, **pas** à bloquer en dur : un opérateur peut
    légitimement nommer un transmetteur `kyber-*`.

    **Topologie 3 : hors de portée du code, à documenter.** Les noms y sont
    légitimes ; c'est un feedback vidéo, exactement comme filmer l'écran qui
    affiche la caméra. Aucun filtre ne peut le distinguer d'un usage voulu →
    responsabilité opérateur, à écrire noir sur blanc dans la doc.

    **Tests unitaires obligatoires** (`shared`, cible Linux) : un sender égal au
    `spout_out` d'un viewer dérivé n'est jamais publié ; un sender `Kyber - …`
    orphelin non plus ; un sender opérateur `Kyber Out A` **l'est** ; et le
    scénario 2 simulé (A publie, B reçoit) ne crée **aucun** tx chez B.
  - **Anti-self** — ignorer `is_self` côté réception (ne pas se réabonner à ses
    propres transmetteurs). Insuffisant seul : ne couvre **pas** la topologie 2.
  - **Anti-flap (stabilité).** Un sender qui apparaît/disparaît rapidement
    (Resolume qui recharge une compo, TD qui recook) ferait spawner/tuer des
    kycontroller en rafale et entrerait en conflit avec le backoff du
    superviseur. Debounce : publier après **N ticks stables**, temporiser avant
    de démonter.
  - **Plafond** — les ports IPC de kycontroller s'auto-allouent en
    **9091..9100 → ~9 instances max**. Plafonner, journaliser et afficher quand
    ça mord ; jamais échouer en silence. C'est aussi le **dernier rempart** si
    une boucle passe malgré tout : il borne les dégâts. *(À vérifier : les
    kyclient consomment-ils ce budget, ou seulement les kycontroller ?)*
  - **Nommage — les deux champs n'ont pas les mêmes règles :** `viewer.id` →
    `[A-Za-z0-9-]` uniquement (segment d'URL + nom de log, `resolve_viewer_id`
    `app.rs:690`) → `kyber-<tx>-<host>` via le slugifieur `app.rs:640-648`.
    `spout_out` → chaîne libre → **`Kyber - <tx>@<host>`** (vu par l'opérateur
    dans Resolume/TD, sert au regroupement visuel **et** de 2ᵉ couche
    anti-boucle → **préfixe réservé**, cf. ci-dessus).
  - **Exclusion, fichier-only** (cohérent avec « advanced settings are file-only
    by design ») : `[emission] spout_passthrough_exclude = ["Preview", …]`.
    Elle survit à la réconciliation, contrairement à une édition.
  - **UI — une bascule par moitié, dans sa propre section** (Émission /
    Réception), et **chacune ne grise que son côté** : `emission.spout_passthrough`
    grise **`kind:"spout"`** dans la création de transmetteur (screen/camera
    restent offerts) ; `reception.spout_passthrough` grise **`spout_out`** dans
    la création de viewer (fullscreen/remote_control restent offerts). Bandeau
    d'avertissement si les **2** sens sont actifs. Précédent : `op_add_spout`
    refuse déjà quand `send_all` est actif (`app.rs:168-171`) — les endpoints
    doivent renvoyer une **erreur explicite**, jamais un succès silencieux.
    Lister les ressources dérivées en lecture seule **avec le `spout_name`
    résolu** (c'est ce que l'opérateur cherchera dans Resolume/TD).
  - **Fichiers :** `shared/src/config.rs` (champ + exclusions + tests
    round-trip) · `kyberfrog/src/app.rs` (`op_set_spout_passthrough` sur le
    modèle de `op_set_send_all`, + réconciliation) · `src/web.rs` +
    `web/index.html` · `src/tray/` (miroir de la bascule) · doc
    `docs/user/spout-passthrough.md`.
  - **⚠️ Libellés — ne pas réutiliser « Tout envoyer »**, déjà pris par
    `send_all` et de mécanique **différente** (1 transmetteur pour tout vs N
    transmetteurs). Deux modes d'émission aux noms voisins = confusion garantie
    dans le tray et l'UI. Pistes : « **Publier tous les Spout** » /
    « **Recevoir tous les Kyber** », à trancher à l'implémentation (FR/EN, cf.
    #22).
  - **Limites à documenter :**
    - **`Kyber - ` est un préfixe réservé** : ne nommez pas un écran Resolume /
      Spout Out TOP avec ce préfixe, il ne serait pas publié (l'UI avertit) ;
    - **boucle via l'app hôte (topologie 3)** — si B affiche un flux Kyber venu
      de A **et** que ce layer part dans son Advanced Output republié vers A,
      c'est un feedback vidéo que **KyberFrog ne peut pas détecter** (les noms
      sont légitimes). Même nature qu'une caméra filmant son propre écran :
      responsabilité opérateur ;
    - le dropdown Advanced Output de Resolume dira toujours « Spout », jamais
      « Kyber » (impossible sans modification de Resolume) ;
    - **un viewer est actif d'office** — il force une session distante + un
      décodage local **en permanence**, même si personne ne consomme le Spout.
      Contrairement à NDI (qui ne décode que ce qu'on utilise), « recevoir
      tout » a donc un coût GPU/CPU proportionnel au nombre de flux du LAN →
      contenu par le plafond, la liste d'exclusion, et surtout par le fait que
      **ce côté s'active séparément** (une machine purement émettrice ne le paie
      jamais) ;
    - gel du transmetteur si la **résolution de la source change en cours de
      stream** (voir #8, Shipped) — un changement de résolution de composition
      Resolume déclenche ça ;
    - avec `multi_client=false` (le réglage **basse latence**), un 2e client sur
      le même flux reçoit un **409 Conflict** — tension assumée entre « recevoir
      tout » et « latence minimale » (voir #28) ;
    - un sender Spout vivant sur un **autre adaptateur GPU** échoue à
      `OpenSharedResource()` (`iosys_spout.c:66-67`) — PC multi-GPU.

