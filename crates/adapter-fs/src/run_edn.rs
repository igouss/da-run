//! Tolerant extraction of the two keys da-state needs from `run.edn`
//! (`:run-id` and `:phase`, both top-level strings written by `bin/run`'s
//! `manifest`). Not an EDN parser — a scan for `:key "value"` pairs, which is
//! exactly the shape `bin/run` emits.

use da_domain::Phase;

/// The extracted facts, still stringly — the caller refines them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EdnFacts {
    pub run_id: Option<String>,
    pub phase: Option<String>,
}

/// Scan EDN text for `:run-id` and `:phase` string values.
pub fn extract_edn_facts(edn: &str) -> EdnFacts {
    EdnFacts {
        run_id: string_value(edn, ":run-id"),
        phase: string_value(edn, ":phase"),
    }
}

/// Parse a `:phase` string; absent defaults to steady-state (as `bin/run`'s
/// `manifest` does), unknown text is a loud `None` for the caller to refuse.
pub fn parse_phase(phase: Option<&str>) -> Option<Phase> {
    match phase {
        None | Some("steady-state") => Some(Phase::SteadyState),
        Some("convergence") => Some(Phase::Convergence),
        Some(_) => None,
    }
}

fn string_value(edn: &str, key: &str) -> Option<String> {
    let after_key: &str = find_key(edn, key)?;
    let after_quote: &str = after_key.trim_start().strip_prefix('"')?;
    let end: usize = after_quote.find('"')?;
    Some(after_quote[..end].to_string())
}

/// The text after the first occurrence of `key` followed by whitespace.
fn find_key<'a>(edn: &'a str, key: &str) -> Option<&'a str> {
    let mut rest: &str = edn;
    loop {
        let position: usize = rest.find(key)?;
        let after: &str = &rest[position + key.len()..];
        let boundary: bool = after
            .chars()
            .next()
            .is_none_or(|next: char| next.is_whitespace());
        if boundary {
            return Some(after);
        }
        rest = after;
    }
}

#[cfg(test)]
mod tests {
    use super::{EdnFacts, extract_edn_facts, parse_phase};
    use da_domain::Phase;

    // The shape bin/run's manifest writes.
    const MANIFEST: &str = "{:run-id \"250718-widget\"\n :arm \"pre\"\n :round \"r1\"\n :phase \"convergence\"\n :target {:project \"/p\" :branch \"da/250718-widget\"}}";

    #[test]
    fn extracts_run_id_and_phase() {
        assert_eq!(
            extract_edn_facts(MANIFEST),
            EdnFacts {
                run_id: Some("250718-widget".to_string()),
                phase: Some("convergence".to_string()),
            }
        );
    }

    #[test]
    fn missing_keys_are_none() {
        assert_eq!(
            extract_edn_facts("{:arm \"pre\"}"),
            EdnFacts {
                run_id: None,
                phase: None,
            }
        );
    }

    #[test]
    fn a_longer_key_does_not_shadow_a_shorter_one() {
        // :run-id-extra must not satisfy a :run-id lookup.
        let edn: &str = "{:run-id-extra \"wrong\" :run-id \"right\"}";
        assert_eq!(extract_edn_facts(edn).run_id, Some("right".to_string()));
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
