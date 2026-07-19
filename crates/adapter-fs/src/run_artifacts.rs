//! Collects a run dir's durable ephemera as [`RunArtifact`]s and, on
//! restore, materializes them back — the file half of the mirror bridge.

use da_domain::Flow;
use da_ports::{ArtifactSink, ArtifactSource, RunArtifact, SnapshotError};
use std::path::{Component, Path, PathBuf};

/// Root files pushed with every run (spec.md may be absent on old runs, and
/// worktree.patch until the first stage commits). `worktree.patch` is what
/// makes the mirror sufficient on its own: it carries the run's code, so a
/// restore needs neither the origin host's paths nor a pushed branch.
const ROOT_FILES: [&str; 4] = ["run.edn", "flow.ron", "spec.md", "worktree.patch"];
const GITKEEP: &str = ".gitkeep";

/// Reads a run dir's artifacts: root files plus every stage's output/ files.
pub struct FsArtifactSource;

impl ArtifactSource for FsArtifactSource {
    fn collect(&self, flow: &Flow, run_dir: &Path) -> Result<Vec<RunArtifact>, SnapshotError> {
        let mut files: Vec<RunArtifact> = Vec::new();
        for name in ROOT_FILES {
            push_file(&mut files, run_dir, Path::new(name))?;
        }
        for (_, stage) in flow.stages() {
            let output_rel: PathBuf = Path::new("stages").join(&stage.dir).join("output");
            let output_dir: PathBuf = run_dir.join(&output_rel);
            if !output_dir.is_dir() {
                continue;
            }
            let mut names: Vec<String> = list_files(&output_dir)?;
            names.sort();
            for name in names {
                if name != GITKEEP {
                    push_file(&mut files, run_dir, &output_rel.join(&name))?;
                }
            }
        }
        Ok(files)
    }
}

/// Writes fetched artifacts into a run dir. Paths are refused unless they are
/// plain relative paths — no `..`, no roots, no prefixes — so a hostile or
/// corrupted mirror cannot write outside the target dir.
pub struct FsArtifactSink;

impl ArtifactSink for FsArtifactSink {
    fn materialize(&self, run_dir: &Path, files: &[RunArtifact]) -> Result<(), SnapshotError> {
        for file in files {
            let rel: &Path = Path::new(&file.path);
            if !is_safe_relative(rel) {
                return Err(SnapshotError::Malformed {
                    path: run_dir.to_path_buf(),
                    detail: format!(
                        "mirror artifact path {:?} is not a plain relative path",
                        file.path
                    ),
                });
            }
            let target: PathBuf = run_dir.join(rel);
            if let Some(parent) = target.parent() {
                std::fs::create_dir_all(parent).map_err(|error: std::io::Error| {
                    SnapshotError::Io {
                        path: parent.to_path_buf(),
                        detail: error.to_string(),
                    }
                })?;
            }
            std::fs::write(&target, &file.content).map_err(|error: std::io::Error| {
                SnapshotError::Io {
                    path: target.clone(),
                    detail: error.to_string(),
                }
            })?;
        }
        Ok(())
    }
}

fn is_safe_relative(path: &Path) -> bool {
    !path.as_os_str().is_empty()
        && path
            .components()
            .all(|component: Component| matches!(component, Component::Normal(_)))
}

fn push_file(
    files: &mut Vec<RunArtifact>,
    run_dir: &Path,
    rel: &Path,
) -> Result<(), SnapshotError> {
    let full: PathBuf = run_dir.join(rel);
    if !full.is_file() {
        return Ok(());
    }
    let bytes: Vec<u8> =
        std::fs::read(&full).map_err(|error: std::io::Error| SnapshotError::Io {
            path: full.clone(),
            detail: error.to_string(),
        })?;
    files.push(RunArtifact {
        path: rel.to_string_lossy().replace('\\', "/"),
        content: String::from_utf8_lossy(&bytes).into_owned(),
    });
    Ok(())
}

fn list_files(dir: &Path) -> Result<Vec<String>, SnapshotError> {
    let entries = std::fs::read_dir(dir).map_err(|error: std::io::Error| SnapshotError::Io {
        path: dir.to_path_buf(),
        detail: error.to_string(),
    })?;
    let mut names: Vec<String> = Vec::new();
    for entry in entries {
        let entry: std::fs::DirEntry =
            entry.map_err(|error: std::io::Error| SnapshotError::Io {
                path: dir.to_path_buf(),
                detail: error.to_string(),
            })?;
        if entry.path().is_file() {
            names.push(entry.file_name().to_string_lossy().into_owned());
        }
    }
    Ok(names)
}
