use simplelog::*;
use std::fs::File;

use log::error;
use std::path::PathBuf;

use rustic::core::generator::prelude::builder::MultiToneGeneratorBuilder;
use rustic::core::{
    filters::prelude::{CombinatorFilter, GainFilter},
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

    let system_1 = {
        let mut system = System::new();
        for _ in 0..2 {
            system.add_source(simple_source(MultiToneGeneratorBuilder::new().build()));
        }

        let combinator = system.add_filter(Box::new(CombinatorFilter::new(2, 5)));

        let filter_1 = {
            let gain_filter = GainFilter::new(0.1);
            system.add_filter(Box::new(gain_filter))
        };
        let filter_2 = {
            let gain_filter = GainFilter::new(0.2);
            system.add_filter(Box::new(gain_filter))
        };
        let filter_3 = {
            let gain_filter = GainFilter::new(0.3);
            system.add_filter(Box::new(gain_filter))
        };
        let filter_4 = {
            let gain_filter = GainFilter::new(0.4);
            system.add_filter(Box::new(gain_filter))
        };
        let filter_5 = {
            let gain_filter = GainFilter::new(0.5);
            system.add_filter(Box::new(gain_filter))
        };

        system.connect(combinator, filter_1, 0, 0);
        system.connect(combinator, filter_2, 1, 0);
        system.connect(combinator, filter_3, 2, 0);
        system.connect(combinator, filter_4, 3, 0);
        system.connect(combinator, filter_5, 4, 0);

        system.connect_source(0, combinator, 0);
        system.connect_source(1, combinator, 1);

        system.connect_sink(filter_1, 0, 0);
        system.connect_sink(filter_2, 1, 0);
        system.connect_sink(filter_3, 2, 0);
        system.connect_sink(filter_4, 3, 0);
        system.connect_sink(filter_5, 4, 0);

        system
    };

    let system_2 = {
        let mut system = System::new();

        for _ in 0..5 {
            system.add_source(simple_source(MultiToneGeneratorBuilder::new().build()));
        }

        let combinator = system.add_filter(Box::new(CombinatorFilter::new(5, 1)));

        system.connect_source(0, combinator, 0);
        system.connect_source(1, combinator, 1);
        system.connect_source(2, combinator, 2);
        system.connect_source(3, combinator, 3);
        system.connect_source(4, combinator, 4);

        system.connect_sink(combinator, 0, 0);

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
