use std::fmt;

#[cfg(feature = "meta")]
use rustic_derive::FilterMetaData;

/// This file contains structural filters i.e. filters that do not modify
/// values that pass through it but rather duplicate/merges its inputs
use crate::core::graph::{AudioGraphElement, Entry, Filter};

use log::trace;

/// Duplicates the content of the input onto two outputs
#[derive(Clone, Debug, Default)]
#[cfg_attr(feature = "meta", derive(FilterMetaData))]
pub struct DuplicateFilter {
    #[cfg_attr(feature = "meta", filter_source)]
    sources: [f32; 1],
    index: usize,
}

impl DuplicateFilter {
    pub fn new() -> Self {
        Self {
            sources: [0.0],
            index: 0,
        }
    }
}

impl Entry for DuplicateFilter {
    fn push(&mut self, value: f32, port: usize) {
        self.sources[port] = value;
    }
}

impl fmt::Display for DuplicateFilter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "DuplicateFilter")
    }
}

impl Filter for DuplicateFilter {
    fn transform(&mut self) -> Vec<f32> {
        let source_value = self.sources[0];

        trace!(
            "Duplicate filter running {} -> {}, {}",
            source_value,
            source_value,
            source_value
        );
        vec![source_value, source_value]
    }

    fn postponable(&self) -> bool {
        false
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

impl AudioGraphElement for DuplicateFilter {
    fn get_name(&self) -> &str {
        "Duplicate"
    }

    fn get_index(&self) -> usize {
        self.index
    }

    fn set_index(&mut self, index: usize) {
        self.index = index;
    }
}
