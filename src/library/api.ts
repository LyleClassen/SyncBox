// Typed wrappers for the Rust library commands (src-tauri/src/library/commands.rs).
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export type PlaylistNodeKind = "playlist" | "folder" | "smart";

export interface PlaylistNode {
  id: string;
  name: string;
  kind: PlaylistNodeKind;
  children: PlaylistNode[];
}

export interface Track {
  id: string;
  title: string;
  artist: string;
  album: string | null;
  bpm: number | null;
  /** Rekordbox key name, e.g. "Am", or "". */
  key: string;
  /** Seconds. */
  length: number | null;
  /** kbps. */
  bitrate: number | null;
  rating: number | null;
  filePath: string | null;
}

export type UnplayableKind =
  | "streaming"
  | "volumeOffline"
  | "missing"
  | "noPath";

/** Why a Track can't be loaded (src-tauri/src/library/track_file.rs). */
export interface UnplayableReason {
  kind: UnplayableKind;
  /** The streaming service, volume root or file path, if any. */
  detail: string | null;
  /** Guidance for the user. */
  message: string;
}

/** Unplayable Reasons by Track id. Tracks not in it are playable. */
export type UnplayableReasons = Record<string, UnplayableReason>;

export type SyncPhase = "playlists" | "tracks" | "cues";

export interface SyncProgress {
  phase: SyncPhase;
  done: number;
  total: number;
}

export type SyncErrorKind =
  | "notFound"
  | "locked"
  | "unreadable"
  | "alreadyRunning"
  | "localDb";

export interface SyncError {
  kind: SyncErrorKind;
  message: string;
}

export interface SyncOutcome {
  summary: { playlists: number; tracks: number; cues: number };
  tree: PlaylistNode[];
}

export const syncFromRekordbox = () =>
  invoke<SyncOutcome>("sync_from_rekordbox");

/** Syncs only when the Local DB is empty; call once on launch. */
export const syncIfEmpty = () => invoke<SyncOutcome | null>("sync_if_empty");

export const getPlaylistTree = () =>
  invoke<PlaylistNode[]>("get_playlist_tree");

export const getPlaylistTracks = (playlistId: string) =>
  invoke<Track[]>("get_playlist_tracks", { playlistId });

export const getAllTracks = () => invoke<Track[]>("get_all_tracks");

/** Checks the Tracks' files now (never cached) and returns the unplayable ones. */
export const checkTrackFiles = (trackIds: string[]) =>
  invoke<UnplayableReasons>("check_track_files", { trackIds });

export const onSyncProgress = (
  handler: (progress: SyncProgress) => void,
): Promise<UnlistenFn> =>
  listen<SyncProgress>("sync-progress", (e) => handler(e.payload));
