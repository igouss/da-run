use da_wire::{StageWire, StatusWire};

/// A human render of the status wire — stderr only, never parsed.
pub fn render_status(wire: &StatusWire) -> String {
    let mut lines: Vec<String> = Vec::new();
    lines.push(format!(
        "run {} — {} ({})",
        wire.run_id, wire.state, wire.phase
    ));
    for stage in &wire.stages {
        lines.push(stage_line(stage));
    }
    lines.push(format!(
        "gate: {}",
        wire.gate.as_deref().unwrap_or("no verdict")
    ));
    if !wire.parked.is_empty() {
        lines.push(format!(
            "parked (steer pending): {}",
            wire.parked.join(", ")
        ));
    }
    for anomaly in &wire.anomalies {
        lines.push(format!(
            "anomaly: {} has output but {} is empty",
            anomaly.later, anomaly.earlier
        ));
    }
    lines.join("\n")
}

fn stage_line(stage: &StageWire) -> String {
    let mark: &str = if stage.complete {
        "COMPLETE"
    } else {
        "PENDING"
    };
    let files: String = if stage.files.is_empty() {
        String::new()
    } else {
        format!("  ({})", stage.files.join(", "))
    };
    let steer: &str = if stage.steer_pending {
        "  [steer pending]"
    } else {
        ""
    };
    format!("  {:<13} {mark}{files}{steer}", stage.stage)
}
