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

    let app: App = App::init();
    println!("{:?}", app);
    app.run();
}
