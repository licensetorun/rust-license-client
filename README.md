# Rust License Client

A small Rust client for [Licensetorun.com](https://licensetorun.com). Activate,
validate, swap and check updates for your licensed product — against
licensetorun.com or your own self-hosted on-prem server.

## Install

```toml
# Cargo.toml
[dependencies]
licensetorun-license-client = "1"
```

## Usage

```rust
use licensetorun_license_client::{Config, LicenseClient};

fn main() {
    let client = LicenseClient::new(Config {
        api_base: "https://licensetorun.com".into(),
        product_id: "PRODUCT-PUBLIC-UUID".into(), // from the product page in the dashboard
        license_key: "CUSTOMER-KEY".into(),
        ..Default::default() // instance defaults to the hostname; 15s timeout
    })
    .expect("config");

    // Activate once (consumes a seat):
    let res = client.activate(&[]);
    if !res.ok {
        eprintln!("activation failed: {}", res.body["message"]);
    }

    // Gate a feature:
    if client.is_valid() {
        // ...licensed feature...
    }

    // Check for updates:
    let update = client.check_for_update("1.2.0");
    if update.body["update_available"] == true {
        println!("new version: {}", update.body["version"]);
    }
}
```

## Result & errors

Every call returns a `LicenseResult` and never panics:

```rust
pub struct LicenseResult {
    pub ok: bool,        // true for a 2xx response
    pub status: u16,     // HTTP status, or 0 on a transport error
    pub body: serde_json::Value, // parsed JSON (or {"error","message"} on failure)
}
```

A transport failure (DNS, refused connection, timeout) comes back as
`ok == false`, `status == 0` and `body["error"] == "network_error"`. An HTTP
error status (e.g. 403) is `ok == false` with the parsed body.

## Notes

- Blocking I/O via [`ureq`](https://crates.io/crates/ureq); JSON via `serde_json`.
- `instance` defaults to the machine hostname — pass it explicitly for a stable id.
- Point `api_base` at `https://licensetorun.com` or at your self-hosted on-prem server.
- Cache `is_valid()` yourself (e.g. for a few hours) if you call it often.

MIT licensed.
