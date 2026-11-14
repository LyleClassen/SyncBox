use serde::Serialize;
use serde::ser::SerializeStruct;

use crate::library::track_file::UnplayableReason;

/// Why a playback command failed.
#[derive(Debug, thiserror::Error)]
pub enum PlaybackError {
    #[error("{}", .0.message())]
    Unplayable(UnplayableReason),
    #[error("Track not found in the Local DB.")]
    UnknownTrack,
    #[error("Could not open the audio file: {0}")]
    Open(String),
    #[error("Could not decode the audio file: {0}")]
    Decode(String),
    #[error("No audio output device is available: {0}")]
    Output(String),
    #[error("Could not seek: {0}")]
    Seek(String),
    #[error("Could not read the Local DB: {0}")]
    LocalDb(#[from] rusqlite::Error),
    #[error("The audio engine has stopped.")]
    EngineStopped,
}

impl PlaybackError {
    pub fn kind(&self) -> &'static str {
        match self {
            Self::Unplayable(_) => "unplayable",
            Self::UnknownTrack => "unknownTrack",
            Self::Open(_) => "open",
            Self::Decode(_) => "decode",
            Self::Output(_) => "output",
            Self::Seek(_) => "seek",
            Self::LocalDb(_) => "localDb",
            Self::EngineStopped => "engineStopped",
        }
    }
}

/// Sent to the UI as `{ kind, message, reason }`; `reason` is the Unplayable
/// Reason when `kind` is `"unplayable"`, otherwise `null`.
impl Serialize for PlaybackError {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut s = serializer.serialize_struct("PlaybackError", 3)?;
        s.serialize_field("kind", self.kind())?;
        s.serialize_field("message", &self.to_string())?;
        let reason = match self {
            Self::Unplayable(r) => Some(r),
            _ => None,
        };
        s.serialize_field("reason", &reason)?;
        s.end()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unplayable_errors_carry_their_reason() {
        let json =
            serde_json::to_value(PlaybackError::Unplayable(UnplayableReason::NoPath)).unwrap();
        assert_eq!(
            json,
            serde_json::json!({
                "kind": "unplayable",
                "message": "Track has no file location",
                "reason": { "kind": "noPath", "detail": null, "message": "Track has no file location" }
            })
        );
    }
}
