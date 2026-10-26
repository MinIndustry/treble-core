//! Sample-block modulation. These buffers never replace a node's saved base.
use crate::core::Block;
use std::sync::Arc;
use treble_meta::{ModulationMapping, ModulationMode, NumericDomain};

#[derive(Clone, Debug)]
pub struct ParameterModulation {
    pub(crate) parameter: String,
    pub(crate) mode: ModulationMode,
    pub(crate) mapping: ModulationMapping,
    pub(crate) domain: Option<NumericDomain>,
    pub(crate) signals: Vec<(Arc<Block>, f64)>,
}

impl ParameterModulation {
    /// Compose this sample with the current effective base (e.g. a held note's
    /// pitch). Bad inputs/intermediate overflow fall back to the bounded base.
    /// Published Sum semantics apply to every mode, including Multiply.
    pub fn value(&self, base: f32, sample: usize) -> f32 {
        let bounded = |value: f64| match &self.domain {
            Some(domain) => value.clamp(domain.min, domain.max),
            None => value,
        };
        let base = bounded(if base.is_finite() {
            f64::from(base)
        } else {
            self.domain.as_ref().map_or(0.0, |domain| domain.min)
        });
        let mut sum = 0.0;
        for (signal, depth) in &self.signals {
            let input = signal.get(sample).map_or(0.0, |frame| f64::from(frame[0]));
            sum += depth * input;
            if !input.is_finite() || !sum.is_finite() {
                return base as f32;
            }
        }
        let coordinate = match self.mapping {
            ModulationMapping::Native => base,
            ModulationMapping::Octaves => base.log2(),
            ModulationMapping::Normalized => {
                let Some(domain) = &self.domain else {
                    return base as f32;
                };
                (base - domain.min) / (domain.max - domain.min)
            }
        };
        let mapped = match self.mode {
            ModulationMode::Add => coordinate + sum,
            ModulationMode::Multiply => coordinate * (1.0 + sum),
            ModulationMode::Replace => sum,
        };
        if !mapped.is_finite() {
            return base as f32;
        }
        let value = match self.mapping {
            ModulationMapping::Native => mapped,
            ModulationMapping::Octaves => mapped.exp2(),
            ModulationMapping::Normalized => {
                let Some(domain) = &self.domain else {
                    return base as f32;
                };
                domain.min + mapped * (domain.max - domain.min)
            }
        };
        // Infinite inverse mapping can be bounded when a domain exists.
        let value = bounded(value);
        if !value.is_finite() || !(value as f32).is_finite() {
            base as f32
        } else {
            value as f32
        }
    }
}

pub fn modulated_value(
    inputs: &[ParameterModulation],
    parameter: &str,
    base: f32,
    sample: usize,
) -> f32 {
    inputs
        .iter()
        .find(|input| input.parameter == parameter)
        .map_or(if base.is_finite() { base } else { 0.0 }, |input| {
            input.value(base, sample)
        })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ExecutionNode {
    Source(usize),
    Filter(petgraph::graph::NodeIndex),
}

#[derive(Clone, Debug)]
pub(crate) struct SignalRoute {
    pub source: usize,
    pub port: usize,
    pub group: usize,
    pub contribution: usize,
}

#[derive(Clone, Debug)]
pub(crate) struct ScheduledNode {
    pub node: ExecutionNode,
    pub parameters: Vec<ParameterModulation>,
    pub routes: Vec<SignalRoute>,
}

#[derive(Clone, Debug)]
pub(crate) struct SamplePlan {
    pub valid: bool,
    pub nodes: Vec<ScheduledNode>,
    pub outputs: Vec<Vec<Arc<Block>>>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use treble_meta::{Scale, Unit};
    fn input(mapping: ModulationMapping, mode: ModulationMode) -> ParameterModulation {
        ParameterModulation {
            parameter: "test".into(),
            mapping,
            mode,
            domain: Some(NumericDomain {
                min: 1.0,
                max: 1000.0,
                unit: Unit::Unitless,
                scale: Scale::Linear,
                step: None,
            }),
            signals: vec![
                (Arc::new(vec![[0.25; 2], [-0.5; 2]]), 2.0),
                (Arc::new(vec![[0.5; 2]; 2]), 1.0),
            ],
        }
    }
    #[test]
    fn published_sum_modes_operate_on_each_sample() {
        let add = input(ModulationMapping::Native, ModulationMode::Add);
        assert_eq!(add.value(10.0, 0), 11.0);
        assert_eq!(add.value(10.0, 1), 9.5);
        let multiply = input(ModulationMapping::Native, ModulationMode::Multiply);
        assert_eq!(multiply.value(10.0, 0), 20.0); // q*(1+sum), not product.
        assert_eq!(multiply.value(10.0, 1), 5.0);
        let replace = input(ModulationMapping::Native, ModulationMode::Replace);
        assert_eq!(replace.value(10.0, 0), 1.0);
    }
    #[test]
    fn mappings_clamps_and_nonfinite_fallback_are_explicit() {
        let octaves = input(ModulationMapping::Octaves, ModulationMode::Add);
        assert_eq!(octaves.value(100.0, 0), 200.0);
        assert!((octaves.value(100.0, 1) - 100.0 / 2.0f32.sqrt()).abs() < 1e-5);
        let normalized = input(ModulationMapping::Normalized, ModulationMode::Add);
        assert_eq!(normalized.value(10.0, 0), 1000.0);
        assert_eq!(normalized.value(10.0, 1), 1.0);
        let mut extreme = octaves.clone();
        extreme.signals[0].1 = 1e300;
        assert_eq!(extreme.value(100.0, 0), 1000.0);
        extreme.signals[0].0 = Arc::new(vec![[f32::NAN; 2]]);
        assert_eq!(extreme.value(100.0, 0), 100.0);
        extreme.signals[0] = (Arc::new(vec![[10.0; 2]]), f64::MAX);
        assert_eq!(extreme.value(100.0, 0), 100.0);
    }
}
