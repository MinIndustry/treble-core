//! Command processing thread implementation

use crate::app::prelude::*;
use std::sync::Arc;
use crate::app::commands::SystemCommand;
use std::sync::atomic::Ordering;
use std::sync::mpsc::{Receiver, Sender};
use std::thread::{self, JoinHandle};

use super::events::BackendEvent;
use super::messages::AudioMessage;
use super::shared_state::SharedAudioState;

/// Spawns the command processing thread
///
/// This thread:
/// - Receives commands from the frontend
/// - Validates commands
/// - Updates app state
/// - Translates commands to audio messages
/// - Sends audio messages to the render thread
/// - Reports errors and events back to the frontend
pub fn spawn_command_thread(
    mut app: App,
    shared_state: Arc<SharedAudioState>,
    command_rx: Receiver<Command>,
    event_tx: Sender<BackendEvent>,
    message_tx: crossbeam::channel::Sender<AudioMessage>,
) -> JoinHandle<()> {
    thread::Builder::new()
        .name("command-processor".to_string())
        .spawn(move || {
            log::info!("Command thread started");

            loop {
                match command_rx.recv() {
                    Ok(Command::System(SystemCommand::Quit)) => {
                        log::info!("Quit command received");
                        shared_state.shutdown.store(true, Ordering::Release);
                        let _ = message_tx.send(AudioMessage::Shutdown);
                        let _ = event_tx.send(BackendEvent::AudioStopped);
                        break;
                    }
                    Ok(cmd) => {
                        // Validate command
                        if let Err(e) = cmd.validate(&app) {
                            let _ = event_tx.send(BackendEvent::CommandError {
                                command: format!("{:?}", cmd),
                                error: e.to_string(),
                            });
                            log::warn!("Command validation failed: {:?} - {}", cmd, e);
                            continue;
                        }

                        // Update app state
                        app.on_event(cmd.clone());

                        // Translate to audio message
                        if let Some(msg) = cmd.translate_to_audio_message(&mut app)
                            && message_tx.send(msg.clone()).is_err()
                        {
                            // Channel closed - audio thread has shut down
                            log::warn!("Audio message channel closed, dropping command: {:?}", cmd);
                        }
                    }
                    Err(_) => {
                        log::info!("Command channel closed");
                        break;
                    } // Channel closed
                }
            }

            log::info!("Command thread shutting down");
        })
        .expect("Failed to spawn command thread")
}
