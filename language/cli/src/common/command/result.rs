use std::collections::BTreeMap;
use std::sync::Arc;

use destack_source::{DiagnosticCollection, File, FileId};
use destack_workspace::{CommandError, CommandOutput, Error, FileImage, JsonValue, Output};

use crate::diagnostic::{ConsoleError, ConsoleResult};

/// Result of one CLI workspace operation.
#[derive(Debug)]
pub(crate) struct CommandResult {
    /// Workspace command output.
    pub(crate) response: Output<JsonValue>,
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
            .map(JsonValue::from_json)
            .map_err(|error| ConsoleError::message(format!("command payload failed: {error}")))?;
        let diagnostics = DiagnosticCollection::from_diagnostics(response.diagnostics.clone());
        let files = files(&response.files);
        let response = Output {
            revision: response.revision,
            success: response.success,
            exit_code: response.exit_code,
            diagnostics: response.diagnostics,
            files: response.files,
            messages: response.messages,
            output: response.output,
            outputs: response.outputs,
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
}

/// Convert a workspace command error into a CLI error.
pub(crate) fn command_error(error: CommandError) -> ConsoleError {
    ConsoleError::message(format!("command failed: {error}"))
}

/// Convert a workspace error into a CLI error.
pub(crate) fn workspace_error(error: Error) -> ConsoleError {
    ConsoleError::message(format!("workspace failed: {error}"))
}

/// Convert file images into a file registry.
fn files(images: &[FileImage]) -> BTreeMap<FileId, Arc<File>> {
    let mut files = BTreeMap::new();
    for image in images {
        let Some(content) = image.content.as_ref() else {
            continue;
        };
        let file = File::from_text(
            image.id,
            image.name.clone(),
            image.uri.clone(),
            image.path.clone(),
            image.file_type,
            content.clone(),
        );
        files.insert(image.id, Arc::new(file));
    }

    files
}
