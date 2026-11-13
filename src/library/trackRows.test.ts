import { describe, expect, test } from "bun:test";
import type { Track } from "./api";
import {
  filterRows,
  formatBpm,
  formatLength,
  formatRating,
  sortRows,
  toTableRows,
} from "./trackRows";

function track(overrides: Partial<Track>): Track {
  return {
    id: "1",
    title: "",
    artist: "",
    album: null,
    bpm: null,
    key: "",
    length: null,
    bitrate: null,
    rating: null,
    filePath: null,
    ...overrides,
  };
}

describe("formatting", () => {
  test("length as m:ss", () => {
    expect(formatLength(null)).toBe("");
    expect(formatLength(0)).toBe("0:00");
    expect(formatLength(65)).toBe("1:05");
    expect(formatLength(3600)).toBe("60:00");
  });

  test("bpm keeps decimals only when present", () => {
    expect(formatBpm(128)).toBe("128");
    expect(formatBpm(127.5)).toBe("127.50");
    expect(formatBpm(null)).toBe("");
    expect(formatBpm(0)).toBe("");
  });

  test("rating as stars", () => {
    expect(formatRating(null)).toBe("");
    expect(formatRating(0)).toBe("");
    expect(formatRating(3)).toBe("★★★");
    expect(formatRating(9)).toBe("★★★★★");
  });

  test("rows carry Camelot key and the Rekordbox key name", () => {
    const [row] = toTableRows([track({ key: "Am", album: null })]);
    expect(row.key).toBe("8A");
    expect(row.keyName).toBe("Am");
    expect(row.album).toBe("");
    expect(row.position).toBe("1");
  });
});

describe("filterRows", () => {
  const rows = toTableRows([
    track({ id: "a", title: "Strobe", artist: "deadmau5", album: "For Lack" }),
    track({ id: "b", title: "Opus", artist: "Eric Prydz", album: "Opus" }),
    track({ id: "c", title: "Ghosts", artist: "deadmau5", album: "4x4=12" }),
  ]);
  const ids = (r: typeof rows) => r.map((x) => x.id);

  test("empty query keeps everything", () => {
    expect(ids(filterRows(rows, "  "))).toEqual(["a", "b", "c"]);
  });

  test("matches title, artist or album, case-insensitively", () => {
    expect(ids(filterRows(rows, "DEADMAU5"))).toEqual(["a", "c"]);
    expect(ids(filterRows(rows, "lack"))).toEqual(["a"]);
    expect(ids(filterRows(rows, "opus"))).toEqual(["b"]);
  });

  test("every word must match", () => {
    expect(ids(filterRows(rows, "deadmau5 ghosts"))).toEqual(["c"]);
    expect(ids(filterRows(rows, "deadmau5 opus"))).toEqual([]);
  });
});

describe("sortRows", () => {
  const rows = toTableRows([
    track({ id: "a", title: "beta", bpm: 128, key: "Am", length: 300 }),
    track({ id: "b", title: "Alpha", bpm: null, key: "C", length: 200 }),
    track({ id: "c", title: "gamma", bpm: 90, key: "", length: 400 }),
  ]);
  const ids = (r: typeof rows) => r.map((x) => x.id);

  test("no sort keeps list order", () => {
    expect(ids(sortRows(rows, []))).toEqual(["a", "b", "c"]);
  });

  test("text sorts ignore case", () => {
    expect(
      ids(sortRows(rows, [{ sortKey: "title", direction: "ascending" }])),
    ).toEqual(["b", "a", "c"]);
  });

  test("numbers sort numerically, empty values last when ascending", () => {
    expect(
      ids(sortRows(rows, [{ sortKey: "bpm", direction: "ascending" }])),
    ).toEqual(["c", "a", "b"]);
    expect(
      ids(sortRows(rows, [{ sortKey: "bpm", direction: "descending" }])),
    ).toEqual(["b", "a", "c"]);
  });

  test("key sorts by Camelot wheel order", () => {
    // 8A (Am) before 8B (C); no key last.
    expect(
      ids(sortRows(rows, [{ sortKey: "key", direction: "ascending" }])),
    ).toEqual(["a", "b", "c"]);
  });

  test("does not mutate its input", () => {
    sortRows(rows, [{ sortKey: "length", direction: "descending" }]);
    expect(ids(rows)).toEqual(["a", "b", "c"]);
  });
});
