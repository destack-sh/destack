use std::path::Path;

use destack_artifact::{BinaryOutput, OutputContent, OutputFile};
use destack_source::{FileType, Uri};
use destack_workspace::{Module, Target};

use crate::link::module_source_path;

/// One error while materializing generated binary outputs.
#[derive(Debug, Clone)]
pub(crate) struct BinaryOutputError {
    /// The related output file type.
    pub file_type: FileType,
}

/// Link one generated binary output into output files.
pub(crate) fn link_binary_output_files(
    module: &Module,
    artifact: &BinaryOutput,
    target: &Target,
    package_dir: &Path,
    root_dir: Option<&Path>,
) -> Result<Vec<OutputFile>, BinaryOutputError> {
    let module_path = module_source_path(module).map_err(|_| BinaryOutputError {
        file_type: FileType::Binary,
    })?;
    let extension = artifact.file_type.extension().ok_or(BinaryOutputError {
        file_type: artifact.file_type,
    })?;
    let output_path = target.resolve_out_file(package_dir, root_dir, &module_path, extension);
    let mut files = vec![OutputFile {
        uri: Uri::from_path(&output_path),
        content: OutputContent::Binary {
            bytes: artifact.bytes.clone(),
            file_type: artifact.file_type,
        },
        source: None,
    }];

    // source map
    if let Some(map) = &artifact.source_map {
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
