// Waveform zoom, in pixels per second of audio.

export const MIN_ZOOM = 10;
export const MAX_ZOOM = 500;
export const DEFAULT_ZOOM = 50;
/** Each zoom in multiplies by this; each zoom out divides by it. */
export const ZOOM_STEP = 1.5;

export function clampZoom(z: number): number {
  if (!Number.isFinite(z)) return DEFAULT_ZOOM;
  return Math.min(MAX_ZOOM, Math.max(MIN_ZOOM, z));
}

export const zoomIn = (z: number) => clampZoom(z * ZOOM_STEP);
export const zoomOut = (z: number) => clampZoom(z / ZOOM_STEP);

/** Parses a stored zoom: the default when missing or invalid, else clamped. */
export function parseZoom(stored: string | null): number {
  if (stored === null || stored.trim() === "") return DEFAULT_ZOOM;
  const z = Number(stored);
  return Number.isFinite(z) ? clampZoom(z) : DEFAULT_ZOOM;
}
