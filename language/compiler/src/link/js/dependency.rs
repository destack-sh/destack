use crate::emit::js::{
    Expression, Literal, LocalNodeId, Module, Node, NodeVisitor, Statement, Tree, walk_expression,
    walk_root, walk_statement,
};
use tspp_source::ModuleId;

/// One JS dependency target.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum JsDependencyTarget {
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

impl JsDependencyTarget {
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

/// One static JS import or re-export.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct StaticJsDependency {
    /// The dependency target.
    pub(crate) target: JsDependencyTarget,
}

/// One dynamic JS import.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DynamicJsDependency {
    /// The dependency target when it is a literal specifier.
    pub(crate) target: Option<JsDependencyTarget>,
}

/// Build one JS dependency target.
pub(crate) fn js_dependency_target(
    specifier: &str,
    target_module: Option<ModuleId>,
) -> JsDependencyTarget {
    if let Some(module) = target_module {
        return JsDependencyTarget::Module {
            module,
            specifier: specifier.to_string(),
        };
    }

    JsDependencyTarget::External {
        specifier: specifier.to_string(),
    }
}

/// Collect static JS dependencies from one module.
pub(crate) fn static_js_dependencies(module: &Module) -> Vec<StaticJsDependency> {
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

/// Collect dynamic JS dependencies from one module.
pub(crate) fn dynamic_js_dependencies(module: &Module) -> Vec<DynamicJsDependency> {
    let mut collector = DynamicDependencyCollector::new(module);

    // dynamic imports can appear under any root expression or statement
    for root in &module.roots {
        walk_root(&mut collector, &module.tree, root);
    }

    collector.into_dependencies()
}

/// Collect one static dependency from one statement when present.
fn collect_static_statement_dependency(
    module: &Module,
    statement: &Statement,
    dependencies: &mut Vec<StaticJsDependency>,
) {
    match statement {
        Statement::Import {
            target,
            target_module,
            ..
        } => {
            let specifier = module.strings.get(*target);
            let target = js_dependency_target(specifier, *target_module);

            dependencies.push(StaticJsDependency { target });
        }
        Statement::Export {
            target: Some(target),
            target_module,
            ..
        } => {
            let specifier = module.strings.get(*target);
            let target = js_dependency_target(specifier, *target_module);

            dependencies.push(StaticJsDependency { target });
        }
        _ => {}
    }
}

/// One visitor that collects dynamic JS imports.
struct DynamicDependencyCollector<'a> {
    /// The module whose tree is being inspected.
    module: &'a Module,
    /// The collected dynamic dependencies.
    dependencies: Vec<DynamicJsDependency>,
}

impl<'a> DynamicDependencyCollector<'a> {
    /// Create a collector for one JS module.
    fn new(module: &'a Module) -> Self {
        Self {
            module,
            dependencies: Vec::new(),
        }
    }

    /// Finish the collector and return its dependencies.
    fn into_dependencies(self) -> Vec<DynamicJsDependency> {
        self.dependencies
    }

    /// Resolve one dynamic import call target.
    fn import_call_target(
        &self,
        target: LocalNodeId<Expression>,
        target_module: Option<ModuleId>,
    ) -> Option<JsDependencyTarget> {
        let expression = self.module.tree.get(target);
        let Expression::Literal {
            value: Literal::String(specifier),
        } = expression
        else {
            return None;
        };

        let specifier = self.module.strings.get(*specifier);

        Some(js_dependency_target(specifier, target_module))
    }
}

impl NodeVisitor for DynamicDependencyCollector<'_> {
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
            self.dependencies.push(DynamicJsDependency { target });
        }

        walk_expression(self, tree, id, expression);
    }

    fn visit_statement(&mut self, tree: &Tree, id: LocalNodeId<Statement>, statement: &Statement) {
        walk_statement(self, tree, id, statement);
    }
}
