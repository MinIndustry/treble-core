use crate::core::Block;
/// This file contains structural filters i.e. filters that do not modify
/// values that pass through it but rather duplicate/merges its inputs
use crate::core::graph::{Entry, Filter};
use log::trace;
use rustic_derive::FilterMetaData;
use std::fmt;

/// Duplicates the content of the input onto two outputs
#[derive(FilterMetaData, Clone, Debug, Default)]
pub struct DuplicateFilter {
    #[filter_source]
    source: Block,
}

impl DuplicateFilter {
    pub fn new() -> Self {
        Self { source: Vec::new() }
    }
}

impl Entry for DuplicateFilter {
    fn push(&mut self, block: Block, _port: usize) {
        self.source = block;
    }
}

impl fmt::Display for DuplicateFilter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "DuplicateFilter")
    }
}

impl Filter for DuplicateFilter {
    fn transform(&mut self) -> Vec<Block> {
        trace!("Duplicate filter running");
        vec![self.source.clone(), self.source.clone()]
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
