//! The two keys da-state needs from `run.json` — the run manifest `bin/run`
//! writes at setup. JSON replaced the earlier EDN manifest precisely so both
//! sides read it with a real parser (serde here, cheshire in babashka)
//! instead of the tolerant key-scan EDN forced on the Rust side (ADR-0003).

use da_domain::Phase;
use serde::Deserialize;

/// The extracted facts, still stringly — the caller refines them.
/// Unknown keys are ignored: the manifest carries more than da-state needs.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct ManifestFacts {
    #[serde(rename = "run-id")]
    pub run_id: Option<String>,
    pub phase: Option<String>,
}

/// Parse the manifest; the error is serde's own message, for the caller to
/// wrap with the file's path.
pub fn parse_manifest(json: &str) -> Result<ManifestFacts, String> {
    serde_json::from_str(json).map_err(|error: serde_json::Error| error.to_string())
}

/// Parse a `"phase"` string; absent defaults to steady-state (as `bin/run`'s
/// `manifest` does), unknown text is a loud `None` for the caller to refuse.
pub fn parse_phase(phase: Option<&str>) -> Option<Phase> {
    match phase {
        None | Some("steady-state") => Some(Phase::SteadyState),
        Some("convergence") => Some(Phase::Convergence),
        Some(_) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{ManifestFacts, parse_manifest, parse_phase};
    use da_domain::Phase;

    // The shape bin/run's manifest writes.
    const MANIFEST: &str = r#"{"run-id":"250718-widget","arm":"pre","round":"r1","phase":"convergence","target":{"project":"/p","branch":"da/250718-widget"}}"#;

    #[test]
    fn extracts_run_id_and_phase() {
        assert_eq!(
            parse_manifest(MANIFEST),
            Ok(ManifestFacts {
                run_id: Some("250718-widget".to_string()),
                phase: Some("convergence".to_string()),
            })
        );
    }

    #[test]
    fn missing_keys_are_none() {
        assert_eq!(
            parse_manifest(r#"{"arm":"pre"}"#),
            Ok(ManifestFacts {
                run_id: None,
                phase: None,
            })
        );
    }

    #[test]
    fn malformed_json_is_a_loud_error() {
        assert!(parse_manifest("{:run-id \"edn-not-json\"}").is_err());
    }

    #[test]
    fn absent_phase_defaults_to_steady_state() {
        assert_eq!(parse_phase(None), Some(Phase::SteadyState));
    }

    #[test]
    fn known_phases_parse() {
        assert_eq!(parse_phase(Some("convergence")), Some(Phase::Convergence));
        assert_eq!(parse_phase(Some("steady-state")), Some(Phase::SteadyState));
    }

    #[test]
    fn unknown_phase_is_loudly_none() {
        assert_eq!(parse_phase(Some("warp-speed")), None);
    }
}
