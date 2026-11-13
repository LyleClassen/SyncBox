use serde::Serialize;
use serde::ser::SerializeStruct;

/// Why a Sync failed. Each kind has its own guidance for the user.
#[derive(Debug, thiserror::Error)]
pub enum SyncError {
    #[error("Rekordbox master.db not found. Install and open Rekordbox first.")]
    NotFound,
    #[error("Rekordbox is running and has locked its database. Close Rekordbox and try again.")]
    Locked,
    #[error("Could not read the Rekordbox database. It may be corrupted or inaccessible.")]
    Unreadable,
    #[error("A Sync is already running.")]
    AlreadyRunning,
    #[error("Could not write the Local DB: {0}")]
    LocalDb(#[from] rusqlite::Error),
}

impl SyncError {
    pub fn kind(&self) -> &'static str {
        match self {
            Self::NotFound => "notFound",
            Self::Locked => "locked",
            Self::Unreadable => "unreadable",
            Self::AlreadyRunning => "alreadyRunning",
            Self::LocalDb(_) => "localDb",
        }
    }
}

/// Sent to the UI as `{ kind, message }`.
impl Serialize for SyncError {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut s = serializer.serialize_struct("SyncError", 2)?;
        s.serialize_field("kind", self.kind())?;
        s.serialize_field("message", &self.to_string())?;
        s.end()
    }
}

/// An error from a Local DB read, sent to the UI as a message.
#[derive(Debug, thiserror::Error)]
pub enum LocalDbError {
    #[error("Could not read the Local DB: {0}")]
    Read(#[from] rusqlite::Error),
    #[error("Reading the Local DB was interrupted.")]
    Interrupted,
}

impl Serialize for LocalDbError {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}
