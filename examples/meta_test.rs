fn main() {
    use rustic::core::filters::prelude::{HighPassFilter, LowPassFilter};
    use rustic_meta::MetaFilter;

    let filter = LowPassFilter::metadata();
    let filter2 = HighPassFilter::metadata();

    println!("Metadata:");
    println!("\tName: {}", filter.name);
    println!("\tDescription: {}", filter.description);
    println!("\tAudio inputs: {}", filter.inputs.iter().filter(|i| i.parameter.is_none()).count());
    println!("\tParameters: {:?}", filter.inputs.iter().filter(|i| i.parameter.is_some()).collect::<Vec<_>>());

    println!("Metadata:");
    println!("\tName: {}", filter2.name);
    println!("\tDescription: {}", filter2.description);
    println!("\tAudio inputs: {}", filter2.inputs.iter().filter(|i| i.parameter.is_none()).count());
    println!("\tParameters: {:?}", filter2.inputs.iter().filter(|i| i.parameter.is_some()).collect::<Vec<_>>());
}
