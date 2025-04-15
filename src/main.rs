//! Used to test app's functionality such as the cli and configuration loading
//!
//! # Examples
//!
//! ```bash
//! cargo run --bin rustic -- --dump-config
//! ```

use rustic::prelude::App;

fn main() {
    colog::init();

    App::init().run();
}
