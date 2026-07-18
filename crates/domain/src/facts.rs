use crate::phase::Phase;
use crate::stage::StageId;
use crate::verdict::Verdict;

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
    pub fn empty() -> StageFacts {
        StageFacts {
            output_files: Vec::new(),
            steer: None,
        }
    }

    pub fn has_output(&self) -> bool {
        !self.output_files.is_empty()
    }

    pub fn steer_pending(&self) -> bool {
        matches!(&self.steer, Some(steer) if !steer.answered)
    }
}

/// Per-stage facts, total over [`StageId`] by construction.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StageFactsMap {
    design: StageFacts,
    tests: StageFacts,
    implement: StageFacts,
    verify: StageFacts,
    commit: StageFacts,
}

impl StageFactsMap {
    pub fn from_fn(mut facts_for: impl FnMut(StageId) -> StageFacts) -> StageFactsMap {
        StageFactsMap {
            design: facts_for(StageId::Design),
            tests: facts_for(StageId::Tests),
            implement: facts_for(StageId::Implement),
            verify: facts_for(StageId::Verify),
            commit: facts_for(StageId::Commit),
        }
    }

    pub fn get(&self, id: StageId) -> &StageFacts {
        match id {
            StageId::Design => &self.design,
            StageId::Tests => &self.tests,
            StageId::Implement => &self.implement,
            StageId::Verify => &self.verify,
            StageId::Commit => &self.commit,
        }
    }
}

/// The refined snapshot of a run dir. Adapters parse; the domain never
/// re-checks strings.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FsFacts {
    pub stages: StageFactsMap,
    /// `None` when `gate-report.md` is absent or its verdict line is
    /// unparseable — commit fails closed on `None`.
    pub gate: Option<Verdict>,
    /// `stages/05-commit/output/` holds a commit record.
    pub commit_recorded: bool,
    pub phase: Phase,
    pub run_id: RunId,
}
