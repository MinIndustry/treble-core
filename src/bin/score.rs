use clap::Parser;

use rustic::prelude::score::{Score, TimeSignature};

use rustic::instruments::prelude::{HiHat, Kick, Snare};

#[derive(Parser, Debug)]
struct Cli {
    #[clap(short, long)]
    dump_default: bool,

    #[clap(short, long)]
    file: Option<String>,

    #[clap(short, long)]
    load: Option<String>,
}

/// Dumps a default score to a file
fn dump_default(output_path: String) {
    let score = Score::new(
        "Example Score",
        rustic::prelude::score::TimeSignature(4, 4),
        120,
        1,
        20,
    );

    let toml_score = match toml::to_string(&score) {
        Err(e) => {
            println!("Error: {}", e);
            return;
        }
        Ok(toml) => toml,
    };
    if let Err(e) = std::fs::write(output_path, toml_score) {
        println!("Error while writing to the ouput file: {}", e);
    }
}

fn main() {
    let args = Cli::parse();
    if args.dump_default {
        if let Some(output_path) = args.file {
            dump_default(output_path);
        } else {
            println!("Error: You must provide a file path to dump the default score");
        }
        std::process::exit(0);
    }

    let mut score = Score::new("Test", TimeSignature(4, 4), 120, 1, 20);
    let _kick_index = score.add_instrument(Box::new(Kick::new()));
    let _snare_index = score.add_instrument(Box::new(Snare::new()));
    let _hihat_index = score.add_instrument(Box::new(HiHat::new().unwrap()));
}
