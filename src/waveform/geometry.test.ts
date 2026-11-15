import { describe, expect, test } from "bun:test";
import {
  extrapolate,
  JITTER_SEC,
  MAX_EXTRAPOLATE_MS,
  maxAbs,
  minimapXToTime,
  normaliser,
  peakRangeForColumn,
  steady,
  timeToMinimapX,
  timeToZoomedX,
  visibleWindow,
  zoomedXToTime,
} from "./geometry";

const PEAKS = 8000;

describe("zoomed view: fixed centre playhead", () => {
  const width = 1000;

  test("the playhead is the centre", () => {
    expect(zoomedXToTime(500, width, 0, 50, 180)).toBe(0);
    expect(zoomedXToTime(500, width, 180, 50, 180)).toBe(180);
    expect(timeToZoomedX(42, width, 42, 50)).toBe(500);
  });

  test("at 0:00 the left half is padding", () => {
    expect(timeToZoomedX(0, width, 0, 50)).toBe(500);
    // Clicking the padding seeks to the start.
    expect(zoomedXToTime(100, width, 0, 50, 180)).toBe(0);
    expect(zoomedXToTime(750, width, 0, 50, 180)).toBe(5);
  });

  test("at the end the right half is padding", () => {
    expect(timeToZoomedX(180, width, 180, 50)).toBe(500);
    expect(zoomedXToTime(900, width, 180, 50, 180)).toBe(180);
    expect(zoomedXToTime(250, width, 180, 50, 180)).toBe(175);
  });

  test("x and time round-trip", () => {
    for (const zoom of [10, 50, 500]) {
      const x = timeToZoomedX(61.5, width, 60, zoom);
      expect(zoomedXToTime(x, width, 60, zoom, 180)).toBeCloseTo(61.5);
    }
  });
});

describe("visibleWindow", () => {
  test("half a view each side of the playhead", () => {
    expect(visibleWindow(60, 1000, 50)).toEqual({ start: 50, end: 70 });
    expect(visibleWindow(60, 1000, 500)).toEqual({ start: 59, end: 61 });
  });

  test("runs past the ends, since the padding is shown", () => {
    expect(visibleWindow(0, 1000, 50)).toEqual({ start: -10, end: 10 });
    expect(visibleWindow(180, 1000, 50)).toEqual({ start: 170, end: 190 });
  });
});

describe("peakRangeForColumn", () => {
  const covered = (zoom: number, duration: number) => {
    const columns = Math.ceil(duration * zoom);
    const seen = new Uint8Array(PEAKS);
    let prevEnd = 0;
    let isValid = true;
    for (let c = 0; c < columns; c++) {
      const [i0, i1] = peakRangeForColumn(c, zoom, duration, PEAKS);
      // Non-empty, in range, and moving forwards without skipping any Peak.
      if (!(i1 > i0 && i0 >= 0 && i1 <= PEAKS && i0 <= prevEnd)) {
        isValid = false;
      }
      prevEnd = i1;
      for (let i = i0; i < i1; i++) seen[i] = 1;
    }
    return isValid && seen.every((v) => v === 1);
  };

  test("every Peak is drawn for a 3-minute Track", () => {
    for (const zoom of [10, 50, 500]) expect(covered(zoom, 180)).toBe(true);
  });

  test("every Peak is drawn for a 2-hour mix", () => {
    for (const zoom of [10, 50, 500]) expect(covered(zoom, 7200)).toBe(true);
  });

  test("zoomed out, a column spans many Peaks", () => {
    // 3 minutes at 10 px/s is 1,800 columns for 8,000 Peaks.
    const [i0, i1] = peakRangeForColumn(0, 10, 180, PEAKS);
    expect(i0).toBe(0);
    expect(i1).toBe(5);
  });

  test("zoomed in, a Peak spans many columns", () => {
    // 2 hours at 500 px/s is 450 columns per Peak.
    expect(peakRangeForColumn(0, 500, 7200, PEAKS)).toEqual([0, 1]);
    expect(peakRangeForColumn(449, 500, 7200, PEAKS)).toEqual([0, 1]);
    expect(peakRangeForColumn(450, 500, 7200, PEAKS)).toEqual([1, 2]);
  });

  test("clamped to the data", () => {
    expect(peakRangeForColumn(1e9, 50, 180, PEAKS)).toEqual([PEAKS - 1, PEAKS]);
    expect(peakRangeForColumn(0, 50, 0, PEAKS)).toEqual([0, 0]);
  });
});

describe("maxAbs and normaliser", () => {
  test("loudest of either sign", () => {
    expect(maxAbs([0.1, -0.6, 0.4], 0, 3)).toBe(0.6);
    expect(maxAbs([0.1, -0.6, 0.4], 2, 3)).toBe(0.4);
  });

  test("scales the loudest Peak to 1", () => {
    expect(normaliser([0.25, -0.5])).toBe(2);
    expect(normaliser([0, 0])).toBe(1);
  });
});

describe("minimap", () => {
  test("spans the whole Track", () => {
    expect(minimapXToTime(0, 800, 200)).toBe(0);
    expect(minimapXToTime(400, 800, 200)).toBe(100);
    expect(minimapXToTime(900, 800, 200)).toBe(200);
    expect(minimapXToTime(-5, 800, 200)).toBe(0);
    expect(timeToMinimapX(50, 800, 200)).toBe(200);
  });

  test("nothing loaded", () => {
    expect(minimapXToTime(400, 800, 0)).toBe(0);
    expect(timeToMinimapX(5, 800, 0)).toBe(0);
  });
});

describe("extrapolate", () => {
  test("moves on by wall time while playing", () => {
    expect(extrapolate(10, 1000, 1020, true, 180)).toBeCloseTo(10.02);
  });

  test("holds still unless playing", () => {
    expect(extrapolate(10, 1000, 1020, false, 180)).toBe(10);
  });

  test("never more than the cap past the last event", () => {
    expect(extrapolate(10, 1000, 5000, true, 180)).toBeCloseTo(
      10 + MAX_EXTRAPOLATE_MS / 1000,
    );
  });

  test("never before the last event or past the end", () => {
    expect(extrapolate(10, 1000, 900, true, 180)).toBe(10);
    expect(extrapolate(179.99, 1000, 1050, true, 180)).toBe(180);
  });
});

describe("steady", () => {
  test("ignores small backward jitter while playing", () => {
    expect(steady(10.03, 10.01, true)).toBe(10.03);
  });

  test("lets real jumps and paused changes through", () => {
    expect(steady(10, 10 - JITTER_SEC - 0.01, true)).toBeCloseTo(9.94);
    expect(steady(10.03, 10.01, false)).toBe(10.01);
    expect(steady(10, 10.02, true)).toBe(10.02);
  });
});
