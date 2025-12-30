//! Audio configuration types with serialization support

use serde::{Deserialize, Serialize};

/// Audio configuration for buffer sizes and latency settings
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AudioConfig {
    /// cpal buffer size in samples (lower = less latency, more CPU)
    #[serde(default = "default_cpal_buffer_size")]
    pub cpal_buffer_size: usize,

    /// Audio render chunk size in samples
    #[serde(default = "default_render_chunk_size")]
    pub render_chunk_size: usize,

    /// Audio ring buffer size in samples (higher = more buffer, higher latency)
    #[serde(default = "default_audio_ring_buffer_size")]
    pub audio_ring_buffer_size: usize,

    /// Message ring buffer size in messages
    #[serde(default = "default_message_ring_buffer_size")]
    pub message_ring_buffer_size: usize,

    /// Target maximum latency in milliseconds
    #[serde(default = "default_target_latency_ms")]
    pub target_latency_ms: f32,
}

fn default_cpal_buffer_size() -> usize {
    64
}
fn default_render_chunk_size() -> usize {
    256
}
fn default_audio_ring_buffer_size() -> usize {
    88200
} // 2s @ 44.1kHz
fn default_message_ring_buffer_size() -> usize {
    1024
}
fn default_target_latency_ms() -> f32 {
    50.0
}

impl Default for AudioConfig {
    fn default() -> Self {
        Self {
            cpal_buffer_size: default_cpal_buffer_size(),
            render_chunk_size: default_render_chunk_size(),
            audio_ring_buffer_size: default_audio_ring_buffer_size(),
            message_ring_buffer_size: default_message_ring_buffer_size(),
            target_latency_ms: default_target_latency_ms(),
        }
    }
}

impl AudioConfig {
    /// Calculate ring buffer size based on sample rate and target latency
    pub fn calculate_ring_buffer_size(&self, sample_rate: u32) -> usize {
        ((sample_rate as f32 * self.target_latency_ms) / 1000.0) as usize
    }

    /// Validate configuration values
    pub fn validate(&self) -> Result<(), String> {
        if self.cpal_buffer_size == 0 || self.cpal_buffer_size > 2048 {
            return Err("cpal_buffer_size must be between 1 and 2048".to_string());
        }
        if self.render_chunk_size < self.cpal_buffer_size {
            return Err("render_chunk_size must be >= cpal_buffer_size".to_string());
        }
        if self.audio_ring_buffer_size < self.render_chunk_size * 2 {
            return Err("audio_ring_buffer_size too small".to_string());
        }
        Ok(())
    }
}

/// Logging configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LogConfig {
    /// Log level: trace, debug, info, warn, error
    #[serde(default = "default_log_level")]
    pub level: String,

    /// Enable logging to file
    #[serde(default)]
    pub log_to_file: bool,

    /// Log file path (relative to config directory)
    #[serde(default = "default_log_file")]
    pub log_file: String,

    /// Enable logging to stdout
    #[serde(default = "default_log_to_stdout")]
    pub log_to_stdout: bool,
}

fn default_log_level() -> String {
    "info".to_string()
}
fn default_log_file() -> String {
    "rustic.log".to_string()
}
fn default_log_to_stdout() -> bool {
    true
}

impl Default for LogConfig {
    fn default() -> Self {
        Self {
            level: default_log_level(),
            log_to_file: false,
            log_file: default_log_file(),
            log_to_stdout: default_log_to_stdout(),
        }
    }
}
