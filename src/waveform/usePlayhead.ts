// A smooth playhead from Rust's ~30 Hz position events. Every event resets
// the base; between events, while playing, the playhead moves on by wall
// time (capped), so it doesn't step at high zoom. There is no webview clock.
import { useCallback, useLayoutEffect, useRef } from "react";
import type { PlaybackStatus } from "../playback/api";
import { extrapolate, steady } from "./geometry";

export function usePlayhead(
  position: number,
  status: PlaybackStatus,
  duration: number,
) {
  const base = useRef({ pos: position, at: 0, isPlaying: false, duration });
  const shown = useRef(position);

  useLayoutEffect(() => {
    base.current = {
      pos: position,
      at: performance.now(),
      isPlaying: status === "playing",
      duration,
    };
    // A pause, seek or new Track shows exactly what Rust reported.
    if (status !== "playing") shown.current = position;
  }, [position, status, duration]);

  /** The playhead in seconds at `now` (a `performance.now()` time). */
  return useCallback((now: number) => {
    const b = base.current;
    const next = extrapolate(b.pos, b.at, now, b.isPlaying, b.duration);
    shown.current = steady(shown.current, next, b.isPlaying);
    return shown.current;
  }, []);
}
