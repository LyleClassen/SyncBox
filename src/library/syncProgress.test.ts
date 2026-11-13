import { expect, test } from "bun:test";
import { overallProgress } from "./syncProgress";

test("each phase fills a third of the bar", () => {
  expect(overallProgress(null)).toBe(0);
  expect(overallProgress({ phase: "playlists", done: 0, total: 10 })).toBe(0);
  expect(overallProgress({ phase: "tracks", done: 5, total: 10 })).toBe(0.5);
  expect(overallProgress({ phase: "cues", done: 10, total: 10 })).toBe(1);
});

test("an empty phase counts as done", () => {
  expect(
    overallProgress({ phase: "playlists", done: 0, total: 0 }),
  ).toBeCloseTo(1 / 3);
});
