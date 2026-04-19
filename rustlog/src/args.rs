use std::path::PathBuf;

use clap::Parser;

/// Log reader: filter a file once or follow with `--tail`. Optional `--config` TOML merges with CLI.
#[derive(Parser, Debug, Clone)]
#[command(author, version, about)]
pub struct Args {
    /// TOML configuration file (`[source]`, `[filters]`, `[output]`).
    #[arg(short = 'C', long = "config", value_name = "FILE")]
    pub config: Option<PathBuf>,

    /// Log file path (overrides `[source].path` from config when both are set).
    #[arg(value_name = "FILE")]
    pub file_path: Option<String>,

    /// Filter keyword (overrides `[filters].patterns` from config when set).
    #[arg(value_name = "KEYWORD")]
    pub keyword: Option<String>,

    /// Follow the file for new lines (`tail -f` semantics).
    #[arg(short, long)]
    pub tail: bool,
}

pub fn parse_args() -> Args {
    Args::parse()
}
