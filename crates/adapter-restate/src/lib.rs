//! Ring 3: the Restate ingress adapter behind the `RunMirror` port.
//!
//! Publishes the wire payload to `DaRun/<run-id>/recordState` with curl —
//! deliberately the same transport `bin/steer` uses (same `RESTATE_CA`
//! semantics, same tailnet setup), so the two bridges never drift. The
//! command is built by a pure function and pinned by tests, `bin/steer`
//! selftest style.

use da_domain::{Derived, RunId};
use da_ports::{MirrorError, RunMirror};
use da_wire::DerivedWire;
use std::path::PathBuf;
use std::process::Command;

/// Publishes to a Restate ingress, e.g. `https://restate-ingress.homelab`.
pub struct RestateIngressMirror {
    pub ingress: String,
    /// PEM bundle for a private CA (`RESTATE_CA`), when the ingress is https.
    pub ca_path: Option<PathBuf>,
}

impl RunMirror for RestateIngressMirror {
    fn publish(&self, run_id: &RunId, derived: &Derived) -> Result<(), MirrorError> {
        let wire: DerivedWire = DerivedWire::from_domain(run_id, derived);
        let body: String =
            serde_json::to_string(&wire).map_err(|error: serde_json::Error| MirrorError {
                detail: format!("serialize mirror payload: {error}"),
            })?;
        let argv: Vec<String> = curl_argv(
            &self.ingress,
            self.ca_path
                .as_deref()
                .map(|p| p.to_string_lossy().into_owned()),
            run_id.as_str(),
            &body,
        );
        run_curl(&argv)
    }
}

/// The full curl argv for a recordState publish — pure, test-pinned.
pub fn curl_argv(ingress: &str, ca_path: Option<String>, run_id: &str, body: &str) -> Vec<String> {
    let mut argv: Vec<String> = vec![
        "curl".to_string(),
        "-fsS".to_string(),
        "-X".to_string(),
        "POST".to_string(),
        "-H".to_string(),
        "content-type: application/json".to_string(),
        "-d".to_string(),
        body.to_string(),
    ];
    if let Some(ca) = ca_path {
        argv.push("--cacert".to_string());
        argv.push(ca);
    }
    argv.push(format!(
        "{}/DaRun/{}/recordState",
        ingress.trim_end_matches('/'),
        run_id
    ));
    argv
}

fn run_curl(argv: &[String]) -> Result<(), MirrorError> {
    let output: std::process::Output =
        Command::new(&argv[0])
            .args(&argv[1..])
            .output()
            .map_err(|error: std::io::Error| MirrorError {
                detail: format!("spawn curl: {error}"),
            })?;
    if output.status.success() {
        Ok(())
    } else {
        Err(MirrorError {
            detail: format!(
                "curl exit {}: {}",
                output.status.code().unwrap_or(-1),
                String::from_utf8_lossy(&output.stderr).trim()
            ),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::curl_argv;

    #[test]
    fn argv_targets_the_record_state_handler() {
        let argv: Vec<String> = curl_argv("https://i", None, "250718-x", "{}");
        assert_eq!(
            argv.last().map(String::as_str),
            Some("https://i/DaRun/250718-x/recordState")
        );
    }

    #[test]
    fn a_trailing_ingress_slash_does_not_double() {
        let argv: Vec<String> = curl_argv("https://i/", None, "r", "{}");
        assert_eq!(
            argv.last().map(String::as_str),
            Some("https://i/DaRun/r/recordState")
        );
    }

    #[test]
    fn ca_path_becomes_cacert() {
        let argv: Vec<String> = curl_argv("https://i", Some("/ca.pem".to_string()), "r", "{}");
        assert!(
            argv.windows(2)
                .any(|pair: &[String]| { pair[0] == "--cacert" && pair[1] == "/ca.pem" })
        );
    }

    #[test]
    fn no_ca_means_no_cacert_flag() {
        let argv: Vec<String> = curl_argv("http://i", None, "r", "{}");
        assert!(!argv.iter().any(|arg: &String| arg == "--cacert"));
    }

    #[test]
    fn body_rides_in_the_d_flag() {
        let argv: Vec<String> = curl_argv("http://i", None, "r", "{\"v\":1}");
        assert!(
            argv.windows(2)
                .any(|pair: &[String]| { pair[0] == "-d" && pair[1] == "{\"v\":1}" })
        );
    }
}
