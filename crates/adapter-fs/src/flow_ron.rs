//! Reads `flow.ron` — the pipeline definition — and refines it through the
//! domain's [`Flow::from_spec`] validation. Parsing is strict: an unknown
//! field is a load error, not a shrug.

use da_domain::{
    AdviseRuleSpec, BlockRuleSpec, DispatchSpec, Flow, FlowError, FlowSpec, RoleSpec, StageSpec,
};
use serde::Deserialize;
use std::path::{Path, PathBuf};

/// The flow file's name inside a run dir (and the algorithm workspace).
pub const FLOW_FILE: &str = "flow.ron";

/// Why a flow file could not become a [`Flow`].
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum FlowLoadError {
    #[error("{path}: {detail}")]
    Io { path: PathBuf, detail: String },
    #[error("{path}: flow parse failed: {detail}")]
    Parse { path: PathBuf, detail: String },
    #[error("{path}: invalid flow: {source}")]
    Invalid { path: PathBuf, source: FlowError },
}

/// Load and validate the flow at `path`.
pub fn load_flow_file(path: &Path) -> Result<Flow, FlowLoadError> {
    let text: String =
        std::fs::read_to_string(path).map_err(|error: std::io::Error| FlowLoadError::Io {
            path: path.to_path_buf(),
            detail: error.to_string(),
        })?;
    let raw: FlowRon = options()
        .from_str(&text)
        .map_err(|error: ron::error::SpannedError| FlowLoadError::Parse {
            path: path.to_path_buf(),
            detail: error.to_string(),
        })?;
    Flow::from_spec(flow_spec(raw)).map_err(|source: FlowError| FlowLoadError::Invalid {
        path: path.to_path_buf(),
        source,
    })
}

/// Load and validate a run dir's `flow.ron`.
pub fn load_run_flow(run_dir: &Path) -> Result<Flow, FlowLoadError> {
    load_flow_file(&run_dir.join(FLOW_FILE))
}

fn options() -> ron::Options {
    ron::Options::default().with_default_extension(ron::extensions::Extensions::IMPLICIT_SOME)
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename = "Flow")]
struct FlowRon {
    initial_label: String,
    stages: Vec<StageRon>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename = "Stage")]
struct StageRon {
    name: String,
    dir: String,
    role: RoleRon,
    #[serde(default)]
    artifact: Option<String>,
    dispatches: Vec<DispatchRon>,
}

#[derive(Deserialize)]
enum RoleRon {
    Handoff { done_label: String },
    Gate,
    Commit,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename = "Dispatch")]
struct DispatchRon {
    kind: String,
    #[serde(default)]
    blocking: Vec<BlockRon>,
    #[serde(default)]
    advisory: Vec<AdviseRon>,
    #[serde(default)]
    warn_on_red_gate: bool,
    #[serde(default)]
    model: Option<String>,
    #[serde(default)]
    strategy: Option<String>,
    #[serde(default)]
    effort: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename = "Needs")]
struct BlockRon {
    stage: String,
    code: String,
    detail: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename = "Needs")]
struct AdviseRon {
    stage: String,
    code: String,
}

fn flow_spec(raw: FlowRon) -> FlowSpec {
    FlowSpec {
        initial_label: raw.initial_label,
        stages: raw.stages.into_iter().map(stage_spec).collect(),
    }
}

fn stage_spec(raw: StageRon) -> StageSpec {
    StageSpec {
        name: raw.name,
        dir: raw.dir,
        role: match raw.role {
            RoleRon::Handoff { done_label } => RoleSpec::Handoff { done_label },
            RoleRon::Gate => RoleSpec::Gate,
            RoleRon::Commit => RoleSpec::Commit,
        },
        artifact: raw.artifact,
        dispatches: raw.dispatches.into_iter().map(dispatch_spec).collect(),
    }
}

fn dispatch_spec(raw: DispatchRon) -> DispatchSpec {
    DispatchSpec {
        kind: raw.kind,
        blocking: raw
            .blocking
            .into_iter()
            .map(|rule: BlockRon| BlockRuleSpec {
                stage: rule.stage,
                code: rule.code,
                detail: rule.detail,
            })
            .collect(),
        advisory: raw
            .advisory
            .into_iter()
            .map(|rule: AdviseRon| AdviseRuleSpec {
                stage: rule.stage,
                code: rule.code,
            })
            .collect(),
        warn_on_red_gate: raw.warn_on_red_gate,
        model: raw.model,
        strategy: raw.strategy,
        effort: raw.effort,
    }
}
