# SyncBox

A desktop DJ library tool that brings a Rekordbox collection into its own database, plays it back, and (later) pushes selected changes back. A rewrite of mixxxa with no Python sidecar. This file is the project's glossary: the agreed word for each concept, and the words we've decided not to use. It is not a spec.

## Library

**Rekordbox Library**:
The user's Pioneer Rekordbox collection (`master.db` and its analysis files). The external source of truth SyncBox imports from.
_Avoid_: RB collection, the library (ambiguous with Local DB)

**Local DB**:
SyncBox's own database. Holds a copy of the Rekordbox Library plus everything SyncBox computes that Rekordbox doesn't know about. The UI only ever reads from here, never from the Rekordbox Library directly.
_Avoid_: App DB, cache, mirror

**Sync**:
Bringing the Rekordbox Library into the Local DB. One-way: a Sync never writes to Rekordbox.
_Avoid_: Import, resync, refresh, reimport

**Write-back**:
The deliberate, user-confirmed act of pushing selected Local DB changes into the Rekordbox Library. The opposite direction to Sync, and never part of it.
_Avoid_: Sync, export, save

**Track**:
One piece of music in the collection.
_Avoid_: Song, content, item, file

**Playlist**:
A named, ordered list of Tracks taken from the Rekordbox Library. Playlists sit in a tree of folders.
_Avoid_: Crate, list, set

**Cue**:
A user-visible marker at a point in a Track, as stored in the Rekordbox Library. What a DJ actually jumps to.
_Avoid_: Marker, point, bookmark

## Playback

**Beat Grid**:
The positions of a Track's beats, as stored by Rekordbox in its analysis files.
_Avoid_: Grid, beat markers, tempo map

**Waveform**:
SyncBox's drawn picture of a Track's loudness over time, made from Peaks. Not the colour waveform Rekordbox stores in its own analysis files, which SyncBox doesn't use yet.
_Avoid_: Wave, preview, overview (the overview is one view of the Waveform)

**Peaks**:
The reduced loudness values SyncBox computes from a Track's decoded audio and keeps in the Local DB, so its Waveform can be redrawn without decoding again.
_Avoid_: Waveform data, samples, cache

**Unplayable Reason**:
Why a Track can't be loaded: it's a streaming Track, its volume is offline, its file is missing, or it has no path. Each reason gets its own user guidance.
_Avoid_: Error, broken track, missing (that's only one reason)
