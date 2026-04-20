use std::env;
use std::fmt::Write as _;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::JSON_SCHEMA_VERSION;
use crate::cli::{Cli, OutputFormat};
use crate::model::{
    JsonResult, Measurement, RequestSummary, ResponseSummary, SloReport, SloViolation,
};
use crate::palette::Palette;

const HTTP_TEMPLATE: &str = concat!(
    "  DNS Lookup   TCP Connection   Server Processing   Content Transfer\n",
    "[   {a0000}  |     {a0001}    |      {a0003}      |      {a0004}     ]\n",
    "             |                |                   |                  |\n",
    "    namelookup:{b0000}        |                   |                  |\n",
    "                        connect:{b0001}           |                  |\n",
    "                                      starttransfer:{b0003}          |\n",
    "                                                                 total:{b0004}\n",
);

const HTTPS_TEMPLATE: &str = concat!(
    "  DNS Lookup   TCP Connection   TLS Handshake   Server Processing   Content Transfer\n",
    "[   {a0000}  |     {a0001}    |    {a0002}    |      {a0003}      |      {a0004}     ]\n",
    "             |                |               |                   |                  |\n",
    "    namelookup:{b0000}        |               |                   |                  |\n",
    "                        connect:{b0001}       |                   |                  |\n",
    "                                    pretransfer:{b0002}           |                  |\n",
    "                                                      starttransfer:{b0003}          |\n",
    "                                                                                 total:{b0004}\n",
);

pub struct RenderedOutput {
    pub display: String,
    pub save_contents: String,
}

pub fn render_output(
    cli: &Cli,
    measurement: &Measurement,
    violations: &[SloViolation],
    disable_color: bool,
) -> Result<RenderedOutput, String> {
    let payload = build_payload(cli, measurement, violations);
    let save_contents = serde_json::to_string_pretty(&payload).map_err(|err| err.to_string())?;

    let display = match cli.format {
        OutputFormat::Human => render_human(cli, measurement, violations, disable_color)?,
        OutputFormat::Json => save_contents.clone(),
        OutputFormat::Jsonl => serde_json::to_string(&payload).map_err(|err| err.to_string())?,
    };

    Ok(RenderedOutput {
        display,
        save_contents,
    })
}

fn build_payload<'a>(
    cli: &'a Cli,
    measurement: &Measurement,
    violations: &[SloViolation],
) -> JsonResult<'a> {
    JsonResult {
        schema: JSON_SCHEMA_VERSION,
        request: RequestSummary {
            method: &cli.method,
            url: &cli.url,
            proxy: cli.proxy.as_deref(),
        },
        response: ResponseSummary {
            status_line: measurement.status_line.clone(),
            status_code: measurement.status_code,
            http_version: measurement.http_version.clone(),
            remote_ip: measurement.remote_ip.clone(),
            remote_port: measurement.remote_port,
            local_ip: measurement.local_ip.clone(),
            local_port: measurement.local_port,
            headers: measurement.response_headers.clone(),
            downloaded_bytes: measurement.downloaded_bytes,
            uploaded_bytes: measurement.uploaded_bytes,
        },
        timings: measurement.timings,
        diagnostics: measurement.diagnostics.clone(),
        slo: SloReport {
            passed: violations.is_empty(),
            violated: violations.to_vec(),
        },
    }
}

fn render_human(
    cli: &Cli,
    measurement: &Measurement,
    violations: &[SloViolation],
    disable_color: bool,
) -> Result<String, String> {
    let mut out = String::new();
    let palette = Palette::new(disable_color);

    let show_ip = env_bool("HTTPSTAT_SHOW_IP", true)?;
    let show_body = env_bool("HTTPSTAT_SHOW_BODY", false)?;
    let show_speed = env_bool("HTTPSTAT_SHOW_SPEED", false)?;
    let save_body = env_bool("HTTPSTAT_SAVE_BODY", true)?;

    let body_path = if save_body {
        Some(write_body_file(&measurement.response_body)?)
    } else {
        None
    };

    if show_ip {
        let _ = writeln!(
            out,
            "Connected to {}:{} from {}:{}",
            palette.blue(measurement.remote_ip.as_deref().unwrap_or("")),
            palette.blue(&measurement.remote_port.to_string()),
            measurement.local_ip.as_deref().unwrap_or(""),
            measurement.local_port
        );
        let _ = writeln!(out);
    }

    write_status_and_headers(&mut out, measurement, &palette);
    let _ = writeln!(out);

    if show_body {
        write_body(&mut out, measurement, body_path.as_deref(), &palette);
    } else if let Some(path) = &body_path {
        let _ = writeln!(out, "{} stored in: {}", palette.green("Body"), path.display());
    }

    let _ = writeln!(out);
    write_httpstat_timeline(&mut out, cli, measurement, &palette);

    if show_speed {
        let _ = writeln!(out);
        let _ = writeln!(
            out,
            "speed_download: {:.1} KiB/s, speed_upload: {:.1} KiB/s",
            measurement.downloaded_bytes as f64 / (measurement.timings.total_ms / 1000.0).max(0.001) / 1024.0,
            measurement.uploaded_bytes as f64 / (measurement.timings.total_ms / 1000.0).max(0.001) / 1024.0
        );
    }

    if !violations.is_empty() {
        let _ = writeln!(out);
        for violation in violations {
            let _ = writeln!(
                out,
                "{}",
                palette.red(&format!(
                    "SLO VIOLATION: {} = {:.0}ms (threshold: {:.0}ms)",
                    violation.metric, violation.actual_ms, violation.threshold_ms
                ))
            );
        }
    }

    Ok(out.trim_end().to_string())
}

fn write_status_and_headers(out: &mut String, measurement: &Measurement, palette: &Palette) {
    let mut parts = measurement.status_line.splitn(2, '/');
    match (parts.next(), parts.next()) {
        (Some(left), Some(right)) => {
            let _ = writeln!(out, "{}{}{}", palette.green(left), palette.dim("/"), palette.blue(right));
        }
        _ => {
            let line = if measurement.status_code >= 400 {
                palette.red(&measurement.status_line)
            } else {
                palette.green(&measurement.status_line)
            };
            let _ = writeln!(out, "{line}");
        }
    }

    for header in &measurement.response_headers {
        if let Some((name, value)) = header.split_once(':') {
            let _ = writeln!(out, "{}{}", palette.dim(&format!("{name}:")), palette.blue(value));
        } else {
            let _ = writeln!(out, "{header}");
        }
    }
}

fn write_body(out: &mut String, measurement: &Measurement, body_path: Option<&std::path::Path>, palette: &Palette) {
    let body = String::from_utf8_lossy(&measurement.response_body).trim().to_string();
    let body_len = body.len();
    let body_limit = 1024usize;

    if body_len > body_limit {
        let _ = writeln!(out, "{}{}", &body[..body_limit], palette.blue("..."));
        let _ = writeln!(out);
        let mut suffix = format!(
            "{} is truncated ({} out of {})",
            palette.green("Body"),
            body_limit,
            body_len
        );
        if let Some(path) = body_path {
            suffix.push_str(&format!(", stored in: {}", path.display()));
        }
        let _ = writeln!(out, "{suffix}");
    } else if !body.is_empty() {
        let _ = writeln!(out, "{body}");
    }
}

fn write_httpstat_timeline(out: &mut String, cli: &Cli, measurement: &Measurement, palette: &Palette) {
    let template = if cli.url.starts_with("https://") {
        HTTPS_TEMPLATE
    } else {
        HTTP_TEMPLATE
    };

    let mut lines = template.lines();
    if let Some(first) = lines.next() {
        let _ = writeln!(out, "{}", palette.dim(first));
    }

    let rest = lines.collect::<Vec<_>>().join("\n");
    let stat = rest
        .replace("{a0000}", &fmt_center_ms(palette, ms(measurement.timings.dns_ms)))
        .replace("{a0001}", &fmt_center_ms(palette, ms(measurement.timings.connect_ms)))
        .replace("{a0002}", &fmt_center_ms(palette, ms(measurement.timings.tls_ms)))
        .replace("{a0003}", &fmt_center_ms(palette, ms(measurement.timings.server_ms)))
        .replace("{a0004}", &fmt_center_ms(palette, ms(measurement.timings.transfer_ms)))
        .replace("{b0000}", &fmt_left_ms(palette, ms(measurement.timings.dns_ms)))
        .replace(
            "{b0001}",
            &fmt_left_ms(
                palette,
                ms(measurement.timings.dns_ms + measurement.timings.connect_ms),
            ),
        )
        .replace(
            "{b0002}",
            &fmt_left_ms(
                palette,
                ms(
                    measurement.timings.dns_ms
                        + measurement.timings.connect_ms
                        + measurement.timings.tls_ms,
                ),
            ),
        )
        .replace(
            "{b0003}",
            &fmt_left_ms(palette, ms(measurement.timings.total_ms - measurement.timings.transfer_ms)),
        )
        .replace("{b0004}", &fmt_left_ms(palette, ms(measurement.timings.total_ms)));
    let _ = writeln!(out, "{stat}");
}

fn fmt_center_ms(palette: &Palette, value: u64) -> String {
    palette.blue(&format!("{:^7}", format!("{value}ms")))
}

fn fmt_left_ms(palette: &Palette, value: u64) -> String {
    palette.blue(&format!("{:<7}", format!("{value}ms")))
}

fn ms(value: f64) -> u64 {
    value.round().max(0.0) as u64
}

fn env_bool(name: &str, default: bool) -> Result<bool, String> {
    match env::var(name) {
        Ok(value) => parse_bool(&value).map_err(|err| format!("{name}: {err}")),
        Err(env::VarError::NotPresent) => Ok(default),
        Err(err) => Err(format!("{name}: {err}")),
    }
}

fn parse_bool(value: &str) -> Result<bool, &'static str> {
    match value.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => Ok(true),
        "0" | "false" | "no" | "off" => Ok(false),
        _ => Err("invalid boolean value"),
    }
}

fn write_body_file(body: &[u8]) -> Result<PathBuf, String> {
    let mut path = env::temp_dir();
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|err| err.to_string())?
        .as_nanos();
    path.push(format!("httpstat-rs-body-{nanos}.tmp"));
    fs::write(&path, body).map_err(|err| format!("failed to save body to {}: {err}", path.display()))?;
    Ok(path)
}
