// Player state for the main window. Rust owns playback; this mirrors its
// events and sends commands. Volume is remembered between sessions.
import { useCallback, useEffect, useRef, useState } from "react";
import * as api from "./api";
import { parseVolume } from "./time";

const VOLUME_KEY = "syncbox.volume";
const DEFAULT_VOLUME = 0.8;

function readVolume(): number {
  try {
    return parseVolume(localStorage.getItem(VOLUME_KEY), DEFAULT_VOLUME);
  } catch {
    return DEFAULT_VOLUME;
  }
}

function writeVolume(volume: number) {
  try {
    localStorage.setItem(VOLUME_KEY, String(volume));
  } catch {
    // Remembering the volume is a convenience only.
  }
}

const IDLE: api.PlayerState = {
  trackId: null,
  status: "idle",
  position: 0,
  duration: 0,
};

/** Runs a transport command, reporting failures through `onError`. */
function run(
  command: Promise<unknown>,
  onError: (e: api.PlaybackError) => void,
) {
  command.catch((e) => onError(api.asPlaybackError(e)));
}

export function usePlayback(onError: (e: api.PlaybackError) => void) {
  const [state, setState] = useState<api.PlayerState>(IDLE);
  const [position, setPosition] = useState(0);
  const [volume, setVolumeState] = useState(readVolume);
  const trackIdRef = useRef<string | null>(null);
  const onErrorRef = useRef(onError);
  onErrorRef.current = onError;
  const report = useCallback(
    (e: api.PlaybackError) => onErrorRef.current(e),
    [],
  );

  useEffect(() => {
    let disposed = false;
    const apply = (next: api.PlayerState) => {
      trackIdRef.current = next.trackId;
      setState(next);
      setPosition(next.position);
    };
    const subscriptions = [
      api.onState(apply),
      api.onPosition((u) => {
        if (u.trackId === trackIdRef.current) setPosition(u.position);
      }),
    ];
    // The engine may already have a Track (e.g. after a webview reload).
    api
      .getState()
      .then((s) => !disposed && apply(s))
      .catch(() => {});
    return () => {
      disposed = true;
      for (const s of subscriptions) s.then((unlisten) => unlisten());
    };
  }, []);

  // Send the remembered volume once, before anything plays.
  // biome-ignore lint/correctness/useExhaustiveDependencies: startup only.
  useEffect(() => run(api.setVolume(volume), report), []);

  const setVolume = useCallback(
    (v: number) => {
      setVolumeState(v);
      writeVolume(v);
      run(api.setVolume(v), report);
    },
    [report],
  );

  const seek = useCallback(
    (seconds: number) => {
      setPosition(seconds);
      run(api.seek(seconds), report);
    },
    [report],
  );

  const toggle = useCallback(() => run(api.toggle(), report), [report]);

  /**
   * Loads a Track. Resolves `true` once it's loaded, `false` if a newer load
   * replaced it, and rejects with a {@link api.PlaybackError}.
   */
  const load = useCallback(async (trackId: string) => {
    try {
      return await api.load(trackId);
    } catch (e) {
      throw api.asPlaybackError(e);
    }
  }, []);

  return { state, position, volume, setVolume, seek, toggle, load };
}

export type Playback = ReturnType<typeof usePlayback>;
