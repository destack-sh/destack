use std::path::{Path, PathBuf};

use crate::{Compiler, EmitError, EmitResult};
use destack_artifact::{BinaryArtifact, ObjectDebugArtifactKind, OutputContent, OutputFile};
use destack_source::{FileType, Uri};
use destack_workspace::{Module, Target};

/// One error while materializing generated binary outputs.
#[derive(Debug, Clone, Copy)]
pub(crate) struct BinaryOutputError {
    /// The related output file type.
    pub file_type: FileType,
}

/// Resolve one stable module source path for output planning.
pub(crate) fn module_source_path(module: &Module) -> PathBuf {
    if let Some(path) = &module.path {
        return path.clone();
    }

    if let Some(path) = module.uri.to_path_buf() {
        return path;
    }

    PathBuf::from("module.ds")
}

/// Materialize one generated binary artifact into output files.
pub(crate) fn render_binary_artifact_entries(
    module: &Module,
    artifact: &BinaryArtifact,
    target: &Target,
    package_dir: &Path,
    root_dir: Option<&Path>,
) -> Result<Vec<OutputFile>, BinaryOutputError> {
    let module_path = module_source_path(module);

    match artifact {
        BinaryArtifact::Object(object) => {
            let output_path = target.resolve_out_file(package_dir, root_dir, &module_path, "o");
            let mut entries = vec![OutputFile {
                uri: Uri::from_path(&output_path),
                content: OutputContent::object(object.bytes.to_vec()),
                source: None,
            }];

            // debug outputs
            for debug in &object.debug {
                let extension = debug.kind.extension();
                let debug_path =
                    target.resolve_out_file(package_dir, root_dir, &module_path, extension);
                let content = object_debug_output_content(debug.kind, debug.bytes.to_vec());

                entries.push(OutputFile {
                    uri: Uri::from_path(&debug_path),
                    content,
                    source: None,
                });
            }

            Ok(entries)
        }
        BinaryArtifact::Wasm(wasm) => {
            let wasm_path = target.resolve_out_file(package_dir, root_dir, &module_path, "wasm");
            let mut entries = vec![OutputFile {
                uri: Uri::from_path(&wasm_path),
                content: OutputContent::wasm(wasm.bytes.to_vec()),
                source: None,
            }];

            // source map
            if let Some(map) = &wasm.source_map {
                let map_path = target.resolve_out_file(package_dir, root_dir, &module_path, "map");
                let content = OutputContent::source_map(map).map_err(|_| BinaryOutputError {
                    file_type: FileType::SourceMap,
                })?;
                entries.push(OutputFile {
                    uri: Uri::from_path(&map_path),
                    content,
                    source: None,
                });
            }

            Ok(entries)
        }
    }
}

/// Materialize one object debug payload to final output content.
fn object_debug_output_content(kind: ObjectDebugArtifactKind, bytes: Vec<u8>) -> OutputContent {
    match kind {
        ObjectDebugArtifactKind::SplitDwarf => OutputContent::Binary {
            bytes,
            file_type: FileType::Binary,
        },
    }
}

impl Compiler {
    /// Write one output file to disk.
    pub(super) fn write_output_file(&self, entry: &OutputFile, path: &Path) -> EmitResult<()> {
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
                    path: parent.to_path_buf(),
                    message: Some(error.to_string()),
                })?;
        }

        // check if file exists and overwrite is disabled
        if !self.options.emit_overwrite
            && let Ok(true) = self.program.fs.exists(path)
        {
            return Err(EmitError::FailedWrite {
                path: path.to_path_buf(),
                message: Some("file already exists and overwrite is disabled".to_string()),
            });
        }

        // write the content
        match &entry.content {
            OutputContent::Text { code, .. } => {
                self.program.fs.write_string(path, code).map_err(|error| {
                    EmitError::FailedWrite {
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
                        path: path.to_path_buf(),
                        message: Some(error.to_string()),
                    })?;
            }
            OutputContent::Binary { bytes, .. } => {
                self.program
                    .fs
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
