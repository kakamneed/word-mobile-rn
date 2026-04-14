use std::path::Path;

/// Evaluates whether the bundled vocabulary snapshot resource is present
/// and readable.
pub fn check_bundled_snapshot(snapshot_path: &Path) -> SnapshotResult {
    if !snapshot_path.exists() {
        return SnapshotResult {
            exists: false,
            status: "missing_required".to_string(),
        };
    }

    match std::fs::metadata(snapshot_path) {
        Ok(meta) if meta.len() > 0 => SnapshotResult {
            exists: true,
            status: "ready".to_string(),
        },
        Ok(_) => SnapshotResult {
            exists: true,
            status: "empty".to_string(),
        },
        Err(_) => SnapshotResult {
            exists: false,
            status: "missing_required".to_string(),
        },
    }
}

/// Result of a bundled snapshot presence check.
#[derive(Debug, Clone)]
pub struct SnapshotResult {
    /// Whether the snapshot file exists and has content.
    pub exists: bool,

    /// Human-readable status: "ready" or "missing_required".
    pub status: String,
}
