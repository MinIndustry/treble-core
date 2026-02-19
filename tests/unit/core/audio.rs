use rustic::core::{
    CHANNELS, Frame,
    audio::{mono_to_frame, silent_block},
};

#[test]
fn test_mono_to_frame() {
    let mono_sample: f32 = 12.0;
    let frame: Frame = mono_to_frame(mono_sample);

    assert_eq!(frame, [mono_sample; CHANNELS]);
}

#[test]
fn test_silent_block() {
    const NUM_SAMPLES: usize = 25;
    let silent_block = silent_block(NUM_SAMPLES);
    assert_eq!(silent_block.len(), NUM_SAMPLES);
    for i in 0..NUM_SAMPLES {
        assert_eq!(silent_block[i], [0.0; CHANNELS]);
    }
}
