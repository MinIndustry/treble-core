use rustic_meta::Parameter;

pub mod structs;
pub mod traits;

use structs::{MetaFilter, MetaGenerator, MetaSink};

#[macro_export]
macro_rules! filters {
    ( $( $x:ident ),* ) => {
        {
            let mut temp_vec = Vec::new();
            $(
                temp_vec.push(Box::new($x::default()) as Box<dyn Filter>);
            )*
            temp_vec
        }
    };
}

pub fn get_filters() -> Vec<MetaFilter> {
    vec![
        crate::core::filters::prelude::GainFilter_META(),
        crate::core::filters::prelude::Clipper_META(),
        crate::core::filters::prelude::CombinatorFilter_META(),
        crate::core::filters::prelude::Tremolo_META(),
        crate::core::filters::prelude::DelayFilter_META(),
        crate::core::filters::prelude::LowPassFilter_META(),
        crate::core::filters::prelude::HighPassFilter_META(),
        crate::core::filters::prelude::BandPass_META(),
        crate::core::filters::prelude::ResonantBandpassFilter_META(),
        crate::core::filters::prelude::MovingAverage_META(),
        crate::core::filters::prelude::DuplicateFilter_META(),
        crate::core::filters::prelude::Compressor_META(),
    ]
}

pub fn get_generators() -> Vec<MetaGenerator> {
    vec![
        MetaGenerator {
            name: "Sine Wave",
            description: "Generates a pure sine wave signal",
            parameters: vec![
                Parameter::Range {
                    title: "Frequency",
                    field_name: "frequency",
                    min: 20.0,
                    max: 20000.0,
                    default: 440.0,
                    value: 440.0,
                },
                Parameter::Range {
                    title: "Amplitude",
                    field_name: "amplitude",
                    min: 0.0,
                    max: 1.0,
                    default: 0.5,
                    value: 0.5,
                },
            ],
            output_count: 1,
        },
        // Square Wave, Sawtooth Wave, Triangle Wave, White Noise
        // Same structure, different name/description
    ]
}

pub fn get_sinks() -> Vec<MetaSink> {
    vec![MetaSink {
        name: "Audio Output",
        description: "Outputs audio to the system audio device",
        input_count: 1,
    }]
}
