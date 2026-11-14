// Pure maths for the Waveform: where a time is drawn, which Peaks a pixel
// column covers, and the smoothed playhead. No DOM, so it runs under bun test.
//
// The zoomed view has a fixed centre playhead: the Track is laid out at `zoom`
// px/s and scrolled so `position` sits at the centre. Before 0:00 and after
// the end there is half a view of empty padding, so the playhead stays centred
// for the whole Track.

const clamp = (v: number, lo: number, hi: number) =>
  Math.min(hi, Math.max(lo, v));

/** The time under `x` (px from the left) in the zoomed view, within the Track. */
export function zoomedXToTime(
  x: number,
  width: number,
  position: number,
  zoom: number,
  duration: number,
): number {
  return clamp(position + (x - width / 2) / zoom, 0, Math.max(0, duration));
}

/** Where `t` is drawn in the zoomed view, in px from the left. */
export function timeToZoomedX(
  t: number,
  width: number,
  position: number,
  zoom: number,
): number {
  return width / 2 + (t - position) * zoom;
}

/**
 * The span of time the zoomed view shows, in seconds. Near the ends it runs
 * past 0:00 or the end, because that padding really is shown.
 */
export function visibleWindow(
  position: number,
  width: number,
  zoom: number,
): { start: number; end: number } {
  const half = width / 2 / zoom;
  return { start: position - half, end: position + half };
}

/**
 * The data indices `[i0, i1)` covered by pixel column `column` of the whole
 * Track drawn at `pxPerSec`. Always at least one index, so when zoomed in past
 * the data's resolution a value spans several columns and draws as a step.
 */
export function peakRangeForColumn(
  column: number,
  pxPerSec: number,
  duration: number,
  count: number,
): [number, number] {
  if (count <= 0 || duration <= 0 || pxPerSec <= 0) return [0, 0];
  const perPx = count / (duration * pxPerSec);
  const i0 = clamp(Math.floor(column * perPx), 0, count - 1);
  const i1 = clamp(Math.ceil((column + 1) * perPx), i0 + 1, count);
  return [i0, i1];
}

/** The largest |value| in `values[i0..i1)`. */
export function maxAbs(values: ArrayLike<number>, i0: number, i1: number) {
  let m = 0;
  for (let i = i0; i < i1; i++) {
    const v = Math.abs(values[i]);
    if (v > m) m = v;
  }
  return m;
}

/** Scales Peaks so the loudest reaches 1. Silence stays silent. */
export function normaliser(values: ArrayLike<number>): number {
  const m = maxAbs(values, 0, values.length);
  return m > 0 ? 1 / m : 1;
}

/** The time under `x` in the minimap, which shows the whole Track. */
export function minimapXToTime(
  x: number,
  width: number,
  duration: number,
): number {
  if (width <= 0 || duration <= 0) return 0;
  return clamp((x / width) * duration, 0, duration);
}

/** Where `t` is drawn in the minimap, in px from the left. */
export function timeToMinimapX(
  t: number,
  width: number,
  duration: number,
): number {
  return duration > 0 ? (t / duration) * width : 0;
}

/** Never extrapolate further than this past the last position from Rust. */
export const MAX_EXTRAPOLATE_MS = 100;

/**
 * The playhead at `now` (ms), from the last position Rust reported (`lastPos`
 * at `lastAt`). While playing it moves on by the wall time since, up to
 * {@link MAX_EXTRAPOLATE_MS}, so a stalled engine doesn't run away.
 */
export function extrapolate(
  lastPos: number,
  lastAt: number,
  now: number,
  isPlaying: boolean,
  duration: number,
): number {
  if (!isPlaying) return lastPos;
  const elapsed = clamp(now - lastAt, 0, MAX_EXTRAPOLATE_MS) / 1000;
  const next = lastPos + elapsed;
  return duration > 0 ? Math.min(next, duration) : next;
}

/** Backward steps smaller than this while playing are event jitter. */
export const JITTER_SEC = 0.05;

/**
 * Keeps the playhead from twitching backwards while playing when a position
 * event arrives a little late. Real jumps (a seek) go through.
 */
export function steady(
  shown: number,
  next: number,
  isPlaying: boolean,
): number {
  if (isPlaying && next < shown && shown - next < JITTER_SEC) return shown;
  return next;
}
