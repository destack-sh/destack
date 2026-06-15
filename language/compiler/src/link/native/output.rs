use std::path::Path;

use destack_artifact::{NativeOutput, OutputFile};
use destack_repository::{Module, Target};
use destack_source::{FileType, Uri};

use crate::Compiler;
use crate::link::module_source_path;

/// One error while materializing emitted native outputs.
#[derive(Debug, Clone)]
pub(crate) struct NativeOutputError {
    /// The related output file type.
    pub file_type: FileType,
}

/// Link one emitted native output into output files.
pub(crate) fn link_native_output_files(
    compiler: &Compiler,
    module: &Module,
    artifact: &NativeOutput,
    target: &Target,
    package_dir: &Path,
    root_dir: Option<&Path>,
) -> Result<Vec<OutputFile>, NativeOutputError> {
    let module_path = module_source_path(module).map_err(|_| NativeOutputError {
        file_type: FileType::Binary,
    })?;
    let extension = artifact.file_type.extension().ok_or(NativeOutputError {
        file_type: artifact.file_type,
    })?;
    let output_path = target.resolve_out_file(package_dir, root_dir, &module_path, extension);
    let _ = compiler
        .repository
        .content(artifact.content)
        .map_err(|_| NativeOutputError {
            file_type: artifact.file_type,
        })?;
    let mut files = vec![OutputFile::new(
        Uri::from_path(&output_path),
        artifact.file_type,
        artifact.content,
        None,
    )];

    // source map
    if let Some(map) = &artifact.source_map {
        let map_path = target.resolve_out_file(package_dir, root_dir, &module_path, "map");
        let content = Compiler::source_map_content(map).map_err(|_| NativeOutputError {
            file_type: FileType::SourceMap,
        })?;
        files.push(
            compiler
                .intern_output_file(
                    Uri::from_path(&map_path),
                    FileType::SourceMap,
                    content,
                    None,
                )
                .map_err(|_| NativeOutputError {
                    file_type: FileType::SourceMap,
                })?,
        );
    }

    Ok(files)
}
