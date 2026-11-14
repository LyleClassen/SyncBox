// Pure helpers for useWaveformData.
import type { WaveformData } from "./api";

export type WaveformStatus = "idle" | "loading" | "ready" | "failed";

/**
 * Status after fetching: ready once Peaks are stored. Otherwise still waiting,
 * unless computing them already failed (`previous`).
 */
export function statusOf(
  data: WaveformData,
  previous: WaveformStatus = "loading",
): WaveformStatus {
  if (data.peaks) return "ready";
  return previous === "failed" ? "failed" : "loading";
}

/** Whether an event about `eventTrackId` concerns the current Track. */
export function isForTrack(
  eventTrackId: string,
  current: string | null,
): boolean {
  return current !== null && eventTrackId === current;
}
