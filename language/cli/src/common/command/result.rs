use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Duration;

use tspp_serde::Value;
use tspp_source::{DiagnosticCollection, File, FileId};
use tspp_workspace::{CommandError, CommandOutput, FileImage, Output};

use crate::console;
use crate::diagnostic::{ConsoleError, ConsoleResult};

/// Result of one CLI workspace operation.
#[derive(Debug)]
pub(crate) struct CommandResult {
    /// Workspace command output.
    pub(crate) response: Output<Value>,
    /// Flattened diagnostics from the response.
    pub(crate) diagnostics: DiagnosticCollection,
    /// Files reconstructed from response images.
    pub(crate) files: BTreeMap<FileId, Arc<File>>,
}

impl CommandResult {
    /// Build a CLI result from one workspace command output.
    pub(crate) fn from_output<O>(response: O) -> ConsoleResult<Self>
    where
        O: CommandOutput,
        O::Data: serde::Serialize,
    {
        let response = response.into_output();
        let data = serde_json::to_value(response.data)
            .map(Value::from)
            .map_err(|error| ConsoleError::message(format!("command payload failed: {error}")))?;
        let diagnostics = DiagnosticCollection::from_diagnostics(response.diagnostics.clone());
        let files = files(&response.files)?;
        let response = Output {
            revision: response.revision,
            success: response.success,
            exit_code: response.exit_code,
            diagnostics: response.diagnostics,
            files: response.files,
            messages: response.messages,
            output: response.output,
            outputs: response.outputs,
            trace: response.trace,
            data,
            module_count: response.module_count,
            profile_count: response.profile_count,
            target_count: response.target_count,
        };

        Ok(Self {
            response,
            diagnostics,
            files,
        })
    }

    /// Emit the command trace when one was requested.
    pub(crate) fn emit_timings(&self, command_duration: Option<Duration>) {
        let Some(trace) = self.response.trace.as_ref() else {
            return;
        };
        let timings = console::render_timings(trace, command_duration);
        if timings.is_empty() {
            return;
        }

        eprintln!("\n{timings}");
    }
}

/// Convert a workspace command error into a CLI error.
pub(crate) fn command_error(error: CommandError) -> ConsoleError {
    ConsoleError::message(format!("command failed: {error}"))
}

/// Convert file images into a file registry.
fn files(images: &[FileImage]) -> ConsoleResult<BTreeMap<FileId, Arc<File>>> {
    let mut files = BTreeMap::new();
    for image in images {
        if image.content.is_none() {
            continue;
        }
        let file = image
            .clone()
            .into_file()
            .map_err(|error| ConsoleError::message(format!("diagnostic file failed: {error}")))?;
        files.insert(image.id, Arc::new(file));
    }

    Ok(files)
}
