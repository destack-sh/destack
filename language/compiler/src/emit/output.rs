use std::path::Path;

use crate::{Compiler, EmitError, EmitResult};
use destack_workspace::{Output, OutputContent};

impl Compiler {
    /// Write an output to disk.
    pub(super) fn write_output(&self, output: &Output, path: &Path) -> EmitResult<()> {
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
                .map_err(|error| EmitError::FailedWrite {
                    output: output.id,
                    path: parent.to_path_buf(),
                    message: Some(error.to_string()),
                })?;
        }

        // check if file exists and overwrite is disabled
        if !self.options.emit_overwrite
            && let Ok(true) = self.program.fs.exists(path)
        {
            return Err(EmitError::FailedWrite {
                output: output.id,
                path: path.to_path_buf(),
                message: Some("file already exists and overwrite is disabled".to_string()),
            });
        }

        // write the content
        match &output.content {
            OutputContent::Text { code, .. } => {
                self.program.fs.write_string(path, code).map_err(|error| {
                    EmitError::FailedWrite {
                        output: output.id,
                        path: path.to_path_buf(),
                        message: Some(error.to_string()),
                    }
                })?;
            }
            OutputContent::Json { content, .. } => {
                self.program
                    .fs
                    .write_string(path, content)
                    .map_err(|error| EmitError::FailedWrite {
                        output: output.id,
                        path: path.to_path_buf(),
                        message: Some(error.to_string()),
                    })?;
            }
            OutputContent::Binary { bytes, .. } => {
                self.program
                    .fs
                    .write(path, bytes)
                    .map_err(|error| EmitError::FailedWrite {
                        output: output.id,
                        path: path.to_path_buf(),
                        message: Some(error.to_string()),
                    })?;
            }
        };

        Ok(())
    }
}
