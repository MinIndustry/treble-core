/// Number of audio channels
pub const CHANNELS: usize = 2;

/// A Single multi-channel audio sample
pub type Frame = [f32; CHANNELS];

/// A block of frames
pub type Block = Vec<Frame>;

/// Helper: create a silent block of `n` samples
pub fn silent_block(n: usize) -> Block {
    vec![[0.0; CHANNELS]; n]
}

/// Helper: mono sample to n channels (e.g. L = R = sample for stereo)
pub fn mono_to_frame(s: f32) -> Frame {
    [s; CHANNELS]
}
