//! A Track's Beat Grid, as Rekordbox analysed it, read lazily from the
//! Track's ANLZ file when it is loaded and kept in the Local DB's
//! `sb_beat_grid` (ADR-0003). It is read again only when the ANLZ file's
//! modified time changes. A Track with no ANLZ data gets no grid: none is
//! ever guessed from the BPM.

use std::path::Path;
use std::time::UNIX_EPOCH;

use serde::Serialize;

use crate::library::Library;
use crate::library::anlz::{self, AnlzError};
use crate::library::local_db;

/// One beat from the ANLZ Beat Grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Beat {
    /// Milliseconds from the start of the Track.
    pub time_ms: u32,
    /// Position in the bar, 1–4. 1 is a downbeat.
    pub number: u8,
}

/// A beat as the UI draws it.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BeatOut {
    pub time_sec: f64,
    /// 1 = downbeat (first beat of a bar).
    pub number: u8,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct BeatGrid {
    pub beats: Vec<BeatOut>,
}

impl BeatGrid {
    /// `None` for no beats, so the UI has a single "no grid" case.
    fn from_beats(beats: &[Beat]) -> Option<Self> {
        if beats.is_empty() {
            return None;
        }
        Some(Self {
            beats: beats
                .iter()
                .map(|b| BeatOut {
                    time_sec: f64::from(b.time_ms) / 1000.0,
                    number: b.number,
                })
                .collect(),
        })
    }
}

/// Bytes per stored beat: `u32` LE time in ms, then the `u8` beat number.
const BEAT_BYTES: usize = 5;

/// Beats as stored in `sb_beat_grid.beats`.
pub fn encode(beats: &[Beat]) -> Vec<u8> {
    beats
        .iter()
        .flat_map(|b| {
            let mut out = [0; BEAT_BYTES];
            out[..4].copy_from_slice(&b.time_ms.to_le_bytes());
            out[4] = b.number;
            out
        })
        .collect()
}

/// The reverse of [`encode`].
pub fn decode(bytes: &[u8]) -> Vec<Beat> {
    bytes
        .as_chunks::<BEAT_BYTES>()
        .0
        .iter()
        .map(|c| Beat {
            time_ms: u32::from_le_bytes([c[0], c[1], c[2], c[3]]),
            number: c[4],
        })
        .collect()
}

/// A file's modified time in ns since the UNIX epoch, or `None` if it can't be
/// read (the file is missing, say).
fn file_mtime(path: &Path) -> Option<i64> {
    let modified = std::fs::metadata(path).and_then(|m| m.modified());
    let modified = match modified {
        Ok(t) => t,
        Err(e) => {
            log::debug!("no Beat Grid: can't stat {}: {e}", path.display());
            return None;
        }
    };
    Some(match modified.duration_since(UNIX_EPOCH) {
        Ok(d) => d.as_nanos() as i64,
        Err(e) => -(e.duration().as_nanos() as i64),
    })
}

/// The Track's Beat Grid, reading its ANLZ file only if the stored grid is
/// missing or was read from a different file or modified time. ANLZ problems
/// mean no grid; only Local DB errors are returned.
pub fn ensure_beat_grid(library: &Library, track_id: &str) -> rusqlite::Result<Option<BeatGrid>> {
    ensure_with(library, track_id, file_mtime, anlz::read_beat_grid)
}

fn ensure_with(
    library: &Library,
    track_id: &str,
    stat: impl Fn(&Path) -> Option<i64>,
    parse: impl Fn(&Path) -> Result<Option<Vec<Beat>>, AnlzError>,
) -> rusqlite::Result<Option<BeatGrid>> {
    let (anlz_path, stored) = {
        let conn = library.conn();
        (
            local_db::track_anlz_path(&conn, track_id)?,
            local_db::read_beat_grid(&conn, track_id)?,
        )
    };
    let Some(anlz_path) = anlz_path else {
        return Ok(None);
    };
    let path = Path::new(&anlz_path);
    let Some(mtime) = stat(path) else {
        return Ok(None);
    };
    if let Some((stored_path, stored_mtime, blob)) = stored
        && stored_path == anlz_path
        && stored_mtime == mtime
    {
        return Ok(BeatGrid::from_beats(&decode(&blob)));
    }

    // Parse without holding the Local DB lock.
    let beats = match parse(path) {
        Ok(beats) => beats.unwrap_or_default(),
        Err(e) => {
            // Store nothing, so the next load tries again.
            log::warn!("no Beat Grid for Track {track_id}: {e}");
            return Ok(None);
        }
    };
    local_db::write_beat_grid(
        &library.conn(),
        track_id,
        &anlz_path,
        mtime,
        &encode(&beats),
    )?;
    Ok(BeatGrid::from_beats(&beats))
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use rusqlite::Connection;

    use super::*;
    use crate::library::snapshot::{Snapshot, Track};

    fn beats() -> Vec<Beat> {
        vec![
            Beat {
                time_ms: 120,
                number: 1,
            },
            Beat {
                time_ms: 589,
                number: 2,
            },
            Beat {
                time_ms: u32::MAX,
                number: 4,
            },
        ]
    }

    #[test]
    fn encode_round_trips() {
        let bytes = encode(&beats());
        assert_eq!(bytes.len(), 3 * BEAT_BYTES);
        assert_eq!(decode(&bytes), beats());
        assert!(decode(&[]).is_empty());
    }

    fn library(anlz_path: Option<&str>) -> Library {
        let mut conn = Connection::open_in_memory().unwrap();
        local_db::init(&conn).unwrap();
        let snap = Snapshot {
            tracks: vec![Track {
                id: "t1".into(),
                anlz_path: anlz_path.map(Into::into),
                ..Default::default()
            }],
            ..Default::default()
        };
        local_db::replace_library(&mut conn, &snap, &mut |_| {}).unwrap();
        Library::new(conn)
    }

    /// Runs `ensure_with` with a fixed mtime, counting parses.
    fn ensure(
        lib: &Library,
        mtime: Option<i64>,
        parses: &Cell<u32>,
        result: Result<Option<Vec<Beat>>, ()>,
    ) -> Option<BeatGrid> {
        ensure_with(
            lib,
            "t1",
            |_| mtime,
            |_| {
                parses.set(parses.get() + 1);
                result.clone().map_err(|()| AnlzError("bad".into()))
            },
        )
        .unwrap()
    }

    #[test]
    fn parses_once_while_mtime_is_unchanged() {
        let lib = library(Some("a.DAT"));
        let parses = Cell::new(0);
        let first = ensure(&lib, Some(10), &parses, Ok(Some(beats()))).unwrap();
        assert_eq!(first.beats.len(), 3);
        assert_eq!(
            first.beats[0],
            BeatOut {
                time_sec: 0.12,
                number: 1
            }
        );
        let again = ensure(&lib, Some(10), &parses, Ok(None));
        assert_eq!(again, Some(first));
        assert_eq!(parses.get(), 1);
    }

    #[test]
    fn reparses_when_mtime_changes() {
        let lib = library(Some("a.DAT"));
        let parses = Cell::new(0);
        ensure(&lib, Some(10), &parses, Ok(Some(beats())));
        let one = vec![Beat {
            time_ms: 5,
            number: 1,
        }];
        let grid = ensure(&lib, Some(11), &parses, Ok(Some(one))).unwrap();
        assert_eq!(grid.beats.len(), 1);
        assert_eq!(parses.get(), 2);
    }

    #[test]
    fn no_beat_grid_is_remembered() {
        let lib = library(Some("a.DAT"));
        let parses = Cell::new(0);
        assert_eq!(ensure(&lib, Some(10), &parses, Ok(None)), None);
        assert_eq!(ensure(&lib, Some(10), &parses, Ok(Some(beats()))), None);
        assert_eq!(parses.get(), 1);
    }

    #[test]
    fn parse_failure_is_retried_next_load() {
        let lib = library(Some("a.DAT"));
        let parses = Cell::new(0);
        assert_eq!(ensure(&lib, Some(10), &parses, Err(())), None);
        assert!(ensure(&lib, Some(10), &parses, Ok(Some(beats()))).is_some());
        assert_eq!(parses.get(), 2);
    }

    #[test]
    fn no_path_or_missing_file_means_no_grid() {
        let parses = Cell::new(0);
        assert_eq!(
            ensure(&library(None), Some(10), &parses, Ok(Some(beats()))),
            None
        );
        assert_eq!(
            ensure(&library(Some("a.DAT")), None, &parses, Ok(Some(beats()))),
            None
        );
        assert_eq!(parses.get(), 0);
    }

    #[test]
    fn missing_file_on_disk_has_no_mtime() {
        assert_eq!(file_mtime(Path::new("Z:/definitely/not/here.DAT")), None);
    }
}
