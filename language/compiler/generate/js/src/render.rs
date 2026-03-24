use std::path::Path;

use destack_artifact::{OutputContent, OutputFile, ScriptArtifact, SourceMapArtifact};
use destack_source::FileType;
use destack_workspace::Target;

use crate::{CodegenJsError, CodegenJsResult};

/// One renderer for generated script artifacts.
#[derive(Debug)]
pub struct ScriptArtifactRenderer<'a> {
    /// The module being rendered.
    module: &'a destack_workspace::Module,
    /// The generated script artifact.
    artifact: &'a ScriptArtifact,
    /// The target configuration.
    target: &'a Target,
    /// The package directory.
    package_dir: &'a Path,
    /// The optional root directory override.
    root_dir: Option<&'a Path>,
}

impl<'a> ScriptArtifactRenderer<'a> {
    /// Create one script artifact renderer.
    pub fn new(
        module: &'a destack_workspace::Module,
        artifact: &'a ScriptArtifact,
        target: &'a Target,
        package_dir: &'a Path,
        root_dir: Option<&'a Path>,
    ) -> Self {
        Self {
            module,
            artifact,
            target,
            package_dir,
            root_dir,
        }
    }

    /// Render one script artifact into final output files.
    pub fn render_output_files(self) -> CodegenJsResult<Vec<OutputFile>> {
        let plan =
            crate::plan_module_output(self.module, self.target, self.package_dir, self.root_dir)?;
        let emit = crate::emit::ModuleEmitOutput {
            module: self.artifact.module.clone(),
            warnings: Vec::new(),
            errors: Vec::new(),
        };
        let emit = crate::bundle_module_output(&plan, emit)?;
        let emit = crate::minify_module_output(&plan, emit)?;
        let warnings = emit.warnings;
        let errors = emit.errors;
        let mut entries = Vec::new();

        // code files
        for file in &plan.files {
            if file.file_type == FileType::TypeScriptDeclaration {
                continue;
            }

            let content = match file.file_type {
                FileType::JavaScript => {
                    let code =
                        crate::print_script_module(self.target, file.file_type, &emit.module)?;
                    OutputContent::javascript(code)
                }
                FileType::TypeScript => {
                    let code =
                        crate::print_script_module(self.target, file.file_type, &emit.module)?;
                    OutputContent::typescript(code)
                }
                FileType::Html => {
                    let code =
                        crate::print_script_module(self.target, file.file_type, &emit.module)?;
                    OutputContent::html(crate::wrap_html_document(&code))
                }
                FileType::SourceMap => {
                    let map =
                        self.artifact.source_map.clone().unwrap_or_else(|| {
                            SourceMapArtifact::empty(self.module.uri.to_string())
                        });
                    OutputContent::source_map(&map).map_err(|error| CodegenJsError::Internal {
                        message: format!("failed to serialize source map: {error}"),
                    })?
                }
                other => {
                    return Err(CodegenJsError::Internal {
                        message: format!("unsupported file type: {other:?}"),
                    });
                }
            };

            entries.push(OutputFile {
                uri: file.uri.clone(),
                content,
                source: None,
            });
        }

        // declarations
        if let Some(declaration) = &self.artifact.declaration {
            for file in &plan.files {
                if file.file_type != FileType::TypeScriptDeclaration {
                    continue;
                }

                entries.push(OutputFile {
                    uri: file.uri.clone(),
                    content: OutputContent::declaration(declaration.text.clone()),
                    source: None,
                });
            }
        }

        if !warnings.is_empty() {
            return Err(CodegenJsError::Internal {
                message: "unexpected warnings while rendering script artifact".to_string(),
            });
        }

        if let Some(error) = errors.into_iter().next() {
            return Err(error);
        }

        Ok(entries)
    }
}
