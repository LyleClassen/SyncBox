//! Mapping Rekordbox `djmdCue` rows to Cues (ported from mixxxa's `mapCues`).

use std::collections::HashMap;

use super::snapshot::Cue;

/// Standard Rekordbox hot-cue palette, keyed by `ColorTableIndex` (1–8).
pub const CUE_COLOR_TABLE: [(i32, &str); 8] = [
    (1, "#F870F8"), // pink
    (2, "#F80000"), // red
    (3, "#F8A030"), // orange
    (4, "#C3AF04"), // yellow
    (5, "#28E214"), // green
    (6, "#25FDE9"), // cyan
    (7, "#0672F8"), // blue
    (8, "#B432F8"), // purple
];

/// A `djmdCue` row with only the fields a Sync needs.
#[derive(Debug, Clone, Default)]
pub struct RawCue {
    pub id: String,
    pub track_id: String,
    pub deleted: bool,
    pub in_msec: i64,
    pub out_msec: i64,
    pub kind: i32,
    /// `djmdCue.Color`: an id into `djmdColor`, or -1 for none.
    pub color: i32,
    pub color_table_index: Option<i32>,
    pub comment: Option<String>,
}

fn palette_color(index: i32) -> Option<&'static str> {
    CUE_COLOR_TABLE
        .iter()
        .find(|(i, _)| *i == index)
        .map(|(_, c)| *c)
}

/// Normalise a `djmdColor.ColorCode` to `#RRGGBB`. Rekordbox stores either a hex
/// string (with or without `#`) or a decimal integer.
pub fn normalize_color_code(code: Option<&str>) -> Option<String> {
    let code = code?.trim();
    if code.is_empty() {
        return None;
    }
    let hex = code.strip_prefix('#').unwrap_or(code);
    if hex.len() == 6 && hex.chars().all(|c| c.is_ascii_hexdigit()) {
        return Some(format!("#{}", hex.to_ascii_uppercase()));
    }
    let n: f64 = code.parse().ok()?;
    if n.is_finite() && n >= 0.0 {
        return Some(format!("#{:06X}", n.round() as u64));
    }
    None
}

/// Resolve a Cue's colour: the hot-cue palette first, then the `djmdColor` table.
fn resolve_color(cue: &RawCue, color_codes: &HashMap<String, Option<String>>) -> Option<String> {
    if let Some(c) = cue.color_table_index.and_then(palette_color) {
        return Some(c.to_string());
    }
    if cue.color != -1 {
        let code = color_codes.get(&cue.color.to_string())?;
        return normalize_color_code(code.as_deref());
    }
    None
}

/// Map raw cues to Cues, skipping deleted cues and cues with a negative position.
/// `color_codes` maps `djmdColor.ID` to its `ColorCode`.
pub fn map_cues(cues: &[RawCue], color_codes: &HashMap<String, Option<String>>) -> Vec<Cue> {
    cues.iter()
        .filter(|c| !c.deleted && c.in_msec >= 0)
        .map(|c| Cue {
            id: c.id.clone(),
            track_id: c.track_id.clone(),
            position_sec: c.in_msec as f64 / 1000.0,
            end_sec: (c.out_msec > 0).then(|| c.out_msec as f64 / 1000.0),
            kind: c.kind as i64,
            color: resolve_color(c, color_codes),
            comment: c.comment.clone(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cue(id: &str) -> RawCue {
        RawCue {
            id: id.into(),
            track_id: "t1".into(),
            in_msec: 1500,
            out_msec: -1,
            color: -1,
            ..Default::default()
        }
    }

    fn colors() -> HashMap<String, Option<String>> {
        HashMap::from([
            ("1".into(), Some("#ff0000".into())),
            ("2".into(), Some("65280".into())),
            ("3".into(), Some("garbage".into())),
        ])
    }

    #[test]
    fn skips_deleted_and_negative_cues() {
        let raw = vec![
            cue("keep"),
            RawCue {
                deleted: true,
                ..cue("deleted")
            },
            RawCue {
                in_msec: -1,
                ..cue("negative")
            },
            RawCue {
                in_msec: 0,
                ..cue("zero")
            },
        ];
        let ids: Vec<_> = map_cues(&raw, &colors())
            .into_iter()
            .map(|c| c.id)
            .collect();
        assert_eq!(ids, ["keep", "zero"]);
    }

    #[test]
    fn converts_positions_to_seconds() {
        let mapped = map_cues(
            &[
                RawCue {
                    out_msec: 4000,
                    ..cue("loop")
                },
                cue("point"),
            ],
            &colors(),
        );
        assert_eq!(mapped[0].position_sec, 1.5);
        assert_eq!(mapped[0].end_sec, Some(4.0));
        assert_eq!(mapped[1].end_sec, None);
    }

    #[test]
    fn palette_index_wins_over_color_table() {
        let raw = RawCue {
            color_table_index: Some(5),
            color: 1,
            ..cue("a")
        };
        assert_eq!(
            map_cues(&[raw], &colors())[0].color.as_deref(),
            Some("#28E214")
        );
    }

    #[test]
    fn falls_back_to_color_table() {
        let hex = RawCue {
            color: 1,
            ..cue("hex")
        };
        let dec = RawCue {
            color: 2,
            ..cue("dec")
        };
        // An index outside the palette falls through to the colour table.
        let out_of_range = RawCue {
            color_table_index: Some(0),
            color: 1,
            ..cue("oor")
        };
        let mapped = map_cues(&[hex, dec, out_of_range], &colors());
        assert_eq!(mapped[0].color.as_deref(), Some("#FF0000"));
        assert_eq!(mapped[1].color.as_deref(), Some("#00FF00"));
        assert_eq!(mapped[2].color.as_deref(), Some("#FF0000"));
    }

    #[test]
    fn no_color_when_unset_unknown_or_unparseable() {
        let none = cue("none");
        let unknown = RawCue {
            color: 99,
            ..cue("unknown")
        };
        let bad = RawCue {
            color: 3,
            ..cue("bad")
        };
        let mapped = map_cues(&[none, unknown, bad], &colors());
        assert!(mapped.iter().all(|c| c.color.is_none()));
    }

    #[test]
    fn normalizes_color_codes() {
        assert_eq!(
            normalize_color_code(Some("abcdef")).as_deref(),
            Some("#ABCDEF")
        );
        assert_eq!(
            normalize_color_code(Some("#AbCdEf")).as_deref(),
            Some("#ABCDEF")
        );
        assert_eq!(normalize_color_code(Some("0")).as_deref(), Some("#000000"));
        assert_eq!(normalize_color_code(Some("-5")), None);
        assert_eq!(normalize_color_code(Some("")), None);
        assert_eq!(normalize_color_code(None), None);
    }
}
