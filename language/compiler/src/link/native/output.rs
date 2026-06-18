use std::path::Path;

use destack_artifact::{BundleFile, BundleSection, Object};
use destack_repository::{Module, Target};
use destack_source::{FileType, Uri};

use crate::Compiler;
use crate::link::module_source_path;

/// One error while materializing emitted objects.
#[derive(Debug, Clone)]
pub(crate) struct ObjectError {
    /// The related output file type.
    pub file_type: FileType,
}

/// Link one emitted object into output files.
pub(crate) fn link_object_files(
    compiler: &Compiler,
    module: &Module,
    artifact: &Object,
    target: &Target,
    package_dir: &Path,
    root_dir: Option<&Path>,
) -> Result<Vec<BundleFile>, ObjectError> {
    let module_path = module_source_path(module).map_err(|_| ObjectError {
        file_type: FileType::Binary,
    })?;
    let file_type = artifact.file_type();
    let extension = file_type.extension().ok_or(ObjectError { file_type })?;
    let output_path = target.resolve_out_file(package_dir, root_dir, &module_path, extension);
    let _ = compiler
        .repository
        .content(artifact.content)
        .map_err(|_| ObjectError { file_type })?;
    let mut files = vec![BundleFile::new(
        BundleSection::Native,
        Uri::from_path(&output_path),
        file_type,
        artifact.content,
        None,
    )];

    // source map
    if let Some(map) = &artifact.map {
        let map_path = target.resolve_out_file(package_dir, root_dir, &module_path, "map");
        let content = Compiler::source_map_content(map).map_err(|_| ObjectError {
            file_type: FileType::SourceMap,
        })?;
        files.push(
            compiler
                .intern_output_file(
                    BundleSection::SourceMap,
                    Uri::from_path(&map_path),
                    FileType::SourceMap,
                    content,
                    None,
                )
                .map_err(|_| ObjectError {
                    file_type: FileType::SourceMap,
                })?,
        );
    }

    Ok(files)
}
