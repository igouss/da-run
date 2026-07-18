use crate::derive::pending_steers;
use crate::dispatch::Dispatch;
use crate::facts::FsFacts;
use crate::phase::Phase;
use crate::refusal::Refusal;
use crate::stage::StageId;
use crate::verdict::Verdict;
use crate::warning::Warning;

/// A dispatch the machine allows, with advisory warnings.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Allowed {
    pub warnings: Vec<Warning>,
}

/// Proof that the commit precondition held: gate green, no pending steer.
/// The private field makes it constructible only by [`commit_precondition`] —
/// no path reports "commit allowed" without one.
#[derive(Debug)]
pub struct GateGreenProof(());

/// Decide a dispatch against the facts. Rules mirror SKILL.md's ordering
/// guards plus the steer law; warnings never block.
pub fn check(facts: &FsFacts, dispatch: &Dispatch) -> Result<Allowed, Refusal> {
    let parked: Vec<StageId> = pending_steers(facts);
    if !parked.is_empty() {
        return Err(Refusal::SteerPending { stages: parked });
    }
    let mut warnings: Vec<Warning> = Vec::new();
    match dispatch {
        Dispatch::Design => {}
        Dispatch::DesignReview => {
            if !facts.stages.get(StageId::Design).has_output() {
                warnings.push(Warning::DesignReviewWithoutDesign);
            }
        }
        Dispatch::Tests => {
            if !facts.stages.get(StageId::Design).has_output() {
                return Err(Refusal::TestsBeforeDesign);
            }
        }
        Dispatch::Implement { .. } => {
            if !facts.stages.get(StageId::Tests).has_output() {
                return Err(Refusal::ImplementBeforeTests);
            }
            if facts.gate == Some(Verdict::Red) {
                warnings.push(Warning::RedGateRework);
            }
        }
        Dispatch::Verify => {
            if !facts.stages.get(StageId::Implement).has_output() {
                warnings.push(Warning::VerifyWithoutImplementation);
            }
        }
        Dispatch::Commit => {
            let _proof: GateGreenProof = commit_precondition(facts)?;
        }
    }
    if facts.phase == Phase::SteadyState && facts.stages.get(dispatch.stage()).has_output() {
        warnings.push(Warning::StageAlreadyComplete {
            stage: dispatch.stage(),
        });
    }
    Ok(Allowed { warnings })
}

/// The commit precondition: no pending steer and a green gate. Absent or
/// unparseable gate report fails closed.
pub fn commit_precondition(facts: &FsFacts) -> Result<GateGreenProof, Refusal> {
    let parked: Vec<StageId> = pending_steers(facts);
    if !parked.is_empty() {
        return Err(Refusal::SteerPending { stages: parked });
    }
    match facts.gate {
        Some(Verdict::Green) => Ok(GateGreenProof(())),
        gate => Err(Refusal::CommitBeforeGreenGate { gate }),
    }
}
