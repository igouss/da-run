use crate::flow::{Flow, StageRef};
use crate::phase::Phase;
use crate::verdict::Verdict;
use crate::worktree::{WorktreeFacts, WorktreeId};

/// A run's identifier from `run.edn` `:run-id`. Never blank.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RunId(String);

/// Refused construction of a blank [`RunId`].
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[error("a run id must hold real text")]
pub struct BlankRunId;

impl RunId {
    pub fn new(raw: &str) -> Result<RunId, BlankRunId> {
        let trimmed: &str = raw.trim();
        if trimmed.is_empty() {
            Err(BlankRunId)
        } else {
            Ok(RunId(trimmed.to_string()))
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// What a stage's `STEER-REQUEST.md` says. `answered` mirrors bin/steer's
/// rule: the `## Answer` section holds real (non-blank) text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SteerFacts {
    pub answered: bool,
}

/// One stage's observable facts.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StageFacts {
    /// Files in the stage's `output/` beyond `.gitkeep` (and any steer file).
    pub output_files: Vec<String>,
    /// Present when the stage wrote a `STEER-REQUEST.md`.
    pub steer: Option<SteerFacts>,
}

impl StageFacts {
    /// A stage with nothing observed — also the total-lookup fallback.
    pub const EMPTY: StageFacts = StageFacts {
        output_files: Vec::new(),
        steer: None,
    };

    pub fn empty() -> StageFacts {
        StageFacts::EMPTY
    }

    pub fn has_output(&self) -> bool {
        !self.output_files.is_empty()
    }

    pub fn steer_pending(&self) -> bool {
        matches!(&self.steer, Some(steer) if !steer.answered)
    }
}

/// Per-stage facts, aligned with a [`Flow`]'s stage order and total by
/// construction — a lookup never fails, a foreign ref reads as empty.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StageFactsMap {
    entries: Vec<StageFacts>,
}

impl StageFactsMap {
    pub fn from_fn(
        flow: &Flow,
        mut facts_for: impl FnMut(StageRef) -> StageFacts,
    ) -> StageFactsMap {
        StageFactsMap {
            entries: flow
                .stages()
                .map(|(stage, _): (StageRef, &crate::flow::StageDef)| facts_for(stage))
                .collect(),
        }
    }

    pub fn get(&self, stage: StageRef) -> &StageFacts {
        static EMPTY: StageFacts = StageFacts::EMPTY;
        self.entries.get(stage.index()).unwrap_or(&EMPTY)
    }
}

/// The refined snapshot of a run dir. Adapters parse; the domain never
/// re-checks strings.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FsFacts {
    pub stages: StageFactsMap,
    /// `None` when the gate report is absent or its verdict line is
    /// unparseable — commit fails closed on `None`.
    pub gate: Option<Verdict>,
    /// The commit stage's `output/` holds a commit record.
    pub commit_recorded: bool,
    /// The worktree's code as it stands now. `None` when the run dir carries
    /// no `worktree.patch` — commit fails closed on `None`.
    pub worktree: Option<WorktreeFacts>,
    /// The worktree identity the gate report says it verified. `None` when
    /// the report records none — commit fails closed on `None`.
    pub gate_worktree: Option<WorktreeId>,
    pub phase: Phase,
    pub run_id: RunId,
}
