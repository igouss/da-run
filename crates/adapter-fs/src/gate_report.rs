//! The gate verdict, parity with `gate.sh:42-48`: the script's last act is to
//! print exactly `GATE GREEN` or `GATE RED — do not ship`. The verdict is the
//! last such line; anything else (absent, garbage) is no verdict at all.

use da_domain::{Verdict, WorktreeId};

/// The provenance line the orchestrator appends after running the gate,
/// naming the worktree the verdict describes.
const WORKTREE_MARKER: &str = "Worktree:";

/// The worktree identity a gate report claims to have verified, if it records
/// one. An older report that names none reads as `None`, and the commit law
/// treats that as a mismatch rather than a pass.
pub fn gate_worktree(report: &str) -> Option<WorktreeId> {
    report
        .lines()
        .filter_map(|line: &str| line.trim().strip_prefix(WORKTREE_MARKER))
        .filter_map(|raw: &str| WorktreeId::new(raw).ok())
        .next_back()
}

/// The verdict from a gate report's contents, if a verdict line exists.
pub fn gate_verdict(report: &str) -> Option<Verdict> {
    report
        .lines()
        .filter_map(|line: &str| verdict_line(line))
        .next_back()
}

fn verdict_line(line: &str) -> Option<Verdict> {
    let trimmed: &str = line.trim_end();
    if trimmed == "GATE GREEN" {
        Some(Verdict::Green)
    } else if trimmed.starts_with("GATE RED") {
        Some(Verdict::Red)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use super::{gate_verdict, gate_worktree};
    use da_domain::Verdict;

    #[test]
    fn a_report_naming_a_worktree_yields_it() {
        let id = gate_worktree("=== gate ===\nWorktree: abc123\n\nGATE GREEN\n")
            .expect("the report names a worktree");
        assert_eq!(id.as_str(), "abc123");
    }

    #[test]
    fn a_report_naming_no_worktree_yields_none() {
        assert_eq!(gate_worktree("all ok\n\nGATE GREEN\n"), None);
    }

    #[test]
    fn a_blank_worktree_line_yields_none() {
        assert_eq!(gate_worktree("Worktree:   \n\nGATE GREEN\n"), None);
    }

    #[test]
    fn the_last_worktree_line_wins() {
        let id = gate_worktree("Worktree: first\nre-run:\nWorktree: second\nGATE GREEN\n")
            .expect("the report names a worktree");
        assert_eq!(id.as_str(), "second");
    }

    #[test]
    fn green_line_is_green() {
        assert_eq!(
            gate_verdict("=== gate ===\nall ok\n\nGATE GREEN\n"),
            Some(Verdict::Green)
        );
    }

    #[test]
    fn red_line_with_suffix_is_red() {
        assert_eq!(
            gate_verdict("tests failed\n\nGATE RED — do not ship\n"),
            Some(Verdict::Red)
        );
    }

    #[test]
    fn garbage_report_has_no_verdict() {
        assert_eq!(gate_verdict("nothing conclusive here"), None);
    }

    #[test]
    fn empty_report_has_no_verdict() {
        assert_eq!(gate_verdict(""), None);
    }

    #[test]
    fn last_verdict_line_wins() {
        assert_eq!(
            gate_verdict("GATE RED — do not ship\nrerun:\nGATE GREEN\n"),
            Some(Verdict::Green)
        );
    }

    #[test]
    fn a_mention_mid_line_is_not_a_verdict() {
        assert_eq!(gate_verdict("the line GATE GREEN would mean pass"), None);
    }
}
