mod cli;
mod diagnostics;
mod model;
mod palette;
mod render;
mod request;
mod slo;

use std::fs;
use std::io::IsTerminal;
use std::process::ExitCode;

use clap::Parser;

pub use cli::{Cli, ColorMode, OutputFormat};
pub use model::{
    Diagnostic, JsonResult, Measurement, PhaseDurations, RequestSummary, ResponseSummary,
    SloReport, SloViolation,
};
pub use slo::{MetricKey, SloCheck};

pub const EXIT_SLO_VIOLATION: u8 = 4;
pub const JSON_SCHEMA_VERSION: &str = "v1";

pub fn run_cli() -> ExitCode {
    let cli = Cli::parse();
    match run(cli) {
        Ok(code) => code,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::from(1)
        }
    }
}

pub fn run(cli: Cli) -> Result<ExitCode, String> {
    let disable_color = match cli.color {
        ColorMode::Always => false,
        ColorMode::Never => true,
        ColorMode::Auto => {
            cli.no_color || std::env::var_os("NO_COLOR").is_some() || !std::io::stdout().is_terminal()
        }
    };
    let slo_checks = slo::parse_slo_checks(&cli.slo)?;
    let measurement = request::perform_request(&cli)?;
    let violations = slo::evaluate_slos(measurement.timings, &slo_checks);
    let rendered = render::render_output(&cli, &measurement, &violations, disable_color)?;

    println!("{rendered}");
    if let Some(path) = &cli.save {
        fs::write(path, &rendered)
            .map_err(|err| format!("failed to save {}: {err}", path.display()))?;
    }

    Ok(if violations.is_empty() {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(EXIT_SLO_VIOLATION)
    })
}
