use crate::derived::DerivedWire;
use da_ports::{MirrorSnapshot, RunArtifact};
use serde::{Deserialize, Serialize};

/// One artifact file on the wire — a `recordSnapshot` payload element and
/// the `getSnapshot` file entry.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtifactWire {
    pub path: String,
    pub content: String,
}

/// The `recordSnapshot` request body: derived state and the run's full
/// artifact set in ONE call, so the mirror can never advertise a state its
/// artifacts do not support (full-replace semantics — the run dir is
/// canonical, the mirror follows).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunSnapshotWire {
    pub state: DerivedWire,
    pub files: Vec<ArtifactWire>,
}

/// The `getSnapshot` response: the last recorded state (opaque here — a
/// tolerant reader survives a newer producer) and the artifact set.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MirrorSnapshotWire {
    #[serde(default)]
    pub state: Option<serde_json::Value>,
    #[serde(default)]
    pub files: Vec<ArtifactWire>,
}

impl RunSnapshotWire {
    pub fn from_parts(state: DerivedWire, files: &[RunArtifact]) -> RunSnapshotWire {
        RunSnapshotWire {
            state,
            files: files
                .iter()
                .map(|file: &RunArtifact| ArtifactWire {
                    path: file.path.clone(),
                    content: file.content.clone(),
                })
                .collect(),
        }
    }
}

impl MirrorSnapshotWire {
    pub fn into_ports(self) -> MirrorSnapshot {
        MirrorSnapshot {
            state_json: self.state.map(|state: serde_json::Value| state.to_string()),
            files: self
                .files
                .into_iter()
                .map(|file: ArtifactWire| RunArtifact {
                    path: file.path,
                    content: file.content,
                })
                .collect(),
        }
    }
}
