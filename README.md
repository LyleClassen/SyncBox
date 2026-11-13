# SyncBox

A desktop DJ library tool (Tauri + Rust + React) that Syncs a Rekordbox Library into its own Local DB and plays it back. See [CONTEXT.md](CONTEXT.md) for vocabulary and [docs/specs](docs/specs) for specs.

## Building

Requires Rust, [Bun](https://bun.sh) and the [Tauri prerequisites](https://tauri.app/start/prerequisites/).

`rbox` reads Rekordbox's encrypted `master.db` through a bundled SQLCipher, which builds OpenSSL from source. On Windows that needs a full Perl:

```powershell
winget install StrawberryPerl.StrawberryPerl
# Git Bash's Perl comes first on PATH and can't build OpenSSL, so point the build at Strawberry's:
$env:OPENSSL_SRC_PERL = "C:\Strawberry\perl\bin\perl.exe"
```

The first build is slow: OpenSSL and diesel (with 128-column tables, enabled by `rbox`) take several minutes and a lot of RAM. Later builds are cached.

```sh
bun install
bun run tauri dev
```

## Testing

```sh
bun run test        # UI unit tests (Camelot, track rows)
cd src-tauri
cargo test --lib
# Dev-only: Sync your real master.db (opened read-only) into a throwaway Local DB
cargo run --example sync_real
```
