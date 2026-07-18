use crate::WIRE_VERSION;
use crate::derived::{DerivedWire, verdict_str};
use da_app::{StageStatus, StatusReport};
use da_domain::Verdict;
use serde::{Deserialize, Serialize};

/// The `status` output: the derived summary plus per-stage detail.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StatusWire {
    pub v: u32,
    pub run_id: String,
    pub state: String,
    pub phase: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gate: Option<String>,
    pub parked: Vec<String>,
    pub stages: Vec<StageWire>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub anomalies: Vec<crate::derived::AnomalyWire>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StageWire {
    pub stage: String,
    pub complete: bool,
    pub files: Vec<String>,
    pub steer_pending: bool,
}

impl StatusWire {
    pub fn from_report(report: &StatusReport) -> StatusWire {
        let derived: DerivedWire = DerivedWire::from_domain(&report.run_id, &report.derived);
        StatusWire {
            v: WIRE_VERSION,
            run_id: derived.run_id,
            state: derived.state,
            phase: derived.phase,
            gate: report
                .gate
                .map(|verdict: Verdict| verdict_str(verdict).to_string()),
            parked: derived.parked,
            stages: report
                .stages
                .iter()
                .map(|stage: &StageStatus| StageWire {
                    stage: stage.stage.dir_name().to_string(),
                    complete: stage.complete,
                    files: stage.files.clone(),
                    steer_pending: stage.steer_pending,
                })
                .collect(),
            anomalies: derived.anomalies,
        }
    }
}
