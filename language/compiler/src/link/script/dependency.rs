use destack_codegen_js::{
    DependencyForm, Expression, LocalNodeId, Module, Node, NodeVisitor, NodeVisitorOptions,
    ScalarLiteral, Statement, Tree, walk_expression, walk_statement,
};
use destack_source::{Loader, ModuleId, StringId};

/// The relation between linked script modules.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(crate) enum ModuleRelation {
    /// Binding import.
    Import,
    /// Binding re-export.
    ReExport,
    /// Non-binding module reference.
    Reference,
}

/// One resolved relation from a source module to another source module.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(crate) struct ModuleEdge {
    /// The target module.
    pub(crate) target: ModuleId,
    /// The source relation.
    pub(crate) relation: ModuleRelation,
    /// The authored specifier when the relation has one.
    pub(crate) specifier: Option<StringId>,
    /// The local source site when the relation came from a document or style node.
    pub(crate) site: Option<u32>,
    /// The requested loader override when the relation has one.
    pub(crate) loader: Option<Loader>,
}

impl ModuleEdge {
    /// Create one module edge.
    pub(crate) fn new(target: ModuleId, relation: ModuleRelation) -> Self {
        Self {
            target,
            relation,
            specifier: None,
            site: None,
            loader: None,
        }
    }

    /// Return this edge with an authored specifier.
    pub(crate) fn with_specifier(mut self, specifier: Option<StringId>) -> Self {
        self.specifier = specifier;
        self
    }

    /// Return this edge with a loader override.
    pub(crate) fn with_loader(mut self, loader: Option<Loader>) -> Self {
        self.loader = loader;
        self
    }
}

/// One script dependency target.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ScriptDependencyTarget {
    /// One dependency resolved to another source module.
    Module {
        /// The target module.
        module: ModuleId,
        /// The original import specifier.
        specifier: String,
    },
    /// One dependency retained as an external import.
    External {
        /// The original import specifier.
        specifier: String,
    },
}

impl ScriptDependencyTarget {
    /// Return the original import specifier.
    pub(crate) fn specifier(&self) -> &str {
        match self {
            Self::Module { specifier, .. } | Self::External { specifier } => specifier,
        }
    }

    /// Return the resolved module when this dependency is internal.
    pub(crate) fn module(&self) -> Option<ModuleId> {
        match self {
            Self::Module { module, .. } => Some(*module),
            Self::External { .. } => None,
        }
    }
}

/// One static script import or re-export.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct StaticScriptDependency {
    /// The dependency form.
    pub(crate) form: DependencyForm,
    /// The dependency target.
    pub(crate) target: ScriptDependencyTarget,
}

/// One dynamic script import.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DynamicScriptDependency {
    /// The dependency target when it is a literal specifier.
    pub(crate) target: Option<ScriptDependencyTarget>,
}

/// Build one script dependency target.
pub(crate) fn script_dependency_target(
    specifier: &str,
    target_module: Option<ModuleId>,
) -> ScriptDependencyTarget {
    if let Some(module) = target_module {
        return ScriptDependencyTarget::Module {
            module,
            specifier: specifier.to_string(),
        };
    }

    ScriptDependencyTarget::External {
        specifier: specifier.to_string(),
    }
}

/// Collect static script dependencies from one module.
pub(crate) fn static_script_dependencies(module: &Module) -> Vec<StaticScriptDependency> {
    let mut dependencies = Vec::new();

    // root statements carry static dependency metadata
    for root in &module.roots {
        if root.ty != Statement::TYPE {
            continue;
        }

        let statement_id = LocalNodeId::<Statement>::new(root.id);
        let statement = module.tree.get(statement_id);
        collect_static_statement_dependency(module, statement, &mut dependencies);
    }

    dependencies
}

/// Collect dynamic script dependencies from one module.
pub(crate) fn dynamic_script_dependencies(module: &Module) -> Vec<DynamicScriptDependency> {
    let mut collector = DynamicDependencyCollector::new(module);

    // dynamic imports can appear under any root expression or statement
    for root in &module.roots {
        destack_codegen_js::walk_root(&mut collector, &module.tree, root);
    }

    collector.into_dependencies()
}

/// Collect one static dependency from one statement when present.
fn collect_static_statement_dependency(
    module: &Module,
    statement: &Statement,
    dependencies: &mut Vec<StaticScriptDependency>,
) {
    match statement {
        Statement::Import {
            form,
            target,
            target_module,
            ..
        } => {
            let specifier = module.strings.get(*target);
            let target = script_dependency_target(&specifier, *target_module);

            dependencies.push(StaticScriptDependency {
                form: *form,
                target,
            });
        }
        Statement::Export {
            form,
            target: Some(target),
            target_module,
            ..
        } => {
            let specifier = module.strings.get(*target);
            let target = script_dependency_target(&specifier, *target_module);

            dependencies.push(StaticScriptDependency {
                form: *form,
                target,
            });
        }
        _ => {}
    }
}

/// One visitor that collects dynamic script imports.
struct DynamicDependencyCollector<'a> {
    /// The module whose tree is being inspected.
    module: &'a Module,
    /// The collected dynamic dependencies.
    dependencies: Vec<DynamicScriptDependency>,
    /// Visitor options.
    options: NodeVisitorOptions,
}

impl<'a> DynamicDependencyCollector<'a> {
    /// Create a collector for one script module.
    fn new(module: &'a Module) -> Self {
        Self {
            module,
            dependencies: Vec::new(),
            options: NodeVisitorOptions::default(),
        }
    }

    /// Finish the collector and return its dependencies.
    fn into_dependencies(self) -> Vec<DynamicScriptDependency> {
        self.dependencies
    }

    /// Resolve one dynamic import call target.
    fn import_call_target(
        &self,
        target: LocalNodeId<Expression>,
        target_module: Option<ModuleId>,
    ) -> Option<ScriptDependencyTarget> {
        let expression = self.module.tree.get(target);
        let Expression::ScalarLiteral {
            value: ScalarLiteral::String(specifier),
        } = expression
        else {
            return None;
        };

        let specifier = self.module.strings.get(*specifier);

        Some(script_dependency_target(&specifier, target_module))
    }
}

impl NodeVisitor for DynamicDependencyCollector<'_> {
    fn options(&self) -> &NodeVisitorOptions {
        &self.options
    }

    fn visit_expression(
        &mut self,
        tree: &Tree,
        id: LocalNodeId<Expression>,
        expression: &Expression,
    ) {
        // collect dynamic imports before visiting nested children
        if let Expression::ImportCall {
            target,
            target_module,
            ..
        } = expression
        {
            let target = self.import_call_target(*target, *target_module);
            self.dependencies.push(DynamicScriptDependency { target });
        }

        walk_expression(self, tree, id, expression);
    }

    fn visit_statement(&mut self, tree: &Tree, id: LocalNodeId<Statement>, statement: &Statement) {
        walk_statement(self, tree, id, statement);
    }
}
