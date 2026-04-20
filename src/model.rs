use serde::Serialize;

#[derive(Debug, Clone, Copy, Serialize)]
pub struct PhaseDurations {
    pub dns_ms: f64,
    pub connect_ms: f64,
    pub tls_ms: f64,
    pub server_ms: f64,
    pub transfer_ms: f64,
    pub total_ms: f64,
}

#[derive(Debug, Serialize)]
pub struct JsonResult<'a> {
    pub schema: &'static str,
    pub request: RequestSummary<'a>,
    pub response: ResponseSummary,
    pub timings: PhaseDurations,
    pub diagnostics: Vec<Diagnostic>,
    pub slo: SloReport,
}

#[derive(Debug, Serialize)]
pub struct RequestSummary<'a> {
    pub method: &'a str,
    pub url: &'a str,
    pub proxy: Option<&'a str>,
}

#[derive(Debug, Serialize)]
pub struct ResponseSummary {
    pub status_code: u32,
    pub http_version: String,
    pub remote_ip: Option<String>,
    pub local_ip: Option<String>,
    pub downloaded_bytes: u64,
    pub uploaded_bytes: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct Diagnostic {
    pub level: &'static str,
    pub code: &'static str,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct SloReport {
    pub passed: bool,
    pub violated: Vec<SloViolation>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SloViolation {
    pub metric: String,
    pub actual_ms: f64,
    pub threshold_ms: f64,
}

#[derive(Debug)]
pub struct Measurement {
    pub status_code: u32,
    pub http_version: String,
    pub remote_ip: Option<String>,
    pub local_ip: Option<String>,
    pub downloaded_bytes: u64,
    pub uploaded_bytes: u64,
    pub timings: PhaseDurations,
    pub diagnostics: Vec<Diagnostic>,
}
