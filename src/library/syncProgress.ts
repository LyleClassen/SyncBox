import type { SyncPhase, SyncProgress } from "./api";

const PHASES: SyncPhase[] = ["playlists", "tracks", "cues"];

/** Whole-Sync progress, 0-1, treating each phase as an equal third. */
export function overallProgress(p: SyncProgress | null): number {
  if (!p) return 0;
  const index = PHASES.indexOf(p.phase);
  const within = p.total > 0 ? Math.min(1, p.done / p.total) : 1;
  return (Math.max(0, index) + within) / PHASES.length;
}

export function progressLabel(p: SyncProgress | null): string {
  if (!p) return "Starting Sync…";
  const n = new Intl.NumberFormat();
  return `Syncing ${p.phase}… ${n.format(p.done)} / ${n.format(p.total)}`;
}
