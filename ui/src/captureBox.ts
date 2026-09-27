import type { ApiSource } from './types'

// An HDMI → USB capture box enumerates as an ordinary UVC camera: nothing in
// DirectShow or V4L2 sets it apart from a webcam but its name. These fragments
// file it under « Boîtier de capture », next to the DeckLink cards; the
// transmitter stays a `camera` source either way. A box matching none of them
// still shows under Webcam, and works the same from there.
const CAPTURE_HINTS = [
  'capture', 'ugreen', 'cam link', 'elgato', 'hdmi', 'avermedia',
  'live gamer', 'magewell', 'blackmagic', 'decklink',
]

export function isCaptureBox(device: string): boolean {
  const name = device.toLowerCase()
  return CAPTURE_HINTS.some(hint => name.includes(hint))
}

/** DeckLink cards and camera sources that are capture boxes. */
export function isCaptureSource(source: ApiSource): boolean {
  return source.type === 'decklink' || (source.type === 'camera' && isCaptureBox(source.device ?? ''))
}
