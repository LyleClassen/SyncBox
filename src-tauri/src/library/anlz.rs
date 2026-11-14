//! Reading a Track's Rekordbox ANLZ file through `rbox`.
//!
//! The only place `rbox::anlz` types appear. Only the `.DAT` file's Beat Grid
//! (`PQTZ`) is read; ANLZ waveforms are out of scope.

use std::path::Path;

use rbox::anlz::Anlz;

use crate::waveform::beat_grid::Beat;

/// Why an ANLZ file couldn't be read.
#[derive(Debug, thiserror::Error)]
#[error("Could not read the ANLZ file: {0}")]
pub struct AnlzError(pub(crate) String);

/// The Beat Grid in the ANLZ file at `path`, in file order. `Ok(None)` if the
/// file has no Beat Grid.
pub fn read_beat_grid(path: &Path) -> Result<Option<Vec<Beat>>, AnlzError> {
    let mut anlz = Anlz::load(path).map_err(|e| AnlzError(e.to_string()))?;
    let grid = anlz.get_beat_grid().map_err(|e| AnlzError(e.to_string()))?;
    Ok(grid.map(|g| {
        g.beats
            .iter()
            .map(|b| Beat {
                time_ms: b.time,
                number: u8::try_from(b.beat_number).unwrap_or(u8::MAX),
            })
            .collect()
    }))
}
