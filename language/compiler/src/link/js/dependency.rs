use destack_artifact::Script;
use destack_js::{LocalNodeId, Statement};
use destack_source::ModuleId;

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
    /// Create one dependency target.
    pub(crate) fn new(specifier: &str, module: Option<ModuleId>) -> Self {
        match module {
            Some(module) => Self::Module {
                module,
                specifier: specifier.to_string(),
            },
            None => Self::External {
                specifier: specifier.to_string(),
            },
        }
    }

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

/// Collect static JS dependencies from one module.
pub(crate) fn static_js_dependencies(script: &Script) -> Vec<JsDependencyTarget> {
    let module = script.module();
    let mut dependencies = Vec::new();

    // root statements carry static dependency metadata
    for statement_id in module.roots.iter().copied() {
        let statement = module.tree.get(statement_id);
        collect_static_statement_dependency(script, statement_id, statement, &mut dependencies);
    }

    dependencies
}

/// Collect one static dependency from one statement when present.
fn collect_static_statement_dependency(
    script: &Script,
    statement_id: LocalNodeId<Statement>,
    statement: &Statement,
    dependencies: &mut Vec<JsDependencyTarget>,
) {
    let module = script.module();
    let target_module = script.dependency_module(statement_id);
    match statement {
        Statement::Import { source, .. }
        | Statement::ReExport { source, .. }
        | Statement::ExportAll { source, .. } => {
            let specifier = module.strings.get(source.value);
            dependencies.push(JsDependencyTarget::new(specifier, target_module));
        }
        _ => {}
    }
}
