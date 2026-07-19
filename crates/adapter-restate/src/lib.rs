//! Ring 3: the Restate ingress adapter behind the `RunMirror` port.
//!
//! Talks to the `DaRun` virtual object over the Restate ingress HTTP API with
//! an in-process client (reqwest + rustls) — no curl subprocess. The Rust
//! `restate-sdk` crate is service-side only (it has no ingress client), so
//! the ingress API is spoken directly; the TS service on the other end is
//! built with the official SDK. `RESTATE_CA` semantics match `bin/steer`:
//! a private-CA PEM bundle for an https ingress on the tailnet.

use da_domain::{Derived, RunId};
use da_ports::{MirrorError, MirrorSnapshot, RunArtifact, RunMirror};
use da_wire::{ArtifactsWire, DerivedWire, MirrorSnapshotWire};
use std::path::PathBuf;
use std::time::Duration;

/// Publishes to a Restate ingress, e.g. `https://restate-ingress.homelab`.
pub struct RestateIngressMirror {
    pub ingress: String,
    /// PEM bundle for a private CA (`RESTATE_CA`), when the ingress is https.
    pub ca_path: Option<PathBuf>,
}

/// The ingress URL of a `DaRun/<run-id>/<handler>` call — pure, test-pinned.
pub fn handler_url(ingress: &str, run_id: &str, handler: &str) -> String {
    format!(
        "{}/DaRun/{}/{}",
        ingress.trim_end_matches('/'),
        run_id,
        handler
    )
}

impl RunMirror for RestateIngressMirror {
    fn publish(&self, run_id: &RunId, derived: &Derived) -> Result<(), MirrorError> {
        let wire: DerivedWire = DerivedWire::from_domain(run_id, derived);
        self.call(run_id.as_str(), "recordState", &wire)?;
        Ok(())
    }

    fn publish_artifacts(&self, run_id: &RunId, files: &[RunArtifact]) -> Result<(), MirrorError> {
        let wire: ArtifactsWire = ArtifactsWire::from_ports(files);
        self.call(run_id.as_str(), "recordArtifacts", &wire)?;
        Ok(())
    }

    fn fetch_snapshot(&self, run_id: &RunId) -> Result<MirrorSnapshot, MirrorError> {
        let body: String = self.call(run_id.as_str(), "getSnapshot", &serde_json::json!({}))?;
        let wire: MirrorSnapshotWire =
            serde_json::from_str(&body).map_err(|error: serde_json::Error| MirrorError {
                detail: format!("getSnapshot returned unreadable JSON: {error}"),
            })?;
        Ok(wire.into_ports())
    }
}

impl RestateIngressMirror {
    fn call<T: serde::Serialize>(
        &self,
        run_id: &str,
        handler: &str,
        body: &T,
    ) -> Result<String, MirrorError> {
        let url: String = handler_url(&self.ingress, run_id, handler);
        let response: reqwest::blocking::Response = self
            .client()?
            .post(&url)
            .json(body)
            .send()
            .map_err(|error: reqwest::Error| MirrorError {
                detail: format!("{handler}: {error}"),
            })?;
        let status: reqwest::StatusCode = response.status();
        let text: String = response
            .text()
            .map_err(|error: reqwest::Error| MirrorError {
                detail: format!("{handler}: read response: {error}"),
            })?;
        if status.is_success() {
            Ok(text)
        } else {
            Err(MirrorError {
                detail: format!("{handler}: HTTP {status}: {}", text.trim()),
            })
        }
    }

    fn client(&self) -> Result<reqwest::blocking::Client, MirrorError> {
        let mut builder: reqwest::blocking::ClientBuilder = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(30))
            .use_rustls_tls();
        if let Some(ca_path) = &self.ca_path {
            let pem: Vec<u8> =
                std::fs::read(ca_path).map_err(|error: std::io::Error| MirrorError {
                    detail: format!("read RESTATE_CA {}: {error}", ca_path.display()),
                })?;
            let certificate: reqwest::Certificate =
                reqwest::Certificate::from_pem(&pem).map_err(|error: reqwest::Error| {
                    MirrorError {
                        detail: format!("parse RESTATE_CA {}: {error}", ca_path.display()),
                    }
                })?;
            builder = builder.add_root_certificate(certificate);
        }
        builder
            .build()
            .map_err(|error: reqwest::Error| MirrorError {
                detail: format!("build http client: {error}"),
            })
    }
}

#[cfg(test)]
mod tests {
    use super::handler_url;

    #[test]
    fn url_targets_the_record_state_handler() {
        assert_eq!(
            handler_url("https://i", "250718-x", "recordState"),
            "https://i/DaRun/250718-x/recordState"
        );
    }

    #[test]
    fn a_trailing_ingress_slash_does_not_double() {
        assert_eq!(
            handler_url("https://i/", "r", "recordState"),
            "https://i/DaRun/r/recordState"
        );
    }

    #[test]
    fn url_targets_the_record_artifacts_handler() {
        assert_eq!(
            handler_url("https://i", "r", "recordArtifacts"),
            "https://i/DaRun/r/recordArtifacts"
        );
    }

    #[test]
    fn url_targets_the_get_snapshot_handler() {
        assert_eq!(
            handler_url("https://i", "r", "getSnapshot"),
            "https://i/DaRun/r/getSnapshot"
        );
    }
}
