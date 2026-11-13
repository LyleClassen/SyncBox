//! Reading the Rekordbox Library (`master.db`) through `rbox`.
//!
//! `master.db` is always opened read-only: a Sync never writes to Rekordbox.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use rbox::db::connection::{ConnectionOptions, DbPool, OpenMode};
use rbox::masterdb::{
    DjmdAlbum, DjmdArtist, DjmdColor, DjmdContent, DjmdCue, DjmdKey, DjmdPlaylist,
    DjmdSongPlaylist, SmartList, create_connection_pool,
};
use rbox::{Model, PlaylistType};

use super::cues::{RawCue, map_cues};
use super::error::SyncError;
use super::snapshot::{Named, Playlist, PlaylistEntry, PlaylistKind, Snapshot, Track};
use super::sync::{Phase, Progress};

/// The `master.db` Rekordbox is configured to use, if Rekordbox is installed.
pub fn default_master_db_path() -> Result<PathBuf, SyncError> {
    let location = rbox::core::locate_library().map_err(|_| SyncError::NotFound)?;
    match location.db_exists() {
        Ok(true) => Ok(location.db_path),
        _ => Err(SyncError::NotFound),
    }
}

/// Called when `master.db` exists but can't be opened or read.
fn unreadable() -> SyncError {
    if rbox::core::is_rekordbox_running() {
        SyncError::Locked
    } else {
        SyncError::Unreadable
    }
}

fn open_read_only(path: &Path) -> Result<DbPool, SyncError> {
    if !path.is_file() {
        return Err(SyncError::NotFound);
    }
    let options = ConnectionOptions {
        open_mode: OpenMode::ReadOnly,
        ..Default::default()
    };
    create_connection_pool(path, &options, 1).map_err(|_| unreadable())
}

fn playlist_kind(attribute: PlaylistType) -> Option<PlaylistKind> {
    match attribute {
        PlaylistType::List => Some(PlaylistKind::Playlist),
        PlaylistType::Folder => Some(PlaylistKind::Folder),
        PlaylistType::SmartList => Some(PlaylistKind::Smart),
        // Cloud-library sync nodes aren't user Playlists.
        _ => None,
    }
}

fn named<T>(rows: Vec<T>, f: impl Fn(T) -> Option<Named>) -> Vec<Named> {
    rows.into_iter().filter_map(f).collect()
}

/// Read everything a Sync copies from the `master.db` at `path`.
///
/// Reports the playlists phase (one step per Playlist whose entries are read).
pub fn read_snapshot(
    path: &Path,
    progress: &mut dyn FnMut(Progress),
) -> Result<Snapshot, SyncError> {
    let pool = open_read_only(path)?;
    let mut conn = pool.get().map_err(|_| unreadable())?;
    let conn = &mut *conn;
    let read = |e: diesel::result::Error| {
        log::warn!("reading master.db failed: {e}");
        unreadable()
    };

    let rb_playlists = DjmdPlaylist::all(conn).map_err(read)?;
    let rb_contents = DjmdContent::all(conn).map_err(read)?;
    let rb_artists = DjmdArtist::all(conn).map_err(read)?;
    let rb_albums = DjmdAlbum::all(conn).map_err(read)?;
    let rb_keys = DjmdKey::all(conn).map_err(read)?;
    let rb_cues = DjmdCue::all(conn).map_err(read)?;
    let rb_colors = DjmdColor::all(conn).map_err(read)?;

    let playlists: Vec<Playlist> = rb_playlists
        .iter()
        .filter(|p| p.rb_local_deleted == 0)
        .filter_map(|p| {
            Some(Playlist {
                id: p.id.clone(),
                name: p.name.clone(),
                kind: playlist_kind(p.attribute)?,
                parent_id: (p.parent_id != "root").then(|| p.parent_id.clone()),
                seq: p.seq as i64,
            })
        })
        .collect();

    let mut entries = Vec::new();
    let total = playlists.len();
    for (i, playlist) in playlists.iter().enumerate() {
        progress(Progress::new(Phase::Playlists, i, total));
        match playlist.kind {
            PlaylistKind::Folder => {}
            PlaylistKind::Playlist => {
                let songs = DjmdSongPlaylist::by_playlist_id(conn, &playlist.id).map_err(read)?;
                let mut songs: Vec<_> = songs
                    .into_iter()
                    .filter(|s| s.rb_local_deleted == 0)
                    .collect();
                songs.sort_by_key(|s| s.track_no);
                entries.extend(songs.into_iter().map(|s| PlaylistEntry {
                    playlist_id: s.playlist_id,
                    track_id: s.content_id,
                    seq: s.track_no as i64,
                }));
            }
            PlaylistKind::Smart => {
                // Smart playlists have no stored entries: evaluate their conditions now.
                let smart = rb_playlists
                    .iter()
                    .find(|p| p.id == playlist.id)
                    .and_then(|p| p.smart_list.as_deref())
                    .filter(|xml| !xml.trim().is_empty());
                let Some(xml) = smart else { continue };
                let contents = SmartList::parse(xml)
                    .and_then(|s| s.build_content_id_sql())
                    .ok()
                    .and_then(|sql| DjmdPlaylist::get_smart_contents(conn, &sql).ok());
                let Some(contents) = contents else {
                    log::warn!(
                        "skipping smart playlist {:?}: its conditions couldn't be read",
                        playlist.name
                    );
                    continue;
                };
                entries.extend(
                    contents
                        .into_iter()
                        .enumerate()
                        .map(|(n, c)| PlaylistEntry {
                            playlist_id: playlist.id.clone(),
                            track_id: c.id,
                            seq: n as i64 + 1,
                        }),
                );
            }
        }
    }
    progress(Progress::new(Phase::Playlists, total, total));

    let tracks = rb_contents
        .into_iter()
        .filter(|c| c.rb_local_deleted == 0)
        .map(|c| Track {
            id: c.id,
            title: c.title,
            artist_id: c.artist_id,
            album_id: c.album_id,
            key_id: c.key_id,
            bpm: c.bpm.map(|b| b as f64 / 100.0),
            length_sec: c.length.map(i64::from),
            bitrate: c.bit_rate,
            rating: c.rating.map(i64::from),
            file_path: c.folder_path,
        })
        .collect();

    let color_codes: HashMap<String, Option<String>> = rb_colors
        .into_iter()
        .map(|c: DjmdColor| (c.id, c.color_code))
        .collect();
    let raw_cues: Vec<RawCue> = rb_cues
        .into_iter()
        .map(|c: DjmdCue| RawCue {
            id: c.id,
            track_id: c.content_id,
            deleted: c.rb_local_deleted == 1,
            in_msec: c.in_msec,
            out_msec: c.out_msec,
            kind: c.kind,
            color: c.color,
            color_table_index: c.color_table_index,
            comment: c.comment,
        })
        .collect();

    let live = |deleted: i32| deleted == 0;
    Ok(Snapshot {
        playlists,
        entries,
        tracks,
        artists: named(rb_artists, |a: DjmdArtist| {
            live(a.rb_local_deleted).then_some(Named {
                id: a.id,
                name: a.name,
            })
        }),
        albums: named(rb_albums, |a: DjmdAlbum| {
            live(a.rb_local_deleted).then_some(Named {
                id: a.id,
                name: a.name,
            })
        }),
        keys: named(rb_keys, |k: DjmdKey| {
            live(k.rb_local_deleted).then_some(Named {
                id: k.id,
                name: k.name,
            })
        }),
        cues: map_cues(&raw_cues, &color_codes),
    })
}
