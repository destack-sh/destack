use destack_dir as dir;
use destack_source::{ModuleId, Span};

use crate::check::{
    BoundMode, CheckState, ConstraintId, Dependency, FlowPointId, FlowSite, ObligationId, Origin,
    Relation, TypeBound, ValueUse, Widening,
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
        match self.check.solver.variable(id) {
            Ok(state) => {
                let origin = self.check.solver.origin(state.origin);
                let module = self.module_label(origin.module());

                format!("{module}:v{}", id.0)
            }
            Err(_) => format!("v{}", id.0),
        }
    }

    /// Return a compact node label.
    pub(in crate::check) fn node_label(&self, node: dir::GlobalNodeIdAny) -> String {
        let module = self.module_label(node.module_id);
        let kind = node.local_id.ty.name().replace(' ', "_");

        format!("{module}:{kind}#{}", node.local_id.id)
    }

    /// Return a compact flow point label.
    pub(in crate::check) fn flow_label(&self, flow: FlowPointId) -> String {
        format!("f{}", flow.index())
    }

    /// Return a compact source use label.
    pub(in crate::check) fn flow_site_label(&self, site: FlowSite) -> String {
        format!(
            "{}@{}",
            self.node_label(site.node),
            self.flow_label(site.flow)
        )
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
            return self.variable_label(variable);
        }

        self.check.format_type(ty)
    }

    /// Return a compact type-bound list label.
    pub(in crate::check) fn type_bound_list_label(&self, bounds: &[TypeBound]) -> String {
        if bounds.is_empty() {
            return "none".to_string();
        }

        bounds
            .iter()
            .map(|bound| self.type_label(bound.ty))
            .collect::<Vec<_>>()
            .join(", ")
    }

    /// Return a compact widening policy label.
    pub(in crate::check) fn widening_label(&self, widening: Widening) -> &'static str {
        match widening {
            Widening::Preserve => "preserve",
            Widening::Widen => "widen",
            Widening::WidenWrites => "widen-writes",
        }
    }

    /// Return a compact bound mode label.
    pub(in crate::check) fn bound_mode_label(&self, mode: BoundMode) -> &'static str {
        match mode {
            BoundMode::Strong => "strong",
            BoundMode::Weak => "weak",
        }
    }

    /// Return a compact dependency label.
    pub(in crate::check) fn dependency_label(&self, dependency: Dependency) -> String {
        match dependency {
            Dependency::Variable(variable) => self.variable_label(variable),
            Dependency::SymbolType(symbol) => self.symbol_label(symbol),
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

    /// Return one variable's origin label.
    pub(in crate::check) fn variable_origin_label(&self, variable: dir::TypeVariableId) -> String {
        let Ok(state) = self.check.solver.variable(variable) else {
            return "unknown".to_string();
        };

        self.origin_label(self.check.solver.origin(state.origin))
    }

    /// Return one variable's source location.
    pub(in crate::check) fn variable_source_label(&self, variable: dir::TypeVariableId) -> String {
        let Ok(state) = self.check.solver.variable(variable) else {
            return "unknown".to_string();
        };

        self.origin_source_label(self.check.solver.origin(state.origin))
    }

    /// Return a compact static key label.
    pub(in crate::check) fn static_key_label(&self, key: &dir::StaticKey) -> String {
        self.check.format_static_key(key)
    }

    /// Return a compact origin label.
    pub(in crate::check) fn origin_label(&self, origin: Origin) -> String {
        match origin {
            Origin::Node(node, _) => self.node_label(node),
            Origin::Symbol(symbol) => self.symbol_label(symbol),
        }
    }

    /// Return the source location for one origin.
    pub(in crate::check) fn origin_source_label(&self, origin: Origin) -> String {
        match origin {
            Origin::Node(node, _) => self.node_source_label(node),
            Origin::Symbol(symbol) => self
                .symbol_source(symbol)
                .map(|node| self.node_source_label(node))
                .unwrap_or_else(|| "unknown".to_string()),
        }
    }

    /// Return a compact relation label.
    pub(in crate::check) fn relation_label(&self, relation: Relation) -> &'static str {
        match relation {
            Relation::Equal => "equal",
            Relation::Assignable => "assignable",
            Relation::Widens => "widens",
            Relation::MethodAssignable => "method-assignable",
            Relation::Writable => "writable",
            Relation::Castable => "castable",
            Relation::Satisfies => "satisfies",
            Relation::Extends => "extends",
            Relation::Implements => "implements",
        }
    }

    /// Return a compact value use label.
    pub(in crate::check) fn value_use_label(&self, use_: Option<ValueUse>) -> &'static str {
        match use_ {
            None => "check",
            Some(ValueUse::Store) => "store",
            Some(ValueUse::Argument) => "argument",
            Some(ValueUse::Output) => "output",
            Some(ValueUse::Condition) => "condition",
            Some(ValueUse::Satisfies) => "satisfies",
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
