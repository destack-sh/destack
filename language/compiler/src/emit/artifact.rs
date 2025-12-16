use std::path::Path;

use crate::{Compiler, EmitError, EmitResult};

use destack_workspace::{Artifact, ArtifactContent};

impl Compiler {
    /// Write an artifact to disk.
    pub(super) fn write_artifact(&self, artifact: &Artifact, path: &Path) -> EmitResult<()> {
        // check dry run mode
        if self.options.emit_dry_run {
            return Ok(());
        }

        // create parent directories if needed
        if self.options.emit_create_dirs
            && let Some(parent) = path.parent()
        {
            self.program
                .fs
                .create_dir_all(parent)
                .map_err(|e| EmitError::FailedWrite {
                    artifact: artifact.id,
                    path: parent.to_path_buf(),
                    message: Some(e.to_string()),
                })?;
        }

        // check if file exists and overwrite is disabled
        if !self.options.emit_overwrite
            && let Ok(true) = self.program.fs.exists(path)
        {
            return Err(EmitError::FailedWrite {
                artifact: artifact.id,
                path: path.to_path_buf(),
                message: Some("file already exists and overwrite is disabled".to_string()),
            });
        }

        // write the content
        match &artifact.content {
            ArtifactContent::Text { code, .. } => {
                self.program
                    .fs
                    .write_string(path, code)
                    .map_err(|e| EmitError::FailedWrite {
                        artifact: artifact.id,
                        path: path.to_path_buf(),
                        message: Some(e.to_string()),
                    })?;
            }
            ArtifactContent::Json { content, .. } => {
                self.program.fs.write_string(path, content).map_err(|e| {
                    EmitError::FailedWrite {
                        artifact: artifact.id,
                        path: path.to_path_buf(),
                        message: Some(e.to_string()),
                    }
                })?;
            }
            ArtifactContent::Binary { bytes, .. } => {
                self.program
                    .fs
                    .write(path, bytes)
                    .map_err(|e| EmitError::FailedWrite {
                        artifact: artifact.id,
                        path: path.to_path_buf(),
                        message: Some(e.to_string()),
                    })?;
            }
        };

        Ok(())
    }
}
