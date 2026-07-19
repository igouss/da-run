//! The run's observability journal (`events.jsonl`, ADR-0004): a fingerprint
//! of the operator-editable surface — `spec.md` plus every `stages/*/output`
//! file except `STEER-REQUEST.md` — recorded at every engine touchpoint.
//! `bin/run` journals its own touchpoints (setup, seal, gate) in babashka;
//! `da-state check` journals the dispatch touchpoint here. Both sides MUST
//! compute the identical fingerprint or capture-time classification breaks —
//! the algorithm is pinned by a shared fixture digest in this module's tests
//! and in `bin/run --selftest`.

use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

/// The journal file, at the run root beside `run.json`.
pub const EVENTS_FILE: &str = "events.jsonl";
const STEER_FILE: &str = "STEER-REQUEST.md";
const SPEC_FILE: &str = "spec.md";

/// Errors are strings: journaling is best-effort by design (a failed append
/// must never fail a check), so the caller only ever prints the detail.
pub fn append_event(run_dir: &Path, trigger: &str) -> Result<(), String> {
    let fingerprint: String = outputs_fingerprint(run_dir)?;
    let timestamp: String = humantime::format_rfc3339_millis(std::time::SystemTime::now()).to_string();
    let line: String = serde_json::json!({
        "ts": timestamp,
        "trigger": trigger,
        "fingerprint": fingerprint,
    })
    .to_string();
    let path: PathBuf = run_dir.join(EVENTS_FILE);
    let mut journal: String = match std::fs::read_to_string(&path) {
        Ok(existing) => existing,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(error) => return Err(format!("{}: {error}", path.display())),
    };
    journal.push_str(&line);
    journal.push('\n');
    std::fs::write(&path, journal).map_err(|error: std::io::Error| format!("{}: {error}", path.display()))
}

/// sha256 over sorted `<relative path> <content sha256>` lines of `spec.md`
/// and every `stages/*/output` file except `STEER-REQUEST.md` — byte-parity
/// with `outputs-fingerprint` in `engine/bin/run`.
pub fn outputs_fingerprint(run_dir: &Path) -> Result<String, String> {
    let mut entries: Vec<(String, Vec<u8>)> = Vec::new();
    let spec: PathBuf = run_dir.join(SPEC_FILE);
    if spec.is_file() {
        entries.push((SPEC_FILE.to_string(), read_bytes(&spec)?));
    }
    let stages: PathBuf = run_dir.join("stages");
    if stages.is_dir() {
        for stage in read_dir_sorted(&stages)? {
            let output: PathBuf = stage.join("output");
            if output.is_dir() {
                collect_files(run_dir, &output, &mut entries)?;
            }
        }
    }
    entries.sort_by(|a: &(String, Vec<u8>), b: &(String, Vec<u8>)| a.0.cmp(&b.0));
    Ok(fingerprint_of(&entries))
}

/// The pure core: entries are already-read `(relative path, content bytes)`
/// pairs; the digest is over their sorted rendering. Exposed for the parity
/// fixture tests.
pub fn fingerprint_of(entries: &[(String, Vec<u8>)]) -> String {
    let lines: Vec<String> = entries
        .iter()
        .map(|(rel, content): &(String, Vec<u8>)| format!("{rel} {}", hex_sha256(content)))
        .collect();
    hex_sha256(lines.join("\n").as_bytes())
}

fn collect_files(
    run_dir: &Path,
    dir: &Path,
    entries: &mut Vec<(String, Vec<u8>)>,
) -> Result<(), String> {
    for path in read_dir_sorted(dir)? {
        let name: String = path
            .file_name()
            .map(|n: &std::ffi::OsStr| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        // Parity: bb's fs/glob never matches hidden entries, so .gitkeep
        // (and any dotfile) is outside the fingerprinted surface here too.
        if name.starts_with('.') {
            continue;
        }
        if path.is_dir() {
            collect_files(run_dir, &path, entries)?;
        } else if path.is_file() {
            if name == STEER_FILE {
                continue;
            }
            let rel: String = path
                .strip_prefix(run_dir)
                .map_err(|error: std::path::StripPrefixError| error.to_string())?
                .to_string_lossy()
                .into_owned();
            entries.push((rel, read_bytes(&path)?));
        }
    }
    Ok(())
}

fn read_dir_sorted(dir: &Path) -> Result<Vec<PathBuf>, String> {
    let reader = std::fs::read_dir(dir).map_err(|error: std::io::Error| format!("{}: {error}", dir.display()))?;
    let mut paths: Vec<PathBuf> = Vec::new();
    for entry in reader {
        let entry: std::fs::DirEntry =
            entry.map_err(|error: std::io::Error| format!("{}: {error}", dir.display()))?;
        paths.push(entry.path());
    }
    paths.sort();
    Ok(paths)
}

fn read_bytes(path: &Path) -> Result<Vec<u8>, String> {
    std::fs::read(path).map_err(|error: std::io::Error| format!("{}: {error}", path.display()))
}

fn hex_sha256(bytes: &[u8]) -> String {
    let mut hasher: Sha256 = Sha256::new();
    hasher.update(bytes);
    hasher.finalize().iter().map(|byte: &u8| format!("{byte:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::fingerprint_of;

    /// The shared parity fixture: `bin/run --selftest` pins the SAME hex over
    /// the SAME entries. A change to either side's algorithm breaks one pin.
    const PARITY_FINGERPRINT: &str =
        "ec51ef27d4bf77772dcb1f3c68107219e72d4d72eac7420899d54f0064abb0a7";
    const EMPTY_FINGERPRINT: &str =
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";

    fn entry(rel: &str, content: &str) -> (String, Vec<u8>) {
        (rel.to_string(), content.as_bytes().to_vec())
    }

    #[test]
    fn zero_entries_is_the_empty_digest() {
        assert_eq!(fingerprint_of(&[]), EMPTY_FINGERPRINT);
    }

    #[test]
    fn parity_fixture_matches_the_babashka_pin() {
        let entries: Vec<(String, Vec<u8>)> = vec![
            entry("spec.md", "# spec\n"),
            entry("stages/01-plan/output/plan.md", "the plan\n"),
        ];
        assert_eq!(fingerprint_of(&entries), PARITY_FINGERPRINT);
    }

    #[test]
    fn content_change_moves_the_fingerprint() {
        let before: Vec<(String, Vec<u8>)> = vec![entry("spec.md", "# spec\n")];
        let after: Vec<(String, Vec<u8>)> = vec![entry("spec.md", "# spec v2\n")];
        assert_ne!(fingerprint_of(&before), fingerprint_of(&after));
    }
}
