use rustic_meta::Parameter;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MetaGenerator {
    pub name: &'static str,
    pub description: &'static str,
    pub parameters: Vec<Parameter<&'static str>>,
    pub output_count: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MetaSink {
    pub name: &'static str,
    pub description: &'static str,
    pub input_count: usize,
}
