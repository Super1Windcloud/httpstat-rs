use std::time::Duration;

use curl::easy::{Easy, List};

use crate::cli::Cli;
use crate::diagnostics::build_diagnostics;
use crate::model::{Diagnostic, Measurement, PhaseDurations};

pub fn perform_request(cli: &Cli) -> Result<Measurement, String> {
    let mut easy = Easy::new();
    easy.url(&cli.url).map_err(to_string)?;
    easy.custom_request(&cli.method).map_err(to_string)?;
    easy.follow_location(true).map_err(to_string)?;
    easy.connect_timeout(Duration::from_millis(cli.timeout_ms))
        .map_err(to_string)?;
    easy.timeout(Duration::from_millis(cli.timeout_ms))
        .map_err(to_string)?;
    easy.ssl_verify_peer(!cli.insecure).map_err(to_string)?;
    easy.ssl_verify_host(!cli.insecure).map_err(to_string)?;

    if let Some(proxy) = &cli.proxy {
        easy.proxy(proxy).map_err(to_string)?;
    }

    if !cli.headers.is_empty() {
        let mut list = List::new();
        for header in &cli.headers {
            list.append(header).map_err(to_string)?;
        }
        easy.http_headers(list).map_err(to_string)?;
    }

    if let Some(body) = &cli.data {
        easy.post(true).map_err(to_string)?;
        easy.post_fields_copy(body.as_bytes()).map_err(to_string)?;
    }

    let mut sink = Vec::new();
    let mut status_line = None::<String>;
    let mut response_headers = Vec::<String>::new();
    let mut transfer = easy.transfer();
    transfer
        .header_function(|header| {
            let line = String::from_utf8_lossy(header).trim().to_string();
            if line.starts_with("HTTP/") {
                status_line = Some(line);
                response_headers.clear();
            } else if !line.is_empty() && status_line.is_some() {
                response_headers.push(line);
            }
            true
        })
        .map_err(to_string)?;
    transfer
        .write_function(|data| {
            sink.extend_from_slice(data);
            Ok(data.len())
        })
        .map_err(to_string)?;
    transfer.perform().map_err(to_string)?;
    drop(transfer);

    let namelookup = duration_ms(easy.namelookup_time().map_err(to_string)?);
    let connect = duration_ms(easy.connect_time().map_err(to_string)?);
    let appconnect = duration_ms(easy.appconnect_time().map_err(to_string)?);
    let pretransfer = duration_ms(easy.pretransfer_time().map_err(to_string)?);
    let starttransfer = duration_ms(easy.starttransfer_time().map_err(to_string)?);
    let total = duration_ms(easy.total_time().map_err(to_string)?);

    let dns_ms = namelookup.max(0.0);
    let connect_ms = (connect - namelookup).max(0.0);
    let tls_end = if appconnect > 0.0 {
        appconnect
    } else {
        pretransfer
    };
    let tls_ms = (tls_end - connect).max(0.0);
    let server_ms = (starttransfer - tls_end).max(0.0);
    let transfer_ms = (total - starttransfer).max(0.0);

    let timings = PhaseDurations {
        dns_ms,
        connect_ms,
        tls_ms,
        server_ms,
        transfer_ms,
        total_ms: total.max(0.0),
    };

    let mut diagnostics = build_diagnostics(timings, cli.proxy.as_deref().is_some());
    if cli.insecure {
        diagnostics.push(Diagnostic {
            level: "warn",
            code: "tls_verification_disabled",
            message: "TLS verification is disabled, so handshake timing is not a trust signal."
                .to_string(),
        });
    }

    Ok(Measurement {
        status_code: easy.response_code().map_err(to_string)?,
        status_line: status_line
            .clone()
            .unwrap_or_else(|| format!("{} {}", parse_http_version(None), 0)),
        http_version: parse_http_version(status_line.as_deref()),
        response_headers,
        remote_ip: easy.primary_ip().ok().flatten().map(ToOwned::to_owned),
        remote_port: easy.primary_port().map_err(to_string)?,
        local_ip: easy.local_ip().ok().flatten().map(ToOwned::to_owned),
        local_port: easy.local_port().map_err(to_string)?,
        downloaded_bytes: easy.download_size().map_err(to_string)?.round() as u64,
        uploaded_bytes: easy.upload_size().map_err(to_string)?.round() as u64,
        response_body: sink,
        timings,
        diagnostics,
    })
}

fn duration_ms(duration: Duration) -> f64 {
    duration.as_secs_f64() * 1000.0
}

fn parse_http_version(status_line: Option<&str>) -> String {
    status_line
        .and_then(|line| line.split_whitespace().next())
        .unwrap_or("HTTP/?")
        .to_string()
}

fn to_string(error: impl std::fmt::Display) -> String {
    error.to_string()
}
