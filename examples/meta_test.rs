#[cfg(not(feature = "meta"))]
fn main() {
    println!("This example can only run with the 'meta' feature flag");
}

#[cfg(feature = "meta")]
fn main() {
    use rustic::core::filters::prelude::{HighPassFilter_META, LowPassFilter_META};

    let filter = LowPassFilter_META().clone();
    let filter2 = HighPassFilter_META().clone();

    println!("Metadata:");
    println!("\tName: {}", filter.name);
    println!("\tDescription: {}", filter.description);
    println!("\tSource amount: {}", filter.source_amount);
    println!("\tParameters: {:?}", filter.parameters);

    println!("Metadata:");
    println!("\tName: {}", filter2.name);
    println!("\tDescription: {}", filter2.description);
    println!("\tSource amount: {}", filter2.source_amount);
    println!("\tParameters: {:?}", filter2.parameters);
}
