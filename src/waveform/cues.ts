// Pure helpers for showing Cues on the Waveform: colour, label, tooltip and
// where a Cue sits in the zoomed view. No DOM, so it runs under bun test.
import type { Cue } from "./api";
import { timeToZoomedX } from "./geometry";

// Cue colours are Rekordbox data, not theme colours, so these are literals.
/** Rekordbox's default hot Cue green (palette index 5 in cues.rs). */
export const HOT_CUE_DEFAULT_COLOR = "#28E214";
/** Memory Cues have no colour of their own. */
export const MEMORY_CUE_COLOR = "#A1A1AA";

/** Hot Cues have a slot (kind 1–8); memory Cues are kind 0. */
export const isHotCue = (cue: Cue) => cue.kind > 0;

/** The Cue's colour, falling back to a default when Rekordbox gave none. */
export function cueColor(cue: Cue): string {
  return (
    cue.color ?? (isHotCue(cue) ? HOT_CUE_DEFAULT_COLOR : MEMORY_CUE_COLOR)
  );
}

/** A–H for hot Cue slots 1–8, `•` for any other hot Cue, null for memory Cues. */
export function cueLabel(cue: Cue): string | null {
  if (!isHotCue(cue)) return null;
  if (cue.kind <= 8) return String.fromCharCode(64 + cue.kind);
  return "•";
}

/** "Hot Cue A — comment" or "Memory Cue", for the tooltip. */
export function cueTitle(cue: Cue): string {
  const name = isHotCue(cue) ? `Hot Cue ${cueLabel(cue)}` : "Memory Cue";
  const comment = cue.comment?.trim();
  return comment ? `${name} — ${comment}` : name;
}

/**
 * Where the Cue is drawn in the zoomed view, in px from the left, or null when
 * it is further than `margin` px outside the view.
 */
export function visibleCueX(
  cue: Cue,
  width: number,
  position: number,
  zoom: number,
  margin = 24,
): number | null {
  const x = timeToZoomedX(cue.positionSec, width, position, zoom);
  return x < -margin || x > width + margin ? null : x;
}
