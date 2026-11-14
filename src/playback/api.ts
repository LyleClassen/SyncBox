// Typed wrappers for the Rust playback commands (src-tauri/src/playback/commands.rs).
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { UnplayableReason } from "../library/api";

export type PlaybackStatus =
  | "idle"
  | "loading"
  | "paused"
  | "playing"
  | "ended";

export interface PlayerState {
  trackId: string | null;
  status: PlaybackStatus;
  /** Seconds. */
  position: number;
  /** Seconds; 0 until a Track has loaded. */
  duration: number;
}

export interface PositionUpdate {
  trackId: string;
  position: number;
}

export type PlaybackErrorKind =
  | "unplayable"
  | "unknownTrack"
  | "open"
  | "decode"
  | "output"
  | "seek"
  | "localDb"
  | "engineStopped";

export interface PlaybackError {
  kind: PlaybackErrorKind;
  message: string;
  /** Set when `kind` is "unplayable". */
  reason: UnplayableReason | null;
}

export function asPlaybackError(err: unknown): PlaybackError {
  if (err && typeof err === "object" && "kind" in err && "message" in err) {
    const e = err as PlaybackError;
    return { ...e, reason: e.reason ?? null };
  }
  return { kind: "engineStopped", message: String(err), reason: null };
}

/** Resolves `true` once loaded, or `false` if a newer load replaced it. */
export const load = (trackId: string) =>
  invoke<boolean>("playback_load", { trackId });
export const play = () => invoke<void>("playback_play");
export const pause = () => invoke<void>("playback_pause");
export const toggle = () => invoke<void>("playback_toggle");
export const seek = (seconds: number) =>
  invoke<void>("playback_seek", { seconds });
export const setVolume = (volume: number) =>
  invoke<void>("playback_set_volume", { volume });
export const getState = () => invoke<PlayerState>("playback_state");

export const onState = (
  handler: (state: PlayerState) => void,
): Promise<UnlistenFn> =>
  listen<PlayerState>("playback-state", (e) => handler(e.payload));

export const onPosition = (
  handler: (update: PositionUpdate) => void,
): Promise<UnlistenFn> =>
  listen<PositionUpdate>("playback-position", (e) => handler(e.payload));
