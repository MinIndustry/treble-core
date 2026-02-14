use rustic_meta::{FilterInfo, MetaFilter, MetaGenerator, MetaSink, Parameter};

pub mod traits;

use crate::core::filters::prelude::*;

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

pub fn get_filters() -> Vec<FilterInfo> {
    vec![
        GainFilter::metadata(),
        Clipper::metadata(),
        CombinatorFilter::metadata(),
        Tremolo::metadata(),
        DelayFilter::metadata(),
        LowPassFilter::metadata(),
        HighPassFilter::metadata(),
        BandPass::metadata(),
        ResonantBandpassFilter::metadata(),
        MovingAverage::metadata(),
        DuplicateFilter::metadata(),
        Compressor::metadata(),
    ]
}

pub fn get_generators() -> Vec<MetaGenerator> {
    let default_frequency_range = Parameter::Range {
        title: "Frequency",
        field_name: "frequency",
        min: 20.0,
        max: 20000.0,
        default: 440.0,
        value: 440.0,
    };

    let default_amplitude_range = Parameter::Range {
        title: "Amplitude",
        field_name: "amplitude",
        min: 0.0,
        max: 1.0,
        default: 0.5,
        value: 0.5,
    };

    vec![
        MetaGenerator {
            name: "Sine Wave",
            description: "Generates a pure sine wave signal",
            parameters: vec![
                default_frequency_range.clone(),
                default_amplitude_range.clone(),
            ],
            output_count: 1,
        },
        MetaGenerator {
            name: "Square Wave",
            description: "Generates a pure sine wave signal",
            parameters: vec![
                default_frequency_range.clone(),
                default_amplitude_range.clone(),
            ],
            output_count: 1,
        },
        MetaGenerator {
            name: "Sawtooth",
            description: "Generates a pure sine wave signal",
            parameters: vec![
                default_frequency_range.clone(),
                default_amplitude_range.clone(),
            ],
            output_count: 1,
        },
        MetaGenerator {
            name: "Triangle",
            description: "Generates a pure sine wave signal",
            parameters: vec![
                default_frequency_range.clone(),
                default_amplitude_range.clone(),
            ],
            output_count: 1,
        },
        MetaGenerator {
            name: "White noise",
            description: "Generates a pure sine wave signal",
            parameters: vec![default_amplitude_range.clone()],
            output_count: 1,
        },
    ]
}

pub fn get_sinks() -> Vec<MetaSink> {
    vec![MetaSink {
        name: "Audio Output",
        description: "Outputs audio to the system audio device",
        input_count: 1,
    }]
}
