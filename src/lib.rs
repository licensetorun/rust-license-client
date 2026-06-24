//! Tiny client for the [Licensetorun.com](https://licensetorun.com) licensing API.
//!
//! Activate, validate, swap and check updates for your licensed product — against
//! licensetorun.com or your own self-hosted on-prem server. Every method returns a
//! [`LicenseResult`] and never panics: a transport failure surfaces as
//! `ok == false`, `status == 0` and `body["error"] == "network_error"`.
//!
//! ```no_run
//! use licensetorun_license_client::{Config, LicenseClient};
//!
//! let client = LicenseClient::new(Config {
//!     api_base: "https://licensetorun.com".into(),
//!     product_id: "PRODUCT-PUBLIC-UUID".into(),
//!     license_key: "CUSTOMER-KEY".into(),
//!     ..Default::default()
//! })
//! .expect("config");
//!
//! if client.is_valid() {
//!     // ...licensed feature...
//! }
//! ```

use std::time::Duration;

use serde_json::{json, Value};

/// Configuration for a [`LicenseClient`].
pub struct Config {
    /// e.g. `"https://licensetorun.com"` or your self-hosted URL.
    pub api_base: String,
    /// The product's public id (UUID) from the dashboard.
    pub product_id: String,
    /// The customer's license key.
    pub license_key: String,
    /// Stable id for this install. Defaults to the machine hostname.
    pub instance: Option<String>,
    /// Per-request timeout. Defaults to 15s.
    pub timeout: Duration,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            api_base: String::new(),
            product_id: String::new(),
            license_key: String::new(),
            instance: None,
            timeout: Duration::from_secs(15),
        }
    }
}

/// Outcome of an API call.
#[derive(Debug, Clone)]
pub struct LicenseResult {
    /// True for a 2xx response.
    pub ok: bool,
    /// HTTP status code, or 0 on a transport error.
    pub status: u16,
    /// Parsed JSON body (or `{"error","message"}` on a transport error).
    pub body: Value,
}

/// Client for the licensing API. Build one with [`LicenseClient::new`].
pub struct LicenseClient {
    api_base: String,
    product_id: String,
    license_key: String,
    instance: String,
    agent: ureq::Agent,
}

impl LicenseClient {
    /// Build a client. Returns `Err` if `api_base` or `product_id` is empty.
    pub fn new(config: Config) -> Result<Self, String> {
        if config.api_base.is_empty() || config.product_id.is_empty() {
            return Err("LicenseClient requires api_base and product_id".into());
        }
        let instance = config
            .instance
            .filter(|s| !s.is_empty())
            .or_else(hostname)
            .unwrap_or_else(|| "app".into());
        let timeout = if config.timeout.is_zero() {
            Duration::from_secs(15)
        } else {
            config.timeout
        };
        Ok(Self {
            api_base: config.api_base.trim_end_matches('/').to_string(),
            product_id: config.product_id,
            license_key: config.license_key,
            instance,
            agent: ureq::AgentBuilder::new().timeout(timeout).build(),
        })
    }

    /// The install identity used for activation/validation.
    pub fn instance(&self) -> &str {
        &self.instance
    }

    /// Activate this install and consume a seat. `extra` adds optional fields.
    pub fn activate(&self, extra: &[(&str, &str)]) -> LicenseResult {
        let mut form = vec![
            ("license_key", self.license_key.as_str()),
            ("instance", self.instance.as_str()),
        ];
        form.extend_from_slice(extra);
        self.post("activate", &form)
    }

    /// Release the seat held by this install.
    pub fn deactivate(&self) -> LicenseResult {
        self.post(
            "deactivate",
            &[
                ("license_key", &self.license_key),
                ("instance", &self.instance),
            ],
        )
    }

    /// Validate the license + this install. Cache the result yourself if hot.
    pub fn validate(&self) -> LicenseResult {
        self.post(
            "validate",
            &[
                ("license_key", &self.license_key),
                ("instance", &self.instance),
            ],
        )
    }

    /// Boolean convenience over [`validate`](Self::validate).
    pub fn is_valid(&self) -> bool {
        self.validate().ok
    }

    /// Move this seat from one install to another.
    pub fn swap(
        &self,
        from_instance: &str,
        to_instance: &str,
        extra: &[(&str, &str)],
    ) -> LicenseResult {
        let mut form = vec![
            ("license_key", self.license_key.as_str()),
            ("from_instance", from_instance),
            ("to_instance", to_instance),
        ];
        form.extend_from_slice(extra);
        self.post("swap", &form)
    }

    /// List the installs currently holding a seat.
    pub fn activations(&self) -> LicenseResult {
        self.post("activations", &[("license_key", &self.license_key)])
    }

    /// Ask the update server whether a newer release exists.
    pub fn check_for_update(&self, current_version: &str) -> LicenseResult {
        let url = self.endpoint("update");
        finish(
            self.agent
                .get(&url)
                .query("license_key", &self.license_key)
                .query("instance", &self.instance)
                .query("version", current_version)
                .call(),
        )
    }

    fn post(&self, path: &str, form: &[(&str, &str)]) -> LicenseResult {
        finish(self.agent.post(&self.endpoint(path)).send_form(form))
    }

    fn endpoint(&self, path: &str) -> String {
        format!(
            "{}/api/v1/products/{}/{}",
            self.api_base,
            encode_segment(&self.product_id),
            path
        )
    }
}

fn finish(result: Result<ureq::Response, ureq::Error>) -> LicenseResult {
    match result {
        Ok(resp) => {
            let status = resp.status();
            LicenseResult {
                ok: (200..300).contains(&status),
                status,
                body: read_json(resp),
            }
        }
        Err(ureq::Error::Status(code, resp)) => LicenseResult {
            ok: false,
            status: code,
            body: read_json(resp),
        },
        Err(ureq::Error::Transport(t)) => LicenseResult {
            ok: false,
            status: 0,
            body: json!({ "error": "network_error", "message": t.to_string() }),
        },
    }
}

fn read_json(resp: ureq::Response) -> Value {
    resp.into_string()
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_else(|| json!({}))
}

/// Percent-encode a single path segment (RFC 3986 unreserved set kept as-is),
/// matching encodeURIComponent for the characters that appear in product ids.
fn encode_segment(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for &b in s.as_bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

fn hostname() -> Option<String> {
    if let Ok(h) = std::env::var("HOSTNAME") {
        if !h.is_empty() {
            return Some(h);
        }
    }
    std::process::Command::new("hostname")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn requires_config() {
        assert!(LicenseClient::new(Config::default()).is_err());
        assert!(LicenseClient::new(Config {
            api_base: "x".into(),
            ..Default::default()
        })
        .is_err());
        assert!(LicenseClient::new(Config {
            product_id: "p".into(),
            ..Default::default()
        })
        .is_err());
    }

    #[test]
    fn trims_trailing_slash_and_builds_endpoint() {
        let c = LicenseClient::new(Config {
            api_base: "https://x.test///".into(),
            product_id: "prod 1/x".into(),
            instance: Some("i".into()),
            ..Default::default()
        })
        .unwrap();
        assert_eq!(
            c.endpoint("activate"),
            "https://x.test/api/v1/products/prod%201%2Fx/activate"
        );
        assert_eq!(c.instance(), "i");
    }

    #[test]
    fn encode_segment_matches_encodeuricomponent() {
        assert_eq!(encode_segment("prod 1/x"), "prod%201%2Fx");
        assert_eq!(encode_segment("abc-_.~09AZ"), "abc-_.~09AZ");
    }

    #[test]
    fn transport_failure_is_network_error() {
        let c = LicenseClient::new(Config {
            api_base: "http://127.0.0.1:1".into(),
            product_id: "p".into(),
            license_key: "k".into(),
            instance: Some("i".into()),
            timeout: Duration::from_millis(500),
        })
        .unwrap();
        let r = c.validate();
        assert!(!r.ok);
        assert_eq!(r.status, 0);
        assert_eq!(r.body["error"], "network_error");
    }
}
