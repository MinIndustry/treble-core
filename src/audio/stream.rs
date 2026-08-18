//! Output-device discovery and stream ownership.

use std::sync::Arc;
use std::thread::{self, JoinHandle};

use cpal::SampleRate;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use crossbeam::channel::{Receiver, Sender, bounded};
use crossbeam::queue::ArrayQueue;

use super::{AudioConfig, AudioError, BackendEvent, ErrorEvent, EventSender, SharedAudioState};

pub(crate) struct OutputStreamRuntime {
    shutdown_tx: Sender<()>,
    thread: JoinHandle<()>,
}

impl OutputStreamRuntime {
    pub fn shutdown(self) {
        let _ = self.shutdown_tx.send(());
        let _ = self.thread.join();
    }
}

pub(crate) struct OutputStreamInfo {
    pub device_name: String,
    pub sample_rate: u32,
}

pub fn output_device_names() -> Result<Vec<String>, AudioError> {
    let host = cpal::default_host();
    let devices = host
        .output_devices()
        .map_err(|error| AudioError::StreamError(error.to_string()))?;
    Ok(devices.filter_map(|device| device.name().ok()).collect())
}

pub(crate) fn spawn_output_stream(
    requested_device: Option<String>,
    audio_queue: Arc<ArrayQueue<f32>>,
    shared_state: Arc<SharedAudioState>,
    config: AudioConfig,
    event_tx: EventSender,
) -> Result<(OutputStreamRuntime, OutputStreamInfo), AudioError> {
    let (startup_tx, startup_rx) = bounded(1);
    let (shutdown_tx, shutdown_rx) = bounded(1);
    let thread = thread::Builder::new()
        .name("treble-output-stream".into())
        .spawn(move || {
            run_output_stream(
                requested_device,
                audio_queue,
                shared_state,
                config,
                event_tx,
                startup_tx,
                shutdown_rx,
            );
        })
        .map_err(|error| AudioError::StreamError(error.to_string()))?;

    match startup_rx.recv() {
        Ok(Ok(info)) => Ok((
            OutputStreamRuntime {
                shutdown_tx,
                thread,
            },
            info,
        )),
        Ok(Err(error)) => {
            let _ = thread.join();
            Err(AudioError::StreamError(error))
        }
        Err(_) => {
            let _ = thread.join();
            Err(AudioError::StreamError(
                "output stream thread exited during startup".into(),
            ))
        }
    }
}

fn run_output_stream(
    requested_device: Option<String>,
    audio_queue: Arc<ArrayQueue<f32>>,
    shared_state: Arc<SharedAudioState>,
    config: AudioConfig,
    event_tx: EventSender,
    startup_tx: Sender<Result<OutputStreamInfo, String>>,
    shutdown_rx: Receiver<()>,
) {
    let result = (|| {
        let host = cpal::default_host();
        let device = match requested_device.as_deref() {
            Some(requested) => host
                .output_devices()
                .map_err(|error| error.to_string())?
                .find(|device| device.name().is_ok_and(|name| name == requested))
                .ok_or_else(|| format!("output device '{requested}' is no longer available"))?,
            None => host
                .default_output_device()
                .ok_or_else(|| "no default output device is available".to_string())?,
        };
        let device_name = device
            .name()
            .unwrap_or_else(|_| "Unnamed output device".into());
        let best_config = device
            .supported_output_configs()
            .map_err(|error| error.to_string())?
            .map(|candidate| {
                let supports_preferred = candidate.min_sample_rate().0 <= 44_100
                    && candidate.max_sample_rate().0 >= 44_100;
                let stereo = candidate.channels() == crate::core::audio::CHANNELS as u16;
                (
                    u8::from(supports_preferred) + u8::from(stereo) * 2,
                    candidate,
                )
            })
            .max_by_key(|(score, _)| *score)
            .map(|(_, candidate)| candidate)
            .ok_or_else(|| format!("output device '{device_name}' has no supported format"))?;
        let sample_rate = 44_100
            .min(best_config.max_sample_rate().0)
            .max(best_config.min_sample_rate().0);
        let mut stream_config = best_config
            .with_sample_rate(SampleRate(sample_rate))
            .config();
        stream_config.buffer_size = cpal::BufferSize::Fixed(config.cpal_buffer_size as u32);
        shared_state
            .sample_rate
            .store(sample_rate, std::sync::atomic::Ordering::Relaxed);
        let callback = super::create_cpal_callback(audio_queue, Arc::clone(&shared_state));
        let errors = event_tx.clone();
        let stream = device
            .build_output_stream(
                &stream_config,
                callback,
                move |error| {
                    errors.send(BackendEvent::Error(ErrorEvent::AudioStream {
                        message: error.to_string(),
                    }));
                },
                None,
            )
            .map_err(|error| error.to_string())?;
        stream.play().map_err(|error| error.to_string())?;
        startup_tx
            .send(Ok(OutputStreamInfo {
                device_name,
                sample_rate,
            }))
            .map_err(|_| "output stream startup receiver closed".to_string())?;
        let _ = shutdown_rx.recv();
        drop(stream);
        Ok::<(), String>(())
    })();

    if let Err(error) = result {
        let _ = startup_tx.send(Err(error));
    }
}
