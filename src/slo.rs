use std::collections::BTreeMap;

use crate::model::{PhaseDurations, SloViolation};

#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd)]
pub enum MetricKey {
    Dns,
    Connect,
    Tls,
    Server,
    Transfer,
    Total,
}

impl MetricKey {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Dns => "dns",
            Self::Connect => "connect",
            Self::Tls => "tls",
            Self::Server => "server",
            Self::Transfer => "transfer",
            Self::Total => "total",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Dns => "DNS Lookup",
            Self::Connect => "TCP Connect",
            Self::Tls => "TLS Handshake",
            Self::Server => "Server Work",
            Self::Transfer => "Content Transfer",
            Self::Total => "Total",
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct SloCheck {
    pub metric: MetricKey,
    pub threshold_ms: f64,
}

impl PhaseDurations {
    pub fn metric(self, key: MetricKey) -> f64 {
        match key {
            MetricKey::Dns => self.dns_ms,
            MetricKey::Connect => self.connect_ms,
            MetricKey::Tls => self.tls_ms,
            MetricKey::Server => self.server_ms,
            MetricKey::Transfer => self.transfer_ms,
            MetricKey::Total => self.total_ms,
        }
    }
}

pub fn parse_slo_checks(parts: &[String]) -> Result<Vec<SloCheck>, String> {
    let mut checks = Vec::new();
    for part in parts {
        if part.trim().is_empty() {
            continue;
        }
        let (name, value) = part
            .split_once('=')
            .ok_or_else(|| format!("invalid SLO '{part}', expected metric=value"))?;
        let metric = match name.trim().to_ascii_lowercase().as_str() {
            "dns" => MetricKey::Dns,
            "connect" => MetricKey::Connect,
            "tls" => MetricKey::Tls,
            "server" => MetricKey::Server,
            "transfer" => MetricKey::Transfer,
            "total" => MetricKey::Total,
            _ => return Err(format!("unsupported SLO metric '{name}'")),
        };
        let threshold_ms = value
            .trim()
            .parse::<f64>()
            .map_err(|_| format!("invalid SLO threshold '{value}'"))?;
        checks.push(SloCheck {
            metric,
            threshold_ms,
        });
    }
    Ok(checks)
}

pub fn evaluate_slos(timings: PhaseDurations, checks: &[SloCheck]) -> Vec<SloViolation> {
    let mut violations = Vec::new();
    let mut by_metric = BTreeMap::<MetricKey, f64>::new();
    for check in checks {
        by_metric.insert(check.metric, check.threshold_ms);
    }
    for (metric, threshold_ms) in by_metric {
        let actual_ms = timings.metric(metric);
        if actual_ms > threshold_ms {
            violations.push(SloViolation {
                metric: metric.as_str().to_string(),
                actual_ms,
                threshold_ms,
            });
        }
    }
    violations
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_slo_checks() {
        let checks = parse_slo_checks(&["total=500".into(), "connect=120".into()]).unwrap();
        assert_eq!(checks.len(), 2);
        assert_eq!(checks[0].metric, MetricKey::Total);
        assert_eq!(checks[1].metric, MetricKey::Connect);
    }

    #[test]
    fn reports_slo_violations() {
        let timings = PhaseDurations {
            dns_ms: 10.0,
            connect_ms: 20.0,
            tls_ms: 30.0,
            server_ms: 40.0,
            transfer_ms: 50.0,
            total_ms: 150.0,
        };
        let violations = evaluate_slos(
            timings,
            &[
                SloCheck {
                    metric: MetricKey::Connect,
                    threshold_ms: 10.0,
                },
                SloCheck {
                    metric: MetricKey::Total,
                    threshold_ms: 200.0,
                },
            ],
        );
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].metric, "connect");
    }
}
