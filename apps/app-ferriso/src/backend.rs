//! Boundary between the native frontend and the Theseus backend.
//!
//! The actor owns backend lifecycle. This module only exposes the pieces the
//! frontend needs before the actor starts (the persisted settings used for the
//! first frame) and the long-lived event stream the actor subscribes to.

use futures::{Stream, stream};
use theseus::{AppEvent, EventState, data::Settings};

/// The same identifier as the Tauri app, so native and web frontends share the
/// existing database, instances, accounts, and settings.
pub const APP_IDENTIFIER: &str = "ModrinthApp";

/// Read persisted settings before the actor's `State::init` has run.
///
/// The GUI calls this before opening its window so the first frame already has
/// the saved theme and locale. Errors are deliberately non-fatal: boot will
/// initialize the backend and surface the real failure in the UI.
pub fn startup_settings() -> Result<Settings, String> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|error| {
            format!("could not start settings preload runtime: {error}")
        })?;
    runtime
        .block_on(theseus::settings::get_startup(APP_IDENTIFIER))
        .map_err(|error| error.to_string())
}

/// One item from the backend event channel.
#[derive(Clone)]
pub enum BackendEvent {
    Event(AppEvent),
    /// The bounded channel dropped this many events; the actor must refresh
    /// job, process, and instance snapshots instead of trusting incremental
    /// updates.
    Lagged(u64),
}

/// Backend events as a frontend-neutral stream.
///
/// Returns `None` until `EventState::init` has completed. Lag is surfaced
/// explicitly so the actor can resynchronize instead of silently missing
/// incremental updates.
pub fn event_stream()
-> Option<impl Stream<Item = BackendEvent> + Send + 'static> {
    let receiver = EventState::get_if_initialized()?.subscribe();
    Some(stream::unfold(receiver, |mut receiver| async move {
        match receiver.recv().await {
            Ok(event) => Some((BackendEvent::Event(event), receiver)),
            Err(tokio::sync::broadcast::error::RecvError::Lagged(skipped)) => {
                Some((BackendEvent::Lagged(skipped), receiver))
            }
            Err(tokio::sync::broadcast::error::RecvError::Closed) => None,
        }
    }))
}
