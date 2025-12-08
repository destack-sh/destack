//! Module lowering from DIR to JS AST.
//!
//! The `ModuleLowerer` converts elaborated DIR (Destack IR) into a JavaScript AST.
//! This is the JS codegen entry point, used for JS/TS targets.

use destack_dir::{NodeTree as DirTree, SymbolTable, TypeTable};
use destack_source::{FileType, StringPool};
use destack_workspace::{
    Artifact, ArtifactContent, ArtifactId, ArtifactScope, Module, OutputFormat, Target,
};

use crate::tree::NodeTree as JsTree;
use crate::{CodegenJsError, CodegenJsResult, CodegenJsWarning, LocalNodeIdAny, format_statements};

/// Output from JS code generation.
#[derive(Debug)]
pub struct CodegenJsOutput {
    /// Generated artifacts.
    pub artifacts: Vec<Artifact>,
    /// Warnings encountered during generation.
    pub warnings: Vec<CodegenJsWarning>,
    /// Non-fatal errors encountered during generation.
    pub errors: Vec<CodegenJsError>,
}

/// Context for lowering a DIR module to JS AST.
#[derive(Debug)]
pub struct ModuleLowerer<'a> {
    /// The source module.
    pub(crate) module: &'a Module,
    /// The DIR tree.
    pub(crate) dir_tree: &'a DirTree,
    /// The symbol table (for future use).
    #[allow(dead_code)]
    pub(crate) symbols: &'a SymbolTable,
    /// The type table.
    pub(crate) types: &'a TypeTable,
    /// The target configuration.
    pub(crate) target: &'a Target,
    /// The output JS AST tree.
    pub(crate) tree: JsTree,
    /// Root nodes in the output.
    pub(crate) roots: Vec<LocalNodeIdAny>,
    /// String pool for the output.
    pub(crate) strings: StringPool,
    /// Collected warnings.
    pub(crate) warnings: Vec<CodegenJsWarning>,
    /// Collected non-fatal errors (treated as warnings for continued processing).
    pub(crate) errors: Vec<CodegenJsError>,
}

impl<'a> ModuleLowerer<'a> {
    /// Create a new module lowerer.
    pub fn new(
        module: &'a Module,
        dir_tree: &'a DirTree,
        symbols: &'a SymbolTable,
        types: &'a TypeTable,
        target: &'a Target,
    ) -> Self {
        Self {
            module,
            dir_tree,
            symbols,
            types,
            target,
            tree: JsTree::new(),
            roots: Vec::new(),
            strings: StringPool::new(),
            warnings: Vec::new(),
            errors: Vec::new(),
        }
    }

    /// Record a non-fatal error (allows lowering to continue).
    pub(crate) fn error(&mut self, error: CodegenJsError) {
        self.errors.push(error);
    }

    /// Record a warning.
    pub(crate) fn warning(&mut self, warning: CodegenJsWarning) {
        self.warnings.push(warning);
    }

    /// Lower the module to JS AST.
    pub fn lower_module(&mut self) -> CodegenJsResult<()> {
        for expression_id in self.module.dir.roots.iter() {
            match self.lower_expression(*expression_id) {
                Ok(root_id) => self.roots.push(root_id),
                Err(error) => self.error(error),
            }
        }
        Ok(())
    }

    /// Finish lowering and produce output.
    pub fn finish(
        self,
        registry_next_id: impl Fn() -> ArtifactId,
        package_dir: &std::path::Path,
        root_dir: Option<&std::path::Path>,
    ) -> CodegenJsResult<CodegenJsOutput>
    where
        Self: Sized,
    {
        use crate::{CodegenJsFormatContext, CodegenJsFormatOptions};
        use destack_fir::format as fir_format;
        use destack_fir::prelude::format_with;
        use destack_source::{File, Uri};

        let mut artifacts = Vec::new();

        // determine what files to generate based on target
        let file_types = match self.target.output {
            OutputFormat::Js => {
                let mut types = vec![FileType::JavaScript];
                if self.target.declaration {
                    types.push(FileType::TypeScriptDeclaration);
                }
                types
            }
            OutputFormat::Ts => vec![FileType::TypeScript],
            _ => {
                return Ok(CodegenJsOutput {
                    artifacts,
                    warnings: self.warnings,
                    errors: self.errors,
                });
            }
        };

        // get module source path for output path computation
        let fallback_path;
        let module_path = if let Some(ref path) = self.module.path {
            path.as_path()
        } else if let Some(path) = self.module.uri.to_path_buf() {
            fallback_path = path;
            fallback_path.as_path()
        } else {
            std::path::Path::new("module.ds")
        };
        let strings = self.strings.clone().into_immutable();

        for file_type in file_types {
            // compute output path using target configuration
            let extension = file_type
                .extension()
                .ok_or_else(|| CodegenJsError::Internal {
                    message: format!("file type has no known extension: {file_type:?}"),
                })?;
            let output_path =
                self.target
                    .resolve_out_file(package_dir, root_dir, module_path, extension);
            let uri = Uri::from_path(&output_path);

            // create format context
            let file = File::empty_text(file_type);
            let options = CodegenJsFormatOptions::from_target(self.target, file_type);
            let context = CodegenJsFormatContext {
                options,
                file: &file,
                tree: &self.tree,
                roots: &self.roots,
                strings: &strings,
            };
            // format to string
            let roots = &self.roots;
            let formatted = fir_format!(context, [format_with(|f| format_statements(f, roots))]);
            let formatted = match formatted {
                Ok(f) => f,
                Err(error) => {
                    return Err(CodegenJsError::Internal {
                        message: format!("failed to format roots: {error}"),
                    });
                }
            };
            let printed = match formatted.print() {
                Ok(p) => p,
                Err(error) => {
                    return Err(CodegenJsError::Internal {
                        message: format!("failed to print formatted: {error}"),
                    });
                }
            };
            let code = printed.as_str().to_string();

            // create artifact
            let content = match file_type {
                FileType::JavaScript => ArtifactContent::javascript(code),
                FileType::TypeScript => ArtifactContent::typescript(code),
                FileType::TypeScriptDeclaration => ArtifactContent::declaration(code),
                _ => {
                    return Err(CodegenJsError::Internal {
                        message: format!("unsupported file type: {file_type:?}"),
                    });
                }
            };

            let artifact = Artifact {
                id: registry_next_id(),
                scope: ArtifactScope::Module(self.module.id),
                target: self.target.name.clone(),
                uri,
                content,
                source: None,
            };
            artifacts.push(artifact);
        }

        Ok(CodegenJsOutput {
            artifacts,
            warnings: self.warnings,
            errors: self.errors,
        })
    }
}
