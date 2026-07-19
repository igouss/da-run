//! RunId construction: the id travels raw into mirror URLs and workflow
//! keys, so the character set is contract.

#![allow(clippy::expect_used)]

use da_domain::{RunId, RunIdError};

#[test]
fn a_blank_id_is_refused() {
    assert!(matches!(RunId::new("   "), Err(RunIdError::Blank)));
}

#[test]
fn a_millis_project_id_is_accepted() {
    let id: RunId = RunId::new("1752888000123-my_project.v2").expect("url-safe id");
    assert_eq!(id.as_str(), "1752888000123-my_project.v2");
}

#[test]
fn surrounding_whitespace_is_trimmed() {
    let id: RunId = RunId::new(" 42-proj \n").expect("trimmed id");
    assert_eq!(id.as_str(), "42-proj");
}

#[test]
fn a_slash_is_refused_and_named() {
    match RunId::new("42/proj") {
        Err(RunIdError::ForbiddenCharacter { offender, .. }) => assert_eq!(offender, '/'),
        other => panic!("expected ForbiddenCharacter, got {other:?}"),
    }
}

#[test]
fn a_space_inside_is_refused() {
    assert!(matches!(
        RunId::new("42 proj"),
        Err(RunIdError::ForbiddenCharacter { offender: ' ', .. })
    ));
}

#[test]
fn a_url_reserved_character_is_refused() {
    assert!(matches!(
        RunId::new("42%2Fproj"),
        Err(RunIdError::ForbiddenCharacter { offender: '%', .. })
    ));
}
