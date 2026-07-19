//! Reads the run dir's `worktree.patch` into [`WorktreeFacts`].
//!
//! The patch is the run's code made durable: `git diff --binary <base>` over
//! a worktree whose stages have each been committed, so it carries new files
//! as well as edits. Its sha256 is the worktree's identity — stable across a
//! restore onto another host, where the commits themselves are not.

use da_domain::{WorktreeFacts, WorktreeId};
use sha2::{Digest, Sha256};

/// The run-dir-relative path of the worktree patch.
pub const WORKTREE_PATCH: &str = "worktree.patch";

/// Refines a patch's contents into worktree facts. A patch with no diff body
/// is `empty` — the worktree still matches the base commit.
pub fn worktree_facts(patch: &str) -> Option<WorktreeFacts> {
    let id: WorktreeId = WorktreeId::new(&digest(patch)).ok()?;
    Some(WorktreeFacts {
        id,
        empty: patch.trim().is_empty(),
    })
}

/// The sha256 of the patch's bytes, lowercase hex — the same digest
/// `sha256sum` and node's `crypto` produce, so the orchestrator can record a
/// matching identity into the gate report.
fn digest(patch: &str) -> String {
    let mut hasher: Sha256 = Sha256::new();
    hasher.update(patch.as_bytes());
    hasher
        .finalize()
        .iter()
        .map(|byte: &u8| format!("{byte:02x}"))
        .collect()
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use super::{digest, worktree_facts};

    #[test]
    fn a_patch_with_a_diff_is_not_empty() {
        let facts = worktree_facts("diff --git a/a.rs b/a.rs\n+fn main() {}\n")
            .expect("a non-blank digest is a valid id");
        assert!(!facts.empty);
    }

    #[test]
    fn an_empty_patch_is_empty() {
        let facts = worktree_facts("").expect("a non-blank digest is a valid id");
        assert!(facts.empty);
    }

    #[test]
    fn whitespace_only_patch_is_empty() {
        let facts = worktree_facts("\n  \n").expect("a non-blank digest is a valid id");
        assert!(facts.empty);
    }

    #[test]
    fn a_changed_patch_changes_the_identity() {
        let before = worktree_facts("diff --git a/a.rs b/a.rs\n+one\n").expect("valid id");
        let after = worktree_facts("diff --git a/a.rs b/a.rs\n+two\n").expect("valid id");
        assert_ne!(before.id, after.id);
    }

    #[test]
    fn the_same_patch_keeps_its_identity() {
        let once = worktree_facts("diff --git a/a.rs b/a.rs\n+one\n").expect("valid id");
        let again = worktree_facts("diff --git a/a.rs b/a.rs\n+one\n").expect("valid id");
        assert_eq!(once.id, again.id);
    }

    /// Parity with `sha256sum` / node `crypto.createHash('sha256')`: the
    /// orchestrator writes the gate's identity, this crate reads it back.
    #[test]
    fn digest_matches_the_known_sha256_of_abc() {
        assert_eq!(
            digest("abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }
}
