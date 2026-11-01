//! Project target descriptors: where the analyzed bytes live.
//!
//! A `.damsel` project directory carries a `target.json` beside
//! `project.db`. Commands that need bytes back (decompile, string values,
//! value rendering) read it to re-materialize the analyzed image instead of
//! inferring a path from the directory name.
//!
//! The descriptor is environment state, not analysis state: it is
//! deliberately excluded from canonical exports because cache and binary
//! paths are machine-specific and would break byte determinism across
//! hosts. Provenance that *is* analysis state (cache UUID, image index,
//! install name) also lives in source records and fact payloads.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;

/// File name of the target descriptor inside a project directory.
pub const TARGET_FILE: &str = "target.json";

/// How to re-materialize the image a project was analyzed from.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ProjectTarget {
    /// A standalone Mach-O file on disk.
    MachOImage {
        /// Path to the binary as recorded at analyze time.
        path: String,
    },
    /// An image projected out of a dyld shared cache.
    SharedCacheProjectedImage {
        /// Path to the cache root member.
        cache_path: String,
        /// Selector used to resolve the image (install name, id, or basename).
        image_selector: String,
        /// Resolved install name.
        install_name: String,
        /// Cache UUID the image belongs to.
        cache_uuid: String,
        /// Image index inside the cache.
        image_index: u32,
    },
}

impl ProjectTarget {
    /// Cache target used by `damsel analyze <cache> --image <selector>`.
    #[allow(clippy::too_many_arguments)]
    pub fn shared_cache_image(
        cache_path: impl Into<String>,
        image_selector: impl Into<String>,
        install_name: impl Into<String>,
        cache_uuid: impl Into<String>,
        image_index: u32,
    ) -> Self {
        Self::SharedCacheProjectedImage {
            cache_path: cache_path.into(),
            image_selector: image_selector.into(),
            install_name: install_name.into(),
            cache_uuid: cache_uuid.into(),
            image_index,
        }
    }

    /// True for targets that project from a dyld shared cache.
    pub fn is_shared_cache(&self) -> bool {
        matches!(self, Self::SharedCacheProjectedImage { .. })
    }
}

#[derive(Debug, Error)]
pub enum ProjectTargetError {
    #[error("failed to read target descriptor: {0}")]
    Io(#[from] std::io::Error),
    #[error("failed to parse target descriptor: {0}")]
    Json(#[from] serde_json::Error),
}

/// Descriptor path inside a project directory.
pub fn target_path(project_dir: &Path) -> PathBuf {
    project_dir.join(TARGET_FILE)
}

/// Write the descriptor, creating the project directory if needed.
pub fn write(project_dir: &Path, target: &ProjectTarget) -> Result<(), ProjectTargetError> {
    fs::create_dir_all(project_dir)?;
    let bytes = serde_json::to_vec_pretty(target)?;
    fs::write(target_path(project_dir), bytes)?;
    Ok(())
}

/// Read the descriptor if one exists. `Ok(None)` preserves legacy projects
/// that predate descriptors and rely on the adjacent-file convention.
pub fn read(project_dir: &Path) -> Result<Option<ProjectTarget>, ProjectTargetError> {
    let path = target_path(project_dir);
    if !path.is_file() {
        return Ok(None);
    }
    let bytes = fs::read(path)?;
    Ok(Some(serde_json::from_slice(&bytes)?))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shared_cache_descriptor_round_trips() {
        let dir = tempfile::tempdir().unwrap();
        let target = ProjectTarget::shared_cache_image(
            "/caches/dyld_shared_cache_arm64e",
            "Foundation",
            "/System/Library/Frameworks/Foundation.framework/Foundation",
            "11111111-2222-3333-4444-555555555555",
            2,
        );
        write(dir.path(), &target).expect("write descriptor");
        let loaded = read(dir.path())
            .expect("read descriptor")
            .expect("descriptor");
        assert_eq!(loaded, target);
        assert!(loaded.is_shared_cache());

        let text = std::fs::read_to_string(target_path(dir.path())).unwrap();
        assert!(text.contains("\"kind\": \"shared_cache_projected_image\""));
    }

    #[test]
    fn missing_descriptor_reads_as_none() {
        let dir = tempfile::tempdir().unwrap();
        assert!(read(dir.path()).expect("read").is_none());
    }
}
