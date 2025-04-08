use rustic::core::filters::prelude::{HighPassFilter, LowPassFilter};
use rustic::meta::traits::FilterMetadata;

fn main() {
    let filter = LowPassFilter::metadata();
    let filter2 = HighPassFilter::metadata();

    println!("Metadata:");
    println!("\tName: {}", filter.name());
    println!("\tDescription: {}", filter.description());
    println!("\tSource amount: {}", filter.source_amount());
    println!("\tParameters: {:?}", filter.parameters());

    println!("Metadata:");
    println!("\tName: {}", filter2.name());
    println!("\tDescription: {}", filter2.description());
    println!("\tSource amount: {}", filter2.source_amount());
    println!("\tParameters: {:?}", filter2.parameters());
}
