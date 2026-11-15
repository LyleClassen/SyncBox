import { describe, expect, test } from "bun:test";
import {
  clampZoom,
  DEFAULT_ZOOM,
  MAX_ZOOM,
  MIN_ZOOM,
  parseZoom,
  zoomIn,
  zoomOut,
} from "./zoom";

describe("clampZoom", () => {
  test("keeps 10–500 px/s", () => {
    expect(clampZoom(5)).toBe(MIN_ZOOM);
    expect(clampZoom(1000)).toBe(MAX_ZOOM);
    expect(clampZoom(120)).toBe(120);
  });

  test("bad input is the default", () => {
    expect(clampZoom(Number.NaN)).toBe(DEFAULT_ZOOM);
  });
});

describe("zoomIn / zoomOut", () => {
  test("step by 1.5", () => {
    expect(zoomIn(50)).toBe(75);
    expect(zoomOut(75)).toBe(50);
  });

  test("stop at the limits", () => {
    expect(zoomIn(400)).toBe(MAX_ZOOM);
    expect(zoomIn(MAX_ZOOM)).toBe(MAX_ZOOM);
    expect(zoomOut(12)).toBe(MIN_ZOOM);
    expect(zoomOut(MIN_ZOOM)).toBe(MIN_ZOOM);
  });
});

describe("parseZoom", () => {
  test("keeps a stored value", () => {
    expect(parseZoom("75")).toBe(75);
    expect(parseZoom("112.5")).toBe(112.5);
  });

  test("clamps out-of-range values", () => {
    expect(parseZoom("2")).toBe(MIN_ZOOM);
    expect(parseZoom("9000")).toBe(MAX_ZOOM);
  });

  test("falls back when missing or invalid", () => {
    expect(parseZoom(null)).toBe(DEFAULT_ZOOM);
    expect(parseZoom("")).toBe(DEFAULT_ZOOM);
    expect(parseZoom("  ")).toBe(DEFAULT_ZOOM);
    expect(parseZoom("close")).toBe(DEFAULT_ZOOM);
    expect(parseZoom("Infinity")).toBe(DEFAULT_ZOOM);
  });
});
