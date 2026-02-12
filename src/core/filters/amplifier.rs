use crate::core::graph::{Entry, Filter};
use std::fmt;

#[cfg(feature = "meta")]
use rustic_derive::FilterMetaData;

/// A filter that returns the input value multiplied by a constant factor.
/// Note: a factor < 1.0 will attenuate the input signal, while a factor > 1.0
/// will amplify it.
#[cfg_attr(feature = "meta", derive(FilterMetaData))]
#[derive(Clone, Debug, Default)]
pub struct GainFilter {
    #[cfg_attr(feature = "meta", filter_source)]
    sources: [f32; 1],
    #[cfg_attr(feature = "meta", filter_parameter(float, 1.0))]
    factor: f32,
}

impl GainFilter {
    pub fn new(factor: f32) -> Self {
        Self {
            sources: [0.0],
            factor,
        }
    }
}

impl Entry for GainFilter {
    fn push(&mut self, value: f32, port: usize) {
        self.sources[port] = value;
    }
}

impl fmt::Display for GainFilter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Gain Filter - factor: {}", self.factor)
    }
}

impl Filter for GainFilter {
    /// Transforms the input value by multiplying it by the factor and sends it to the sink.
    /// If multiple sources are connected to the filter, the output will be the sum of all
    /// the sources multiplied by the factor.
    fn transform(&mut self) -> Vec<f32> {
        let output: f32 = self.sources.map(|f| f * self.factor).iter().sum();
        vec![output]
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
