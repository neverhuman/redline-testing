mod cli;
mod evidence;
mod report;
mod sqlite_parity;

use anyhow::Result;
use clap::Parser;

fn main() -> Result<()> {
    cli::run(cli::Cli::parse())
}
