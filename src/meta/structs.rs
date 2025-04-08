use rustic_meta::Parameter;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MetaFilter {
    pub name: &'static str,
    pub description: &'static str,
    pub source_amount: usize,
    pub parameters: Vec<Parameter<&'static str>>,
}
