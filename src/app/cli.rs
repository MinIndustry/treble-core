use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Default, Debug)]
#[clap(
    author = "Minigrim0",
    version,
    about = "Rustic - Rustic; a UNIX Symphony Tool Implemented in Code"
)]
pub struct Cli {
    #[arg(short = 'D', long = "dump-config")]
    /// Dumps the default configuration into a file
    pub dump_config: bool,

    #[arg(short = 'c', long = "config")]
    /// The configuration file to read
    pub config: Option<PathBuf>,

    #[arg(short, long)]
    /// The score file to read
    pub score: Option<PathBuf>,

    #[arg(short, long)]
    /// Wether to start the app in live mode
    pub live: bool,
}
