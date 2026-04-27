use destack_artifact::{
    DynamicScriptDependency, DynamicScriptDependencyTarget, ScriptDependencyKind,
    ScriptDependencyTarget, ScriptLinkage, StaticScriptDependency, StaticScriptDependencyUsage,
};
use destack_codegen_lib::CodegenBackend;
use destack_core::{StringId, StringPool};
use destack_js as js;
use destack_source::ModuleId;
use destack_workspace::Target;

use crate::ScriptModule;

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

impl JsBackend {
    /// Collect linkage metadata for one generated script module.
    pub(crate) fn collect_script_linkage(module: &ScriptModule) -> ScriptLinkage {
        ScriptLinkageCollector::collect(module)
    }
}

/// One visitor that collects script linkage metadata.
#[derive(Debug, Clone, Default)]
struct ScriptLinkageCollector {
    /// The collected linkage metadata.
    linkage: ScriptLinkage,
    /// The shared string pool for the module.
    strings: StringPool,
    /// Visitor options.
    options: js::NodeVisitorOptions,
}

impl ScriptLinkageCollector {
    /// Collect linkage metadata for one generated script module.
    fn collect(module: &ScriptModule) -> ScriptLinkage {
        let mut collector = Self {
            linkage: ScriptLinkage::default(),
            strings: module.strings.clone(),
            options: js::NodeVisitorOptions::default(),
        };

        // walk each root through the visitor entry points
        js::walk_roots(&mut collector, &module.tree, &module.roots);

        collector.into_linkage()
    }

    /// Return one interned string as owned text.
    fn string_value(&self, id: StringId) -> String {
        self.strings.get(id).to_string()
    }

    /// Lower one JS dependency kind into artifact linkage metadata.
    fn dependency_kind(kind: js::DependencyKind) -> ScriptDependencyKind {
        match kind {
            js::DependencyKind::Type => ScriptDependencyKind::Type,
            js::DependencyKind::Value => ScriptDependencyKind::Value,
        }
    }

    /// Lower one static JS dependency target into artifact metadata.
    fn static_dependency_target(
        specifier: String,
        target_module: Option<ModuleId>,
    ) -> ScriptDependencyTarget {
        if let Some(module) = target_module {
            return ScriptDependencyTarget::Module { module, specifier };
        }

        ScriptDependencyTarget::External { specifier }
    }

    /// Lower one dynamic JS dependency target into artifact metadata.
    fn dynamic_dependency_target(
        specifier: Option<String>,
        target_module: Option<ModuleId>,
    ) -> DynamicScriptDependencyTarget {
        if let Some(specifier) = specifier {
            return DynamicScriptDependencyTarget::Resolved(Self::static_dependency_target(
                specifier,
                target_module,
            ));
        }

        DynamicScriptDependencyTarget::Opaque
    }

    /// Return the resolved target module carried by one script statement.
    fn statement_target_module(statement: &js::Statement) -> Option<ModuleId> {
        match statement {
            js::Statement::Import { target_module, .. }
            | js::Statement::Export { target_module, .. } => *target_module,
            _ => None,
        }
    }

    /// Return the static target for one dynamic import expression when it is known.
    fn dynamic_import_target(
        tree: &js::Tree,
        strings: &StringPool,
        expression: &js::Expression,
    ) -> Option<(Option<String>, Option<ModuleId>)> {
        let (target_expression, target_module) = match expression {
            js::Expression::ImportCall {
                target,
                target_module,
                ..
            } => (*target, *target_module),
            js::Expression::Call {
                left, arguments, ..
            } => {
                let receiver = tree.get(*left);
                if !Self::is_import_receiver(strings, receiver) {
                    return None;
                }

                let first_argument = arguments.first()?;
                let first_argument = tree.get(*first_argument);
                let js::Argument::Positional { value } = first_argument else {
                    return Some((None, None));
                };

                (*value, None)
            }
            _ => return None,
        };

        let value = tree.get(target_expression);
        let js::Expression::ScalarLiteral {
            value: js::ScalarLiteral::String(target),
        } = value
        else {
            return Some((None, target_module));
        };

        Some((Some(strings.get(*target).to_string()), target_module))
    }

    /// Return whether one expression is the bare `import` receiver.
    fn is_import_receiver(strings: &StringPool, expression: &js::Expression) -> bool {
        let js::Expression::Path {
            path: js::Path { segments },
            ..
        } = expression
        else {
            return false;
        };

        if segments.len() != 1 {
            return false;
        }

        strings.get(segments[0]) == "import"
    }

    /// Finish the collector and return the linkage metadata.
    fn into_linkage(self) -> ScriptLinkage {
        self.linkage
    }
}

impl js::NodeVisitor for ScriptLinkageCollector {
    fn options(&self) -> &js::NodeVisitorOptions {
        &self.options
    }

    fn visit_statement(
        &mut self,
        tree: &js::Tree,
        id: js::LocalNodeId<js::Statement>,
        statement: &js::Statement,
    ) {
        // collect top-level static linkage first
        match statement {
            js::Statement::Import { kind, target, .. } => {
                self.linkage
                    .static_dependencies
                    .push(StaticScriptDependency {
                        usage: StaticScriptDependencyUsage::Import,
                        kind: Self::dependency_kind(*kind),
                        target: Self::static_dependency_target(
                            self.string_value(*target),
                            Self::statement_target_module(statement),
                        ),
                    });
            }
            js::Statement::Export {
                kind,
                target: Some(target),
                ..
            } => {
                self.linkage
                    .static_dependencies
                    .push(StaticScriptDependency {
                        usage: StaticScriptDependencyUsage::Reexport,
                        kind: Self::dependency_kind(*kind),
                        target: Self::static_dependency_target(
                            self.string_value(*target),
                            Self::statement_target_module(statement),
                        ),
                    });
            }
            _ => {}
        }

        js::walk_statement(self, tree, id, statement);
    }

    fn visit_expression(
        &mut self,
        tree: &js::Tree,
        id: js::LocalNodeId<js::Expression>,
        expression: &js::Expression,
    ) {
        // collect dynamic import calls while the tree is still structured
        if let Some((target, target_module)) =
            Self::dynamic_import_target(tree, &self.strings, expression)
        {
            self.linkage
                .dynamic_dependencies
                .push(DynamicScriptDependency {
                    target: Self::dynamic_dependency_target(target, target_module),
                });
        }

        js::walk_expression(self, tree, id, expression);
    }
}
