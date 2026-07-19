//! The worktree's content identity.
//!
//! The run's code does not live in the run dir — it lives in a git worktree
//! the stages mutate. For the mirror to restore a run on another host, and
//! for the gate to mean anything, the machine needs a stable name for "the
//! code as it stood at this moment".
//!
//! That name is the digest of the run's `worktree.patch` — the full diff from
//! the run's base commit. Deliberately NOT the worktree's HEAD sha: restoring
//! on another host re-applies the patch and produces different commit shas,
//! so a HEAD-based identity would refuse every restored run. The patch's
//! content survives the move; its commits do not.

/// A worktree's content identity: the digest of its patch from the base
/// commit. Opaque to the domain — the adapter chooses the digest.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorktreeId(String);

/// Refused construction of a blank [`WorktreeId`].
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[error("a worktree id must hold real text")]
pub struct BlankWorktreeId;

impl WorktreeId {
    pub fn new(raw: &str) -> Result<WorktreeId, BlankWorktreeId> {
        let trimmed: &str = raw.trim();
        if trimmed.is_empty() {
            Err(BlankWorktreeId)
        } else {
            Ok(WorktreeId(trimmed.to_string()))
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for WorktreeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// What the run dir says about the worktree's code.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorktreeFacts {
    pub id: WorktreeId,
    /// The patch carries no change — the worktree still matches the base
    /// commit, so the run has produced no code.
    pub empty: bool,
}
