use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(name = "bwrapped")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

// TODO: Add commands for calling the different configs
#[derive(Subcommand, Debug)]
pub enum Commands {
    Default { input: Vec<String> },
}
