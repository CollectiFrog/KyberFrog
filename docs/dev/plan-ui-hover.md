# Hover cohérent sur tout le cockpit (#22)

> Archi arrêtée le 2026-07-14, extraite d'`IMPROVEMENTS.md` le 2026-08-25 sans
> réécriture. Le suivi de l'item vit dans le board ([backlog.md](backlog.md)).

- **Livré le 2026-07-14** (branche `feat/ui-v2.1`) :
  - Responsive vertical : en fenêtre étroite les sections Émission/Réception
    se dimensionnent sur leur contenu (fin du `min-height: 64vh` forcé et du
    scroll interne ; c'est la page qui défile).
  - Header : bloc Hostname/IP empilé (IP copiable au clic, fallback
    `execCommand` pour l'accès http LAN), LED d'état respirante avec
    « En ligne » en tooltip — l'indicateur-pilule qui ressemblait à un bouton
    est supprimé.
  - Modale « À propos » → « Options » (roue crantée dans le header) : les
    choix thème (clair/sombre) et langue (FR/EN) y migrent depuis le header,
    libellés de la modale traduits FR/EN, route `/about` → `/options`.
  - Quick-fix bouton Supprimer des tuiles : peint en `--k-danger` (rouge) au
    lieu de `--k-faint` qui le faisait paraître désactivé. Le hover rouge
    viendra avec la passe globale ci-dessous.
- **What (reste) :** état hover cohérent sur tous les boutons de l'app.
- **Why deferred :** décision 2026-07-14 — à implémenter **après #21
  (Tauri)**, pour ne pas polir deux fois si le wrap fait bouger l'IHM.
  **#21 livré le 2026-07-15 → plus rien ne bloque.**
- **How (archi arrêtée 2026-07-14, analyse) :**
  - *Cause racine :* tout le styling est en `style={{…}}` inline, qui ne peut
    pas exprimer `:hover` — d'où l'absence totale d'états hover aujourd'hui.
  - *Approche retenue :* classes CSS + design tokens, zéro dépendance.
    Écartés : CSS Modules (éparpille le look par composant alors qu'on veut
    mutualiser) et Tailwind/styled-components (réécriture massive +
    dépendance, à éviter avant le wrap Tauri).
  - *3 couches :*
    1. **Tokens d'état** dans `global.css`, déclinés dans les 3 thèmes
       (dark/light/frog) : `--k-hover` (fond hover neutre),
       `--k-accent-hover` (accent éclairci/assombri), `--k-danger-hover`.
    2. **`ui/src/buttons.css`** : base `.kf-btn` (inline-flex, radius 8,
       transition .15s sur background/color/border-color, ring
       `:focus-visible` accent, `:disabled` centralisé opacity .4 +
       not-allowed) + 4 variantes : `--primary` (fond accent →
       `--k-accent-hover`), `--ghost` (bordure, fond transparent → fond
       `--k-hover`), `--quiet` (sans bordure → fond `--k-hover`), `--danger`
       (apparence normale au repos → fond `--k-danger-soft` + texte/icône
       `--k-danger` au hover) ; modificateurs `--icon` (carré) et `--sm`.
    3. **Composant `<Btn variant size icon>`** (~30 lignes) qui assemble les
       classNames et forwarde le reste vers `<button>` ; le `style` inline
       reste pour le layout uniquement. Migration incrémentale : les
       constantes locales `iconBtnStyle`/`textBtnStyle`/`tbBtn`/`BarBtn`
       meurent fichier par fichier.
  - *Ordre de migration* (1 commit par étape, l'app reste cohérente entre
    chaque) : ① tokens + buttons.css + Btn.tsx → ② TopBar + PaneHeader →
    ③ cartes émetteur/récepteur (hover rouge Supprimer ici) → ④ LogDrawer →
    ⑤ drawers/modales (Segmented d'OptionsModal inclus). Bonus au passage :
    hover des `<select>` et des liens, navigation clavier gratuite via
    `:focus-visible`. Volume : ~35 boutons dans 8 fichiers, mécanique une
    fois la couche ① posée.
