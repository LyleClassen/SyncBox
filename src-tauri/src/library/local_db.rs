//! The Local DB: SyncBox's own SQLite database (ADR-0001).
//!
//! Tables are split by who owns the data (ADR-0003):
//! - `rb_*` tables hold only data copied from the Rekordbox Library. A Sync
//!   replaces them wholesale.
//! - `sb_*` tables hold data SyncBox computes (Peaks, Beat Grid…). Each has a
//!   `track_id` column holding the Rekordbox Track id. A Sync never touches
//!   them, except to remove rows whose Track no longer exists in Rekordbox.
//!   New `sb_*` tables are picked up automatically; there is no list to update.
//!   `sb_peaks` holds each Track's Peaks.

use std::path::Path;

use rusqlite::{Connection, Transaction, params};
use serde::Serialize;

use super::snapshot::{PlaylistKind, Snapshot};
use super::sync::{Phase, Progress};

const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS rb_playlist (
    id        TEXT PRIMARY KEY,
    name      TEXT NOT NULL,
    kind      INTEGER NOT NULL,   -- 0 playlist, 1 folder, 4 smart
    parent_id TEXT,               -- NULL for top-level nodes
    seq       INTEGER NOT NULL
);
CREATE TABLE IF NOT EXISTS rb_playlist_entry (
    playlist_id TEXT NOT NULL,
    track_id    TEXT NOT NULL,
    seq         INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS rb_playlist_entry_playlist ON rb_playlist_entry (playlist_id, seq);
CREATE TABLE IF NOT EXISTS rb_track (
    id         TEXT PRIMARY KEY,
    title      TEXT,
    artist_id  TEXT,
    album_id   TEXT,
    key_id     TEXT,
    bpm        REAL,
    length_sec INTEGER,
    bitrate    INTEGER,
    rating     INTEGER,
    file_path  TEXT
);
CREATE TABLE IF NOT EXISTS rb_artist (id TEXT PRIMARY KEY, name TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS rb_album  (id TEXT PRIMARY KEY, name TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS rb_key    (id TEXT PRIMARY KEY, name TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS rb_cue (
    id           TEXT PRIMARY KEY,
    track_id     TEXT NOT NULL,
    position_sec REAL NOT NULL,
    end_sec      REAL,
    kind         INTEGER NOT NULL,  -- 0 memory, 1-8 hot cue A-H
    color        TEXT,
    comment      TEXT
);
CREATE INDEX IF NOT EXISTS rb_cue_track ON rb_cue (track_id);
CREATE TABLE IF NOT EXISTS sb_peaks (
    track_id     TEXT PRIMARY KEY,  -- Rekordbox Track id (ADR-0003)
    peaks        BLOB NOT NULL,     -- i16 little-endian, value × 1000
    duration_sec REAL NOT NULL,     -- decoded length
    version      INTEGER NOT NULL   -- Peaks algorithm version
);
"#;

const COPIED_TABLES: [&str; 7] = [
    "rb_playlist",
    "rb_playlist_entry",
    "rb_track",
    "rb_artist",
    "rb_album",
    "rb_key",
    "rb_cue",
];

/// Send progress at most this many times per phase.
const PROGRESS_STEPS: usize = 100;

pub fn open(path: &Path) -> rusqlite::Result<Connection> {
    let conn = Connection::open(path)?;
    conn.pragma_update(None, "journal_mode", "WAL")?;
    init(&conn)?;
    Ok(conn)
}

pub fn init(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(SCHEMA)
}

pub fn is_empty(conn: &Connection) -> rusqlite::Result<bool> {
    conn.query_row(
        "SELECT NOT EXISTS (SELECT 1 FROM rb_track) AND NOT EXISTS (SELECT 1 FROM rb_playlist)",
        [],
        |r| r.get(0),
    )
}

/// Insert `rows`, reporting progress for `phase` every so often.
fn insert_all<T>(
    tx: &Transaction,
    sql: &str,
    rows: &[T],
    phase: Option<Phase>,
    progress: &mut dyn FnMut(Progress),
    bind: impl Fn(&mut rusqlite::Statement, &T) -> rusqlite::Result<usize>,
) -> rusqlite::Result<()> {
    let mut stmt = tx.prepare(sql)?;
    let total = rows.len();
    let step = (total / PROGRESS_STEPS).max(1);
    for (i, row) in rows.iter().enumerate() {
        if let Some(phase) = phase
            && i % step == 0
        {
            progress(Progress::new(phase, i, total));
        }
        bind(&mut stmt, row)?;
    }
    if let Some(phase) = phase {
        progress(Progress::new(phase, total, total));
    }
    Ok(())
}

/// Replace every Rekordbox-copied table with `snapshot` and prune SyncBox-only
/// rows for Tracks that no longer exist, all in one transaction. On error the
/// transaction rolls back and the Local DB is left as it was.
///
/// Reports the tracks and cues phases.
pub fn replace_library(
    conn: &mut Connection,
    snapshot: &Snapshot,
    progress: &mut dyn FnMut(Progress),
) -> rusqlite::Result<()> {
    let tx = conn.transaction()?;
    for table in COPIED_TABLES {
        tx.execute(&format!("DELETE FROM {table}"), [])?;
    }

    insert_all(
        &tx,
        "INSERT INTO rb_playlist (id, name, kind, parent_id, seq) VALUES (?1, ?2, ?3, ?4, ?5)",
        &snapshot.playlists,
        None,
        progress,
        |s, p| s.execute(params![p.id, p.name, p.kind.as_i64(), p.parent_id, p.seq]),
    )?;
    insert_all(
        &tx,
        "INSERT INTO rb_playlist_entry (playlist_id, track_id, seq) VALUES (?1, ?2, ?3)",
        &snapshot.entries,
        None,
        progress,
        |s, e| s.execute(params![e.playlist_id, e.track_id, e.seq]),
    )?;
    for (table, rows) in [
        ("rb_artist", &snapshot.artists),
        ("rb_album", &snapshot.albums),
        ("rb_key", &snapshot.keys),
    ] {
        insert_all(
            &tx,
            &format!("INSERT INTO {table} (id, name) VALUES (?1, ?2)"),
            rows,
            None,
            progress,
            |s, n| s.execute(params![n.id, n.name]),
        )?;
    }
    insert_all(
        &tx,
        "INSERT INTO rb_track (id, title, artist_id, album_id, key_id, bpm, length_sec, bitrate, rating, file_path)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        &snapshot.tracks,
        Some(Phase::Tracks),
        progress,
        |s, t| {
            s.execute(params![
                t.id, t.title, t.artist_id, t.album_id, t.key_id, t.bpm, t.length_sec, t.bitrate,
                t.rating, t.file_path
            ])
        },
    )?;
    insert_all(
        &tx,
        "INSERT INTO rb_cue (id, track_id, position_sec, end_sec, kind, color, comment)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        &snapshot.cues,
        Some(Phase::Cues),
        progress,
        |s, c| {
            s.execute(params![
                c.id,
                c.track_id,
                c.position_sec,
                c.end_sec,
                c.kind,
                c.color,
                c.comment
            ])
        },
    )?;

    prune_local_rows(&tx)?;
    tx.commit()
}

/// The SyncBox-only tables: every `sb_*` table with a `track_id` column.
pub fn local_tables(conn: &Connection) -> rusqlite::Result<Vec<String>> {
    let mut stmt = conn.prepare(
        "SELECT m.name FROM sqlite_master m
         WHERE m.type = 'table' AND m.name LIKE 'sb\\_%' ESCAPE '\\'
           AND EXISTS (SELECT 1 FROM pragma_table_info(m.name) WHERE name = 'track_id')
         ORDER BY m.name",
    )?;
    stmt.query_map([], |r| r.get(0))?.collect()
}

/// Remove SyncBox-only rows whose Track is no longer in the Rekordbox Library.
fn prune_local_rows(tx: &Transaction) -> rusqlite::Result<()> {
    for table in local_tables(tx)? {
        tx.execute(
            &format!("DELETE FROM \"{table}\" WHERE track_id NOT IN (SELECT id FROM rb_track)"),
            [],
        )?;
    }
    Ok(())
}

// ── Reads ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum PlaylistNodeKind {
    Playlist,
    Folder,
    Smart,
}

impl From<PlaylistKind> for PlaylistNodeKind {
    fn from(kind: PlaylistKind) -> Self {
        match kind {
            PlaylistKind::Playlist => Self::Playlist,
            PlaylistKind::Folder => Self::Folder,
            PlaylistKind::Smart => Self::Smart,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaylistNode {
    pub id: String,
    pub name: String,
    pub kind: PlaylistNodeKind,
    pub children: Vec<PlaylistNode>,
}

/// A Track as the UI shows it.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TrackRow {
    pub id: String,
    pub title: String,
    pub artist: String,
    pub album: Option<String>,
    pub bpm: Option<f64>,
    /// Rekordbox key name (e.g. `Am`), or empty.
    pub key: String,
    /// Length in seconds.
    pub length: Option<i64>,
    /// Bitrate in kbps.
    pub bitrate: Option<i64>,
    pub rating: Option<i64>,
    pub file_path: Option<String>,
}

pub fn playlist_tree(conn: &Connection) -> rusqlite::Result<Vec<PlaylistNode>> {
    struct Row {
        id: String,
        name: String,
        kind: PlaylistKind,
        parent_id: Option<String>,
    }
    let mut stmt =
        conn.prepare("SELECT id, name, kind, parent_id FROM rb_playlist ORDER BY seq, id")?;
    let rows: Vec<Row> = stmt
        .query_map([], |r| {
            Ok(Row {
                id: r.get(0)?,
                name: r.get(1)?,
                kind: PlaylistKind::from_i64(r.get(2)?),
                parent_id: r.get(3)?,
            })
        })?
        .collect::<rusqlite::Result<_>>()?;

    fn children(rows: &[Row], parent: Option<&str>) -> Vec<PlaylistNode> {
        rows.iter()
            .filter(|r| r.parent_id.as_deref() == parent)
            .map(|r| PlaylistNode {
                id: r.id.clone(),
                name: r.name.clone(),
                kind: r.kind.into(),
                children: children(rows, Some(&r.id)),
            })
            .collect()
    }
    // Nodes whose parent is missing are shown at the top level rather than lost.
    let ids: std::collections::HashSet<&str> = rows.iter().map(|r| r.id.as_str()).collect();
    let mut tree = children(&rows, None);
    for orphan in rows
        .iter()
        .filter(|r| r.parent_id.as_deref().is_some_and(|p| !ids.contains(p)))
    {
        tree.push(PlaylistNode {
            id: orphan.id.clone(),
            name: orphan.name.clone(),
            kind: orphan.kind.into(),
            children: children(&rows, Some(&orphan.id)),
        });
    }
    Ok(tree)
}

const TRACK_SELECT: &str = "
    SELECT t.id, t.title, a.name, al.name, t.bpm, k.name, t.length_sec, t.bitrate, t.rating, t.file_path
    FROM rb_track t
    LEFT JOIN rb_artist a ON a.id = t.artist_id
    LEFT JOIN rb_album al ON al.id = t.album_id
    LEFT JOIN rb_key k ON k.id = t.key_id";

fn track_row(r: &rusqlite::Row) -> rusqlite::Result<TrackRow> {
    Ok(TrackRow {
        id: r.get(0)?,
        title: r.get::<_, Option<String>>(1)?.unwrap_or_default(),
        artist: r.get::<_, Option<String>>(2)?.unwrap_or_default(),
        album: r.get(3)?,
        bpm: r.get(4)?,
        key: r.get::<_, Option<String>>(5)?.unwrap_or_default(),
        length: r.get(6)?,
        bitrate: r.get(7)?,
        rating: r.get(8)?,
        file_path: r.get(9)?,
    })
}

/// What playback needs to know about a Track: its raw Rekordbox file path
/// and the length Rekordbox stored, in seconds. `None` if there's no such Track.
pub fn track_source(
    conn: &Connection,
    track_id: &str,
) -> rusqlite::Result<Option<(Option<String>, Option<u64>)>> {
    use rusqlite::OptionalExtension;
    conn.query_row(
        "SELECT file_path, length_sec FROM rb_track WHERE id = ?1",
        [track_id],
        |r| {
            Ok((
                r.get(0)?,
                r.get::<_, Option<i64>>(1)?.map(|s| s.max(0) as u64),
            ))
        },
    )
    .optional()
}

/// Each Track's raw Rekordbox file path. A Track that doesn't exist gets
/// `None`, the same as one with no path.
pub fn track_file_paths(
    conn: &Connection,
    track_ids: Vec<String>,
) -> rusqlite::Result<Vec<(String, Option<String>)>> {
    use rusqlite::OptionalExtension;
    let mut stmt = conn.prepare("SELECT file_path FROM rb_track WHERE id = ?1")?;
    track_ids
        .into_iter()
        .map(|id| {
            let path = stmt
                .query_row([&id], |r| r.get::<_, Option<String>>(0))
                .optional()?
                .flatten();
            Ok((id, path))
        })
        .collect()
}

/// A Playlist's Tracks in Playlist order. Folders have none.
pub fn playlist_tracks(conn: &Connection, playlist_id: &str) -> rusqlite::Result<Vec<TrackRow>> {
    let sql = format!(
        "{TRACK_SELECT} JOIN rb_playlist_entry e ON e.track_id = t.id
         WHERE e.playlist_id = ?1 ORDER BY e.seq"
    );
    conn.prepare(&sql)?
        .query_map([playlist_id], track_row)?
        .collect()
}

pub fn all_tracks(conn: &Connection) -> rusqlite::Result<Vec<TrackRow>> {
    let sql = format!("{TRACK_SELECT} ORDER BY t.title COLLATE NOCASE, t.id");
    conn.prepare(&sql)?.query_map([], track_row)?.collect()
}

// ── Peaks ────────────────────────────────────────────────────────────────────

/// A Track's stored Peaks (encoded) and duration in seconds, if they were
/// computed with algorithm `version`. Peaks from another version count as
/// missing.
pub fn read_peaks(
    conn: &Connection,
    track_id: &str,
    version: i64,
) -> rusqlite::Result<Option<(Vec<u8>, f64)>> {
    use rusqlite::OptionalExtension;
    conn.query_row(
        "SELECT peaks, duration_sec FROM sb_peaks WHERE track_id = ?1 AND version = ?2",
        params![track_id, version],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )
    .optional()
}

/// Whether a Track has Peaks from algorithm `version`.
pub fn has_peaks(conn: &Connection, track_id: &str, version: i64) -> rusqlite::Result<bool> {
    conn.query_row(
        "SELECT EXISTS (SELECT 1 FROM sb_peaks WHERE track_id = ?1 AND version = ?2)",
        params![track_id, version],
        |r| r.get(0),
    )
}

/// Stores a Track's Peaks, replacing any it had. Writes nothing if the Track
/// is no longer in the Local DB (a Sync removed it while Peaks were being
/// computed), so no orphan row is left. Returns whether a row was written.
pub fn write_peaks(
    conn: &Connection,
    track_id: &str,
    peaks: &[u8],
    duration_sec: f64,
    version: i64,
) -> rusqlite::Result<bool> {
    let n = conn.execute(
        "INSERT OR REPLACE INTO sb_peaks (track_id, peaks, duration_sec, version)
         SELECT ?1, ?2, ?3, ?4 WHERE EXISTS (SELECT 1 FROM rb_track WHERE id = ?1)",
        params![track_id, peaks, duration_sec, version],
    )?;
    Ok(n > 0)
}

/// A Cue as the UI shows it.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CueRow {
    pub id: String,
    pub position_sec: f64,
    /// End of a loop, if the Cue is a loop.
    pub end_sec: Option<f64>,
    /// 0 = memory Cue, 1–8 = hot Cue slot A–H.
    pub kind: i64,
    /// `#RRGGBB`, if Rekordbox gave the Cue a colour.
    pub color: Option<String>,
    pub comment: Option<String>,
}

/// A Track's Cues in position order.
pub fn track_cues(conn: &Connection, track_id: &str) -> rusqlite::Result<Vec<CueRow>> {
    conn.prepare(
        "SELECT id, position_sec, end_sec, kind, color, comment FROM rb_cue
         WHERE track_id = ?1 ORDER BY position_sec, id",
    )?
    .query_map([track_id], |r| {
        Ok(CueRow {
            id: r.get(0)?,
            position_sec: r.get(1)?,
            end_sec: r.get(2)?,
            kind: r.get(3)?,
            color: r.get(4)?,
            comment: r.get(5)?,
        })
    })?
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::library::snapshot::{Cue, Named, Playlist, PlaylistEntry, Track};

    fn db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        init(&conn).unwrap();
        // Stand-ins for SyncBox-only tables not built yet (Beat Grid, settings).
        conn.execute_batch(
            "CREATE TABLE sb_beat_grid (track_id TEXT, beat INTEGER);
             CREATE TABLE sb_settings (key TEXT PRIMARY KEY, value TEXT);",
        )
        .unwrap();
        conn
    }

    fn track(id: &str, title: &str) -> Track {
        Track {
            id: id.into(),
            title: Some(title.into()),
            artist_id: Some("a1".into()),
            key_id: Some("k1".into()),
            bpm: Some(128.0),
            ..Default::default()
        }
    }

    fn snapshot(track_ids: &[&str]) -> Snapshot {
        Snapshot {
            playlists: vec![
                Playlist {
                    id: "f".into(),
                    name: "Folder".into(),
                    kind: PlaylistKind::Folder,
                    parent_id: None,
                    seq: 1,
                },
                Playlist {
                    id: "p2".into(),
                    name: "Second".into(),
                    kind: PlaylistKind::Playlist,
                    parent_id: Some("f".into()),
                    seq: 2,
                },
                Playlist {
                    id: "p1".into(),
                    name: "First".into(),
                    kind: PlaylistKind::Playlist,
                    parent_id: Some("f".into()),
                    seq: 1,
                },
                Playlist {
                    id: "s".into(),
                    name: "Smart".into(),
                    kind: PlaylistKind::Smart,
                    parent_id: None,
                    seq: 2,
                },
            ],
            entries: track_ids
                .iter()
                .rev()
                .enumerate()
                .map(|(i, id)| PlaylistEntry {
                    playlist_id: "p1".into(),
                    track_id: (*id).into(),
                    seq: i as i64 + 1,
                })
                .collect(),
            tracks: track_ids
                .iter()
                .map(|id| track(id, &format!("Title {id}")))
                .collect(),
            artists: vec![Named {
                id: "a1".into(),
                name: "Artist".into(),
            }],
            albums: vec![],
            keys: vec![Named {
                id: "k1".into(),
                name: "Am".into(),
            }],
            cues: track_ids
                .iter()
                .map(|id| Cue {
                    id: format!("c-{id}"),
                    track_id: (*id).into(),
                    position_sec: 1.0,
                    end_sec: None,
                    kind: 1,
                    color: None,
                    comment: None,
                })
                .collect(),
        }
    }

    fn count(conn: &Connection, sql: &str) -> i64 {
        conn.query_row(sql, [], |r| r.get(0)).unwrap()
    }

    fn sync(conn: &mut Connection, snapshot: &Snapshot) {
        replace_library(conn, snapshot, &mut |_| {}).unwrap();
    }

    #[test]
    fn finds_only_sb_tables_with_track_id() {
        assert_eq!(local_tables(&db()).unwrap(), ["sb_beat_grid", "sb_peaks"]);
    }

    #[test]
    fn sync_replaces_copied_tables_and_keeps_local_rows() {
        let mut conn = db();
        sync(&mut conn, &snapshot(&["t1", "t2", "t3"]));
        conn.execute_batch(
            "INSERT INTO sb_peaks VALUES ('t1', x'00', 1.0, 1), ('t2', x'01', 1.0, 1);
             INSERT INTO sb_beat_grid VALUES ('t2', 1), ('t3', 1);
             INSERT INTO sb_settings VALUES ('volume', '0.5');",
        )
        .unwrap();

        // t2 is deleted in Rekordbox; t4 is new.
        sync(&mut conn, &snapshot(&["t1", "t3", "t4"]));

        assert_eq!(count(&conn, "SELECT COUNT(*) FROM rb_track"), 3);
        assert_eq!(count(&conn, "SELECT COUNT(*) FROM rb_cue"), 3);
        assert_eq!(
            count(&conn, "SELECT COUNT(*) FROM rb_track WHERE id = 't2'"),
            0
        );
        // SyncBox-only rows for surviving Tracks are kept; rows for t2 are removed.
        assert_eq!(
            count(&conn, "SELECT COUNT(*) FROM sb_peaks WHERE track_id = 't1'"),
            1
        );
        assert_eq!(
            count(&conn, "SELECT COUNT(*) FROM sb_peaks WHERE track_id = 't2'"),
            0
        );
        assert_eq!(count(&conn, "SELECT COUNT(*) FROM sb_beat_grid"), 1);
        // Tables without a track_id are never touched.
        assert_eq!(count(&conn, "SELECT COUNT(*) FROM sb_settings"), 1);
    }

    #[test]
    fn failed_sync_leaves_previous_local_db() {
        let mut conn = db();
        sync(&mut conn, &snapshot(&["t1", "t2"]));
        conn.execute("INSERT INTO sb_peaks VALUES ('t2', x'00', 1.0, 1)", [])
            .unwrap();

        // A duplicate Cue id violates the primary key part-way through the Sync.
        let mut bad = snapshot(&["t1"]);
        bad.cues.push(bad.cues[0].clone());
        assert!(replace_library(&mut conn, &bad, &mut |_| {}).is_err());

        assert_eq!(count(&conn, "SELECT COUNT(*) FROM rb_track"), 2);
        assert_eq!(count(&conn, "SELECT COUNT(*) FROM rb_cue"), 2);
        assert_eq!(count(&conn, "SELECT COUNT(*) FROM sb_peaks"), 1);
    }

    #[test]
    fn reads_tree_in_order() {
        let mut conn = db();
        sync(&mut conn, &snapshot(&["t1"]));
        let tree = playlist_tree(&conn).unwrap();
        let names: Vec<_> = tree.iter().map(|n| n.name.as_str()).collect();
        assert_eq!(names, ["Folder", "Smart"]);
        assert_eq!(tree[0].kind, PlaylistNodeKind::Folder);
        assert_eq!(tree[1].kind, PlaylistNodeKind::Smart);
        let children: Vec<_> = tree[0].children.iter().map(|n| n.name.as_str()).collect();
        assert_eq!(children, ["First", "Second"]);
    }

    #[test]
    fn reads_playlist_tracks_in_entry_order() {
        let mut conn = db();
        sync(&mut conn, &snapshot(&["t1", "t2", "t3"]));
        let ids: Vec<_> = playlist_tracks(&conn, "p1")
            .unwrap()
            .into_iter()
            .map(|t| t.id)
            .collect();
        assert_eq!(ids, ["t3", "t2", "t1"]);
        let t = &playlist_tracks(&conn, "p1").unwrap()[0];
        assert_eq!(
            (t.artist.as_str(), t.key.as_str(), t.album.as_deref()),
            ("Artist", "Am", None)
        );
        assert!(playlist_tracks(&conn, "f").unwrap().is_empty());
    }

    #[test]
    fn reports_tracks_and_cues_progress() {
        let mut conn = db();
        let mut seen = Vec::new();
        replace_library(&mut conn, &snapshot(&["t1", "t2"]), &mut |p| seen.push(p)).unwrap();
        let last = |phase| {
            seen.iter()
                .rev()
                .find(|p| p.phase == phase)
                .copied()
                .unwrap()
        };
        assert_eq!(
            (last(Phase::Tracks).done, last(Phase::Tracks).total),
            (2, 2)
        );
        assert_eq!((last(Phase::Cues).done, last(Phase::Cues).total), (2, 2));
    }

    #[test]
    fn reads_file_paths_in_request_order() {
        let mut conn = db();
        let mut snap = snapshot(&["t1", "t2"]);
        snap.tracks[1].file_path = Some("E:/Music/b.mp3".into());
        sync(&mut conn, &snap);
        let paths = track_file_paths(&conn, vec!["t2".into(), "t1".into(), "gone".into()]).unwrap();
        assert_eq!(
            paths,
            [
                ("t2".to_owned(), Some("E:/Music/b.mp3".to_owned())),
                ("t1".to_owned(), None),
                ("gone".to_owned(), None),
            ]
        );
    }

    #[test]
    fn empty_until_first_sync() {
        let mut conn = db();
        assert!(is_empty(&conn).unwrap());
        sync(&mut conn, &snapshot(&["t1"]));
        assert!(!is_empty(&conn).unwrap());
    }

    #[test]
    fn track_source_for_playback() {
        let mut conn = db();
        let mut snap = snapshot(&["t1", "t2"]);
        snap.tracks[0].file_path = Some("E:\\Music\\a.aiff".into());
        snap.tracks[0].length_sec = Some(245);
        sync(&mut conn, &snap);
        assert_eq!(
            track_source(&conn, "t1").unwrap(),
            Some((Some("E:\\Music\\a.aiff".into()), Some(245)))
        );
        assert_eq!(track_source(&conn, "t2").unwrap(), Some((None, None)));
        assert_eq!(track_source(&conn, "nope").unwrap(), None);
    }

    #[test]
    fn peaks_round_trip() {
        let mut conn = db();
        sync(&mut conn, &snapshot(&["t1"]));
        assert!(!has_peaks(&conn, "t1", 1).unwrap());
        assert!(write_peaks(&conn, "t1", &[1, 2, 3, 4], 245.5, 1).unwrap());
        assert!(has_peaks(&conn, "t1", 1).unwrap());
        assert_eq!(
            read_peaks(&conn, "t1", 1).unwrap(),
            Some((vec![1, 2, 3, 4], 245.5))
        );
        // Writing again replaces the row.
        assert!(write_peaks(&conn, "t1", &[9], 1.0, 1).unwrap());
        assert_eq!(read_peaks(&conn, "t1", 1).unwrap(), Some((vec![9], 1.0)));
    }

    #[test]
    fn peaks_from_another_version_count_as_missing() {
        let mut conn = db();
        sync(&mut conn, &snapshot(&["t1"]));
        write_peaks(&conn, "t1", &[1, 2], 1.0, 1).unwrap();
        assert!(!has_peaks(&conn, "t1", 2).unwrap());
        assert_eq!(read_peaks(&conn, "t1", 2).unwrap(), None);
    }

    #[test]
    fn peaks_for_unknown_track_are_not_written() {
        let mut conn = db();
        sync(&mut conn, &snapshot(&["t1"]));
        assert!(!write_peaks(&conn, "gone", &[1, 2], 1.0, 1).unwrap());
        assert_eq!(count(&conn, "SELECT COUNT(*) FROM sb_peaks"), 0);
    }

    #[test]
    fn reads_track_cues_in_position_order() {
        let mut conn = db();
        let mut snap = snapshot(&["t1", "t2"]);
        snap.cues.push(Cue {
            id: "c-early".into(),
            track_id: "t1".into(),
            position_sec: 0.5,
            end_sec: Some(4.5),
            kind: 0,
            color: Some("#FF0000".into()),
            comment: Some("Loop".into()),
        });
        sync(&mut conn, &snap);
        let cues = track_cues(&conn, "t1").unwrap();
        let ids: Vec<_> = cues.iter().map(|c| c.id.as_str()).collect();
        assert_eq!(ids, ["c-early", "c-t1"]);
        assert_eq!(cues[0].end_sec, Some(4.5));
        assert_eq!(cues[0].color.as_deref(), Some("#FF0000"));
        assert!(track_cues(&conn, "nope").unwrap().is_empty());
    }
}
