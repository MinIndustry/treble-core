use rustic::core::filters::prelude::{HighPassFilter, LowPassFilter};
#[cfg(feature = "meta")]
use rustic::meta::FilterMetadata;

fn main() {
    let filter = LowPassFilter::new(1000.0);
    let filter2 = HighPassFilter::new(1000.0);

    #[cfg(feature = "meta")]
    {
        println!("{}", filter);
        println!("Metadata:");
        println!("\tName: {}", filter.name());
        println!("\tDescription: {}", filter.description());
        println!("\tSource amount: {}", filter.source_amount());
        println!("\tParameters: {:?}", filter.parameters());

        println!("{}", filter2);
        println!("Metadata:");
        println!("\tName: {}", filter2.name());
        println!("\tDescription: {}", filter2.description());
        println!("\tSource amount: {}", filter2.source_amount());
        println!("\tParameters: {:?}", filter2.parameters());
    }
}
