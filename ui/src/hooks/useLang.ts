import { useState } from 'react'

export const L = {
  fr: {
    online: 'En ligne',
    offline: 'Hors ligne',
    about: 'À propos',
    themeTitle: 'Thème clair / sombre',
    setupLabel: 'Config',
    loadTitle: 'Charger une configuration',
    saveAs: 'Enregistrer sous',
    saveAsTitle: 'Enregistrer la configuration courante sous un nouveau nom',
    saveAsPrompt: 'Nom de la configuration :',
    download: 'Télécharger',
    downloadTitle: 'Télécharger le fichier de configuration actif (.toml)',
    importFile: 'Importer',
    importTitle: 'Importer un fichier de configuration (.toml)',
    emission: 'Émission',
    reception: 'Réception',
    txSection: 'Transmetteur',
    rxSection: 'Récepteur',
    addTxHeader: 'Ajouter un transmetteur',
    addRxHeader: 'Créer un récepteur',
    start: 'Lancer',
    stop: 'Arrêter',
    restart: 'Redémarrer',
    edit: 'Éditer',
    del: 'Supprimer',
    emptyTxTitle: 'Aucun transmetteur',
    emptyTxSub: 'Ajoutez une source à diffuser sur le réseau local.',
    emptyRxTitle: 'Aucun récepteur',
    emptyRxSub: 'Connectez-vous à une machine émettrice du réseau.',
  },
  en: {
    online: 'Online',
    offline: 'Offline',
    about: 'About',
    themeTitle: 'Light / dark theme',
    setupLabel: 'Setup',
    loadTitle: 'Load a setup',
    saveAs: 'Save as',
    saveAsTitle: 'Save the current setup under a new name',
    saveAsPrompt: 'Setup name:',
    download: 'Download',
    downloadTitle: 'Download the active setup file (.toml)',
    importFile: 'Import',
    importTitle: 'Import a setup file (.toml)',
    emission: 'Emission',
    reception: 'Reception',
    txSection: 'Transmitter',
    rxSection: 'Receiver',
    addTxHeader: 'Add transmitter',
    addRxHeader: 'Create receiver',
    start: 'Start',
    stop: 'Stop',
    restart: 'Restart',
    edit: 'Edit',
    del: 'Delete',
    emptyTxTitle: 'No transmitters',
    emptyTxSub: 'Add a source to broadcast over the local network.',
    emptyRxTitle: 'No receivers',
    emptyRxSub: 'Connect to a transmitting machine on the network.',
  },
} as const

export type Lang = keyof typeof L
export type LangStrings = Record<keyof typeof L.fr, string>

export function useLang() {
  const [lang, setLang] = useState<Lang>('fr')
  return { lang, setLang, t: L[lang] }
}
