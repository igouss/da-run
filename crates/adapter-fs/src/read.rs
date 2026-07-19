//! The one file-reading policy for everything this adapter touches: run
//! files and artifacts are text by contract, and a non-UTF-8 byte is a loud
//! typed error naming the file — never a silent U+FFFD substitution. The
//! lossy path da-run shipped corrupted content in the durability layer, the
//! one place whose whole job is returning exactly what it was given.

use da_ports::SnapshotError;
use std::path::Path;

/// Read a file as UTF-8, refusing invalid bytes with the path named.
pub(crate) fn read_utf8(path: &Path) -> Result<String, SnapshotError> {
    let bytes: Vec<u8> = std::fs::read(path).map_err(|error: std::io::Error| SnapshotError::Io {
        path: path.to_path_buf(),
        detail: error.to_string(),
    })?;
    String::from_utf8(bytes).map_err(|error: std::string::FromUtf8Error| {
        SnapshotError::Malformed {
            path: path.to_path_buf(),
            detail: format!(
                "not valid UTF-8 at byte {} — run files and artifacts are text by contract",
                error.utf8_error().valid_up_to()
            ),
        }
    })
}
