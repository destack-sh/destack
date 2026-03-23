//! JS codegen backend implementation.

use std::path::Path;
use std::sync::Arc;

use destack_artifact::{
    ArtifactStore, DynamicScriptDependency, DynamicScriptDependencyTarget, EmitFormat,
    OutputContent, OutputFile, ScriptArtifact, ScriptDeclaration, ScriptDependencyKind,
    ScriptDependencyTarget, ScriptLanguage, ScriptLinkage, SourceMapArtifact,
    StaticScriptDependency, StaticScriptDependencyUsage,
};
use destack_codegen_lib::CodegenBackend;
use destack_js::{
    self as js, DependencyKind, Expression, LocalNodeId, NodeTree, NodeVisitor, NodeVisitorOptions,
    Path as ScriptPath, ScalarLiteral, ScriptModule, Statement,
};
use destack_source::{FileType, ModuleId};
use destack_workspace::{ProfileId, Program, Target};

use crate::{CodegenJsError, CodegenJsResult};

/// JavaScript/TypeScript codegen backend.
#[derive(Debug, Default)]
pub struct JsBackend;

impl CodegenBackend for JsBackend {
    fn name(&self) -> &'static str {
        "js"
    }

    fn supports_target(&self, target: &Target) -> bool {
        target.uses_js_generate_pipeline()
    }
}

/// One visitor that collects script linkage metadata.
#[derive(Debug, Clone, Default)]
struct ScriptLinkageCollector {
    /// The collected linkage metadata.
    linkage: ScriptLinkage,
    /// The shared string pool for the module.
    strings: destack_core::StringPool,
    /// Visitor options.
    options: NodeVisitorOptions,
}

impl ScriptLinkageCollector {
    /// Finish the collector and return the linkage metadata.
    fn into_linkage(self) -> ScriptLinkage {
        self.linkage
    }
}

impl NodeVisitor for ScriptLinkageCollector {
    fn options(&self) -> &NodeVisitorOptions {
        &self.options
    }

    fn visit_statement(
        &mut self,
        tree: &NodeTree,
        id: LocalNodeId<Statement>,
        statement: &Statement,
    ) {
        // collect top-level static linkage first
        match statement {
            Statement::Import { kind, target, .. } => {
                self.linkage
                    .static_dependencies
                    .push(StaticScriptDependency {
                        usage: StaticScriptDependencyUsage::Import,
                        kind: lower_script_dependency_kind(*kind),
                        target: lower_static_script_dependency_target(
                            string_value(&self.strings, *target),
                            statement_target_module(statement),
                        ),
                    });
            }
            Statement::Export {
                kind,
                target: Some(target),
                ..
            } => {
                self.linkage
                    .static_dependencies
                    .push(StaticScriptDependency {
                        usage: StaticScriptDependencyUsage::Reexport,
                        kind: lower_script_dependency_kind(*kind),
                        target: lower_static_script_dependency_target(
                            string_value(&self.strings, *target),
                            statement_target_module(statement),
                        ),
                    });
            }
            _ => {}
        }

        js::walk_statement(self, tree, id, statement);
    }

    fn visit_expression(
        &mut self,
        tree: &NodeTree,
        id: LocalNodeId<Expression>,
        expression: &Expression,
    ) {
        // collect dynamic import calls while the tree is still structured
        if let Some(target) = dynamic_import_target(tree, &self.strings, expression) {
            self.linkage
                .dynamic_dependencies
                .push(DynamicScriptDependency {
                    target: lower_dynamic_script_dependency_target(target, None),
                });
        }

        js::walk_expression(self, tree, id, expression);
    }
}

/// Return one interned string as owned text.
fn string_value(strings: &destack_core::StringPool, id: destack_core::StringId) -> String {
    strings.get(id).to_string()
}

/// Lower one JS dependency kind into artifact linkage metadata.
fn lower_script_dependency_kind(kind: DependencyKind) -> ScriptDependencyKind {
    match kind {
        DependencyKind::Type => ScriptDependencyKind::Type,
        DependencyKind::Value => ScriptDependencyKind::Value,
    }
}

/// Lower one static JS dependency target into artifact metadata.
fn lower_static_script_dependency_target(
    specifier: String,
    target_module: Option<ModuleId>,
) -> ScriptDependencyTarget {
    if let Some(module) = target_module {
        return ScriptDependencyTarget::Module { module, specifier };
    }

    ScriptDependencyTarget::External { specifier }
}

/// Lower one dynamic JS dependency target into artifact metadata.
fn lower_dynamic_script_dependency_target(
    specifier: Option<String>,
    target_module: Option<ModuleId>,
) -> DynamicScriptDependencyTarget {
    if let Some(specifier) = specifier {
        return DynamicScriptDependencyTarget::Resolved(lower_static_script_dependency_target(
            specifier,
            target_module,
        ));
    }

    DynamicScriptDependencyTarget::Opaque
}

/// Return the resolved target module carried by one script statement.
fn statement_target_module(statement: &Statement) -> Option<destack_source::ModuleId> {
    match statement {
        Statement::Import { target_module, .. } | Statement::Export { target_module, .. } => {
            *target_module
        }
        _ => None,
    }
}

/// Return the static target for one dynamic import expression when it is known.
fn dynamic_import_target(
    tree: &NodeTree,
    strings: &destack_core::StringPool,
    expression: &Expression,
) -> Option<Option<String>> {
    let Expression::Call {
        left,
        dynamic_arguments,
        ..
    } = expression
    else {
        return None;
    };

    let receiver = tree.get(*left);
    if !is_import_receiver(strings, receiver) {
        return None;
    }

    let first_argument = dynamic_arguments.first()?;
    let first_argument = tree.get(*first_argument);
    let js::Argument::Positional { value } = first_argument else {
        return Some(None);
    };

    let value = tree.get(*value);
    let Expression::ScalarLiteral {
        value: ScalarLiteral::String(target),
    } = value
    else {
        return Some(None);
    };

    Some(Some(string_value(strings, *target)))
}

/// Return whether one expression is the bare `import` receiver.
fn is_import_receiver(strings: &destack_core::StringPool, expression: &Expression) -> bool {
    let Expression::Path {
        path: ScriptPath { segments },
        ..
    } = expression
    else {
        return false;
    };

    if segments.len() != 1 {
        return false;
    }

    string_value(strings, segments[0]) == "import"
}

/// Collect linkage metadata for one generated script module.
fn collect_script_linkage(module: &ScriptModule) -> ScriptLinkage {
    let mut collector = ScriptLinkageCollector {
        linkage: ScriptLinkage::default(),
        strings: module.strings.clone(),
        options: NodeVisitorOptions::default(),
    };

    // walk each root node exactly once
    for root in &module.roots {
        js::walk_any(&mut collector, &module.tree, root.ty, root.id);
    }

    collector.into_linkage()
}

/// Build one minimal source map payload for a generated script module.
fn default_script_map(module: &destack_workspace::Module) -> SourceMapArtifact {
    SourceMapArtifact::empty(module.uri.to_string())
}

/// Render one generated script artifact into output files.
pub fn render_artifact(
    module: &destack_workspace::Module,
    artifact: &ScriptArtifact,
    target: &Target,
    package_dir: &Path,
    root_dir: Option<&Path>,
) -> CodegenJsResult<Vec<OutputFile>> {
    let plan = crate::plan_module_output(module, target, package_dir, root_dir)?;
    let emit = crate::emit::ModuleEmitOutput {
        module: artifact.module.clone(),
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
                let code = crate::print_script_module(target, file.file_type, &emit.module)?;
                OutputContent::javascript(code)
            }
            FileType::TypeScript => {
                let code = crate::print_script_module(target, file.file_type, &emit.module)?;
                OutputContent::typescript(code)
            }
            FileType::Html => {
                let code = crate::print_script_module(target, file.file_type, &emit.module)?;
                OutputContent::html(crate::wrap_html_document(&code))
            }
            FileType::SourceMap => {
                let map = artifact
                    .source_map
                    .clone()
                    .unwrap_or_else(|| default_script_map(module));
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
    if let Some(declaration) = &artifact.declaration {
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

/// Generate one script artifact for a module.
pub fn generate_artifact(
    program: Arc<Program>,
    artifacts: Arc<ArtifactStore>,
    module_id: ModuleId,
    target: &Target,
    profile: ProfileId,
) -> CodegenJsResult<(
    ScriptArtifact,
    Vec<crate::CodegenJsWarning>,
    Vec<CodegenJsError>,
)> {
    // validate target
    if !target.uses_js_generate_pipeline() {
        return Err(CodegenJsError::UnsupportedTarget {
            format: format!("{:?}", target.emit),
            message: Some("expected JS, TS, or HTML".to_string()),
        });
    }

    // get module
    let module_ref = program.modules.get(module_id);
    let module = module_ref.as_ref();
    let ast = artifacts
        .ast(module_id)
        .unwrap_or_else(|| panic!("missing committed AST artifact for {module_id:?}"));
    let dir = artifacts
        .dir_analyzed(module_id, profile)
        .unwrap_or_else(|| panic!("missing committed dir artifact for {module_id:?}"));
    let dir_tree = &dir.tree;
    let dir_roots = dir.roots.as_ref().clone();
    let symbols = &dir.symbols;
    let types = &dir.types;

    // emit one lowered JavaScript module tree
    let emit = crate::emit_module(&module, &ast, dir_tree, &dir_roots, symbols, types, target)?;
    let warnings = emit.warnings;
    let errors = emit.errors;

    // current JS generation only knows that a declaration output exists
    let declaration = if target.declaration && matches!(target.emit, EmitFormat::Js) {
        Some(ScriptDeclaration::default())
    } else {
        None
    };

    let language = match target.emit {
        EmitFormat::Js | EmitFormat::Html => ScriptLanguage::JavaScript,
        EmitFormat::Ts => ScriptLanguage::TypeScript,
        _ => {
            return Err(CodegenJsError::UnsupportedTarget {
                format: format!("{:?}", target.emit),
                message: Some("expected JS, TS, or HTML".to_string()),
            });
        }
    };

    let linkage = collect_script_linkage(&emit.module);

    let artifact = ScriptArtifact {
        language,
        module: emit.module,
        linkage,
        declaration,
        source_map: target
            .emits_source_maps()
            .then(|| default_script_map(module)),
        has_top_level_side_effects: true,
    };

    Ok((artifact, warnings, errors))
}
