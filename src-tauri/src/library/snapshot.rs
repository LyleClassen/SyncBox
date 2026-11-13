//! The plain data a Sync copies out of the Rekordbox Library.
//!
//! These types carry no `rbox` types, so the Local DB and Cue mapping can be
//! tested without a real `master.db`.

/// The kind of a Playlist tree node, as Rekordbox stores it (`djmdPlaylist.Attribute`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaylistKind {
    Playlist,
    Folder,
    Smart,
}

impl PlaylistKind {
    pub fn as_i64(self) -> i64 {
        match self {
            Self::Playlist => 0,
            Self::Folder => 1,
            Self::Smart => 4,
        }
    }

    pub fn from_i64(value: i64) -> Self {
        match value {
            1 => Self::Folder,
            4 => Self::Smart,
            _ => Self::Playlist,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Playlist {
    pub id: String,
    pub name: String,
    pub kind: PlaylistKind,
    /// `None` for top-level nodes.
    pub parent_id: Option<String>,
    pub seq: i64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PlaylistEntry {
    pub playlist_id: String,
    pub track_id: String,
    /// 1-based position in the Playlist.
    pub seq: i64,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Track {
    pub id: String,
    pub title: Option<String>,
    pub artist_id: Option<String>,
    pub album_id: Option<String>,
    pub key_id: Option<String>,
    /// Beats per minute (Rekordbox stores BPM × 100; this is already divided).
    pub bpm: Option<f64>,
    /// Length in seconds.
    pub length_sec: Option<i64>,
    /// Bitrate in kbps.
    pub bitrate: Option<i64>,
    pub rating: Option<i64>,
    pub file_path: Option<String>,
}

/// A named lookup row (artist, album or key).
#[derive(Debug, Clone, PartialEq)]
pub struct Named {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Cue {
    pub id: String,
    pub track_id: String,
    pub position_sec: f64,
    /// End of a loop, if the Cue is a loop.
    pub end_sec: Option<f64>,
    /// Rekordbox `djmdCue.Kind`: 0 = memory Cue, 1–8 = hot Cue slot A–H.
    pub kind: i64,
    /// `#RRGGBB`, if Rekordbox gave the Cue a colour.
    pub color: Option<String>,
    pub comment: Option<String>,
}

/// Everything a Sync copies, ready to replace the Rekordbox-copied tables.
#[derive(Debug, Clone, Default)]
pub struct Snapshot {
    pub playlists: Vec<Playlist>,
    pub entries: Vec<PlaylistEntry>,
    pub tracks: Vec<Track>,
    pub artists: Vec<Named>,
    pub albums: Vec<Named>,
    pub keys: Vec<Named>,
    pub cues: Vec<Cue>,
}
