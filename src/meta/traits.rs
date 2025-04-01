pub trait FilterMetadata {
    fn name(&self) -> String;
    fn description(&self) -> String;
    fn source_amount(&self) -> usize;
    fn parameters(&self) -> Vec<rustic_meta::Parameter>;
}
