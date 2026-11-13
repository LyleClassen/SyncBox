import { describe, expect, test } from "bun:test";
import { parseAnyKey, toCamelot } from "./camelot";

describe("toCamelot", () => {
  test.each([
    // Minor keys, every wheel slot.
    ["Abm", "1A"],
    ["G#m", "1A"],
    ["Ebm", "2A"],
    ["D#m", "2A"],
    ["Bbm", "3A"],
    ["Fm", "4A"],
    ["Cm", "5A"],
    ["Gm", "6A"],
    ["Dm", "7A"],
    ["Am", "8A"],
    ["Em", "9A"],
    ["Bm", "10A"],
    ["F#m", "11A"],
    ["Gbm", "11A"],
    ["Dbm", "12A"],
    ["C#m", "12A"],
    // Major keys, every wheel slot.
    ["B", "1B"],
    ["F#", "2B"],
    ["Gb", "2B"],
    ["Db", "3B"],
    ["C#", "3B"],
    ["Ab", "4B"],
    ["Eb", "5B"],
    ["Bb", "6B"],
    ["F", "7B"],
    ["C", "8B"],
    ["G", "9B"],
    ["D", "10B"],
    ["A", "11B"],
    ["E", "12B"],
  ])("%s → %s", (rekordbox, camelot) => {
    expect(toCamelot(rekordbox)).toBe(camelot);
  });

  test("accepts unicode accidentals and spelled-out modes", () => {
    expect(toCamelot("F♯m")).toBe("11A");
    expect(toCamelot("B♭")).toBe("6B");
    expect(toCamelot("A minor")).toBe("8A");
    expect(toCamelot("C major")).toBe("8B");
  });

  test("keys already in Camelot or Open Key notation", () => {
    expect(toCamelot("8A")).toBe("8A");
    expect(toCamelot("12b")).toBe("12B");
    expect(toCamelot("1m")).toBe("8A");
    expect(toCamelot("1d")).toBe("8B");
  });

  test("empty and unknown keys", () => {
    expect(toCamelot("")).toBe("");
    expect(toCamelot(null)).toBe("");
    expect(toCamelot("???")).toBe("???");
    expect(toCamelot("13A")).toBe("13A");
    expect(parseAnyKey("Cx")).toBeNull();
  });
});
