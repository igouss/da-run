//! Snapshot tests pin the published JSON shapes; tolerant-reader tests prove
//! a consumer survives a producer that added fields.

#![allow(clippy::unwrap_used)]

use da_domain::{
    Allowed, Anomaly, Derived, Phase, Refusal, RunId, RunState, StageId, Verdict, Warning,
};
use da_wire::{CheckWire, DerivedWire};

fn sample_derived() -> Derived {
    Derived {
        state: RunState::Gated(Verdict::Red),
        parked: vec![StageId::Tests],
        phase: Phase::Convergence,
        anomalies: vec![Anomaly::LaterOutputWithoutEarlier {
            later: StageId::Implement,
            earlier: StageId::Tests,
        }],
    }
}

#[test]
fn derived_wire_shape_is_pinned() {
    let run_id: RunId = RunId::new("250718-widget").unwrap();
    let wire: DerivedWire = DerivedWire::from_domain(&run_id, &sample_derived());
    insta::assert_json_snapshot!(wire);
}

#[test]
fn check_allowed_shape_is_pinned() {
    let allowed: Allowed = Allowed {
        warnings: vec![Warning::StageAlreadyComplete {
            stage: StageId::Design,
        }],
    };
    insta::assert_json_snapshot!(CheckWire::allowed(&allowed));
}

#[test]
fn check_refused_steer_shape_is_pinned() {
    let refusal: Refusal = Refusal::SteerPending {
        stages: vec![StageId::Tests, StageId::Implement],
    };
    insta::assert_json_snapshot!(CheckWire::refused(&refusal));
}

#[test]
fn check_refused_gate_shape_is_pinned() {
    let refusal: Refusal = Refusal::CommitBeforeGreenGate {
        gate: Some(Verdict::Red),
    };
    insta::assert_json_snapshot!(CheckWire::refused(&refusal));
}

// Tolerant reader: a payload with extra fields still parses.
#[test]
fn check_wire_reader_tolerates_added_fields() {
    let payload: &str = r#"{"v":1,"allowed":false,"reason":{"code":"steer-pending","detail":"x","stages":["02-tests"],"novel_field":true},"another_novel":42}"#;
    let parsed: CheckWire = serde_json::from_str(payload).unwrap();
    assert!(!parsed.allowed);
    assert_eq!(parsed.reason.unwrap().code, "steer-pending");
}

#[test]
fn derived_wire_reader_tolerates_added_fields() {
    let payload: &str = r#"{"v":2,"run_id":"r","state":"future-state","phase":"steady-state","parked":[],"anomalies":[],"added_later":"yes"}"#;
    let parsed: DerivedWire = serde_json::from_str(payload).unwrap();
    assert_eq!(parsed.state, "future-state");
}
