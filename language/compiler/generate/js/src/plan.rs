use std::path::{Path, PathBuf};

use destack_artifact::EmitFormat;
use destack_source::{FileType, Uri};
use destack_workspace::{Module, Target};

use crate::CodegenJsError;

/// Output assembly mode for one JavaScript generation request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JsGenerateMode {
    /// Emit one file per source module.
    PerModule,
    /// Assemble one target level output shape.
    Assembled,
}

/// Output file plan for one generated JavaScript artifact.
#[derive(Debug, Clone)]
pub struct GeneratedFilePlan {
    /// The file type to emit.
    pub file_type: FileType,
    /// The resolved output path on disk.
    pub output_path: PathBuf,
    /// The resolved output URI.
    pub uri: Uri,
}

/// High level plan for one JavaScript generation request.
#[derive(Debug, Clone)]
pub struct ModuleGeneratePlan {
    /// The output assembly mode for this target.
    pub mode: JsGenerateMode,
    /// Whether the final output should be minified.
    pub should_minify: bool,
    /// The files to print for this module.
    pub files: Vec<GeneratedFilePlan>,
}

/// Build one JavaScript generation plan for a module target.
pub fn plan_module_output(
    module: &Module,
    target: &Target,
    package_dir: &Path,
    root_dir: Option<&Path>,
) -> Result<ModuleGeneratePlan, CodegenJsError> {
    let module_path = module_source_path(module);
    let file_types = planned_file_types(target)?;
    let mut files = Vec::new();

    // resolve the output path for each emitted file type
    for file_type in file_types {
        let extension = file_type
            .extension()
            .ok_or_else(|| CodegenJsError::Internal {
                message: format!("file type has no known extension: {file_type:?}"),
            })?;
        let output_path = target.resolve_out_file(package_dir, root_dir, &module_path, extension);
        let uri = Uri::from_path(&output_path);

        files.push(GeneratedFilePlan {
            file_type,
            output_path,
            uri,
        });
    }

    Ok(ModuleGeneratePlan {
        mode: plan_generate_mode(target),
        should_minify: target.should_minify_bundle_output(),
        files,
    })
}

/// Plan the assembly mode for one target.
fn plan_generate_mode(target: &Target) -> JsGenerateMode {
    if target.emits_per_module_output() {
        JsGenerateMode::PerModule
    } else {
        JsGenerateMode::Assembled
    }
}

/// Choose the emitted file types for one target.
fn planned_file_types(target: &Target) -> Result<Vec<FileType>, CodegenJsError> {
    match target.emit {
        EmitFormat::Js => {
            let mut file_types = vec![FileType::JavaScript];
            if target.declaration {
                file_types.push(FileType::TypeScriptDeclaration);
            }
            if target.emits_source_map_output() {
                file_types.push(FileType::SourceMap);
            }

            Ok(file_types)
        }
        EmitFormat::Ts => {
            let mut file_types = vec![FileType::TypeScript];
            if target.emits_source_map_output() {
                file_types.push(FileType::SourceMap);
            }

            Ok(file_types)
        }
        EmitFormat::Html => {
            let mut file_types = vec![FileType::Html];
            if target.emits_source_map_output() {
                file_types.push(FileType::SourceMap);
            }

            Ok(file_types)
        }
        _ => Err(CodegenJsError::UnsupportedTarget {
            format: format!("{:?}", target.emit),
            message: Some("expected JS, TS, or HTML".to_string()),
        }),
    }
}

/// Resolve one stable source path for module based output planning.
fn module_source_path(module: &Module) -> PathBuf {
    if let Some(path) = &module.path {
        return path.clone();
    }

    if let Some(path) = module.uri.to_path_buf() {
        return path;
    }

    PathBuf::from("module.ds")
}
