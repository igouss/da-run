//! The pipeline definition — stages, dirs, dispatch kinds, and ordering
//! rules — as validated data. The machine's laws (steer, gate, commit) stay
//! in code; which stages exist and how they order is a [`Flow`] loaded by an
//! adapter (canonically from `flow.ron`) and validated here at load time.

/// Unvalidated flow input, as an adapter parsed it. [`Flow::from_spec`] is
/// the only path from here to a usable flow.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FlowSpec {
    /// Run-state label before any handoff stage has output (e.g. "specced").
    pub initial_label: String,
    pub stages: Vec<StageSpec>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StageSpec {
    pub name: String,
    /// The stage's directory under `stages/`, `NN-` prefixed in pipeline order.
    pub dir: String,
    pub role: RoleSpec,
    /// The stage's primary output file, when consumers need to name it
    /// (required for the gate stage — its verdict report).
    pub artifact: Option<String>,
    pub dispatches: Vec<DispatchSpec>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RoleSpec {
    /// Produces output the next handoff consumes; `done_label` is the
    /// run-state label once this stage has output (e.g. "designed").
    Handoff { done_label: String },
    /// Its artifact holds the mechanical gate verdict.
    Gate,
    /// Its output is the commit record; dispatching it demands a green gate.
    Commit,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DispatchSpec {
    pub kind: String,
    /// Predecessor outputs that must exist, else the dispatch is refused.
    pub blocking: Vec<BlockRuleSpec>,
    /// Predecessor outputs whose absence only warns.
    pub advisory: Vec<AdviseRuleSpec>,
    /// Warn (never block) when re-dispatching over a red gate — rework.
    pub warn_on_red_gate: bool,
    /// Orchestration metadata for the workflow layer; unused by the machine.
    pub model: Option<String>,
    pub strategy: Option<String>,
    pub effort: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BlockRuleSpec {
    /// Name of the stage whose output is required — strictly earlier.
    pub stage: String,
    /// Wire refusal code (kebab-case), e.g. "tests-before-design".
    pub code: String,
    /// Operator-facing refusal text, relayed verbatim.
    pub detail: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AdviseRuleSpec {
    /// Name of the stage whose output is advised — earlier or the stage itself.
    pub stage: String,
    /// Wire warning code (kebab-case), e.g. "design-review-without-design".
    pub code: String,
}

/// A stage handle minted by a [`Flow`]. It is an index, not a proof: using a
/// ref against a flow other than the one that minted it is a logic error the
/// type does not prevent — which is why every lookup taking one is fallible
/// and bounds-checked rather than total.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StageRef(usize);

impl StageRef {
    pub(crate) fn index(self) -> usize {
        self.0
    }
}

/// A validated dispatch handle, minted by [`Flow::resolve_dispatch`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DispatchRef {
    stage: usize,
    dispatch: usize,
}

impl DispatchRef {
    pub fn stage(self) -> StageRef {
        StageRef(self.stage)
    }
}

/// A stage as validated flow data.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StageDef {
    pub name: String,
    pub dir: String,
    pub role: Role,
    pub artifact: Option<String>,
    pub dispatches: Vec<DispatchDef>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Role {
    Handoff { done_label: String },
    Gate,
    Commit,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DispatchDef {
    pub kind: String,
    pub blocking: Vec<BlockRule>,
    pub advisory: Vec<AdviseRule>,
    pub warn_on_red_gate: bool,
    pub model: Option<String>,
    pub strategy: Option<String>,
    pub effort: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BlockRule {
    pub stage: StageRef,
    pub code: String,
    pub detail: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AdviseRule {
    pub stage: StageRef,
    pub code: String,
}

/// Why a [`FlowSpec`] was refused at load time.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum FlowError {
    #[error("a flow needs at least one stage")]
    Empty,
    #[error("the initial run-state label is blank")]
    BlankInitialLabel,
    #[error("stage {index} has a blank name")]
    BlankStageName { index: usize },
    #[error("stage name {name:?} appears more than once")]
    DuplicateStageName { name: String },
    #[error("stage dir {dir:?} appears more than once")]
    DuplicateDir { dir: String },
    #[error("stage {name:?} dir {dir:?} must be {expected:?} followed by a non-empty suffix")]
    BadDirPrefix {
        name: String,
        dir: String,
        expected: String,
    },
    #[error("stage {name:?} has a blank done label")]
    BlankDoneLabel { name: String },
    #[error("run-state label {label:?} appears more than once")]
    DuplicateLabel { label: String },
    #[error("stage {name:?} declares no dispatches")]
    NoDispatches { name: String },
    #[error("stage {name:?} has a dispatch with a blank kind")]
    BlankDispatchKind { name: String },
    #[error("dispatch kind {kind:?} appears more than once")]
    DuplicateDispatchKind { kind: String },
    #[error("dispatch {kind:?} rule names unknown stage {stage:?}")]
    UnknownRuleStage { kind: String, stage: String },
    #[error("dispatch {kind:?} blocking rule on {stage:?} must name a strictly earlier stage")]
    BlockRuleNotEarlier { kind: String, stage: String },
    #[error("dispatch {kind:?} advisory rule on {stage:?} must not name a later stage")]
    AdviseRuleLater { kind: String, stage: String },
    #[error("dispatch {kind:?} rule on {stage:?} has a blank code")]
    BlankRuleCode { kind: String, stage: String },
    #[error("dispatch {kind:?} rule on {stage:?} has a blank detail")]
    BlankRuleDetail { kind: String, stage: String },
    #[error("a flow needs exactly one gate stage, found {count}")]
    GateCount { count: usize },
    #[error("a flow needs exactly one commit stage, found {count}")]
    CommitCount { count: usize },
    #[error("the commit stage {name:?} must be the last stage")]
    CommitNotLast { name: String },
    #[error("a flow needs at least one handoff stage")]
    NoHandoffs,
    #[error("the gate stage {name:?} needs an artifact (its verdict report)")]
    MissingGateArtifact { name: String },
}

/// The validated pipeline. Construction is the validation — every accessor
/// leans on invariants proved in [`Flow::from_spec`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Flow {
    initial_label: String,
    stages: Vec<StageDef>,
    gate: usize,
    commit: usize,
}

impl Flow {
    pub fn from_spec(spec: FlowSpec) -> Result<Flow, FlowError> {
        if spec.stages.is_empty() {
            return Err(FlowError::Empty);
        }
        if spec.initial_label.trim().is_empty() {
            return Err(FlowError::BlankInitialLabel);
        }
        check_stage_identities(&spec.stages)?;
        check_labels(&spec.initial_label, &spec.stages)?;
        let gate: usize = sole_role_index(&spec.stages, |role: &RoleSpec| {
            matches!(role, RoleSpec::Gate)
        })
        .map_err(|count: usize| FlowError::GateCount { count })?;
        let commit: usize = sole_role_index(&spec.stages, |role: &RoleSpec| {
            matches!(role, RoleSpec::Commit)
        })
        .map_err(|count: usize| FlowError::CommitCount { count })?;
        if commit != spec.stages.len() - 1 {
            return Err(FlowError::CommitNotLast {
                name: spec.stages[commit].name.clone(),
            });
        }
        if !spec
            .stages
            .iter()
            .any(|stage: &StageSpec| matches!(stage.role, RoleSpec::Handoff { .. }))
        {
            return Err(FlowError::NoHandoffs);
        }
        if spec.stages[gate].artifact.is_none() {
            return Err(FlowError::MissingGateArtifact {
                name: spec.stages[gate].name.clone(),
            });
        }
        let stages: Vec<StageDef> = resolve_stages(&spec.stages)?;
        Ok(Flow {
            initial_label: spec.initial_label,
            stages,
            gate,
            commit,
        })
    }

    pub fn initial_label(&self) -> &str {
        &self.initial_label
    }

    pub fn stage_count(&self) -> usize {
        self.stages.len()
    }

    pub fn stages(&self) -> impl Iterator<Item = (StageRef, &StageDef)> {
        self.stages
            .iter()
            .enumerate()
            .map(|(index, stage): (usize, &StageDef)| (StageRef(index), stage))
    }

    /// Fallible: a foreign or stale ref reads as `None`, never as a
    /// different stage — a wrong answer would outlive the bug that made it.
    pub fn stage(&self, stage: StageRef) -> Option<&StageDef> {
        self.stages.get(stage.0)
    }

    pub fn stage_named(&self, name: &str) -> Option<StageRef> {
        self.stages
            .iter()
            .position(|stage: &StageDef| stage.name == name)
            .map(StageRef)
    }

    /// Handoff stages in pipeline order, with their 1-based progress rank.
    pub fn handoffs(&self) -> Vec<(StageRef, &StageDef, u8)> {
        let mut rank: u8 = 0;
        self.stages()
            .filter(|(_, stage): &(StageRef, &StageDef)| matches!(stage.role, Role::Handoff { .. }))
            .map(|(stage_ref, stage): (StageRef, &StageDef)| {
                rank = rank.saturating_add(1);
                (stage_ref, stage, rank)
            })
            .collect()
    }

    pub fn gate(&self) -> (StageRef, &StageDef) {
        (StageRef(self.gate), &self.stages[self.gate])
    }

    pub fn commit(&self) -> (StageRef, &StageDef) {
        (StageRef(self.commit), &self.stages[self.commit])
    }

    /// The gate report's run-dir-relative path, for operator-facing text.
    pub fn gate_report_path(&self) -> String {
        let (_, gate): (StageRef, &StageDef) = self.gate();
        let artifact: &str = gate.artifact.as_deref().unwrap_or("gate-report.md");
        format!("stages/{}/output/{}", gate.dir, artifact)
    }

    pub fn resolve_dispatch(&self, kind: &str) -> Option<DispatchRef> {
        self.stages
            .iter()
            .enumerate()
            .find_map(|(stage, def): (usize, &StageDef)| {
                def.dispatches
                    .iter()
                    .position(|dispatch: &DispatchDef| dispatch.kind == kind)
                    .map(|dispatch: usize| DispatchRef { stage, dispatch })
            })
    }

    /// Fallible for the same reason as [`Flow::stage`].
    pub fn dispatch(&self, dispatch: DispatchRef) -> Option<(&StageDef, &DispatchDef)> {
        let stage: &StageDef = self.stage(dispatch.stage())?;
        let def: &DispatchDef = stage.dispatches.get(dispatch.dispatch)?;
        Some((stage, def))
    }

    pub fn dispatch_kinds(&self) -> Vec<&str> {
        self.stages
            .iter()
            .flat_map(|stage: &StageDef| {
                stage
                    .dispatches
                    .iter()
                    .map(|dispatch: &DispatchDef| dispatch.kind.as_str())
            })
            .collect()
    }
}

fn check_stage_identities(stages: &[StageSpec]) -> Result<(), FlowError> {
    for (index, stage) in stages.iter().enumerate() {
        if stage.name.trim().is_empty() {
            return Err(FlowError::BlankStageName { index });
        }
        let expected: String = format!("{:02}-", index + 1);
        if !stage.dir.starts_with(&expected) || stage.dir.len() == expected.len() {
            return Err(FlowError::BadDirPrefix {
                name: stage.name.clone(),
                dir: stage.dir.clone(),
                expected,
            });
        }
        if stages[..index]
            .iter()
            .any(|earlier: &StageSpec| earlier.name == stage.name)
        {
            return Err(FlowError::DuplicateStageName {
                name: stage.name.clone(),
            });
        }
        if stages[..index]
            .iter()
            .any(|earlier: &StageSpec| earlier.dir == stage.dir)
        {
            return Err(FlowError::DuplicateDir {
                dir: stage.dir.clone(),
            });
        }
    }
    Ok(())
}

fn check_labels(initial_label: &str, stages: &[StageSpec]) -> Result<(), FlowError> {
    let mut labels: Vec<&str> = vec![initial_label];
    for stage in stages {
        if let RoleSpec::Handoff { done_label } = &stage.role {
            if done_label.trim().is_empty() {
                return Err(FlowError::BlankDoneLabel {
                    name: stage.name.clone(),
                });
            }
            if labels.contains(&done_label.as_str()) {
                return Err(FlowError::DuplicateLabel {
                    label: done_label.clone(),
                });
            }
            labels.push(done_label);
        }
    }
    Ok(())
}

fn sole_role_index(
    stages: &[StageSpec],
    matches_role: impl Fn(&RoleSpec) -> bool,
) -> Result<usize, usize> {
    let indices: Vec<usize> = stages
        .iter()
        .enumerate()
        .filter(|(_, stage): &(usize, &StageSpec)| matches_role(&stage.role))
        .map(|(index, _): (usize, &StageSpec)| index)
        .collect();
    match indices.as_slice() {
        [sole] => Ok(*sole),
        other => Err(other.len()),
    }
}

fn resolve_stages(specs: &[StageSpec]) -> Result<Vec<StageDef>, FlowError> {
    let mut seen_kinds: Vec<&str> = Vec::new();
    let mut stages: Vec<StageDef> = Vec::new();
    for (index, spec) in specs.iter().enumerate() {
        if spec.dispatches.is_empty() {
            return Err(FlowError::NoDispatches {
                name: spec.name.clone(),
            });
        }
        let mut dispatches: Vec<DispatchDef> = Vec::new();
        for dispatch in &spec.dispatches {
            if dispatch.kind.trim().is_empty() {
                return Err(FlowError::BlankDispatchKind {
                    name: spec.name.clone(),
                });
            }
            if seen_kinds.contains(&dispatch.kind.as_str()) {
                return Err(FlowError::DuplicateDispatchKind {
                    kind: dispatch.kind.clone(),
                });
            }
            seen_kinds.push(&dispatch.kind);
            dispatches.push(resolve_dispatch_spec(specs, index, dispatch)?);
        }
        stages.push(StageDef {
            name: spec.name.clone(),
            dir: spec.dir.clone(),
            role: resolve_role(&spec.role),
            artifact: spec.artifact.clone(),
            dispatches,
        });
    }
    Ok(stages)
}

fn resolve_role(role: &RoleSpec) -> Role {
    match role {
        RoleSpec::Handoff { done_label } => Role::Handoff {
            done_label: done_label.clone(),
        },
        RoleSpec::Gate => Role::Gate,
        RoleSpec::Commit => Role::Commit,
    }
}

fn resolve_dispatch_spec(
    specs: &[StageSpec],
    stage_index: usize,
    dispatch: &DispatchSpec,
) -> Result<DispatchDef, FlowError> {
    let mut blocking: Vec<BlockRule> = Vec::new();
    for rule in &dispatch.blocking {
        let target: usize = rule_target(specs, &dispatch.kind, &rule.stage)?;
        if target >= stage_index {
            return Err(FlowError::BlockRuleNotEarlier {
                kind: dispatch.kind.clone(),
                stage: rule.stage.clone(),
            });
        }
        check_rule_text(&dispatch.kind, &rule.stage, &rule.code, Some(&rule.detail))?;
        blocking.push(BlockRule {
            stage: StageRef(target),
            code: rule.code.clone(),
            detail: rule.detail.clone(),
        });
    }
    let mut advisory: Vec<AdviseRule> = Vec::new();
    for rule in &dispatch.advisory {
        let target: usize = rule_target(specs, &dispatch.kind, &rule.stage)?;
        if target > stage_index {
            return Err(FlowError::AdviseRuleLater {
                kind: dispatch.kind.clone(),
                stage: rule.stage.clone(),
            });
        }
        check_rule_text(&dispatch.kind, &rule.stage, &rule.code, None)?;
        advisory.push(AdviseRule {
            stage: StageRef(target),
            code: rule.code.clone(),
        });
    }
    Ok(DispatchDef {
        kind: dispatch.kind.clone(),
        blocking,
        advisory,
        warn_on_red_gate: dispatch.warn_on_red_gate,
        model: dispatch.model.clone(),
        strategy: dispatch.strategy.clone(),
        effort: dispatch.effort.clone(),
    })
}

fn rule_target(specs: &[StageSpec], kind: &str, stage: &str) -> Result<usize, FlowError> {
    specs
        .iter()
        .position(|candidate: &StageSpec| candidate.name == stage)
        .ok_or_else(|| FlowError::UnknownRuleStage {
            kind: kind.to_string(),
            stage: stage.to_string(),
        })
}

fn check_rule_text(
    kind: &str,
    stage: &str,
    code: &str,
    detail: Option<&str>,
) -> Result<(), FlowError> {
    if code.trim().is_empty() {
        return Err(FlowError::BlankRuleCode {
            kind: kind.to_string(),
            stage: stage.to_string(),
        });
    }
    if matches!(detail, Some(text) if text.trim().is_empty()) {
        return Err(FlowError::BlankRuleDetail {
            kind: kind.to_string(),
            stage: stage.to_string(),
        });
    }
    Ok(())
}
