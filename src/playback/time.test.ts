import { describe, expect, test } from "bun:test";
import { formatTime, parseVolume } from "./time";

describe("formatTime", () => {
  test("minutes and seconds", () => {
    expect(formatTime(0)).toBe("0:00");
    expect(formatTime(5.9)).toBe("0:05");
    expect(formatTime(245)).toBe("4:05");
  });

  test("hours from an hour up", () => {
    expect(formatTime(3600)).toBe("1:00:00");
    expect(formatTime(3725)).toBe("1:02:05");
  });

  test("bad input shows zero", () => {
    expect(formatTime(-3)).toBe("0:00");
    expect(formatTime(Number.NaN)).toBe("0:00");
    expect(formatTime(Number.POSITIVE_INFINITY)).toBe("0:00");
  });
});

describe("parseVolume", () => {
  test("keeps a stored value in range", () => {
    expect(parseVolume("0.35", 0.8)).toBe(0.35);
    expect(parseVolume("0", 0.8)).toBe(0);
    expect(parseVolume("1", 0.8)).toBe(1);
  });

  test("falls back when missing or invalid", () => {
    expect(parseVolume(null, 0.8)).toBe(0.8);
    expect(parseVolume("", 0.8)).toBe(0.8);
    expect(parseVolume("loud", 0.8)).toBe(0.8);
    expect(parseVolume("1.5", 0.8)).toBe(0.8);
    expect(parseVolume("-1", 0.8)).toBe(0.8);
  });
});
