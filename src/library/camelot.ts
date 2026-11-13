// Key notation: shows a Rekordbox key name in Camelot notation.
// Ported from mixxxa `shared/camelot.ts` (parsing and Camelot formatting only).

export type Mode = "A" | "B";

export interface CamelotKey {
  /** Wheel number, 1-12. */
  n: number;
  /** 'A' = minor, 'B' = major. */
  mode: Mode;
}

// Natural-note pitch classes; accidentals are applied on top.
const NOTE_BASE: Record<string, number> = {
  c: 0,
  d: 2,
  e: 4,
  f: 5,
  g: 7,
  a: 9,
  b: 11,
};

// Camelot wheel number for each pitch class (0 = C). Enharmonic spellings
// (C#/Db, D#/Eb, ...) collapse to one slot.
const MAJOR_CAMELOT = [8, 3, 10, 5, 12, 7, 2, 9, 4, 11, 6, 1];
const MINOR_CAMELOT = [5, 12, 7, 2, 9, 4, 11, 6, 1, 8, 3, 10];

/** Parse a note like "C", "C#", "Db", "F♯" to a pitch class 0-11, or null. */
function parsePitchClass(note: string): number | null {
  const m = /^([a-g])([#b♯♭]*)$/i.exec(note.trim());
  if (!m) return null;
  let pc = NOTE_BASE[m[1].toLowerCase()];
  for (const ch of m[2]) {
    if (ch === "#" || ch === "♯") pc += 1;
    else if (ch === "b" || ch === "♭") pc -= 1;
  }
  return ((pc % 12) + 12) % 12;
}

/**
 * Recognise a key written in Camelot (`12A`), Open Key (`1m`/`1d`) or musical
 * (`F#m`, `Db`, `A minor`) notation. These forms don't collide. Returns null
 * when the input isn't a key.
 */
export function parseAnyKey(input: string): CamelotKey | null {
  const s = input.trim();
  if (!s) return null;

  let m = /^(\d{1,2})\s*([AB])$/i.exec(s);
  if (m) {
    const n = Number(m[1]);
    if (n >= 1 && n <= 12) return { n, mode: m[2].toUpperCase() as Mode };
  }

  m = /^(\d{1,2})\s*([dm])$/i.exec(s);
  if (m) {
    const openN = Number(m[1]);
    if (openN >= 1 && openN <= 12) {
      return {
        n: ((openN + 6) % 12) + 1,
        mode: m[2].toLowerCase() === "d" ? "B" : "A",
      };
    }
  }

  const mm = /^([a-g][#b♯♭]*)\s*(.*)$/i.exec(s);
  if (mm) {
    const pc = parsePitchClass(mm[1]);
    if (pc != null) {
      const rest = mm[2].trim().toLowerCase();
      if (rest === "" || rest.startsWith("maj")) {
        return { n: MAJOR_CAMELOT[pc], mode: "B" };
      }
      if (rest === "m" || rest.startsWith("min")) {
        return { n: MINOR_CAMELOT[pc], mode: "A" };
      }
    }
  }

  return null;
}

export function formatCamelot(key: CamelotKey): string {
  return `${key.n}${key.mode}`;
}

/**
 * A Rekordbox key name in Camelot notation, e.g. "Am" → "8A". Returns the
 * input unchanged when it can't be parsed, and "" for an empty key.
 */
export function toCamelot(raw: string | null | undefined): string {
  if (!raw) return "";
  const key = parseAnyKey(raw);
  return key ? formatCamelot(key) : raw;
}
