# Spec 0001: Rekordbox Sync and playback

Status: agreed, waiting for your final confirmation
Vocabulary: [CONTEXT.md](../../CONTEXT.md). Decisions: [ADR-0001](../adr/0001-local-db-copy-of-rekordbox.md), [ADR-0002](../adr/0002-native-rust-playback.md), [ADR-0003](../adr/0003-separate-synced-and-local-tables.md).
Reference implementation: mixxxa (`apps/mixxxa/src/bun/rpc/rekordbox.ts`, `bun/db/localDb.ts`, `bun/paths/resolveTrackFile.ts`, `shared/trackPath.ts`, `shared/camelot.ts`, `mainview/features/player/WaveformPlayer.tsx`).

## Goal

Open SyncBox, Sync the Rekordbox Library into the Local DB, browse Playlists and Tracks, and play a Track with a Waveform, Beat Grid and Cues. No Python sidecar and no local HTTP server: all of this runs in Rust inside the Tauri process.

## Out of scope

Write-back of any kind · editing Cues, Playlists or Track metadata · analysis (key/BPM/energy…) · drawing Rekordbox's ANLZ colour waveforms · artwork · volume-mapping settings · computing Peaks in the background · pitch/tempo · choosing the output device · auto-playing the next Track.

## 1. Sync (Rekordbox → Local DB)

- Use the `rbox` crate (0.3.x) with the `master-db` feature (this also enables `anlz`), bundled SQLCipher. The default `master.db` location comes from `rbox`/the platform: `%APPDATA%\Pioneer\rekordbox\master.db` on Windows and `~/Library/Pioneer/rekordbox/master.db` on macOS.
- **What's copied:** the Playlist tree (folders, playlists, smart playlists), playlist entries in order, Tracks (title, artist, album, BPM, key, length, bitrate, rating, file path), artists, albums, keys, and Cues (hot and memory; skip deleted cues and cues with a negative position; resolve colours the way mixxxa's `mapCues` does).
- **Beat Grid:** not read during a Sync. It's read from the Track's ANLZ file via `rbox` on first load, stored as SyncBox-only data, and refreshed when the ANLZ file changes (compare its modified time).
- **Storage (ADR-0003):** Rekordbox-copied tables are replaced inside a single transaction. SyncBox-only tables (Peaks and Beat Grid) are left alone, except that rows for Tracks no longer in Rekordbox are removed.
- **Starting a Sync:** a manual Sync button, plus one automatic Sync on launch when the Local DB is empty.
- **Progress:** the Rust side sends Tauri events for each phase (playlists → tracks → cues) and the UI shows a determinate Astryx `ProgressBar`.
- **Errors**, each with its own guidance:
  - `master.db` not found → "Install and open Rekordbox first."
  - It can't be opened or read and Rekordbox is running → "Close Rekordbox and try again."
  - Otherwise unreadable → "may be corrupted or inaccessible."
  - A failed Sync leaves the previous Local DB unchanged (the transaction rolls back).

## 2. Browsing

- **Layout:** the same as mixxxa. Player across the top; Astryx `AppShell` with a `SideNav`/`TreeList` holding the Playlist tree; a `Resizable` split; the track table fills the rest. Gothic theme.
- **All Tracks** is the first entry in the sidebar, above the Playlist tree.
- **Filter:** a text box that filters the current list by title, artist and album.
- **Track table:** Astryx `Table` with virtualization, sorting and column resize. Columns: #, Title, Artist, Album, BPM, Key, Length, Bitrate, Rating.
- **Key** is shown in Camelot notation (port `shared/camelot.ts` to Rust or TypeScript), with the Rekordbox key name in a tooltip.
- Each row shows its Unplayable Reason, if it has one.

## 3. Playback (ADR-0002)

- Rust decodes with `symphonia` (must handle MP3, AAC/M4A, WAV, AIFF and FLAC) and outputs with `cpal`/`rodio` to the default device.
- **Commands:** load(trackId), play, pause, toggle, seek(sec), setVolume(0–1). **Events:** position (often enough for smooth scrolling, e.g. about 30 Hz or interpolated in the UI), state (playing/paused/ended), load errors.
- **Interaction:** double-click a row to load it, Space to play/pause, click or drag the Waveform to seek, click a Cue to jump to it, and a volume slider. Volume and zoom are remembered for each user.
- **Unplayable Reasons** (port `resolveTrackFile` + `classifyRekordboxPath`): streaming, volume offline, missing, no path. Check them when a Track is loaded and never cache the result, because drives come and go.

## 4. Waveform

- **Peaks:** on a Track's first load, Rust decodes it and reduces it to 8,000 mono peaks plus duration (the same shape as mixxxa), then stores them in a SyncBox-only table. Later loads read the stored Peaks. Computing Peaks must not hold up the start of playback.
- **Rendering:** a custom canvas component (no wavesurfer) that copies mixxxa's behaviour:
  - a zoomed view with a fixed centre playhead and the Waveform scrolling underneath it, padded half a screen width at each end;
  - a minimap overview above it showing the visible window and Cue ticks;
  - zoom from 10 to 500 px/s, default 50;
  - colours from Astryx theme tokens.
- **Overlays:** the Beat Grid (from ANLZ) and Cue markers (read-only, click to jump).
- ANLZ colour waveforms come later. The renderer should take its colour from the data source so it can grow into them.

## 5. Theme

Dark mode only (gothic theme, `mode="dark"`). Light mode is out of scope.

## 6. Testing

- **Rust unit tests:**
  - Sync table separation, including removing SyncBox-only rows for deleted Tracks;
  - Unplayable Reason classification (port mixxxa's `trackPath.test.ts` cases);
  - Camelot mapping;
  - Cue mapping (skipping deleted cues and cues with a negative position, colour resolution).
- **A dev-only command** that runs a Sync against the real `master.db`, opened read-only. No sample `master.db` is committed, since it's encrypted and personal.
- **Manual playback checks** with one file in each format: MP3, M4A, WAV, AIFF and FLAC.
