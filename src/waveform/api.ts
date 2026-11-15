// Typed wrappers for the Rust Waveform command (src-tauri/src/waveform/commands.rs).
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export interface Cue {
  id: string;
  /** Seconds. */
  positionSec: number;
  /** Seconds; set when the Cue is a loop. */
  endSec: number | null;
  /** 0 = memory Cue, 1–8 = hot Cue slot A–H. */
  kind: number;
  /** `#RRGGBB`, if Rekordbox gave the Cue a colour. */
  color: string | null;
  comment: string | null;
}

export interface WaveformData {
  /** 8,000 signed values in [-1, 1]; null until computed on first load. */
  peaks: number[] | null;
  /** Seconds, from decoding the file; null until Peaks are computed. */
  duration: number | null;
  /** Always null until the Beat Grid exists. */
  beatGrid: null;
  cues: Cue[];
}

export interface PeaksReady {
  trackId: string;
}

export interface PeaksFailed {
  trackId: string;
  message: string;
}

export const getWaveformData = (trackId: string) =>
  invoke<WaveformData>("get_waveform_data", { trackId });

export const onPeaksReady = (
  handler: (e: PeaksReady) => void,
): Promise<UnlistenFn> =>
  listen<PeaksReady>("waveform-peaks-ready", (e) => handler(e.payload));

export const onPeaksFailed = (
  handler: (e: PeaksFailed) => void,
): Promise<UnlistenFn> =>
  listen<PeaksFailed>("waveform-peaks-failed", (e) => handler(e.payload));
