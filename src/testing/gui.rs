//! Simple GUI for testing the rustic audio system

use crate::audio::{AudioError, BackendEvent};
use crate::prelude::*;
use crate::{AudioHandle, start_app};
use eframe::egui;
use std::sync::mpsc::{Receiver, Sender, TryRecvError, channel};

/// Main testing GUI application
pub struct TestingApp {
    /// Handle to the audio system
    audio_handle: Option<AudioHandle>,

    /// Channel to send commands to the audio system
    command_tx: Sender<Commands>,

    /// Channel to receive events from the audio system
    event_rx: Receiver<BackendEvent>,

    /// Current octave for testing
    octave: u8,

    /// Current volume (0.0 - 1.0)
    volume: f32,

    /// Event log entries
    event_log: Vec<String>,

    /// Current metrics
    buffer_underruns: u64,
    sample_rate: u32,

    /// Whether a note is currently playing
    note_playing: bool,
}

impl TestingApp {
    fn new() -> Result<Self, AudioError> {
        // Create bidirectional channels
        let (backend_tx, event_rx) = channel();
        let (command_tx, backend_rx) = channel();

        // Start the audio system
        let audio_handle = start_app(backend_tx, backend_rx)?;

        Ok(Self {
            audio_handle: Some(audio_handle),
            command_tx,
            event_rx,
            octave: 4,
            volume: 0.5,
            event_log: Vec::new(),
            buffer_underruns: 0,
            sample_rate: 0,
            note_playing: false,
        })
    }

    /// Send a command to the audio system
    fn send_command(&mut self, cmd: Commands) {
        if let Err(e) = self.command_tx.send(cmd) {
            self.event_log.push(format!("Error sending command: {}", e));
        }
    }

    /// Process backend events
    fn process_events(&mut self) {
        loop {
            match self.event_rx.try_recv() {
                Ok(event) => match &event {
                    BackendEvent::AudioStarted { sample_rate } => {
                        self.sample_rate = *sample_rate;
                        self.event_log
                            .push(format!("Audio started: {} Hz", sample_rate));
                    }
                    BackendEvent::AudioStopped => {
                        self.event_log.push("Audio stopped".to_string());
                    }
                    BackendEvent::CommandError { command, error } => {
                        self.event_log
                            .push(format!("Command error: {} - {}", command, error));
                    }
                    BackendEvent::BufferUnderrun { count } => {
                        self.buffer_underruns = *count;
                        self.event_log.push(format!("Buffer underrun #{}", count));
                    }
                    BackendEvent::Metrics {
                        cpu_usage,
                        latency_ms,
                    } => {
                        self.event_log.push(format!(
                            "Metrics: CPU={:.1}%, Latency={:.1}ms",
                            cpu_usage, latency_ms
                        ));
                    }
                },
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    self.event_log
                        .push("Event channel disconnected".to_string());
                    break;
                }
            }
        }

        // Update metrics if audio handle exists
        if let Some(handle) = &self.audio_handle {
            let metrics = handle.get_metrics();
            self.buffer_underruns = metrics.buffer_underruns;
            self.sample_rate = metrics.sample_rate;
        }
    }
}

impl eframe::App for TestingApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Process events from the audio system
        self.process_events();

        // Request continuous repaints to keep UI responsive
        ctx.request_repaint();

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Rustic Audio System - Testing GUI");
            ui.add_space(10.0);

            // Audio Controls
            ui.group(|ui| {
                ui.label("Audio Controls");
                ui.add_space(5.0);

                ui.horizontal(|ui| {
                    if ui.button("Start Note (Middle C)").clicked() {
                        // NoteName::C corresponds to note value 0
                        self.send_command(Commands::NoteStart(0, 0, self.volume));
                        self.note_playing = true;
                    }

                    if ui.button("Stop Note").clicked() {
                        // NoteName::C corresponds to note value 0
                        self.send_command(Commands::NoteStop(0, 0));
                        self.note_playing = false;
                    }

                    ui.label(if self.note_playing {
                        "🔊 Playing"
                    } else {
                        "🔇 Silent"
                    });
                });

                ui.add_space(5.0);

                ui.horizontal(|ui| {
                    ui.label("Volume (velocity):");
                    ui.add(egui::Slider::new(&mut self.volume, 0.0..=1.0).text(""));
                    ui.label(format!("{:.0}%", self.volume * 100.0));
                });
            });

            ui.add_space(10.0);

            // Octave Controls
            ui.group(|ui| {
                ui.label("Octave Controls");
                ui.add_space(5.0);

                ui.horizontal(|ui| {
                    if ui.button("Octave Down").clicked() && self.octave > 0 {
                        self.octave -= 1;
                        self.send_command(Commands::SetOctave(self.octave, 0));
                    }

                    ui.label(format!("Current Octave: {}", self.octave));

                    if ui.button("Octave Up").clicked() && self.octave < 8 {
                        self.octave += 1;
                        self.send_command(Commands::SetOctave(self.octave, 0));
                    }
                });
            });

            ui.add_space(10.0);

            // Metrics Display
            ui.group(|ui| {
                ui.label("System Metrics");
                ui.add_space(5.0);

                ui.horizontal(|ui| {
                    ui.label(format!("Sample Rate: {} Hz", self.sample_rate));
                    ui.separator();
                    ui.label(format!("Buffer Underruns: {}", self.buffer_underruns));
                });
            });

            ui.add_space(10.0);

            // Event Log
            ui.group(|ui| {
                ui.label("Event Log");
                ui.add_space(5.0);

                egui::ScrollArea::vertical()
                    .max_height(200.0)
                    .stick_to_bottom(true)
                    .show(ui, |ui| {
                        if self.event_log.is_empty() {
                            ui.label("No events yet...");
                        } else {
                            for (i, event) in self.event_log.iter().enumerate().rev().take(50) {
                                ui.label(format!("[{}] {}", i, event));
                            }
                        }
                    });

                if ui.button("Clear Log").clicked() {
                    self.event_log.clear();
                }
            });

            ui.add_space(10.0);

            // Quit Button
            ui.horizontal(|ui| {
                if ui.button("Quit").clicked() {
                    self.send_command(Commands::Quit);
                    std::process::exit(0);
                }
            });
        });
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        // Send quit command
        let _ = self.command_tx.send(Commands::Quit);

        // Shutdown audio system
        if let Some(handle) = self.audio_handle.take() {
            let _ = handle.shutdown();
        }
    }
}

/// Run the testing GUI
pub fn run_testing_gui() -> Result<(), Box<dyn std::error::Error>> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([600.0, 700.0])
            .with_resizable(true),
        ..Default::default()
    };

    eframe::run_native(
        "Rustic Audio Testing",
        options,
        Box::new(|_cc| {
            let app = TestingApp::new().expect("Failed to initialize audio system");
            Ok(Box::new(app))
        }),
    )?;

    Ok(())
}
