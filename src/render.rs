use std::fmt::Write as _;

use crate::JSON_SCHEMA_VERSION;
use crate::cli::{Cli, OutputFormat};
use crate::model::{
    JsonResult, Measurement, RequestSummary, ResponseSummary, SloReport, SloViolation,
};
use crate::palette::Palette;

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

    let status_line = if measurement.status_code >= 400 {
        palette.red(&measurement.status_line)
    } else {
        palette.green(&measurement.status_line)
    };
    let _ = writeln!(
        out,
        "{}",
        palette.dim(&format!(
            "Connected to {}:{}",
            measurement.remote_ip.as_deref().unwrap_or("unknown"),
            measurement.remote_port
        ))
    );
    let _ = writeln!(out);
    let _ = writeln!(
        out,
        "{}",
        status_line
    );
    for header in &measurement.response_headers {
        let _ = writeln!(out, "{header}");
    }
    let _ = writeln!(out);

    write_httpstat_timeline(&mut out, cli, measurement, &palette);

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

fn write_httpstat_timeline(
    out: &mut String,
    cli: &Cli,
    measurement: &Measurement,
    palette: &Palette,
) {
    let has_tls = cli.url.starts_with("https://");
    let dns = ms(measurement.timings.dns_ms);
    let tcp = ms(measurement.timings.connect_ms);
    let tls = ms(measurement.timings.tls_ms);
    let server = ms(measurement.timings.server_ms);
    let transfer = ms(measurement.timings.transfer_ms);
    let connect = ms(measurement.timings.dns_ms + measurement.timings.connect_ms);
    let pretransfer = ms(
        measurement.timings.dns_ms + measurement.timings.connect_ms + measurement.timings.tls_ms,
    );
    let starttransfer = ms(measurement.timings.total_ms - measurement.timings.transfer_ms);
    let total = ms(measurement.timings.total_ms);

    if has_tls {
        let _ = writeln!(
            out,
            "  DNS Lookup   TCP Connection   TLS Handshake   Server Processing   Content Transfer"
        );
        let _ = writeln!(
            out,
            "{}",
            palette.bold(&format!(
                "[{:>10}  |{:>14}  |{:>13}  |{:>19}  |{:>15}  ]",
                format!("{dns}ms"),
                format!("{tcp}ms"),
                format!("{tls}ms"),
                format!("{server}ms"),
                format!("{transfer}ms")
            ))
        );
        let _ = writeln!(
            out,
            "            |                |               |                   |                  |"
        );
        let _ = writeln!(
            out,
            "   {:<21}|               |                   |                  |",
            format!("namelookup:{dns}ms")
        );
        let _ = writeln!(
            out,
            "                       {:<14}|                   |                  |",
            format!("connect:{connect}ms")
        );
        let _ = writeln!(
            out,
            "                                   {:<18}|                  |",
            format!("pretransfer:{pretransfer}ms")
        );
        let _ = writeln!(
            out,
            "                                                     {:<19}|",
            format!("starttransfer:{starttransfer}ms")
        );
        let _ = writeln!(out, "                                                                                total:{total}ms");
    } else {
        let _ = writeln!(
            out,
            "  DNS Lookup   TCP Connection   Server Processing   Content Transfer"
        );
        let _ = writeln!(
            out,
            "{}",
            palette.bold(&format!(
                "[{:>10}  |{:>14}  |{:>19}  |{:>15}  ]",
                format!("{dns}ms"),
                format!("{tcp}ms"),
                format!("{server}ms"),
                format!("{transfer}ms")
            ))
        );
        let _ = writeln!(
            out,
            "            |                |                   |                  |"
        );
        let _ = writeln!(
            out,
            "   {:<21}|                   |                  |",
            format!("namelookup:{dns}ms")
        );
        let _ = writeln!(
            out,
            "                       {:<14}|                  |",
            format!("connect:{connect}ms")
        );
        let _ = writeln!(
            out,
            "                                       {:<18}|",
            format!("starttransfer:{starttransfer}ms")
        );
        let _ = writeln!(out, "                                                          total:{total}ms");
    }
}

fn ms(value: f64) -> u64 {
    value.round().max(0.0) as u64
}
