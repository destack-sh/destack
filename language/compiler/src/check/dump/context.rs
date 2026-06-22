use destack_dir as dir;
use destack_source::{ModuleId, Span};

use crate::check::{
    CheckState, ConstraintCause, ConstraintId, Dependency, ObligationId, Origin, Relation, Task,
};

/// Rendering context for check trace values.
pub(in crate::check) struct DumpContext<'a, 'b> {
    /// The check state that owns tables referenced by trace ids.
    pub(in crate::check) check: &'a CheckState<'b>,
}

impl<'a, 'b> DumpContext<'a, 'b> {
    /// Create a dump context for one check state.
    pub(in crate::check) fn new(check: &'a CheckState<'b>) -> Self {
        Self { check }
    }

    /// Return a compact constraint id label.
    pub(in crate::check) fn constraint_label(&self, id: ConstraintId) -> String {
        format!("c{}", id.index())
    }

    /// Return a compact obligation id label.
    pub(in crate::check) fn obligation_label(&self, id: ObligationId) -> String {
        format!("o{}", id.index())
    }

    /// Return a compact type variable label.
    pub(in crate::check) fn variable_label(&self, id: dir::TypeVariableId) -> String {
        format!("v{}", id.index)
    }

    /// Return a compact node label.
    pub(in crate::check) fn node_label(&self, node: dir::GlobalNodeIdAny) -> String {
        let module = self.module_label(node.module_id);
        let kind = node.local_id.ty.name().replace(' ', "_");

        format!("{module}:{kind}#{}", node.local_id.id)
    }

    /// Return the source location for one node.
    pub(in crate::check) fn node_source_label(&self, node: dir::GlobalNodeIdAny) -> String {
        self.node_span(node)
            .and_then(|span| self.span_label(span))
            .unwrap_or_else(|| "unknown".to_string())
    }

    /// Return a compact symbol label.
    pub(in crate::check) fn symbol_label(&self, symbol: dir::GlobalSymbolId) -> String {
        let module = self.module_label(symbol.module_id);
        let name = self.check.format_symbol(symbol);

        format!("{module}:{name}")
    }

    /// Return a compact type label.
    pub(in crate::check) fn type_label(&self, ty: dir::GlobalTypeId) -> String {
        if let Ok(dir::Type::Variable(variable)) = self.check.ty(ty) {
            return self.variable_label(*variable);
        }

        self.check.format_type(ty)
    }

    /// Return a compact type list label.
    pub(in crate::check) fn type_list_label(&self, types: &[dir::GlobalTypeId]) -> String {
        if types.is_empty() {
            return "none".to_string();
        }

        types
            .iter()
            .map(|ty| self.type_label(*ty))
            .collect::<Vec<_>>()
            .join(", ")
    }

    /// Return a compact optional type label.
    pub(in crate::check) fn optional_type_label(&self, ty: Option<dir::GlobalTypeId>) -> String {
        ty.map(|ty| self.type_label(ty))
            .unwrap_or_else(|| "none".to_string())
    }

    /// Return a compact dependency label.
    pub(in crate::check) fn dependency_label(&self, dependency: Dependency) -> String {
        match dependency {
            Dependency::Variable(variable) => self.variable_label(variable),
            Dependency::Decision(node) => self.node_label(node),
        }
    }

    /// Return a compact dependency list label.
    pub(in crate::check) fn dependency_list_label(&self, dependencies: &[Dependency]) -> String {
        if dependencies.is_empty() {
            return "none".to_string();
        }

        dependencies
            .iter()
            .map(|dependency| self.dependency_label(*dependency))
            .collect::<Vec<_>>()
            .join(", ")
    }

    /// Return a compact task label.
    pub(in crate::check) fn task_label(&self, task: Task) -> String {
        match task {
            Task::Relate(constraint) => format!("relate {}", self.constraint_label(constraint)),
            Task::Select(node) => format!("select {}", self.node_label(node)),
            Task::Solve(variable) => format!("solve {}", self.variable_label(variable)),
            Task::Oblige(obligation) => format!("oblige {}", self.obligation_label(obligation)),
        }
    }

    /// Return a compact task list label.
    pub(in crate::check) fn task_list_label(&self, tasks: &[Task]) -> String {
        if tasks.is_empty() {
            return "none".to_string();
        }

        tasks
            .iter()
            .map(|task| self.task_label(*task))
            .collect::<Vec<_>>()
            .join(", ")
    }

    /// Return a compact static key label.
    pub(in crate::check) fn static_key_label(&self, key: &dir::StaticKey) -> String {
        self.check.format_static_key(key)
    }

    /// Return a compact origin label.
    pub(in crate::check) fn origin_label(&self, origin: Origin) -> String {
        match origin {
            Origin::Node(node) => self.node_label(node),
            Origin::Symbol(symbol) => self.symbol_label(symbol),
            Origin::Type(ty) => self.type_label(ty),
        }
    }

    /// Return the source location for one origin.
    pub(in crate::check) fn origin_source_label(&self, origin: Origin) -> String {
        match origin {
            Origin::Node(node) => self.node_source_label(node),
            Origin::Symbol(symbol) => self
                .symbol_source(symbol)
                .map(|node| self.node_source_label(node))
                .unwrap_or_else(|| "unknown".to_string()),
            Origin::Type(ty) => self
                .type_source(ty)
                .map(|node| self.node_source_label(node))
                .unwrap_or_else(|| "unknown".to_string()),
        }
    }

    /// Return a compact relation label.
    pub(in crate::check) fn relation_label(&self, relation: Relation) -> &'static str {
        match relation {
            Relation::Equal => "equal",
            Relation::Assignable => "assignable",
            Relation::Writable => "writable",
            Relation::Castable => "castable",
            Relation::Satisfies => "satisfies",
            Relation::Extends => "extends",
            Relation::Implements => "implements",
        }
    }

    /// Return a compact constraint cause label.
    pub(in crate::check) fn cause_label(&self, cause: ConstraintCause) -> &'static str {
        match cause {
            ConstraintCause::General => "general",
            ConstraintCause::Argument => "argument",
            ConstraintCause::Return => "return",
            ConstraintCause::Yield => "yield",
            ConstraintCause::Condition => "condition",
        }
    }

    /// Return a compact module label.
    fn module_label(&self, module: ModuleId) -> String {
        if let Some(module) = self.check.modules.get(&module) {
            return trim_builtin_uri(module.module.uri.as_ref());
        }

        if let Ok(Some(module)) = self
            .check
            .compiler
            .repository
            .module(self.check.context.revision(), module)
        {
            return trim_builtin_uri(module.uri.as_ref());
        }

        format!("module#{module}")
    }

    /// Return the visible span for one node.
    fn node_span(&self, node: dir::GlobalNodeIdAny) -> Option<Span> {
        if let Some(module) = self.check.modules.get(&node.module_id) {
            return module.view().get_span_by_id(node.local_id.id);
        }

        let external = self.check.external_modules.get(&node.module_id)?;

        external.view().get_span_by_id(node.local_id.id)
    }

    /// Return the declaration node for one symbol.
    fn symbol_source(&self, symbol: dir::GlobalSymbolId) -> Option<dir::GlobalNodeIdAny> {
        if let Some(module) = self.check.modules.get(&symbol.module_id) {
            return module
                .binding_table()
                .get_symbol_maybe(symbol.local_id)
                .and_then(|binding| binding.declaration);
        }

        let external = self.check.external_modules.get(&symbol.module_id)?;

        external
            .bindings
            .get_symbol_maybe(symbol.local_id)
            .and_then(|binding| binding.declaration)
    }

    /// Return the source node for one type.
    fn type_source(&self, ty: dir::GlobalTypeId) -> Option<dir::GlobalNodeIdAny> {
        if let Some(module) = self.check.modules.get(&ty.module_id) {
            let source = module.types.get_type_source(ty.local_id);

            return Some(source.into_global(ty.module_id));
        }

        let external = self.check.external_modules.get(&ty.module_id)?;
        let source = external.types.get_type_source(ty.local_id);

        Some(source.into_global(ty.module_id))
    }

    /// Return the source location for one span.
    fn span_label(&self, span: Span) -> Option<String> {
        let file = self
            .check
            .compiler
            .repository
            .file(self.check.context.revision(), span.file)
            .ok()??;
        let (line, column) = file.get_position(span.start)?;
        let line = line + 1;
        let column = column + 1;

        Some(format!(
            "{}:{line}:{column}",
            trim_builtin_uri(file.uri.as_ref())
        ))
    }
}

/// Trim builtin URIs in human trace output.
fn trim_builtin_uri(uri: &str) -> String {
    uri.strip_prefix("destack://")
        .map(|path| format!("/{path}"))
        .unwrap_or_else(|| uri.to_string())
}
