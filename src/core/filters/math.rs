//! Sample-wise arithmetic and explicit channel/domain adapters.
use crate::core::{
    Block,
    graph::{Entry, Filter, ParameterModulation, Source, modulated_value},
};
use std::sync::Arc;
use treble_meta::{FilterInfo, MetaFilter};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MathOperation {
    Add,
    Multiply,
    Minimum,
    Maximum,
    Absolute,
    Offset,
    Clamp,
    Split,
    Merge,
    MonoToStereo,
    ControlToAudio,
    MonoToControl,
}

/// Mono buffers use channel zero and duplicate it in the internal stereo block.
/// Port types, not the internal block width, determine legal connections.
#[derive(Clone, Debug)]
pub struct MathFilter {
    operation: MathOperation,
    inputs: [Option<Arc<Block>>; 2],
    offset: f32,
    minimum: f32,
    maximum: f32,
}
fn finite(value: f32) -> f64 {
    if value.is_finite() {
        f64::from(value)
    } else {
        0.0
    }
}
fn bounded(value: f64) -> f32 {
    if value.is_nan() {
        0.0
    } else {
        value.clamp(-f64::from(f32::MAX), f64::from(f32::MAX)) as f32
    }
}
impl MathFilter {
    pub fn new(operation: MathOperation) -> Self {
        Self {
            operation,
            inputs: [None, None],
            offset: 0.0,
            minimum: -1.0,
            maximum: 1.0,
        }
    }
}
impl std::fmt::Display for MathFilter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.operation)
    }
}
impl Entry for MathFilter {
    fn push(&mut self, block: Arc<Block>, port: usize) {
        if let Some(input) = self.inputs.get_mut(port) {
            *input = Some(block);
        }
    }
}
impl MetaFilter for MathFilter {
    fn metadata() -> FilterInfo {
        FilterInfo {
            name: "Math",
            type_id: "treble.math",
            description: "Sample-wise arithmetic",
            inputs: vec![],
            outputs: 1,
        }
    }
    fn supports_parameter(&self, name: &str) -> bool {
        matches!(
            (self.operation, name),
            (MathOperation::Offset, "offset") | (MathOperation::Clamp, "minimum" | "maximum")
        )
    }
    fn set_parameter(&mut self, name: &str, value: f32) -> bool {
        if !value.is_finite() || !self.supports_parameter(name) {
            return false;
        }
        match name {
            "offset" => self.offset = value,
            "minimum" => self.minimum = value,
            "maximum" => self.maximum = value,
            _ => return false,
        }
        true
    }
}
impl Filter for MathFilter {
    fn transform(&mut self) -> Vec<Block> {
        self.transform_modulated(&[])
    }
    fn transform_modulated(&mut self, modulation: &[ParameterModulation]) -> Vec<Block> {
        let inputs = [self.inputs[0].take(), self.inputs[1].take()];
        let frames = inputs.iter().flatten().map(|b| b.len()).max().unwrap_or(0);
        let sample = |port: usize, frame: usize, channel: usize| {
            inputs[port]
                .as_ref()
                .and_then(|b| b.get(frame))
                .map_or(0.0, |f| f[channel])
        };
        if self.operation == MathOperation::Split {
            return (0..2)
                .map(|channel| {
                    (0..frames)
                        .map(|i| [bounded(finite(sample(0, i, channel))); 2])
                        .collect()
                })
                .collect();
        }
        let output = (0..frames)
            .map(|i| {
                if self.operation == MathOperation::Merge {
                    return [
                        bounded(finite(sample(0, i, 0))),
                        bounded(finite(sample(1, i, 0))),
                    ];
                }
                if matches!(
                    self.operation,
                    MathOperation::MonoToStereo
                        | MathOperation::ControlToAudio
                        | MathOperation::MonoToControl
                ) {
                    let value = bounded(finite(sample(0, i, 0)));
                    return [if self.operation == MathOperation::MonoToControl {
                        value.clamp(-1.0, 1.0)
                    } else {
                        value
                    }; 2];
                }
                let offset = modulated_value(modulation, "offset", self.offset, i);
                let minimum = modulated_value(modulation, "minimum", self.minimum, i);
                let maximum = modulated_value(modulation, "maximum", self.maximum, i);
                std::array::from_fn(|channel| {
                    let a = finite(sample(0, i, channel));
                    let b = finite(sample(1, i, channel));
                    bounded(match self.operation {
                        MathOperation::Add => a + b,
                        MathOperation::Multiply => a * b,
                        MathOperation::Minimum => a.min(b),
                        MathOperation::Maximum => a.max(b),
                        MathOperation::Absolute => a.abs(),
                        MathOperation::Offset => a + finite(offset),
                        MathOperation::Clamp => {
                            a.clamp(finite(minimum.min(maximum)), finite(minimum.max(maximum)))
                        }
                        _ => unreachable!("adapters handled above"),
                    })
                })
            })
            .collect();
        vec![output]
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

/// Always available for math/control patches, independent of note routing.
/// As an audio source it responds to explicit start/stop/panic controls.
#[derive(Clone, Debug)]
pub struct ConstantSource {
    value: f32,
    active: bool,
}
impl ConstantSource {
    pub fn new(value: f32) -> Self {
        Self {
            value: bounded(finite(value)),
            active: true,
        }
    }
}
impl Source for ConstantSource {
    fn pull(&mut self, frames: usize) -> Block {
        self.pull_modulated(frames, &[])
    }
    fn pull_modulated(&mut self, frames: usize, inputs: &[ParameterModulation]) -> Block {
        (0..frames)
            .map(|i| {
                [if self.active {
                    modulated_value(inputs, "value", self.value, i)
                } else {
                    0.0
                }; 2]
            })
            .collect()
    }
    fn is_active(&self) -> bool {
        self.active
    }
    fn start(&mut self) {
        self.active = true;
    }
    fn stop(&mut self) {
        self.active = false;
    }
    fn kill(&mut self) {
        self.stop();
    }
    fn supports_parameter(&self, name: &str) -> bool {
        name == "value"
    }
    fn set_parameter(&mut self, name: &str, value: f32) -> bool {
        if name != "value" || !value.is_finite() {
            return false;
        }
        self.value = value;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn run(operation: MathOperation, a: Block, b: Block) -> Vec<Block> {
        let mut filter = MathFilter::new(operation);
        filter.push(Arc::new(a), 0);
        filter.push(Arc::new(b), 1);
        filter.transform()
    }
    #[test]
    fn arithmetic_preserves_stereo_signs_and_does_not_clip_to_audio_range() {
        let a = vec![[2.0, -0.5], [-0.25, 0.5]];
        let b = vec![[3.0, -2.0], [2.0, 4.0]];
        assert_eq!(
            run(MathOperation::Add, a.clone(), b.clone())[0],
            vec![[5.0, -2.5], [1.75, 4.5]]
        );
        assert_eq!(
            run(MathOperation::Multiply, a.clone(), b.clone())[0],
            vec![[6.0, 1.0], [-0.5, 2.0]]
        );
        assert_eq!(
            run(MathOperation::Minimum, a.clone(), b.clone())[0],
            vec![[2.0, -2.0], [-0.25, 0.5]]
        );
        assert_eq!(
            run(MathOperation::Maximum, a.clone(), b)[0],
            vec![[3.0, -0.5], [2.0, 4.0]]
        );
        assert_eq!(
            run(MathOperation::Absolute, a, vec![])[0],
            vec![[2.0, 0.5], [0.25, 0.5]]
        );
    }
    #[test]
    fn missing_inputs_are_zero_and_old_blocks_are_consumed() {
        let mut filter = MathFilter::new(MathOperation::Multiply);
        filter.push(Arc::new(vec![[1.0; 2]; 3]), 1);
        assert_eq!(filter.transform()[0], vec![[0.0; 2]; 3]);
        assert!(filter.transform()[0].is_empty());
    }
    #[test]
    fn clamp_accepts_crossing_bounds_and_arithmetic_stays_finite() {
        let mut filter = MathFilter::new(MathOperation::Clamp);
        assert!(filter.set_parameter("minimum", 0.5));
        assert!(filter.set_parameter("maximum", -0.5));
        filter.push(Arc::new(vec![[-2.0, 2.0]]), 0);
        assert_eq!(filter.transform()[0], vec![[-0.5, 0.5]]);
        let result = run(
            MathOperation::Multiply,
            vec![[f32::MAX, f32::NAN]],
            vec![[f32::MAX, 1.0]],
        );
        assert_eq!(result[0], vec![[f32::MAX, 0.0]]);
    }
    #[test]
    fn split_and_merge_preserve_channel_identity_exactly() {
        let input = vec![[0.2, -0.7], [0.8, 0.3]];
        let split = run(MathOperation::Split, input.clone(), vec![]);
        assert_eq!(split[0], vec![[0.2; 2], [0.8; 2]]);
        assert_eq!(split[1], vec![[-0.7; 2], [0.3; 2]]);
        assert_eq!(
            run(MathOperation::Merge, split[0].clone(), split[1].clone())[0],
            input
        );
    }
    #[test]
    fn constants_are_note_independent_and_respond_to_panic() {
        let mut source = ConstantSource::new(-0.25);
        assert_eq!(source.pull(2), vec![[-0.25; 2]; 2]);
        source.kill();
        assert_eq!(source.pull(2), vec![[0.0; 2]; 2]);
        source.start();
        assert!(source.set_parameter("value", 0.3));
        assert_eq!(source.pull(1), vec![[0.3; 2]]);
    }
}
