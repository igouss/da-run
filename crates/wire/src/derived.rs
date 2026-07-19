use crate::WIRE_VERSION;
use da_domain::{Anomaly, Derived, Phase, RunId, Verdict};
use serde::{Deserialize, Serialize};

/// A run's derived state — the `derive` output and the mirror payload.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DerivedWire {
    pub v: u32,
    pub run_id: String,
    pub state: String,
    pub phase: String,
    pub parked: Vec<String>,
    pub anomalies: Vec<AnomalyWire>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnomalyWire {
    pub code: String,
    pub later: String,
    pub earlier: String,
}

impl DerivedWire {
    pub fn from_domain(run_id: &RunId, derived: &Derived) -> DerivedWire {
        DerivedWire {
            v: WIRE_VERSION,
            run_id: run_id.as_str().to_string(),
            state: derived.state.label(),
            phase: phase_str(derived.phase).to_string(),
            parked: derived.parked.clone(),
            anomalies: derived
                .anomalies
                .iter()
                .map(|anomaly: &Anomaly| anomaly_wire(anomaly))
                .collect(),
        }
    }
}

pub(crate) fn phase_str(phase: Phase) -> &'static str {
    match phase {
        Phase::Convergence => "convergence",
        Phase::SteadyState => "steady-state",
    }
}

pub(crate) fn verdict_str(verdict: Verdict) -> &'static str {
    match verdict {
        Verdict::Green => "green",
        Verdict::Red => "red",
    }
}

fn anomaly_wire(anomaly: &Anomaly) -> AnomalyWire {
    match anomaly {
        Anomaly::LaterOutputWithoutEarlier { later, earlier } => AnomalyWire {
            code: "later-output-without-earlier".to_string(),
            later: later.clone(),
            earlier: earlier.clone(),
        },
        _ => AnomalyWire {
            code: "unknown".to_string(),
            later: String::new(),
            earlier: String::new(),
        },
    }
}
