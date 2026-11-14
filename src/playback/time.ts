/** Formats seconds as m:ss, or h:mm:ss from an hour up. */
export function formatTime(seconds: number): string {
  const total = Number.isFinite(seconds) ? Math.max(0, Math.floor(seconds)) : 0;
  const h = Math.floor(total / 3600);
  const m = Math.floor((total % 3600) / 60);
  const s = String(total % 60).padStart(2, "0");
  return h > 0 ? `${h}:${String(m).padStart(2, "0")}:${s}` : `${m}:${s}`;
}

/** Parses a stored volume, falling back to `fallback` when it isn't 0–1. */
export function parseVolume(stored: string | null, fallback: number): number {
  if (stored === null || stored.trim() === "") return fallback;
  const v = Number(stored);
  return Number.isFinite(v) && v >= 0 && v <= 1 ? v : fallback;
}
