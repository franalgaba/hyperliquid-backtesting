use anyhow::Result;
use clap::Parser;

use hl_backtest::cli::Cli;

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    cli.execute().await
}
