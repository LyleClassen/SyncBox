import { describe, expect, test } from "bun:test";
import type { WaveformData } from "./api";
import { isForTrack, statusOf } from "./status";

const data = (peaks: number[] | null): WaveformData => ({
  peaks,
  duration: peaks ? 1 : null,
  beatGrid: null,
  cues: [],
});

describe("statusOf", () => {
  test("ready once Peaks are stored", () => {
    expect(statusOf(data([0, 0.5]))).toBe("ready");
  });

  test("still loading without Peaks", () => {
    expect(statusOf(data(null))).toBe("loading");
  });

  test("a reported failure stands until Peaks arrive", () => {
    expect(statusOf(data(null), "failed")).toBe("failed");
    expect(statusOf(data([0]), "failed")).toBe("ready");
  });
});

describe("isForTrack", () => {
  test("matches the current Track only", () => {
    expect(isForTrack("t1", "t1")).toBe(true);
    expect(isForTrack("t1", "t2")).toBe(false);
    expect(isForTrack("t1", null)).toBe(false);
  });
});
