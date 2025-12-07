use std::path::Path;

use crate::{Compiler, EmitError, EmitResult, EmittedArtifact};

use destack_workspace::{Artifact, ArtifactContent};

impl Compiler {
    /// Write an artifact to disk.
    pub(super) fn write_artifact(
        &self,
        artifact: &Artifact,
        path: &Path,
    ) -> EmitResult<EmittedArtifact> {
        // check dry run mode
        if self.options.emit.dry_run {
            let size = match &artifact.content {
                ArtifactContent::Text { code, .. } => code.len(),
                ArtifactContent::Json { content, .. } => content.len(),
                ArtifactContent::Binary { bytes, .. } => bytes.len(),
            };
            return Ok(EmittedArtifact {
                artifact: artifact.id,
                path: path.to_path_buf(),
                size,
            });
        }

        // create parent directories if needed
        if self.options.emit.create_dirs
            && let Some(parent) = path.parent()
        {
            self.program
                .fs
                .create_dir_all(parent)
                .map_err(|e| EmitError::FailedWrite {
                    artifact: artifact.id,
                    node: self.program.root_node_id,
                    path: parent.to_path_buf(),
                    message: Some(e.to_string()),
                })?;
        }

        // check if file exists and overwrite is disabled
        if !self.options.emit.overwrite
            && let Ok(true) = self.program.fs.exists(path)
        {
            return Err(EmitError::FailedWrite {
                artifact: artifact.id,
                node: self.program.root_node_id,
                path: path.to_path_buf(),
                message: Some("file already exists and overwrite is disabled".to_string()),
            });
        }

        // write the content
        let size = match &artifact.content {
            ArtifactContent::Text { code, .. } => {
                self.program
                    .fs
                    .write_string(path, code)
                    .map_err(|e| EmitError::FailedWrite {
                        artifact: artifact.id,
                        node: self.program.root_node_id,
                        path: path.to_path_buf(),
                        message: Some(e.to_string()),
                    })?;
                code.len()
            }
            ArtifactContent::Json { content, .. } => {
                self.program.fs.write_string(path, content).map_err(|e| {
                    EmitError::FailedWrite {
                        artifact: artifact.id,
                        node: self.program.root_node_id,
                        path: path.to_path_buf(),
                        message: Some(e.to_string()),
                    }
                })?;
                content.len()
            }
            ArtifactContent::Binary { bytes, .. } => {
                self.program
                    .fs
                    .write(path, bytes)
                    .map_err(|e| EmitError::FailedWrite {
                        artifact: artifact.id,
                        node: self.program.root_node_id,
                        path: path.to_path_buf(),
                        message: Some(e.to_string()),
                    })?;
                bytes.len()
            }
        };

        Ok(EmittedArtifact {
            artifact: artifact.id,
            path: path.to_path_buf(),
            size,
        })
    }
}
