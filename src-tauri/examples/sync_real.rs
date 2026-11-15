//! Dev-only: run a Sync against your real Rekordbox `master.db` (opened
//! read-only) into a throwaway Local DB, and print what was copied.
//!
//!     cargo run --example sync_real [-- <path to master.db>]

use std::path::PathBuf;

use syncbox_lib::library::track_file::FileCheck;
use syncbox_lib::library::{Library, local_db, rekordbox};
use syncbox_lib::waveform::beat_grid;

fn main() {
    let master_db = match std::env::args().nth(1) {
        Some(path) => PathBuf::from(path),
        None => rekordbox::default_master_db_path().unwrap_or_else(|e| fail(e)),
    };
    let out = std::env::temp_dir().join("syncbox-sync-real.db");
    let _ = std::fs::remove_file(&out);
    println!(
        "master.db: {}\nLocal DB:  {}",
        master_db.display(),
        out.display()
    );

    let library = Library::new(local_db::open(&out).unwrap_or_else(|e| fail(e)));
    let started = std::time::Instant::now();
    let summary = library
        .sync(&master_db, &mut |p| {
            if p.done == p.total {
                println!("  {:?}: {}/{}", p.phase, p.done, p.total);
            }
        })
        .unwrap_or_else(|e| fail(e));
    println!("Synced in {:.2?}: {summary:?}", started.elapsed());

    let conn = library.conn();
    let tree = local_db::playlist_tree(&conn).unwrap();
    println!("Top-level playlist nodes: {}", tree.len());
    for node in tree.iter().take(10) {
        println!(
            "  {:?} {} ({} children)",
            node.kind,
            node.name,
            node.children.len()
        );
    }
    let tracks = local_db::all_tracks(&conn).unwrap();
    for t in tracks.iter().take(5) {
        println!("  {} – {} [{} {:?} bpm]", t.artist, t.title, t.key, t.bpm);
    }

    // The Beat Grid of the first analysed Track, read the way a Track load does.
    let analysed = tracks.iter().find(|t| {
        local_db::track_anlz_path(&conn, &t.id)
            .ok()
            .flatten()
            .is_some()
    });
    drop(conn);
    match analysed {
        Some(t) => match beat_grid::ensure_beat_grid(&library, &t.id) {
            Ok(Some(grid)) => {
                let downbeats = grid.beats.iter().filter(|b| b.number == 1).count();
                println!(
                    "Beat Grid of {:?}: {} beats, {downbeats} downbeats; first: {:?}",
                    t.title,
                    grid.beats.len(),
                    grid.beats.iter().take(4).collect::<Vec<_>>()
                );
            }
            Ok(None) => println!("{:?} has an ANLZ path but no readable Beat Grid", t.title),
            Err(e) => println!("reading the Beat Grid failed: {e}"),
        },
        None => println!("No Track has an ANLZ path"),
    }

    let started = std::time::Instant::now();
    let mut check = FileCheck::on_disk();
    let reasons: Vec<_> = tracks
        .iter()
        .filter_map(|t| check.reason(t.file_path.as_deref()))
        .collect();
    let mut by_kind = std::collections::BTreeMap::<&str, (usize, String)>::new();
    for reason in &reasons {
        let entry = by_kind
            .entry(reason.kind())
            .or_insert((0, reason.message()));
        entry.0 += 1;
    }
    println!(
        "Checked {} files in {:.2?}; unplayable:",
        tracks.len(),
        started.elapsed()
    );
    for (kind, (n, example)) in by_kind {
        println!("  {kind}: {n} (e.g. \"{example}\")");
    }
}

fn fail(e: impl std::fmt::Display) -> ! {
    eprintln!("error: {e}");
    std::process::exit(1)
}
