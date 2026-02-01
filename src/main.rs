use morph::cli::{Cli, execute};
use clap::Parser;
use anyhow::Result;

fn main() -> Result<()> {
    env_logger::init();
    
    let cli = Cli::parse();
    execute(cli)
}