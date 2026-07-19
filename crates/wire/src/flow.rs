use crate::WIRE_VERSION;
use da_domain::{DispatchDef, Flow, Role, StageDef, StageRef};
use serde::{Deserialize, Serialize};

/// The `flow` output: the validated pipeline definition, for consumers that
/// cannot parse RON themselves (bb scripts, workflow JS).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FlowWire {
    pub v: u32,
    pub initial_label: String,
    pub stages: Vec<FlowStageWire>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FlowStageWire {
    pub name: String,
    pub dir: String,
    /// "handoff" | "gate" | "commit"
    pub role: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub done_label: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub artifact: Option<String>,
    pub dispatches: Vec<FlowDispatchWire>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FlowDispatchWire {
    pub kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub strategy: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effort: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub design_from: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tests_from: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub judge_reference: Option<String>,
}

impl FlowWire {
    pub fn from_domain(flow: &Flow) -> FlowWire {
        FlowWire {
            v: WIRE_VERSION,
            initial_label: flow.initial_label().to_string(),
            stages: flow
                .stages()
                .map(|(_, stage): (StageRef, &StageDef)| stage_wire(stage))
                .collect(),
        }
    }
}

fn stage_wire(stage: &StageDef) -> FlowStageWire {
    let (role, done_label): (&str, Option<String>) = match &stage.role {
        Role::Handoff { done_label } => ("handoff", Some(done_label.clone())),
        Role::Gate => ("gate", None),
        Role::Commit => ("commit", None),
    };
    FlowStageWire {
        name: stage.name.clone(),
        dir: stage.dir.clone(),
        role: role.to_string(),
        done_label,
        artifact: stage.artifact.clone(),
        dispatches: stage
            .dispatches
            .iter()
            .map(|dispatch: &DispatchDef| FlowDispatchWire {
                kind: dispatch.kind.clone(),
                model: dispatch.model.clone(),
                strategy: dispatch.strategy.clone(),
                effort: dispatch.effort.clone(),
                design_from: dispatch.design_from.clone(),
                tests_from: dispatch.tests_from.clone(),
                judge_reference: dispatch.judge_reference.clone(),
            })
            .collect(),
    }
}
