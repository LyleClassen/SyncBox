//! Finding a Track's audio file, and its Unplayable Reason when there isn't one.
//!
//! Ported from mixxxa `shared/trackPath.ts` and `bun/paths/resolveTrackFile.ts`.
//! [`classify`] is pure and works on both Windows and macOS paths whatever the
//! host OS. [`FileCheck`] adds the filesystem checks, which are never cached
//! between calls because drives come and go while the app runs.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::Serialize;
use serde::ser::SerializeStruct;

/// A raw Rekordbox `folderPath` value, classified.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClassifiedPath {
    /// A local or network file, with `/` separators.
    File(String),
    /// A streaming Track, with the lower-case service name (e.g. `spotify`).
    Streaming(String),
    /// No path, an empty one, or only whitespace.
    None,
}

/// Turns every `\` into `/` and collapses repeated separators, keeping a
/// leading `//` (a UNC root).
pub fn normalize_separators(path: &str) -> String {
    let unified = path.replace('\\', "/");
    let (lead, rest) = match unified.strip_prefix("//") {
        Some(rest) => ("//", rest),
        None => ("", unified.as_str()),
    };
    let mut out = String::with_capacity(unified.len());
    out.push_str(lead);
    let mut prev_slash = false;
    for c in rest.chars() {
        if c == '/' && prev_slash {
            continue;
        }
        prev_slash = c == '/';
        out.push(c);
    }
    out
}

/// A Windows drive letter followed by a separator: `E:\` or `E:/`.
fn is_drive_path(s: &str) -> bool {
    let b = s.as_bytes();
    b.len() >= 3 && b[0].is_ascii_alphabetic() && b[1] == b':' && matches!(b[2], b'\\' | b'/')
}

/// Splits `scheme:rest` where scheme is `[a-zA-Z][a-zA-Z0-9+.-]*`.
fn split_scheme(s: &str) -> Option<(&str, &str)> {
    let (scheme, rest) = s.split_once(':')?;
    let mut chars = scheme.chars();
    let first = chars.next()?;
    let valid = first.is_ascii_alphabetic()
        && chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '.' | '-'));
    valid.then_some((scheme, rest))
}

/// Percent-decodes like `decodeURIComponent`, or returns `None` if the input
/// has a broken escape or doesn't decode to UTF-8.
fn percent_decode(s: &str) -> Option<String> {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            let hex = s.get(i + 1..i + 3)?;
            out.push(u8::from_str_radix(hex, 16).ok()?);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8(out).ok()
}

pub fn classify(raw: Option<&str>) -> ClassifiedPath {
    let Some(trimmed) = raw.map(str::trim).filter(|s| !s.is_empty()) else {
        return ClassifiedPath::None;
    };

    // Apple Music catalog paths have no scheme: "/v4/catalog/tracks/24341612/".
    if trimmed.starts_with("/v4/catalog/tracks/") {
        return ClassifiedPath::Streaming("applemusic".into());
    }
    // A drive letter looks like a one-letter scheme, so rule it out first.
    if is_drive_path(trimmed) {
        return ClassifiedPath::File(normalize_separators(trimmed));
    }
    if let Some((scheme, rest)) = split_scheme(trimmed) {
        let scheme = scheme.to_ascii_lowercase();
        if scheme != "file" {
            return ClassifiedPath::Streaming(scheme);
        }
        // file://localhost/... or file:///...: drop the scheme and host, then decode.
        let without_host = match rest.strip_prefix("//") {
            Some(after) => &after[after.find('/').unwrap_or(after.len())..],
            None => rest,
        };
        let decoded = percent_decode(without_host).unwrap_or_else(|| without_host.to_owned());
        return ClassifiedPath::File(normalize_separators(&decoded));
    }
    ClassifiedPath::File(normalize_separators(trimmed))
}

/// The part of a normalised path whose absence means "drive not connected"
/// rather than "file moved": `E:/`, `//nas/share` or `/Volumes/External`.
/// Anything else (e.g. a home directory) gives `/`.
pub fn volume_root(path: &str) -> String {
    let b = path.as_bytes();
    if b.len() >= 2 && b[0].is_ascii_alphabetic() && b[1] == b':' {
        return format!("{}/", &path[..2]);
    }
    if let Some(rest) = path.strip_prefix("//") {
        let mut parts = rest.split('/');
        if let (Some(server), Some(share)) = (parts.next(), parts.next())
            && !server.is_empty()
            && !share.is_empty()
        {
            return format!("//{server}/{share}");
        }
    }
    if let Some(rest) = path.strip_prefix("/Volumes/") {
        let name = rest.split('/').next().unwrap_or_default();
        if !name.is_empty() {
            return format!("/Volumes/{name}");
        }
    }
    "/".into()
}

/// Why a Track can't be loaded. Each has its own message for the user.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UnplayableReason {
    Streaming { service: String },
    VolumeOffline { root: String },
    Missing { path: String },
    NoPath,
}

impl UnplayableReason {
    pub fn kind(&self) -> &'static str {
        match self {
            Self::Streaming { .. } => "streaming",
            Self::VolumeOffline { .. } => "volumeOffline",
            Self::Missing { .. } => "missing",
            Self::NoPath => "noPath",
        }
    }

    /// The service, volume root or path the reason is about.
    pub fn detail(&self) -> Option<&str> {
        match self {
            Self::Streaming { service } => Some(service),
            Self::VolumeOffline { root } => Some(root),
            Self::Missing { path } => Some(path),
            Self::NoPath => None,
        }
    }

    pub fn message(&self) -> String {
        match self {
            Self::Streaming { service } => format!("Streaming track ({service}), not a local file"),
            Self::VolumeOffline { root } => format!("Drive not connected: {root}"),
            Self::Missing { .. } => {
                "Audio file not found. It may have moved or been deleted.".into()
            }
            Self::NoPath => "Track has no file location".into(),
        }
    }
}

/// Sent to the UI as `{ kind, detail, message }`.
impl Serialize for UnplayableReason {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut s = serializer.serialize_struct("UnplayableReason", 3)?;
        s.serialize_field("kind", self.kind())?;
        s.serialize_field("detail", &self.detail())?;
        s.serialize_field("message", &self.message())?;
        s.end()
    }
}

/// A normalised path as the host OS expects it.
fn native(path: &str) -> PathBuf {
    if cfg!(windows) {
        PathBuf::from(path.replace('/', "\\"))
    } else {
        PathBuf::from(path)
    }
}

/// Resolves Track files against the filesystem.
///
/// Make a new one for each request: it remembers which volume roots are
/// connected only for its own lifetime, so a list of 10,000 Tracks on an
/// unplugged drive costs one check, not 10,000. Nothing outlives it.
pub struct FileCheck<F> {
    exists: F,
    roots: HashMap<String, bool>,
}

impl FileCheck<fn(&Path) -> bool> {
    pub fn on_disk() -> Self {
        Self::new(Path::exists)
    }
}

impl<F: FnMut(&Path) -> bool> FileCheck<F> {
    /// Uses `exists` in place of the filesystem (for tests).
    pub fn new(exists: F) -> Self {
        Self {
            exists,
            roots: HashMap::new(),
        }
    }

    /// The Track's file if it exists right now, or why it can't be loaded.
    pub fn resolve(&mut self, raw: Option<&str>) -> Result<PathBuf, UnplayableReason> {
        let path = match classify(raw) {
            ClassifiedPath::None => return Err(UnplayableReason::NoPath),
            ClassifiedPath::Streaming(service) => {
                return Err(UnplayableReason::Streaming { service });
            }
            ClassifiedPath::File(path) => path,
        };
        let root = volume_root(&path);
        if root != "/" {
            let exists = &mut self.exists;
            let online = *self
                .roots
                .entry(root.clone())
                .or_insert_with(|| exists(&native(&root)));
            if !online {
                return Err(UnplayableReason::VolumeOffline { root });
            }
        }
        let file = native(&path);
        if (self.exists)(&file) {
            Ok(file)
        } else {
            Err(UnplayableReason::Missing { path })
        }
    }

    /// Like [`Self::resolve`], keeping only the reason.
    pub fn reason(&mut self, raw: Option<&str>) -> Option<UnplayableReason> {
        self.resolve(raw).err()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file(p: &str) -> ClassifiedPath {
        ClassifiedPath::File(p.into())
    }

    // Cases ported from mixxxa shared/trackPath.test.ts.

    #[test]
    fn posix_path_unchanged() {
        assert_eq!(
            classify(Some("/Volumes/External/Music/a.mp3")),
            file("/Volumes/External/Music/a.mp3")
        );
    }

    #[test]
    fn windows_path_with_backslashes() {
        assert_eq!(
            classify(Some("E:\\Music\\DJ\\a.mp3")),
            file("E:/Music/DJ/a.mp3")
        );
    }

    #[test]
    fn windows_path_with_forward_slashes_unchanged() {
        assert_eq!(classify(Some("E:/Music/a.mp3")), file("E:/Music/a.mp3"));
    }

    #[test]
    fn unc_path() {
        assert_eq!(
            classify(Some("\\\\nas\\share\\a.mp3")),
            file("//nas/share/a.mp3")
        );
    }

    #[test]
    fn file_url_with_encoded_space() {
        assert_eq!(
            classify(Some("file://localhost/Users/x/a%20b.mp3")),
            file("/Users/x/a b.mp3")
        );
    }

    #[test]
    fn file_url_without_host() {
        assert_eq!(
            classify(Some("file:///Volumes/External/a.mp3")),
            file("/Volumes/External/a.mp3")
        );
    }

    #[test]
    fn file_url_with_broken_escape_is_kept_as_is() {
        assert_eq!(
            classify(Some("file:///Users/x/100%.mp3")),
            file("/Users/x/100%.mp3")
        );
    }

    #[test]
    fn file_url_with_encoded_utf8() {
        assert_eq!(
            classify(Some("file:///Users/x/caf%C3%A9.mp3")),
            file("/Users/x/café.mp3")
        );
    }

    #[test]
    fn spotify_uri() {
        assert_eq!(
            classify(Some("spotify:track:0JI8koDoC5crQbIjhSty59")),
            ClassifiedPath::Streaming("spotify".into())
        );
    }

    #[test]
    fn scheme_is_lower_cased() {
        assert_eq!(
            classify(Some("TIDAL:tracks:123")),
            ClassifiedPath::Streaming("tidal".into())
        );
    }

    #[test]
    fn apple_music_catalog_path() {
        assert_eq!(
            classify(Some("/v4/catalog/tracks/24341612/")),
            ClassifiedPath::Streaming("applemusic".into())
        );
    }

    #[test]
    fn none_empty_and_whitespace() {
        assert_eq!(classify(None), ClassifiedPath::None);
        assert_eq!(classify(Some("")), ClassifiedPath::None);
        assert_eq!(classify(Some("   ")), ClassifiedPath::None);
    }

    #[test]
    fn duplicate_separators_collapsed() {
        assert_eq!(
            classify(Some("/Volumes/External//Music///a.mp3")),
            file("/Volumes/External/Music/a.mp3")
        );
    }

    #[test]
    fn volume_roots() {
        assert_eq!(
            volume_root("/Volumes/External/Music/a.mp3"),
            "/Volumes/External"
        );
        assert_eq!(volume_root("E:/Music/a.mp3"), "E:/");
        assert_eq!(volume_root("//nas/share/a.mp3"), "//nas/share");
        assert_eq!(volume_root("/Users/me/Music/a.mp3"), "/");
        // Not enough segments to be a root.
        assert_eq!(volume_root("//nas"), "/");
        assert_eq!(volume_root("/Volumes/"), "/");
    }

    // Resolving against a fake filesystem.

    fn fs(existing: &'static [&'static str]) -> FileCheck<impl FnMut(&Path) -> bool> {
        FileCheck::new(move |p: &Path| {
            let p = p.to_string_lossy().replace('\\', "/");
            existing.contains(&p.as_str())
        })
    }

    #[test]
    fn resolves_an_existing_file() {
        let mut check = fs(&["E:/", "E:/Music/a.mp3"]);
        assert_eq!(
            check.resolve(Some("E:\\Music\\a.mp3")),
            Ok(native("E:/Music/a.mp3"))
        );
    }

    #[test]
    fn missing_file_on_a_connected_drive() {
        let mut check = fs(&["/Volumes/External"]);
        assert_eq!(
            check.reason(Some("/Volumes/External/a.mp3")),
            Some(UnplayableReason::Missing {
                path: "/Volumes/External/a.mp3".into()
            })
        );
    }

    #[test]
    fn drive_not_connected() {
        let mut check = fs(&[]);
        assert_eq!(
            check.reason(Some("\\\\nas\\share\\a.mp3")),
            Some(UnplayableReason::VolumeOffline {
                root: "//nas/share".into()
            })
        );
        assert_eq!(
            check.reason(Some("/Volumes/External/a.mp3")),
            Some(UnplayableReason::VolumeOffline {
                root: "/Volumes/External".into()
            })
        );
    }

    #[test]
    fn missing_file_outside_any_volume_is_missing_not_offline() {
        let mut check = fs(&[]);
        assert_eq!(
            check.reason(Some("/Users/me/a.mp3")),
            Some(UnplayableReason::Missing {
                path: "/Users/me/a.mp3".into()
            })
        );
    }

    #[test]
    fn streaming_and_no_path() {
        let mut check = fs(&[]);
        assert_eq!(
            check.reason(Some("spotify:track:1")),
            Some(UnplayableReason::Streaming {
                service: "spotify".into()
            })
        );
        assert_eq!(check.reason(None), Some(UnplayableReason::NoPath));
    }

    #[test]
    fn checks_each_volume_root_once_per_check() {
        let mut calls = Vec::new();
        let mut check = FileCheck::new(|p: &Path| {
            calls.push(p.to_owned());
            false
        });
        for name in ["a", "b", "c"] {
            check.reason(Some(&format!("E:/Music/{name}.mp3")));
        }
        drop(check);
        assert_eq!(calls, [native("E:/")]);
    }

    #[test]
    fn each_reason_has_its_own_message() {
        let reasons = [
            UnplayableReason::Streaming {
                service: "spotify".into(),
            },
            UnplayableReason::VolumeOffline { root: "E:/".into() },
            UnplayableReason::Missing { path: "x".into() },
            UnplayableReason::NoPath,
        ];
        let messages: std::collections::HashSet<_> = reasons.iter().map(|r| r.message()).collect();
        assert_eq!(messages.len(), reasons.len());
        assert_eq!(
            reasons[0].message(),
            "Streaming track (spotify), not a local file"
        );
        assert_eq!(reasons[1].message(), "Drive not connected: E:/");
    }

    #[test]
    fn serializes_kind_detail_and_message() {
        let json =
            serde_json::to_value(UnplayableReason::VolumeOffline { root: "E:/".into() }).unwrap();
        assert_eq!(
            json,
            serde_json::json!({
                "kind": "volumeOffline",
                "detail": "E:/",
                "message": "Drive not connected: E:/"
            })
        );
    }
}
