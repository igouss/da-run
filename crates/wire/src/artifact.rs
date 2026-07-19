use da_ports::{MirrorSnapshot, RunArtifact};
use serde::{Deserialize, Serialize};

/// One artifact file on the wire — the `recordArtifacts` payload element and
/// the `getSnapshot` file entry.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtifactWire {
    pub path: String,
    pub content: String,
}

/// The `recordArtifacts` request body: the run's full artifact set
/// (full-replace semantics — the run dir is canonical, the mirror follows).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtifactsWire {
    pub files: Vec<ArtifactWire>,
}

/// The `getSnapshot` response: the last recorded state (opaque here) and the
/// artifact set.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MirrorSnapshotWire {
    #[serde(default)]
    pub state: Option<serde_json::Value>,
    #[serde(default)]
    pub files: Vec<ArtifactWire>,
}

impl ArtifactsWire {
    pub fn from_ports(files: &[RunArtifact]) -> ArtifactsWire {
        ArtifactsWire {
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
