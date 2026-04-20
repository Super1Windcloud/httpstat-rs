use crate::model::{Diagnostic, PhaseDurations};
use crate::slo::MetricKey;

pub fn build_diagnostics(timings: PhaseDurations, proxy_enabled: bool) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let total = timings.total_ms.max(1.0);
    let dominant = [
        (MetricKey::Dns, timings.dns_ms),
        (MetricKey::Connect, timings.connect_ms),
        (MetricKey::Tls, timings.tls_ms),
        (MetricKey::Server, timings.server_ms),
        (MetricKey::Transfer, timings.transfer_ms),
    ]
    .into_iter()
    .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

    if let Some((metric, value)) = dominant {
        if value / total >= 0.45 {
            diagnostics.push(Diagnostic {
                level: "info",
                code: "dominant_phase",
                message: format!(
                    "{} dominates the request path at {:.0}% of total time.",
                    metric.label(),
                    value / total * 100.0
                ),
            });
        }
    }

    if proxy_enabled {
        diagnostics.push(Diagnostic {
            level: "info",
            code: "proxy_enabled",
            message: "Proxy is enabled, so connect and TLS timings include the proxy hop before the origin path.".to_string(),
        });
    }

    if timings.server_ms >= 300.0 && timings.server_ms / total >= 0.35 {
        diagnostics.push(Diagnostic {
            level: "warn",
            code: "origin_latency",
            message: "Server processing is a large share of total latency; inspect application or upstream service time.".to_string(),
        });
    }

    diagnostics
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn emits_proxy_diagnostic() {
        let diagnostics = build_diagnostics(
            PhaseDurations {
                dns_ms: 20.0,
                connect_ms: 50.0,
                tls_ms: 100.0,
                server_ms: 70.0,
                transfer_ms: 20.0,
                total_ms: 260.0,
            },
            true,
        );
        assert!(diagnostics.iter().any(|item| item.code == "proxy_enabled"));
    }
}
