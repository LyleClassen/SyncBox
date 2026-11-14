// Imperative canvas drawing for the Waveform. Sizes are device pixels. All
// amplitude and colour comes from the WaveformSource; overlay colours come
// from the palette.
import type { Beat, BeatGrid } from "./api";
import { peakRangeForColumn, timeToMinimapX, visibleBeats } from "./geometry";
import type { WaveformPalette } from "./palette";
import type { WaveformSource } from "./source";

/** Minimap visible-window shading alpha, and its edges. */
const WINDOW_ALPHA = 0.15;
const WINDOW_EDGE_ALPHA = 0.5;
/** Beat Grid line alpha: downbeats stand out from other beats. */
const DOWNBEAT_ALPHA = 0.5;
const BEAT_ALPHA = 0.2;

/**
 * Batches bars of one colour into a single path, so a single-colour source
 * costs one fill per run rather than one per column.
 */
class Bars {
  private color: string | null = null;

  constructor(private ctx: CanvasRenderingContext2D) {}

  add(color: string, x: number, y: number, w: number, h: number) {
    if (color !== this.color) {
      this.flush();
      this.color = color;
      this.ctx.beginPath();
    }
    this.ctx.rect(x, y, w, h);
  }

  flush() {
    if (this.color === null) return;
    this.ctx.fillStyle = this.color;
    this.ctx.fill();
    this.color = null;
  }
}

/** The loudest data index in `[i0, i1)`, and its amplitude. */
function loudest(source: WaveformSource, i0: number, i1: number) {
  let best = i0;
  let amp = 0;
  for (let i = i0; i < i1; i++) {
    const a = source.amplitude(i);
    if (a > amp) {
      amp = a;
      best = i;
    }
  }
  return { index: best, amp };
}

/** One mirrored bar about the centre line, at least half a pixel tall. */
function bar(
  bars: Bars,
  color: string,
  x: number,
  amp: number,
  height: number,
) {
  const mid = height / 2;
  const h = Math.max(amp * mid, 0.5);
  bars.add(color, x, mid - h, 1, h * 2);
}

function playheadLine(
  ctx: CanvasRenderingContext2D,
  x: number,
  height: number,
  dpr: number,
  color: string,
) {
  const w = Math.max(1, Math.round(dpr));
  ctx.fillStyle = color;
  ctx.fillRect(Math.round(x - w / 2), 0, w, height);
}

/**
 * One kind of Beat Grid line (downbeats or other beats) at `x` device px,
 * batched into a single path.
 */
function beatLines(
  ctx: CanvasRenderingContext2D,
  xs: number[],
  lineWidth: number,
  height: number,
  color: string,
  alpha: number,
) {
  if (xs.length === 0) return;
  const left = Math.floor(lineWidth / 2);
  ctx.beginPath();
  for (const x of xs) ctx.rect(x - left, 0, lineWidth, height);
  ctx.globalAlpha = alpha;
  ctx.fillStyle = color;
  ctx.fill();
  ctx.globalAlpha = 1;
}

/**
 * The Beat Grid over the zoomed view. Placed with the same `pxPerSec` and
 * `origin` as the Waveform's columns, so it stays locked to them at every
 * zoom level. Only beats within the Track (`[0, duration]`) are drawn.
 */
function drawBeatGrid(
  ctx: CanvasRenderingContext2D,
  beats: readonly Beat[],
  origin: number,
  pxPerSec: number,
  width: number,
  height: number,
  dpr: number,
  duration: number,
  palette: WaveformPalette,
) {
  const downWidth = Math.max(2, Math.round(2 * dpr));
  const beatWidth = Math.max(1, Math.round(dpr));
  // Widen the window by a line's width so lines at the edges still show.
  const pad = downWidth / pxPerSec;
  const start = Math.max(0, origin / pxPerSec - pad);
  const end = Math.min(duration, (origin + width) / pxPerSec + pad);
  const [i0, i1] = visibleBeats(beats, start, end);
  const down: number[] = [];
  const other: number[] = [];
  for (let i = i0; i < i1; i++) {
    const b = beats[i];
    const x = Math.round(b.timeSec * pxPerSec) - origin;
    if (x < -downWidth || x >= width + downWidth) continue;
    (b.number === 1 ? down : other).push(x);
  }
  beatLines(ctx, other, beatWidth, height, palette.beat, BEAT_ALPHA);
  beatLines(ctx, down, downWidth, height, palette.downbeat, DOWNBEAT_ALPHA);
}

export interface ZoomedFrame {
  source: WaveformSource | null;
  /** Null when the Track has no Beat Grid; none is guessed. */
  beatGrid: BeatGrid | null;
  /** Seconds the Track lasts; the Beat Grid is clipped to it. */
  duration: number;
  width: number;
  height: number;
  dpr: number;
  /** Seconds at the centre playhead. */
  position: number;
  /** CSS px per second. */
  zoom: number;
  palette: WaveformPalette;
}

/** The zoomed view: the Track scrolled under a fixed centre playhead. */
export function drawZoomed(
  ctx: CanvasRenderingContext2D,
  {
    source,
    beatGrid,
    duration,
    width,
    height,
    dpr,
    position,
    zoom,
    palette,
  }: ZoomedFrame,
) {
  ctx.clearRect(0, 0, width, height);
  const half = Math.floor(width / 2);
  const pxPerSec = zoom * dpr;
  // Track column at screen x = 0. Anchored to whole Track columns so bar
  // shapes don't shimmer while scrolling.
  const origin = Math.floor(position * pxPerSec) - half;
  if (source) {
    const total = source.duration * pxPerSec;
    const bars = new Bars(ctx);
    const start = Math.max(0, -origin);
    const end = Math.min(width, Math.ceil(total - origin));
    for (let x = start; x < end; x++) {
      const [i0, i1] = peakRangeForColumn(
        origin + x,
        pxPerSec,
        source.duration,
        source.length,
      );
      const { index, amp } = loudest(source, i0, i1);
      bar(bars, source.color(index, x < half), x, amp, height);
    }
    bars.flush();
  }
  // Needs only beat times, so it draws while Peaks are still computing.
  const gridSpan = source?.duration ?? duration;
  if (beatGrid && beatGrid.beats.length > 0 && gridSpan > 0) {
    drawBeatGrid(
      ctx,
      beatGrid.beats,
      origin,
      pxPerSec,
      width,
      height,
      dpr,
      gridSpan,
      palette,
    );
  }
  playheadLine(ctx, half, height, dpr, palette.playhead);
}

/** The whole Track drawn across `width`, all played or all unplayed. */
function drawOverview(
  ctx: CanvasRenderingContext2D,
  source: WaveformSource,
  width: number,
  height: number,
  isPlayed: boolean,
) {
  ctx.clearRect(0, 0, width, height);
  const pxPerSec = width / source.duration;
  const bars = new Bars(ctx);
  for (let x = 0; x < width; x++) {
    const [i0, i1] = peakRangeForColumn(
      x,
      pxPerSec,
      source.duration,
      source.length,
    );
    const { index, amp } = loudest(source, i0, i1);
    bar(bars, source.color(index, isPlayed), x, amp, height);
  }
  bars.flush();
}

function offscreen(width: number, height: number) {
  const c = document.createElement("canvas");
  c.width = width;
  c.height = height;
  return c;
}

/**
 * The minimap's Waveform in both colourings, redrawn only when the source or
 * size changes. Each frame blits these and draws the overlays.
 */
export class MinimapCache {
  private key: { source: WaveformSource; w: number; h: number } | null = null;
  private images: {
    unplayed: HTMLCanvasElement;
    played: HTMLCanvasElement;
  } | null = null;

  get(source: WaveformSource, width: number, height: number) {
    const k = this.key;
    if (
      !this.images ||
      !k ||
      k.source !== source ||
      k.w !== width ||
      k.h !== height
    ) {
      const unplayed = offscreen(width, height);
      const played = offscreen(width, height);
      const u = unplayed.getContext("2d");
      const p = played.getContext("2d");
      if (u) drawOverview(u, source, width, height, false);
      if (p) drawOverview(p, source, width, height, true);
      this.images = { unplayed, played };
      this.key = { source, w: width, h: height };
    }
    return this.images;
  }
}

export interface MinimapFrame {
  source: WaveformSource | null;
  /** Seconds the minimap spans. */
  duration: number;
  width: number;
  height: number;
  dpr: number;
  position: number;
  /** The span the zoomed view shows, in seconds. */
  window: { start: number; end: number };
  palette: WaveformPalette;
  cache: MinimapCache;
}

/** The minimap: the whole Track, played progress and the visible window. */
export function drawMinimap(
  ctx: CanvasRenderingContext2D,
  {
    source,
    duration,
    width,
    height,
    dpr,
    position,
    window,
    palette,
    cache,
  }: MinimapFrame,
) {
  ctx.clearRect(0, 0, width, height);
  if (!(duration > 0)) return;
  const playX = Math.min(width, timeToMinimapX(position, width, duration));
  if (source) {
    const images = cache.get(source, width, height);
    ctx.drawImage(images.unplayed, 0, 0);
    const w = Math.floor(playX);
    if (w > 0) ctx.drawImage(images.played, 0, 0, w, height, 0, 0, w, height);
  }

  // The zoomed view's window, clipped to the minimap.
  const x0 = Math.max(0, timeToMinimapX(window.start, width, duration));
  const x1 = Math.min(width, timeToMinimapX(window.end, width, duration));
  if (x1 > x0) {
    const edge = Math.max(1, Math.round(dpr));
    ctx.fillStyle = palette.window;
    ctx.globalAlpha = WINDOW_ALPHA;
    ctx.fillRect(x0, 0, x1 - x0, height);
    ctx.globalAlpha = WINDOW_EDGE_ALPHA;
    ctx.fillRect(x0, 0, edge, height);
    ctx.fillRect(x1 - edge, 0, edge, height);
    ctx.globalAlpha = 1;
  }

  playheadLine(ctx, playX, height, dpr, palette.playhead);
}
