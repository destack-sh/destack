use std::path::Path;

use crate::Compiler;
use destack_artifact::{OutputContent, OutputFile};

use super::EmitError;

impl Compiler {
    /// Emit one output file to disk.
    pub(super) fn emit_output_file(&self, file: &OutputFile, path: &Path) -> Result<(), EmitError> {
        // check dry run mode
        if self.options.emit_dry_run {
            return Ok(());
        }

        // create parent directories if needed
        if self.options.emit_create_dirs
            && let Some(parent) = path.parent()
        {
            self.repository
                .file_system()
                .create_dir_all(parent)
                .map_err(|error| EmitError::FailedWrite {
                    path: parent.to_path_buf(),
                    message: Some(error.to_string()),
                })?;
        }

        // check if file exists and overwrite is disabled
        if !self.options.emit_overwrite
            && let Ok(true) = self.repository.file_system().exists(path)
        {
            return Err(EmitError::FailedWrite {
                path: path.to_path_buf(),
                message: Some("file already exists and overwrite is disabled".to_string()),
            });
        }

        // write the content
        match &file.content {
            OutputContent::Text { code, .. } => {
                self.repository
                    .file_system()
                    .write_string(path, code)
                    .map_err(|error| EmitError::FailedWrite {
                        path: path.to_path_buf(),
                        message: Some(error.to_string()),
                    })?;
            }
            OutputContent::Json { content, .. } => {
                self.repository
                    .file_system()
                    .write_string(path, content)
                    .map_err(|error| EmitError::FailedWrite {
                        path: path.to_path_buf(),
                        message: Some(error.to_string()),
                    })?;
            }
            OutputContent::Binary { bytes, .. } => {
                self.repository
                    .file_system()
                    .write(path, bytes)
                    .map_err(|error| EmitError::FailedWrite {
                        path: path.to_path_buf(),
                        message: Some(error.to_string()),
                    })?;
            }
        };

        Ok(())
    }
}
