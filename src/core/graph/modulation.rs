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

/// A parameter resolved once for a block. Borrows its signal buffers and caches
/// the bounded base and mapping coordinate; no allocation or base mutation.
#[derive(Clone, Copy, Debug)]
pub struct ModulationView<'a> {
    input: Option<&'a ParameterModulation>,
    base: f64,
    coordinate: f64,
}

impl ParameterModulation {
    /// Resolve the effective base (for example the current held note's pitch)
    /// once before iterating samples. Rebind after the base or input block changes.
    pub fn bind(&self, base: f32) -> ModulationView<'_> {
        let base = if base.is_finite() {
            f64::from(base)
        } else {
            self.domain.as_ref().map_or(0.0, |domain| domain.min)
        };
        let base = self
            .domain
            .as_ref()
            .map_or(base, |domain| base.clamp(domain.min, domain.max));
        let coordinate = match self.mapping {
            ModulationMapping::Native => base,
            ModulationMapping::Octaves => base.log2(),
            ModulationMapping::Normalized => self.domain.as_ref().map_or(f64::NAN, |domain| {
                (base - domain.min) / (domain.max - domain.min)
            }),
        };
        ModulationView {
            input: Some(self),
            base,
            coordinate,
        }
    }

    /// Compose this sample with the current effective base. Prefer `bind` when
    /// the base remains constant for a block. Sum and fallback semantics match.
    pub fn value(&self, base: f32, sample: usize) -> f32 {
        self.bind(base).value(sample)
    }
}

impl ModulationView<'_> {
    /// Evaluate one sample, retaining sample-rate modulation and finite fallback.
    #[inline]
    pub fn value(&self, sample: usize) -> f32 {
        let Some(input) = self.input else {
            return self.base as f32;
        };
        let mut sum = 0.0;
        for (signal, depth) in &input.signals {
            let value = signal.get(sample).map_or(0.0, |frame| f64::from(frame[0]));
            sum += depth * value;
            if !value.is_finite() || !sum.is_finite() {
                return self.base as f32;
            }
        }
        let mapped = match input.mode {
            ModulationMode::Add => self.coordinate + sum,
            ModulationMode::Multiply => self.coordinate * (1.0 + sum),
            ModulationMode::Replace => sum,
        };
        if !mapped.is_finite() {
            return self.base as f32;
        }
        let value = match input.mapping {
            ModulationMapping::Native => mapped,
            ModulationMapping::Octaves => mapped.exp2(),
            ModulationMapping::Normalized => {
                let Some(domain) = &input.domain else {
                    return self.base as f32;
                };
                domain.min + mapped * (domain.max - domain.min)
            }
        };
        let value = input
            .domain
            .as_ref()
            .map_or(value, |domain| value.clamp(domain.min, domain.max));
        if !value.is_finite() || !(value as f32).is_finite() {
            self.base as f32
        } else {
            value as f32
        }
    }
}

/// Bind a named parameter once outside the sample loop. Missing inputs resolve
/// to a finite constant base, preserving the legacy helper's behavior.
pub fn bind_modulation<'a>(
    inputs: &'a [ParameterModulation],
    parameter: &str,
    base: f32,
) -> ModulationView<'a> {
    inputs
        .iter()
        .find(|input| input.parameter == parameter)
        .map_or_else(
            || ModulationView {
                input: None,
                base: if base.is_finite() {
                    f64::from(base)
                } else {
                    0.0
                },
                coordinate: 0.0,
            },
            |input| input.bind(base),
        )
}

pub fn modulated_value(
    inputs: &[ParameterModulation],
    parameter: &str,
    base: f32,
    sample: usize,
) -> f32 {
    bind_modulation(inputs, parameter, base).value(sample)
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
pub(crate) struct SourceRoute {
    pub source: usize,
    pub port: usize,
    pub input: usize,
}

#[derive(Clone, Debug)]
pub(crate) struct ScheduledNode {
    pub node: ExecutionNode,
    pub parameters: Vec<ParameterModulation>,
    pub routes: Vec<SignalRoute>,
    pub inputs: Vec<super::SourceInput>,
    pub input_routes: Vec<SourceRoute>,
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

    #[test]
    fn bound_views_preserve_mapping_modes_and_effective_base_changes() {
        for (mapping, mode, base, expected) in [
            (
                ModulationMapping::Native,
                ModulationMode::Add,
                10.0,
                [11.0, 9.5],
            ),
            (
                ModulationMapping::Native,
                ModulationMode::Multiply,
                10.0,
                [20.0, 5.0],
            ),
            (
                ModulationMapping::Native,
                ModulationMode::Replace,
                10.0,
                [1.0, 1.0],
            ),
            (
                ModulationMapping::Octaves,
                ModulationMode::Add,
                100.0,
                [200.0, 100.0 / 2.0f32.sqrt()],
            ),
            (
                ModulationMapping::Octaves,
                ModulationMode::Multiply,
                100.0,
                [1000.0, 10.0],
            ),
            (
                ModulationMapping::Octaves,
                ModulationMode::Replace,
                100.0,
                [2.0, 1.0],
            ),
            (
                ModulationMapping::Normalized,
                ModulationMode::Add,
                10.0,
                [1000.0, 1.0],
            ),
            (
                ModulationMapping::Normalized,
                ModulationMode::Multiply,
                10.0,
                [19.0, 5.5],
            ),
            (
                ModulationMapping::Normalized,
                ModulationMode::Replace,
                10.0,
                [1000.0, 1.0],
            ),
        ] {
            let inputs = [input(mapping, mode)];
            let view = bind_modulation(&inputs, "test", base);
            for (sample, expected) in expected.into_iter().enumerate() {
                assert!((view.value(sample) - expected).abs() < 1e-5);
                assert_eq!(
                    view.value(sample),
                    modulated_value(&inputs, "test", base, sample)
                );
            }
        }
        let inputs = [input(ModulationMapping::Octaves, ModulationMode::Add)];
        assert_eq!(bind_modulation(&inputs, "test", 100.0).value(0), 200.0);
        assert_eq!(bind_modulation(&inputs, "test", 200.0).value(0), 400.0);
        assert_eq!(bind_modulation(&inputs, "missing", f32::NAN).value(0), 0.0);
        assert_eq!(bind_modulation(&inputs, "test", f32::NAN).value(0), 2.0);
    }
}
