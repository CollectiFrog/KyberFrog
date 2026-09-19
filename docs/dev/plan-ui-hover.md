# Hover cohérent sur tout le cockpit (#22)

## Constat

Tout le styling des boutons est en `style={{…}}` inline, qui ne peut pas
exprimer `:hover` : le cockpit n'a aucun état hover.

## Principe

**Classes CSS + design tokens, zéro dépendance**, en trois couches.

1. **Tokens d'état** dans `global.css`, déclinés dans les 3 thèmes
   (dark/light/frog) : `--k-hover` (fond hover neutre), `--k-accent-hover`
   (accent éclairci/assombri), `--k-danger-hover`.
2. **`ui/src/buttons.css`** : base `.kf-btn` (inline-flex, radius 8, transition
   .15s sur background/color/border-color, ring `:focus-visible` accent,
   `:disabled` centralisé opacity .4 + not-allowed) et 4 variantes :
    - `--primary` : fond accent → `--k-accent-hover` ;
    - `--ghost` : bordure, fond transparent → fond `--k-hover` ;
    - `--quiet` : sans bordure → fond `--k-hover` ;
    - `--danger` : apparence normale au repos → fond `--k-danger-soft` +
      texte/icône `--k-danger` au hover ;
    - modificateurs `--icon` (carré) et `--sm`.
3. **Composant `<Btn variant size icon>`** (~30 lignes) qui assemble les
   classNames et forwarde le reste vers `<button>`. Le `style` inline reste pour
   le layout uniquement.

## Bilan

**Pour**

- **Aucune dépendance** ajoutée, aucun outil de build en plus.
- **Le look est mutualisé** : un seul fichier décide de l'apparence de tous les
  boutons, dans les trois thèmes.
- **Migration incrémentale** : les styles locaux meurent fichier par fichier,
  l'app reste cohérente à chaque commit.
- **Gratuit au passage** : navigation clavier via `:focus-visible`, état
  `:disabled` uniforme, hover des `<select>` et des liens.

**Contre**

- Deux mécanismes de style coexistent (classes pour l'apparence, inline pour le
  layout) : la frontière doit être tenue en revue.
- Des noms de classes globaux, sans isolation par composant : le préfixe
  `kf-` sert de garde-fou.

## Migration

Un commit par étape :

1. tokens + `buttons.css` + `Btn.tsx` ;
2. TopBar + PaneHeader ;
3. cartes émetteur/récepteur (hover rouge du bouton Supprimer) ;
4. LogDrawer ;
5. drawers et modales (Segmented d'OptionsModal inclus).

Volume : ~35 boutons dans 8 fichiers. Les constantes locales `iconBtnStyle`,
`textBtnStyle`, `tbBtn` et `BarBtn` disparaissent à la fin.
