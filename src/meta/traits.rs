use crate::core::graph::Filter;
use rustic_meta::Parameter;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NodeKind {
    Generator,
    Filter,
    Sink,
}

pub trait FilterFactory: Send + Sync {
    fn create_instance(&self) -> Box<dyn Filter>;
}

pub trait FilterMetadata: Send + Sync {
    fn name(&self) -> String;
    fn description(&self) -> String;
    fn source_amount(&self) -> usize;
    fn parameters(&self) -> Vec<Parameter<&'static str>>;
}

pub trait FrontendFilter: FilterMetadata + FilterFactory {}
