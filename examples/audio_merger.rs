use simplelog::*;
use std::fs::File;

use log::error;
use std::path::PathBuf;

use rustic::core::generator::prelude::builder::MultiToneGeneratorBuilder;
use rustic::core::{
    filters::prelude::GainFilter,
    graph::{System, simple_source},
};

pub fn main() {
    CombinedLogger::init(vec![
        TermLogger::new(
            LevelFilter::Info,
            Config::default(),
            TerminalMode::Mixed,
            ColorChoice::Auto,
        ),
        WriteLogger::new(
            LevelFilter::Trace,
            Config::default(),
            File::create("app.log").unwrap(),
        ),
    ])
    .unwrap();

    // system_1: 2 sources summed into 5 parallel gain stages.
    // Both sources connect to port 0 of a GainFilter (mixed via Sum);
    // the single output fans out to 5 downstream gain filters.
    let system_1 = {
        let mut system = System::new();
        for _ in 0..2 {
            system.add_source(simple_source(MultiToneGeneratorBuilder::new().build()));
        }

        // Mixer: receives both sources on port 0 (accumulated + summed by run())
        let mixer = system.add_filter(Box::new(GainFilter::new(1.0)));
        system.connect_source(0, mixer, 0);
        system.connect_source(1, mixer, 0);

        // Fan out: one output port → five downstream gain nodes
        let filter_1 = system.add_filter(Box::new(GainFilter::new(0.1)));
        let filter_2 = system.add_filter(Box::new(GainFilter::new(0.2)));
        let filter_3 = system.add_filter(Box::new(GainFilter::new(0.3)));
        let filter_4 = system.add_filter(Box::new(GainFilter::new(0.4)));
        let filter_5 = system.add_filter(Box::new(GainFilter::new(0.5)));

        system.connect(mixer, filter_1, 0, 0);
        system.connect(mixer, filter_2, 0, 0);
        system.connect(mixer, filter_3, 0, 0);
        system.connect(mixer, filter_4, 0, 0);
        system.connect(mixer, filter_5, 0, 0);

        system.connect_sink(filter_1, 0, 0);
        system.connect_sink(filter_2, 1, 0);
        system.connect_sink(filter_3, 2, 0);
        system.connect_sink(filter_4, 3, 0);
        system.connect_sink(filter_5, 4, 0);

        system
    };

    // system_2: 5 sources all summed at port 0 of a single GainFilter.
    let system_2 = {
        let mut system = System::new();
        for _ in 0..5 {
            system.add_source(simple_source(MultiToneGeneratorBuilder::new().build()));
        }

        let mixer = system.add_filter(Box::new(GainFilter::new(1.0)));
        for i in 0..5 {
            system.connect_source(i, mixer, 0);
        }
        system.connect_sink(mixer, 0, 0);

        system
    };

    if let Err(e) = system_1.save_to_file(&PathBuf::from("system1.viz")) {
        log::error!("{}", e);
    }

    if let Err(e) = system_2.save_to_file(&PathBuf::from("system2.viz")) {
        log::error!("{}", e);
    }

    match system_1.merge(system_2, vec![(0, 0), (1, 1), (2, 2), (3, 3), (4, 4)]) {
        Ok(merged_system) => {
            if let Err(e) = merged_system.save_to_file(&PathBuf::from("merged_system.viz")) {
                log::error!("{}", e);
            }
        }
        Err(e) => error!("Unable to merge systems: {}", e),
    }
}
