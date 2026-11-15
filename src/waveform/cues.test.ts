import { describe, expect, test } from "bun:test";
import type { Cue } from "./api";
import {
  cueColor,
  cueLabel,
  cueTitle,
  HOT_CUE_DEFAULT_COLOR,
  isHotCue,
  MEMORY_CUE_COLOR,
  visibleCueX,
} from "./cues";

const cue = (over: Partial<Cue> = {}): Cue => ({
  id: "c",
  positionSec: 10,
  endSec: null,
  kind: 1,
  color: null,
  comment: null,
  ...over,
});

describe("labels", () => {
  test("memory Cues have no label", () => {
    expect(cueLabel(cue({ kind: 0 }))).toBeNull();
    expect(isHotCue(cue({ kind: 0 }))).toBe(false);
  });

  test("hot Cue slots 1–8 are A–H", () => {
    expect(cueLabel(cue({ kind: 1 }))).toBe("A");
    expect(cueLabel(cue({ kind: 8 }))).toBe("H");
  });

  test("any other hot Cue is a dot", () => {
    expect(cueLabel(cue({ kind: 9 }))).toBe("•");
  });
});

describe("colours", () => {
  test("a stored colour wins", () => {
    expect(cueColor(cue({ color: "#FF0000" }))).toBe("#FF0000");
    expect(cueColor(cue({ kind: 0, color: "#00FF00" }))).toBe("#00FF00");
  });

  test("uncoloured Cues fall back by kind", () => {
    expect(cueColor(cue({ kind: 3 }))).toBe(HOT_CUE_DEFAULT_COLOR);
    expect(cueColor(cue({ kind: 0 }))).toBe(MEMORY_CUE_COLOR);
  });
});

describe("titles", () => {
  test("without a comment", () => {
    expect(cueTitle(cue({ kind: 1 }))).toBe("Hot Cue A");
    expect(cueTitle(cue({ kind: 0 }))).toBe("Memory Cue");
  });

  test("with a comment", () => {
    expect(cueTitle(cue({ kind: 2, comment: "Drop" }))).toBe(
      "Hot Cue B — Drop",
    );
    expect(cueTitle(cue({ kind: 0, comment: " Intro " }))).toBe(
      "Memory Cue — Intro",
    );
  });

  test("a blank comment is ignored", () => {
    expect(cueTitle(cue({ kind: 1, comment: "" }))).toBe("Hot Cue A");
    expect(cueTitle(cue({ kind: 1, comment: "   " }))).toBe("Hot Cue A");
  });
});

describe("visibleCueX", () => {
  const width = 1000;
  const zoom = 50;

  test("a Cue at the playhead is at the centre", () => {
    expect(visibleCueX(cue({ positionSec: 42 }), width, 42, zoom)).toBe(500);
  });

  test("Cues at the edges are shown", () => {
    expect(visibleCueX(cue({ positionSec: 0 }), width, 10, zoom)).toBe(0);
    expect(visibleCueX(cue({ positionSec: 20 }), width, 10, zoom)).toBe(1000);
    // Within the margin past either edge.
    expect(visibleCueX(cue({ positionSec: 20.25 }), width, 10, zoom)).toBe(
      1012.5,
    );
    expect(visibleCueX(cue({ positionSec: 0 }), width, 10.25, zoom)).toBe(
      -12.5,
    );
  });

  test("Cues beyond the margin are culled", () => {
    expect(visibleCueX(cue({ positionSec: 21 }), width, 10, zoom)).toBeNull();
    expect(visibleCueX(cue({ positionSec: 0 }), width, 11, zoom)).toBeNull();
    expect(visibleCueX(cue({ positionSec: 21 }), width, 10, zoom, 100)).toBe(
      1050,
    );
  });
});
