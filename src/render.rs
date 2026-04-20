use std::fmt::Write as _;

use crate::JSON_SCHEMA_VERSION;
use crate::cli::{Cli, OutputFormat};
use crate::model::{
    JsonResult, Measurement, RequestSummary, ResponseSummary, SloReport, SloViolation,
};
use crate::palette::Palette;
use crate::slo::MetricKey;

pub fn render_output(
    cli: &Cli,
    measurement: &Measurement,
    violations: &[SloViolation],
    disable_color: bool,
) -> Result<String, String> {
    let payload = JsonResult {
        schema: JSON_SCHEMA_VERSION,
        request: RequestSummary {
            method: &cli.method,
            url: &cli.url,
            proxy: cli.proxy.as_deref(),
        },
        response: ResponseSummary {
            status_code: measurement.status_code,
            http_version: measurement.http_version.clone(),
            remote_ip: measurement.remote_ip.clone(),
            local_ip: measurement.local_ip.clone(),
            downloaded_bytes: measurement.downloaded_bytes,
            uploaded_bytes: measurement.uploaded_bytes,
        },
        timings: measurement.timings,
        diagnostics: measurement.diagnostics.clone(),
        slo: SloReport {
            passed: violations.is_empty(),
            violated: violations.to_vec(),
        },
    };

    match cli.format {
        OutputFormat::Human => Ok(render_human(cli, measurement, violations, disable_color)),
        OutputFormat::Json => serde_json::to_string_pretty(&payload).map_err(|err| err.to_string()),
        OutputFormat::Jsonl => serde_json::to_string(&payload).map_err(|err| err.to_string()),
    }
}

fn render_human(
    cli: &Cli,
    measurement: &Measurement,
    violations: &[SloViolation],
    disable_color: bool,
) -> String {
    let mut out = String::new();
    let palette = Palette::new(disable_color);

    let status = if measurement.status_code >= 400 {
        palette.red(&measurement.status_code.to_string())
    } else {
        palette.green(&measurement.status_code.to_string())
    };
    let _ = writeln!(
        out,
        "{} {} {}",
        palette.bold(&cli.method),
        palette.cyan(&cli.url),
        status
    );

    let _ = writeln!(
        out,
        "{}  {}  {}",
        palette.dim(&measurement.http_version),
        palette.dim(&format!(
            "remote={}",
            measurement.remote_ip.as_deref().unwrap_or("unknown")
        )),
        palette.dim(&format!("bytes={}", measurement.downloaded_bytes))
    );
    let _ = writeln!(out);

    let max_ms = measurement.timings.total_ms.max(1.0);
    for key in [
        MetricKey::Dns,
        MetricKey::Connect,
        MetricKey::Tls,
        MetricKey::Server,
        MetricKey::Transfer,
        MetricKey::Total,
    ] {
        let value = measurement.timings.metric(key);
        let width = ((value / max_ms) * 28.0).round().clamp(1.0, 28.0) as usize;
        let bar = "█".repeat(width);
        let painted_bar = match key {
            MetricKey::Dns => palette.blue(&bar),
            MetricKey::Connect => palette.cyan(&bar),
            MetricKey::Tls => palette.magenta(&bar),
            MetricKey::Server => palette.yellow(&bar),
            MetricKey::Transfer => palette.green(&bar),
            MetricKey::Total => palette.bold(&bar),
        };
        let _ = writeln!(
            out,
            "{:<18} {:>8.2} ms  {}",
            key.label(),
            value,
            painted_bar
        );
    }

    if !measurement.diagnostics.is_empty() {
        let _ = writeln!(out);
        let _ = writeln!(out, "{}", palette.bold("Diagnostics"));
        for diagnostic in &measurement.diagnostics {
            let level = match diagnostic.level {
                "warn" => palette.yellow("warn"),
                "info" => palette.blue("info"),
                _ => palette.dim(diagnostic.level),
            };
            let _ = writeln!(out, "[{}] {}", level, diagnostic.message);
        }
    }

    if !violations.is_empty() {
        let _ = writeln!(out);
        let _ = writeln!(out, "{}", palette.red("SLO violations"));
        for violation in violations {
            let _ = writeln!(
                out,
                "{} actual={:.2}ms threshold={:.2}ms",
                violation.metric, violation.actual_ms, violation.threshold_ms
            );
        }
    }

    out.trim_end().to_string()
}
