//! JS codegen backend implementation.

use destack_artifact::{
    DynamicScriptDependency, DynamicScriptDependencyTarget, ScriptDependencyKind,
    ScriptDependencyTarget, ScriptLinkage, SourceMapArtifact, StaticScriptDependency,
    StaticScriptDependencyUsage,
};
use destack_codegen_lib::CodegenBackend;
use destack_js::{
    self as js, DependencyKind, Expression, LocalNodeId, NodeTree, NodeVisitor, NodeVisitorOptions,
    Path as ScriptPath, ScalarLiteral, ScriptModule, Statement,
};
use destack_source::ModuleId;
use destack_workspace::{Program, Target};

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
pub(crate) fn collect_script_linkage(module: &ScriptModule) -> ScriptLinkage {
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

/// Return one stable package-relative source path when possible.
fn module_source_map_path(program: &Program, module: &destack_workspace::Module) -> String {
    // prefer package-relative filesystem paths
    if let Some(path) = &module.path {
        let package = program.packages.get(module.package_id);
        let package = package.read();
        let relative = package
            .path
            .as_ref()
            .and_then(|package_path| path.strip_prefix(package_path).ok())
            .unwrap_or(path);
        let relative = relative.to_string_lossy().replace('\\', "/");

        return relative;
    }

    module.uri.to_string()
}

/// Build one minimal source map payload for a generated script module.
pub(crate) fn default_script_map(
    program: &Program,
    module: &destack_workspace::Module,
) -> SourceMapArtifact {
    SourceMapArtifact::empty(module_source_map_path(program, module))
}
