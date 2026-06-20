Tu vas concevoir une interface web professionnelle pour un logiciel d'exploitation
appelé KyberFrog. C'est un outil de régie : il diffuse de la vidéo sur réseau local
(LAN) entre des machines. J'ai déjà un design system (palette, tokens, composants) :
ne décris PAS l'esthétique. Concentre-toi sur l'ORGANISATION DE LA PAGE et sur les
CAS D'USAGE des utilisateurs. Contrainte transversale : interface PROFESSIONNELLE,
donc esthétique très simple, épurée, dense en information mais lisible, sans
décoration superflue. Langue de l'UI : français.

═══════════════════════════════════════════════════════════════════════════
CONTEXTE MÉTIER (pour comprendre les besoins, pas à afficher tel quel)
═══════════════════════════════════════════════════════════════════════════
Une même machine peut jouer deux rôles, séparément ou en même temps :
  • ÉMISSION  — elle capture une source vidéo et la diffuse sur le LAN. La source
    peut être un flux "Spout" (produit par un logiciel tiers comme Resolume), une
    capture d'écran, et À TERME d'autres protocoles. Chaque source diffusée = un
    "transmetteur".
  • RÉCEPTION — elle se connecte à une machine émettrice distante et fait quelque
    chose du flux reçu (l'afficher, le re-publier, ou piloter la machine distante).
    Chaque connexion = un "viewer".
Une machine peut donc être émettrice seule, réceptrice seule, ou les deux.
L'opérateur surveille l'ensemble depuis cette page et agit en quelques clics.

═══════════════════════════════════════════════════════════════════════════
ORGANISATION DE LA PAGE
═══════════════════════════════════════════════════════════════════════════
Page unique de type "cockpit" plein écran (100vh), NON scrollable globalement :
tout tient à l'écran sur un poste opérateur large. Trois zones :

1. BARRE SUPÉRIEURE (hauteur fixe), de gauche à droite :
   - marque + nom de la machine (hostname) ;
   - adresse IP locale de la machine ;
   - un indicateur d'état de connexion (en ligne / perdu) ;
   - un bouton "À propos / infos" (version, hostname, liens). Pas de réglages ici :
     les réglages avancés ne sont volontairement PAS dans l'UI.

2. DEUX PANNEAUX CÔTE À CÔTE (moitié/moitié) :
   - À GAUCHE : "ÉMISSION" — liste des transmetteurs.
   - À DROITE : "RÉCEPTION" — liste des viewers.
   Chaque panneau = un en-tête (titre + bouton d'ajout) + une liste de cartes qui
   scrolle INDÉPENDAMMENT (le scroll est interne au panneau, jamais la page).
   Quand une liste est vide, afficher un état vide explicite et invitant à l'action
   (ex. "Aucun transmetteur. Ajoutez une source à diffuser.").

3. TIROIR LOGS (bas de page, pleine largeur, REPLIABLE) : une console de journaux
   temps réel, accessible sans casser le "non-scrollable" de la page.

Responsive : la cible primaire est un grand écran d'opérateur. En dessous d'une
largeur réduite (~1100px), les deux panneaux s'EMPILENT et la page peut redevenir
scrollable ; le tiroir logs peut alors passer en vue plein écran dédiée. Le
"non-scrollable" est un objectif desktop, pas une contrainte mobile.

═══════════════════════════════════════════════════════════════════════════
ÉTATS À AFFICHER (chaque transmetteur et chaque viewer a un état)
═══════════════════════════════════════════════════════════════════════════
en cours d'exécution / démarrage / redémarrage / arrêté / inconnu.
L'état doit être lisible d'un coup d'œil sur chaque carte (indicateur + libellé).

═══════════════════════════════════════════════════════════════════════════
CAS D'USAGE UTILISATEUR
═══════════════════════════════════════════════════════════════════════════

── Panneau ÉMISSION ──────────────────────────────────────────────────────
• L'utilisateur doit pouvoir AJOUTER UN TRANSMETTEUR en cliquant sur le bouton
  d'ajout de l'en-tête Émission. L'ajout commence par le CHOIX DU TYPE DE SOURCE
  (protocole) : "Spout" et "Capture d'écran" aujourd'hui, et des emplacements
  prévus pour de futurs protocoles (affichés mais désactivés / marqués "à venir").
  Si "Spout" est choisi, l'utilisateur sélectionne parmi les sources Spout
  détectées en direct (les sources actives mises en avant). Il peut éventuellement
  préciser un port ; sinon il est attribué automatiquement.
• L'utilisateur doit pouvoir VOIR pour chaque transmetteur : son nom, le type de
  source / protocole, son port, et son état.
• L'utilisateur doit pouvoir LANCER, ARRÊTER et REDÉMARRER un transmetteur en
  actionnant les boutons dédiés de sa carte.
• L'utilisateur doit pouvoir SUPPRIMER un transmetteur depuis sa carte (avec
  confirmation).

── Panneau RÉCEPTION ─────────────────────────────────────────────────────
• L'utilisateur doit pouvoir CRÉER UN VIEWER en cliquant sur le bouton d'ajout de
  l'en-tête Réception, qui ouvre un formulaire (panneau latéral / drawer) avec :
    - un nom (défini À LA CRÉATION uniquement — voir édition ci-dessous),
    - l'adresse de la machine émettrice (IP) et le port,
    - un sélecteur de TYPE DE RÉCEPTION (voir ci-dessous),
    - une option "plein écran" (pertinente pour les modes d'affichage).
• TYPE DE RÉCEPTION — sélecteur unique (un seul choix à la fois) qui détermine ce
  que le viewer fait du flux reçu. C'est l'axe d'extension principal de l'outil :
  conçois-le pour ACCUEILLIR DE NOUVELLES OPTIONS dans le futur. Options :
    - "Affichage" — afficher le flux (mode classique, plein écran possible) ;
    - "Redirection Spout" — re-publier localement la vidéo reçue comme source Spout
      pour d'autres logiciels (Resolume, MadMapper…) ;
    - "Bureau à distance" — piloter la machine distante : le clavier et la souris
      sont transférés ; le viewer s'ouvre alors en FENÊTRÉ (pas plein écran) ;
    - des emplacements pour de FUTURS PROTOCOLES, affichés mais désactivés /
      marqués "à venir".
  Comme c'est un sélecteur unique, les modes sont naturellement mutuellement
  exclusifs.
• L'utilisateur doit pouvoir MODIFIER un viewer existant (adresse, port, type de
  réception, plein écran) en ouvrant le formulaire d'édition depuis sa carte. LE
  NOM N'EST PAS MODIFIABLE après création : l'afficher en lecture seule dans le
  formulaire d'édition.
• Depuis chaque carte de viewer, l'utilisateur doit pouvoir : LANCER, ARRÊTER,
  REDÉMARRER, ÉDITER et SUPPRIMER le viewer (boutons d'action sur la carte).
• L'utilisateur doit pouvoir VOIR pour chaque viewer : son nom, l'adresse:port
  cible, son type de réception (affichage / redirection Spout / bureau à distance /
  futur protocole) et son état. Le mode "Bureau à distance" doit être clairement
  identifiable sur la carte.

── Tiroir LOGS ───────────────────────────────────────────────────────────
• L'utilisateur doit pouvoir CHOISIR LA SOURCE des logs via un sélecteur :
  l'application globale, OU un transmetteur précis, OU un viewer précis.
• L'utilisateur doit pouvoir BASCULER le flux en direct (live) on/off, et METTRE EN
  PAUSE le défilement.
• L'utilisateur doit pouvoir VIDER la console.
• L'utilisateur doit pouvoir OUVRIR LE FICHIER DE LOG de la source actuellement
  affichée (bouton dédié qui ouvre le fichier sur disque).
• La console scrolle ; l'auto-défilement suit les nouvelles lignes tant que
  l'utilisateur est déjà en bas.
• L'utilisateur doit pouvoir REPLIER / DÉPLIER le tiroir pour récupérer de la place.

── Barre supérieure ──────────────────────────────────────────────────────
• L'utilisateur doit pouvoir OUVRIR un panneau "À propos" (version, hostname,
  liens) via le bouton infos.

═══════════════════════════════════════════════════════════════════════════
NAVIGATION
═══════════════════════════════════════════════════════════════════════════
Le cockpit reste toujours visible en fond. Les formulaires (ajout/édition de
viewer, ajout de transmetteur), la vue logs plein écran et le "À propos"
apparaissent en surcouche (drawer/modal) PAR-DESSUS le cockpit, et sont
deep-linkables (URL dédiée, le bouton Précédent du navigateur les ferme). Fermer
une surcouche = revenir au cockpit.

═══════════════════════════════════════════════════════════════════════════
EXTENSIBILITÉ / FONCTIONNALITÉS FUTURES (à anticiper dans la conception)
═══════════════════════════════════════════════════════════════════════════
L'outil va grandir ; conçois l'UI pour que ces ajouts s'intègrent sans refonte :
  • ÉMISSION — d'autres protocoles de source que Spout/Écran arriveront. Le choix
    du type de source doit être un sélecteur extensible ; montre des options
    futures désactivées / "à venir" pour signaler la direction.
  • RÉCEPTION — d'autres types de réception (au-delà d'Affichage / Redirection
    Spout / Bureau à distance) arriveront. Même principe : sélecteur extensible
    avec emplacements futurs désactivés / "à venir".
Les options non encore implémentées doivent être visiblement présentes mais
non sélectionnables (désactivées + libellé "à venir"), pas masquées.

═══════════════════════════════════════════════════════════════════════════
À LIVRER
═══════════════════════════════════════════════════════════════════════════
Le cockpit complet (barre supérieure + panneaux Émission/Réception remplis de
cartes d'exemple dans des états variés + tiroir logs ouvert avec des lignes
d'exemple), PLUS les surcouches : formulaire d'ajout transmetteur (avec le
sélecteur de type de source et ses options futures désactivées), formulaire de
création/édition viewer (avec le sélecteur de type de réception et le nom en
lecture seule en édition), et le "À propos". Privilégie la clarté opérationnelle
et la densité maîtrisée à toute fioriture.
