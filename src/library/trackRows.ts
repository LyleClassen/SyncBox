// Turns Local DB Tracks into the rows the track table shows, and filters and
// sorts them. Every column value is a ready-made string so the table can use
// its default cell renderer, which keeps row virtualization on.
import type {
  Track,
  UnplayableKind,
  UnplayableReason,
  UnplayableReasons,
} from "./api";
import { toCamelot } from "./camelot";

export interface TrackTableRow extends Record<string, unknown> {
  id: string;
  /** Position in the current list (playlist order), 1-based, as text. */
  position: string;
  title: string;
  artist: string;
  album: string;
  bpm: string;
  /** Camelot notation. */
  key: string;
  /** Rekordbox key name, for the tooltip. */
  keyName: string;
  length: string;
  bitrate: string;
  rating: string;
  /** Short Unplayable Reason label, or "" when playable. */
  status: string;
  unplayable: UnplayableReason | null;
  /** Raw values, for sorting. */
  sortValues: {
    position: number;
    bpm: number | null;
    length: number | null;
    bitrate: number | null;
    rating: number | null;
    /** Camelot wheel order: 1A, 1B, 2A ... */
    key: number | null;
  };
}

export function formatLength(seconds: number | null): string {
  if (seconds == null || seconds < 0) return "";
  const s = Math.round(seconds);
  return `${Math.floor(s / 60)}:${String(s % 60).padStart(2, "0")}`;
}

export function formatBpm(bpm: number | null): string {
  if (bpm == null || bpm <= 0) return "";
  return Number.isInteger(bpm) ? String(bpm) : bpm.toFixed(2);
}

/** Rekordbox stores a 0-5 star rating. */
export function formatRating(rating: number | null): string {
  if (rating == null || rating <= 0) return "";
  return "★".repeat(Math.min(5, Math.round(rating)));
}

const STATUS_LABELS: Record<UnplayableKind, string> = {
  streaming: "Streaming",
  volumeOffline: "Offline",
  missing: "Missing",
  noPath: "No file",
};

export function statusLabel(reason: UnplayableReason | null): string {
  return reason ? STATUS_LABELS[reason.kind] : "";
}

function camelotOrder(camelot: string): number | null {
  const m = /^(\d{1,2})([AB])$/.exec(camelot);
  if (!m) return null;
  return Number(m[1]) * 2 + (m[2] === "B" ? 1 : 0);
}

export function toTableRows(
  tracks: Track[],
  reasons: UnplayableReasons = {},
): TrackTableRow[] {
  return tracks.map((t, i) => {
    const key = toCamelot(t.key);
    const unplayable = reasons[t.id] ?? null;
    return {
      id: t.id,
      position: String(i + 1),
      title: t.title,
      artist: t.artist,
      album: t.album ?? "",
      bpm: formatBpm(t.bpm),
      key,
      keyName: t.key,
      length: formatLength(t.length),
      bitrate: t.bitrate != null && t.bitrate > 0 ? String(t.bitrate) : "",
      rating: formatRating(t.rating),
      status: statusLabel(unplayable),
      unplayable,
      sortValues: {
        position: i + 1,
        bpm: t.bpm,
        length: t.length,
        bitrate: t.bitrate,
        rating: t.rating,
        key: camelotOrder(key),
      },
    };
  });
}

/**
 * Keeps rows where every word of `query` appears in the title, artist or
 * album (case-insensitive).
 */
export function filterRows(
  rows: TrackTableRow[],
  query: string,
): TrackTableRow[] {
  const terms = query.toLowerCase().split(/\s+/).filter(Boolean);
  if (terms.length === 0) return rows;
  return rows.filter((r) => {
    const haystack = `${r.title}\n${r.artist}\n${r.album}`.toLowerCase();
    return terms.every((term) => haystack.includes(term));
  });
}

export type SortKey =
  | "position"
  | "title"
  | "artist"
  | "album"
  | "bpm"
  | "key"
  | "length"
  | "bitrate"
  | "rating"
  | "status";

export interface SortEntry {
  sortKey: SortKey;
  direction: "ascending" | "descending";
}

const collator = new Intl.Collator(undefined, {
  sensitivity: "base",
  numeric: true,
});

type NumericKey = keyof TrackTableRow["sortValues"];
const NUMERIC_KEYS = new Set<string>([
  "position",
  "bpm",
  "length",
  "bitrate",
  "rating",
  "key",
]);

function compare(a: TrackTableRow, b: TrackTableRow, key: SortKey): number {
  if (NUMERIC_KEYS.has(key)) {
    const av = a.sortValues[key as NumericKey];
    const bv = b.sortValues[key as NumericKey];
    if (av == null && bv == null) return 0;
    // Empty values sort last in ascending order.
    if (av == null) return 1;
    if (bv == null) return -1;
    return av - bv;
  }
  type TextKey = "title" | "artist" | "album" | "status";
  const av = a[key as TextKey];
  const bv = b[key as TextKey];
  if (!av && !bv) return 0;
  if (!av) return 1;
  if (!bv) return -1;
  return collator.compare(av, bv);
}

/** Sorts a copy of `rows`; ties fall back to list position. */
export function sortRows(
  rows: TrackTableRow[],
  sort: SortEntry[],
): TrackTableRow[] {
  if (sort.length === 0) return rows;
  return [...rows].sort((a, b) => {
    for (const { sortKey, direction } of sort) {
      const c = compare(a, b, sortKey);
      if (c !== 0) return direction === "ascending" ? c : -c;
    }
    return a.sortValues.position - b.sortValues.position;
  });
}
