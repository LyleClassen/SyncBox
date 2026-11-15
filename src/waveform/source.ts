// Where the renderer gets amplitude and colour. It only ever asks a
// WaveformSource, so an ANLZ colour Waveform can be added later as another
// source without touching the drawing code.
import type { WaveformData } from "./api";
import { normaliser } from "./geometry";
import type { WaveformPalette } from "./palette";

export interface WaveformSource {
  /** Seconds the data spans. */
  duration: number;
  /** Number of columns of amplitude data, spread evenly over `duration`. */
  length: number;
  /** 0–1, already normalised. */
  amplitude(i: number): number;
  /** CSS colour for column `i`, played (left of the playhead) or not. */
  color(i: number, isPlayed: boolean): string;
}

/**
 * The stored Peaks, in the theme's played and unplayed colours. Null until
 * Peaks are computed.
 */
export function peaksSource(
  data: WaveformData,
  palette: WaveformPalette,
  fallbackDuration: number,
): WaveformSource | null {
  const { peaks } = data;
  const duration = data.duration ?? fallbackDuration;
  if (!peaks || peaks.length === 0 || !(duration > 0)) return null;
  const scale = normaliser(peaks);
  const amp = Float32Array.from(peaks, (p) => Math.min(1, Math.abs(p) * scale));
  return {
    duration,
    length: amp.length,
    amplitude: (i) => amp[i] ?? 0,
    color: (_i, isPlayed) => (isPlayed ? palette.played : palette.unplayed),
  };
}
