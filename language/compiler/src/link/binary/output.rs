use std::path::Path;

use destack_artifact::{BinaryArtifact, ObjectDebugArtifactKind, OutputContent, OutputFile};
use destack_source::{FileType, Uri};
use destack_workspace::{Module, Target};

use crate::link::module_source_path;

/// One error while materializing generated binary outputs.
#[derive(Debug, Clone)]
pub(crate) struct BinaryOutputError {
    /// The related output file type.
    pub file_type: FileType,
}

/// Link one generated binary artifact into output files.
pub(crate) fn link_binary_artifact_files(
    module: &Module,
    artifact: &BinaryArtifact,
    target: &Target,
    package_dir: &Path,
    root_dir: Option<&Path>,
) -> Result<Vec<OutputFile>, BinaryOutputError> {
    let module_path = module_source_path(module).map_err(|_| BinaryOutputError {
        file_type: FileType::Binary,
    })?;

    match artifact {
        BinaryArtifact::Object(object) => {
            let output_path = target.resolve_out_file(package_dir, root_dir, &module_path, "o");
            let mut files = vec![OutputFile {
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

                files.push(OutputFile {
                    uri: Uri::from_path(&debug_path),
                    content,
                    source: None,
                });
            }

            Ok(files)
        }
        BinaryArtifact::Wasm(wasm) => {
            let wasm_path = target.resolve_out_file(package_dir, root_dir, &module_path, "wasm");
            let mut files = vec![OutputFile {
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
                files.push(OutputFile {
                    uri: Uri::from_path(&map_path),
                    content,
                    source: None,
                });
            }

            Ok(files)
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
