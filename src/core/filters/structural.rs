use std::fmt;

/// This file contains structural filters i.e. filters that do not modify
/// values that pass through it but rather duplicate/merges its inputs
use crate::core::graph::{AudioGraphElement, Entry, Filter};

use log::trace;

#[cfg(feature = "meta")]
use super::{FilterMetadata, Metadata};

/// Duplicates the content of the input onto two outputs
#[derive(Clone, Debug)]
pub struct DuplicateFilter {
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

#[cfg(feature = "meta")]
impl Metadata for DuplicateFilter {
    fn get_metadata() -> FilterMetadata {
        FilterMetadata {
            name: "DuplicateFilter".to_string(),
            description: "Duplicates the content of the input onto two outputs".to_string(),
            inputs: 1,
            outputs: 2,
        }
    }
}
