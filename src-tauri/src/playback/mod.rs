//! Playing Tracks in Rust (ADR-0002): symphonia decodes, rodio/cpal outputs
//! to the default device. The webview only sends commands and shows events.

pub mod commands;
pub mod engine;
pub mod error;
pub mod rodio_deck;
pub mod transport;

use tauri::{AppHandle, Emitter};

use engine::{Engine, EngineEvent};

/// Starts the audio engine, sending its events to the webview.
pub fn start(app: AppHandle) -> Engine {
    Engine::start(move |event| {
        let _ = match event {
            EngineEvent::State(state) => app.emit(commands::STATE_EVENT, state),
            EngineEvent::Position { track_id, position } => app.emit(
                commands::POSITION_EVENT,
                serde_json::json!({ "trackId": track_id, "position": position }),
            ),
        };
    })
}
