# httpstat-rs

`httpstat-rs` is a Rust CLI inspired by [`reorx/httpstat`](https://github.com/reorx/httpstat).
It measures end-to-end HTTP timings with a compact terminal view and stable machine-readable output.

## Features

- Compact terminal output for `DNS`, `TCP`, `TLS`, `Server`, `Transfer`, and `Total`
- Structured output with `--format json` and `--format jsonl` using a stable `v1` schema
- SLO checks with `--slo total=500,connect=100` and exit code `4` on violations
- Output persistence with `--save path.json`
- `NO_COLOR` support
- Built-in diagnostics, including proxy-aware timing hints

## Install

```bash
cargo install httpstat-rs
```

For local development:

```bash
cargo build
```

## Usage

```bash
httpstat-rs https://example.com
httpstat-rs --format json https://example.com
httpstat-rs https://example.com --slo total=500,connect=100
httpstat-rs https://example.com --proxy http://127.0.0.1:8080
httpstat-rs https://example.com --save result.json
```

## Output Formats

`--format human` is the default and prints a concise colored timing summary when color is enabled.

`--format json` emits pretty-printed JSON:

```json
{
  "schema": "v1",
  "request": {
    "method": "GET",
    "url": "https://example.com",
    "proxy": null
  },
  "response": {
    "status_code": 200,
    "http_version": "HTTP/1.1",
    "remote_ip": "203.0.113.10",
    "local_ip": "192.0.2.20",
    "downloaded_bytes": 528,
    "uploaded_bytes": 0
  },
  "timings": {
    "dns_ms": 4.1,
    "connect_ms": 22.0,
    "tls_ms": 48.8,
    "server_ms": 75.3,
    "transfer_ms": 0.7,
    "total_ms": 150.9
  },
  "diagnostics": [],
  "slo": {
    "passed": true,
    "violated": []
  }
}
```

`--format jsonl` emits the same `v1` payload as a single line.

## Exit Codes

- `0`: Success
- `1`: Runtime or argument error
- `4`: SLO violation
