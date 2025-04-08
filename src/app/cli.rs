use clap::Parser;

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
    pub config: Option<String>,
}
