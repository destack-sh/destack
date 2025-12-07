//! Module lowering from DIR to JS AST.
//!
//! The `ModuleLowerer` converts elaborated DIR (Destack IR) into a JavaScript AST.
//! This is the JS codegen entry point, used for JS/TS targets.

use destack_dir::{NodeTree as DirTree, SymbolTable, TypeTable};
use destack_source::{
    Diagnostic, DiagnosticCollector, DiagnosticSeverity, FileType, LabeledSpan, StringPool,
};
use destack_workspace::{
    Artifact, ArtifactContent, ArtifactId, ArtifactScope, Module, OutputFormat, Program, Target,
};

use crate::tree::NodeTree as JsTree;
use crate::{CodegenJsError, CodegenJsResult, CodegenJsWarning, LocalNodeIdAny};

/// Context for lowering a DIR module to JS AST.
#[derive(Debug)]
pub struct ModuleLowerer<'a> {
    /// The program.
    pub(crate) program: &'a Program,
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
    /// Collected diagnostics.
    pub(crate) diagnostics: DiagnosticCollector,
}

impl<'a> ModuleLowerer<'a> {
    /// Create a new module lowerer.
    pub fn new(
        program: &'a Program,
        module: &'a Module,
        dir_tree: &'a DirTree,
        symbols: &'a SymbolTable,
        types: &'a TypeTable,
        target: &'a Target,
    ) -> Self {
        Self {
            program,
            module,
            dir_tree,
            symbols,
            types,
            target,
            tree: JsTree::new(),
            roots: Vec::new(),
            strings: StringPool::new(),
            diagnostics: DiagnosticCollector::new(),
        }
    }

    /// Create a diagnostic from an error or warning.
    fn make_diagnostic(
        &self,
        severity: DiagnosticSeverity,
        node_id: destack_dir::GlobalNodeIdAny,
        message: String,
    ) -> Diagnostic {
        let module = self.program.modules.get(node_id.module_id);
        let module = module.read();
        let source_node_id = module.dir.tree.read().get_source(node_id.local_id.id);
        let primary_span = module.ast.tree.get_span_by_id(source_node_id);
        let primary_span = LabeledSpan {
            span: primary_span,
            label: message.clone(),
        };

        Diagnostic {
            code: String::new(),
            original_code: None,
            severity,
            original_severity: None,
            message,
            file_id: module.file_id,
            primary_span,
            primary_highlight_spans: None,
            secondary_spans: None,
            suggestions: None,
        }
    }

    /// Add an error diagnostic.
    pub(crate) fn error(&self, error: CodegenJsError) {
        let diagnostic =
            self.make_diagnostic(DiagnosticSeverity::Error, error.node_id(), error.message());
        self.diagnostics.insert(diagnostic);
    }

    /// Add a warning diagnostic.
    pub(crate) fn warning(&self, warning: CodegenJsWarning) {
        let diagnostic = self.make_diagnostic(
            DiagnosticSeverity::Warning,
            warning.node_id(),
            warning.message(),
        );
        self.diagnostics.insert(diagnostic);
    }

    /// Lower the module to JS AST.
    pub fn lower(&mut self) -> CodegenJsResult<()> {
        for expression_id in self.module.dir.roots.iter() {
            match self.lower_expression(*expression_id) {
                Ok(root_id) => self.roots.push(root_id),
                Err(error) => self.error(error),
            }
        }
        Ok(())
    }

    /// Finish lowering and produce artifacts.
    pub fn finish(self, registry_next_id: impl Fn() -> ArtifactId) -> CodegenJsResult<Vec<Artifact>>
    where
        Self: Sized,
    {
        use crate::{JavaScriptFormatContext, JavaScriptFormatOptions};
        use destack_fir::format as fir_format;
        use destack_source::File;

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
            _ => return Ok(artifacts),
        };

        // base URI without extension
        let base_uri = self.module.uri.without_extension();
        let strings = self.strings.clone().into_immutable();

        // helper for formatting roots
        struct RootsFormatter<'a>(&'a [LocalNodeIdAny]);
        impl<'a> destack_fir::format::Format<JavaScriptFormatContext<'a>> for RootsFormatter<'a> {
            fn format(
                &self,
                f: &mut destack_fir::format::Formatter<'_, JavaScriptFormatContext<'a>>,
            ) -> destack_fir::format::FormatResult<()> {
                use destack_fir::prelude::*;
                f.join_with(hard_line_break()).entries(self.0).finish()?;
                Ok(())
            }
        }

        for file_type in file_types {
            let extension = file_type.extension().unwrap_or("js");
            let uri = base_uri.with_extension(extension);

            // create format context
            let file = File::empty_text_with_type(file_type);
            let options = JavaScriptFormatOptions::from_target(self.target, file_type);
            let context = JavaScriptFormatContext {
                options,
                file: &file,
                tree: &self.tree,
                roots: &self.roots,
                strings: &strings,
            };
            let roots_formatter = RootsFormatter(&self.roots);

            // format to string
            let formatted = fir_format!(context, [roots_formatter]);
            let formatted = match formatted {
                Ok(f) => f,
                Err(_) => continue,
            };
            let printed = match formatted.print() {
                Ok(p) => p,
                Err(_) => continue,
            };
            let code = printed.as_str().to_string();

            // create artifact
            let content = match file_type {
                FileType::JavaScript => ArtifactContent::javascript(code),
                FileType::TypeScript => ArtifactContent::typescript(code),
                FileType::TypeScriptDeclaration => ArtifactContent::declaration(code),
                _ => continue,
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

        // flush diagnostics to program
        self.program.diagnostics.take_from(&self.diagnostics);

        Ok(artifacts)
    }
}
