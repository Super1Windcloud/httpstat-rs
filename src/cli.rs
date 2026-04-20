use std::path::PathBuf;

use clap::{Parser, ValueEnum};

#[derive(Parser, Debug)]
#[command(
    name = "httpstat-rs",
    version,
    about = "Measure HTTP timing phases with concise terminal output or structured JSON."
)]
pub struct Cli {
    #[arg(help = "Target URL to measure")]
    pub url: String,

    #[arg(long, value_enum, default_value_t = OutputFormat::Human)]
    pub format: OutputFormat,

    #[arg(long, value_enum, default_value_t = ColorMode::Auto)]
    pub color: ColorMode,

    #[arg(long, help = "Save the rendered result to a file")]
    pub save: Option<PathBuf>,

    #[arg(
        long,
        value_delimiter = ',',
        help = "SLO checks like total=500,connect=100 in milliseconds"
    )]
    pub slo: Vec<String>,

    #[arg(short = 'X', long, default_value = "GET")]
    pub method: String,

    #[arg(
        short = 'H',
        long = "header",
        help = "Extra request header",
        action = clap::ArgAction::Append
    )]
    pub headers: Vec<String>,

    #[arg(short = 'd', long = "data", help = "Request body")]
    pub data: Option<String>,

    #[arg(long, help = "HTTP proxy, for example http://127.0.0.1:8080")]
    pub proxy: Option<String>,

    #[arg(
        long,
        default_value_t = 30_000,
        help = "Overall timeout in milliseconds"
    )]
    pub timeout_ms: u64,

    #[arg(long, help = "Skip TLS certificate verification")]
    pub insecure: bool,

    #[arg(long, help = "Do not emit ANSI colors")]
    pub no_color: bool,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, ValueEnum)]
pub enum OutputFormat {
    Human,
    Json,
    Jsonl,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, ValueEnum)]
pub enum ColorMode {
    Auto,
    Always,
    Never,
}
