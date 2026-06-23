import { useState } from 'react'

export const L = {
  fr: {
    online: 'En ligne',
    offline: 'Hors ligne',
    about: 'À propos',
    themeTitle: 'Thème clair / sombre',
    saveConfig: 'Sauvegarder',
    saveConfigTitle: 'Exporter la configuration (.json)',
    importConfig: 'Importer',
    importConfigTitle: 'Importer une configuration (.json)',
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
    saveConfig: 'Save config',
    saveConfigTitle: 'Export configuration (.json)',
    importConfig: 'Import',
    importConfigTitle: 'Import a configuration (.json)',
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
