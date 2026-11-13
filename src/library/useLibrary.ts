// Library state for the main window: the Playlist tree, the selected list and
// its Tracks, and Sync (manual, plus one automatic Sync when the Local DB is
// empty).
import { useCallback, useEffect, useRef, useState } from "react";
import {
  getAllTracks,
  getPlaylistTracks,
  getPlaylistTree,
  onSyncProgress,
  type PlaylistNode,
  type SyncError,
  type SyncOutcome,
  type SyncProgress,
  syncFromRekordbox,
  syncIfEmpty,
  type Track,
} from "./api";

/** Sidebar id for the All Tracks entry. Never a Rekordbox playlist id. */
export const ALL_TRACKS_ID = "syncbox:all-tracks";

const SELECTED_KEY = "syncbox.selectedList";

function readSelected(): string {
  try {
    return localStorage.getItem(SELECTED_KEY) ?? ALL_TRACKS_ID;
  } catch {
    return ALL_TRACKS_ID;
  }
}

function writeSelected(id: string) {
  try {
    localStorage.setItem(SELECTED_KEY, id);
  } catch {
    // Remembering the selection is a convenience only.
  }
}

function containsList(nodes: PlaylistNode[], id: string): boolean {
  return nodes.some(
    (n) => (n.id === id && n.kind !== "folder") || containsList(n.children, id),
  );
}

function asSyncError(err: unknown): SyncError {
  if (err && typeof err === "object" && "kind" in err && "message" in err) {
    return err as SyncError;
  }
  return { kind: "unreadable", message: String(err) };
}

export type SyncState =
  | { status: "idle" }
  | { status: "running"; progress: SyncProgress | null }
  | { status: "error"; error: SyncError };

export function useLibrary() {
  const [tree, setTree] = useState<PlaylistNode[]>([]);
  const [selectedId, setSelectedId] = useState(readSelected);
  const [tracks, setTracks] = useState<Track[]>([]);
  const [isLoadingTracks, setIsLoadingTracks] = useState(true);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [sync, setSync] = useState<SyncState>({ status: "idle" });
  // Bumped after each Sync so the selected list reloads.
  const [generation, setGeneration] = useState(0);
  const isSyncing = useRef(false);

  const select = useCallback((id: string) => {
    setSelectedId(id);
    writeSelected(id);
  }, []);

  const applyOutcome = useCallback((outcome: SyncOutcome) => {
    setTree(outcome.tree);
    setSelectedId((current) => {
      if (current === ALL_TRACKS_ID || containsList(outcome.tree, current)) {
        return current;
      }
      writeSelected(ALL_TRACKS_ID);
      return ALL_TRACKS_ID;
    });
    setGeneration((g) => g + 1);
  }, []);

  const runSync = useCallback(
    async (start: () => Promise<SyncOutcome | null>) => {
      if (isSyncing.current) return;
      isSyncing.current = true;
      setSync({ status: "running", progress: null });
      const unlisten = await onSyncProgress((progress) =>
        setSync({ status: "running", progress }),
      );
      try {
        const outcome = await start();
        if (outcome) applyOutcome(outcome);
        setSync({ status: "idle" });
      } catch (err) {
        setSync({ status: "error", error: asSyncError(err) });
      } finally {
        unlisten();
        isSyncing.current = false;
      }
    },
    [applyOutcome],
  );

  const startSync = useCallback(() => runSync(syncFromRekordbox), [runSync]);

  const dismissSyncError = useCallback(() => setSync({ status: "idle" }), []);

  // On launch: show what the Local DB has, then Sync once if it's empty.
  useEffect(() => {
    getPlaylistTree()
      .then(setTree)
      .catch((e) => setLoadError(String(e)));
    runSync(syncIfEmpty);
  }, [runSync]);

  // biome-ignore lint/correctness/useExhaustiveDependencies: `generation` reloads the list after a Sync.
  useEffect(() => {
    let cancelled = false;
    setIsLoadingTracks(true);
    const load =
      selectedId === ALL_TRACKS_ID
        ? getAllTracks()
        : getPlaylistTracks(selectedId);
    load
      .then((result) => {
        if (cancelled) return;
        setTracks(result);
        setLoadError(null);
      })
      .catch((e) => {
        if (cancelled) return;
        setTracks([]);
        setLoadError(String(e));
      })
      .finally(() => {
        if (!cancelled) setIsLoadingTracks(false);
      });
    return () => {
      cancelled = true;
    };
  }, [selectedId, generation]);

  return {
    tree,
    selectedId,
    select,
    tracks,
    isLoadingTracks,
    loadError,
    sync,
    startSync,
    dismissSyncError,
  };
}
