use crate::WIRE_VERSION;
use da_domain::{Allowed, Refusal, Verdict, Warning};
use serde::{Deserialize, Serialize};

/// The `check` output: allowed with warnings, or refused with a typed reason.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CheckWire {
    pub v: u32,
    pub allowed: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<WarningWire>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<ReasonWire>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WarningWire {
    pub code: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stage: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReasonWire {
    pub code: String,
    /// Human-readable relay text, straight from the domain's Display.
    pub detail: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gate: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub stages: Vec<String>,
}

impl CheckWire {
    pub fn allowed(allowed: &Allowed) -> CheckWire {
        CheckWire {
            v: WIRE_VERSION,
            allowed: true,
            warnings: allowed
                .warnings
                .iter()
                .map(|warning: &Warning| warning_wire(warning))
                .collect(),
            reason: None,
        }
    }

    pub fn refused(refusal: &Refusal) -> CheckWire {
        CheckWire {
            v: WIRE_VERSION,
            allowed: false,
            warnings: Vec::new(),
            reason: Some(reason_wire(refusal)),
        }
    }
}

fn warning_wire(warning: &Warning) -> WarningWire {
    match warning {
        Warning::Advisory { code } => WarningWire {
            code: code.clone(),
            stage: None,
        },
        Warning::StageAlreadyComplete { stage } => WarningWire {
            code: "stage-already-complete".to_string(),
            stage: Some(stage.clone()),
        },
        Warning::RedGateRework => WarningWire {
            code: "red-gate-rework".to_string(),
            stage: None,
        },
        _ => WarningWire {
            code: "unknown".to_string(),
            stage: None,
        },
    }
}

fn reason_wire(refusal: &Refusal) -> ReasonWire {
    let detail: String = refusal.to_string();
    match refusal {
        Refusal::OrderingViolation { code, .. } => ReasonWire {
            code: code.clone(),
            detail,
            gate: None,
            stages: Vec::new(),
        },
        Refusal::CommitBeforeGreenGate { gate, .. } => ReasonWire {
            code: "commit-before-green-gate".to_string(),
            detail,
            gate: gate.map(|verdict: Verdict| crate::derived::verdict_str(verdict).to_string()),
            stages: Vec::new(),
        },
        Refusal::SteerPending { stages } => ReasonWire {
            code: "steer-pending".to_string(),
            detail,
            gate: None,
            stages: stages.clone(),
        },
        Refusal::WorktreeAbsent => ReasonWire {
            code: "worktree-absent".to_string(),
            detail,
            gate: None,
            stages: Vec::new(),
        },
        Refusal::WorktreeEmpty => ReasonWire {
            code: "worktree-empty".to_string(),
            detail,
            gate: None,
            stages: Vec::new(),
        },
        Refusal::WorktreeMovedSinceGate { .. } => ReasonWire {
            code: "worktree-moved-since-gate".to_string(),
            detail,
            gate: None,
            stages: Vec::new(),
        },
        _ => ReasonWire {
            code: "unknown".to_string(),
            detail,
            gate: None,
            stages: Vec::new(),
        },
    }
}
