//! Used to test app's functionality such as the cli and configuration loading
//!
//! # Examples
//!
//! ```bash
//! cargo run --bin rustic -- --dump-config
//! ```

use simplelog::*;
use std::fs::File;

use rustic::prelude::App;

fn main() {
    CombinedLogger::init(vec![
        TermLogger::new(LevelFilter::Info, Config::default(), TerminalMode::Mixed, ColorChoice::Auto),
        WriteLogger::new(LevelFilter::Trace, Config::default(), File::create("app.log").unwrap()),
    ]).unwrap();

    App::init().run();
}
