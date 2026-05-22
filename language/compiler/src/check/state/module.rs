use std::sync::Arc;

use destack_artifact::{
    DiagnosticAnchor, DirBound, DirCheckedModule, DirExpanded, DirExported, DirImported, DirParsed,
    DirResolved,
};
use destack_dir as dir;
use destack_source::{ModuleId, ProfileId};
use indexmap::IndexMap;
use smallvec::SmallVec;

use crate::CheckError;

use super::{
    CheckFailure, Constraint, Obligation, Step, TypeRelation, Variable, VariableId, VariableKind,
    VariableOrigin, VariableValue,
};

/// Check state for one module inside a checked component.
#[derive(Debug)]
pub(in crate::check) struct CheckModuleState {
    /// The requested module.
    module: ModuleId,
    /// The requested profile.
    profile: ProfileId,

    /// The parsed DIR input.
    parsed: Arc<DirParsed>,
    /// The bound DIR input.
    bound: Arc<DirBound>,
    /// The imported DIR input.
    imported: Arc<DirImported>,
    /// The exported DIR input.
    exported: Arc<DirExported>,
    /// The resolved DIR input.
    resolved: Arc<DirResolved>,
    /// The expanded DIR input.
    expanded: Arc<DirExpanded>,

    /// Checked type segment.
    types: dir::TypeSegment,
    /// Checked static value segment.
    statics: dir::StaticSegment,
    /// Checked resolution segment.
    resolutions: dir::ResolutionSegment,
    /// Checked instance segment.
    instances: dir::InstanceSegment,
    /// Checked relation segment.
    relations: dir::RelationSegment,
    /// Checked extension segment.
    extensions: dir::ExtensionSegment,
    /// Checked layout segment.
    layouts: dir::LayoutSegment,
    /// Checked capture segment.
    captures: dir::CaptureSegment,

    /// The visitor options used while walking this module.
    options: dir::NodeVisitorOptions,

    /// Check variables owned by this module.
    variables: Vec<Variable>,
    /// Type variables keyed by node.
    node_type_variables: IndexMap<dir::GlobalNodeIdAny, VariableId>,
    /// Type variables keyed by symbol.
    symbol_type_variables: IndexMap<dir::GlobalSymbolId, VariableId>,
    /// Static variables keyed by node.
    node_static_variables: IndexMap<dir::GlobalNodeIdAny, VariableId>,
    /// Static variables keyed by symbol.
    symbol_static_variables: IndexMap<dir::GlobalSymbolId, VariableId>,
    /// Layout variables keyed by local type.
    layout_variables: IndexMap<dir::LocalTypeId, VariableId>,

    /// Constraints produced by walking DIR.
    constraints: Vec<Constraint>,
    /// Obligations produced by walking DIR.
    obligations: Vec<Obligation>,
    /// Failures found during solving.
    failures: Vec<CheckFailure>,

    /// Recoverable diagnostics collected while checking.
    diagnostics: Vec<CheckError>,
}

impl CheckModuleState {
    /// Create check state for one requested module.
    pub(in crate::check) fn new(
        module: ModuleId,
        profile: ProfileId,
        parsed: Arc<DirParsed>,
        bound: Arc<DirBound>,
        imported: Arc<DirImported>,
        exported: Arc<DirExported>,
        resolved: Arc<DirResolved>,
        expanded: Arc<DirExpanded>,
    ) -> Self {
        let types = dir::TypeSegment::from_base(&expanded.types);
        let statics = dir::StaticSegment::from_base(&expanded.statics);
        let resolutions = dir::ResolutionSegment::new(module);
        let instances = dir::InstanceSegment::new(module);
        let relations = dir::RelationSegment::new(module);
        let extensions = dir::ExtensionSegment::new(module);
        let layouts = dir::LayoutSegment::new(module);
        let captures = dir::CaptureSegment::new(module);

        Self {
            module,
            profile,
            parsed,
            bound,
            imported,
            exported,
            resolved,
            expanded,
            types,
            statics,
            resolutions,
            instances,
            relations,
            extensions,
            layouts,
            captures,
            options: dir::NodeVisitorOptions::default(),
            variables: Vec::new(),
            node_type_variables: IndexMap::new(),
            symbol_type_variables: IndexMap::new(),
            node_static_variables: IndexMap::new(),
            symbol_static_variables: IndexMap::new(),
            layout_variables: IndexMap::new(),
            constraints: Vec::new(),
            obligations: Vec::new(),
            failures: Vec::new(),
            diagnostics: Vec::new(),
        }
    }

    /// Return the requested module.
    pub(in crate::check) fn module(&self) -> ModuleId {
        self.module
    }

    /// Return the requested profile.
    pub(in crate::check) fn profile(&self) -> ProfileId {
        self.profile
    }

    /// Return the parsed DIR input.
    pub(in crate::check) fn parsed(&self) -> &DirParsed {
        &self.parsed
    }

    /// Return the parsed DIR input handle.
    pub(in crate::check) fn parsed_arc(&self) -> Arc<DirParsed> {
        Arc::clone(&self.parsed)
    }

    /// Return the bound DIR input.
    pub(in crate::check) fn bound(&self) -> &DirBound {
        &self.bound
    }

    /// Return the imported DIR input.
    pub(in crate::check) fn imported(&self) -> &DirImported {
        &self.imported
    }

    /// Return the exported DIR input.
    pub(in crate::check) fn exported(&self) -> &DirExported {
        &self.exported
    }

    /// Return the resolved DIR input.
    pub(in crate::check) fn resolved(&self) -> &DirResolved {
        &self.resolved
    }

    /// Return the expanded DIR input.
    pub(in crate::check) fn expanded(&self) -> &DirExpanded {
        &self.expanded
    }

    /// Return the expanded DIR input handle.
    pub(in crate::check) fn expanded_arc(&self) -> Arc<DirExpanded> {
        Arc::clone(&self.expanded)
    }

    /// Return the cumulative binding table visible to check.
    pub(in crate::check) fn binding_table(&self) -> dir::BindingTable<'static> {
        self.expanded.binding_table(&self.bound)
    }

    /// Return the DIR visitor options.
    pub(in crate::check) fn visitor_options(&self) -> &dir::NodeVisitorOptions {
        &self.options
    }

    /// Return the cumulative type table visible to check inputs.
    pub(in crate::check) fn input_type_table(&self) -> dir::TypeTable<'static> {
        self.expanded.type_table(&self.bound)
    }

    /// Return the cumulative static table visible to check inputs.
    pub(in crate::check) fn input_static_table(&self) -> dir::StaticTable<'static> {
        self.expanded.static_table(&self.bound)
    }

    /// Return the symbol introduced by a source declaration node.
    pub(in crate::check) fn declaration_symbol(
        &self,
        node: dir::LocalNodeIdAny,
    ) -> Option<dir::GlobalSymbolId> {
        self.binding_table()
            .symbol_for_declaration(node.into_global(self.module()))
            .map(|symbol| symbol.into_global(self.module()))
    }

    /// Add one checked type.
    pub(in crate::check) fn intern_type(
        &mut self,
        ty: dir::Type,
        source: dir::LocalNodeIdAny,
    ) -> dir::LocalTypeId {
        self.types.insert_type_from_any(ty, source)
    }

    /// Add or reuse one checked static value.
    pub(in crate::check) fn intern_static(&mut self, term: dir::StaticTerm) -> dir::LocalStaticId {
        let table = self.input_static_table();

        table.intern_static(&mut self.statics, term)
    }

    /// Return one visible type by id.
    pub(in crate::check) fn get_type(&self, type_id: dir::LocalTypeId) -> dir::Type {
        if let Some(ty) = self.types.get_type_maybe(type_id) {
            return ty.clone();
        }

        self.input_type_table().get_type(type_id).clone()
    }

    /// Return one visible static value by id.
    pub(in crate::check) fn get_static(&self, static_id: dir::LocalStaticId) -> dir::StaticTerm {
        if let Some(term) = self.statics.get_static_maybe(static_id) {
            return term.clone();
        }

        self.input_static_table().get_static(static_id).clone()
    }

    /// Return the effective checked type id for one node.
    pub(in crate::check) fn node_type_id(
        &self,
        node_id: dir::GlobalNodeIdAny,
    ) -> Option<dir::LocalTypeId> {
        self.types
            .get_node_type_id(node_id)
            .or_else(|| self.input_type_table().get_node_type_id(node_id))
    }

    /// Return the effective checked type id for one symbol.
    pub(in crate::check) fn symbol_type_id(
        &self,
        symbol_id: dir::GlobalSymbolId,
    ) -> Option<dir::LocalTypeId> {
        self.types
            .get_symbol_type_id(symbol_id)
            .or_else(|| self.input_type_table().get_symbol_type_id(symbol_id))
    }

    /// Return the effective checked static value id for one symbol.
    pub(in crate::check) fn symbol_static_id(
        &self,
        symbol_id: dir::GlobalSymbolId,
    ) -> Option<dir::LocalStaticId> {
        self.statics
            .get_symbol_static_id(symbol_id)
            .or_else(|| self.input_static_table().get_symbol_static_id(symbol_id))
    }

    /// Push one check variable.
    pub(in crate::check) fn push_variable(
        &mut self,
        kind: VariableKind,
        origin: VariableOrigin,
    ) -> VariableId {
        let id = VariableId::new(self.module, self.variables.len() as u32);
        let variable = Variable::new(id, kind, origin);
        self.variables.push(variable);

        id
    }

    /// Return one variable.
    pub(in crate::check) fn variable(&self, id: VariableId) -> &Variable {
        assert_eq!(
            id.module, self.module,
            "check variable belongs to another module"
        );

        &self.variables[id.index as usize]
    }

    /// Return one variable mutably.
    pub(in crate::check) fn variable_mut(&mut self, id: VariableId) -> &mut Variable {
        assert_eq!(
            id.module, self.module,
            "check variable belongs to another module"
        );

        &mut self.variables[id.index as usize]
    }

    /// Return the solved value for one variable.
    pub(in crate::check) fn variable_value(&self, id: VariableId) -> Option<&VariableValue> {
        self.variable(id).value.as_ref()
    }

    /// Bind the solved value for one variable.
    pub(in crate::check) fn bind_variable(&mut self, id: VariableId, value: VariableValue) -> Step {
        let variable = self.variable_mut(id);
        let Some(existing) = &variable.value else {
            variable.value = Some(value);

            return Step::applied(id);
        };

        if existing == &value {
            Step::Pending
        } else {
            Step::Failed(CheckFailure::VariableConflict { variable: id })
        }
    }

    /// Return one variable's source node for diagnostics.
    pub(in crate::check) fn variable_source_node(&self, id: VariableId) -> dir::LocalNodeIdAny {
        match &self.variable(id).origin {
            VariableOrigin::Node(node) if node.module_id == self.module => node.local_id,
            VariableOrigin::StaticExpression(node) if node.module_id == self.module => {
                node.local_id.into_any()
            }
            VariableOrigin::TypeExpression(node) if node.module_id == self.module => {
                node.local_id.into_any()
            }
            VariableOrigin::Symbol(symbol) if symbol.module_id == self.module => {
                let binding_table = self.binding_table();
                let symbol = binding_table.get_symbol(symbol.local_id);

                symbol
                    .declaration
                    .filter(|declaration| declaration.module_id == self.module)
                    .map(|declaration| declaration.local_id)
                    .unwrap_or(self.bound.module_node)
            }
            VariableOrigin::Layout { .. }
            | VariableOrigin::Synthetic
            | VariableOrigin::Symbol(_)
            | VariableOrigin::Node(_) => self.bound.module_node,
            VariableOrigin::TypeExpression(_) | VariableOrigin::StaticExpression(_) => {
                self.bound.module_node
            }
        }
    }

    /// Return the solved type for one variable.
    pub(in crate::check) fn variable_type_value(&self, id: VariableId) -> Option<dir::LocalTypeId> {
        match self.variable_value(id) {
            Some(VariableValue::Type(type_id)) => Some(*type_id),
            _ => None,
        }
    }

    /// Return a symbol as visible from this module.
    pub(in crate::check) fn resolve_imported_symbol(
        &self,
        symbol: dir::LocalSymbolId,
    ) -> dir::GlobalSymbolId {
        let target = self
            .resolved
            .imports
            .symbol_target(symbol)
            .and_then(|target| match target {
                dir::ImportTarget::Symbol(symbol) => Some(symbol),
                dir::ImportTarget::Namespace(_) => None,
            });

        target.unwrap_or_else(|| symbol.into_global(self.module))
    }

    /// Return or create a type variable for one node.
    pub(in crate::check) fn node_type_variable(
        &mut self,
        node: dir::GlobalNodeIdAny,
    ) -> VariableId {
        if let Some(variable) = self.node_type_variables.get(&node).copied() {
            return variable;
        }

        let variable = self.push_variable(VariableKind::Type, VariableOrigin::Node(node));
        self.node_type_variables.insert(node, variable);

        variable
    }

    /// Return or create a type variable for one symbol.
    pub(in crate::check) fn symbol_type_variable(
        &mut self,
        symbol: dir::GlobalSymbolId,
    ) -> VariableId {
        if let Some(variable) = self.symbol_type_variables.get(&symbol).copied() {
            return variable;
        }

        let variable = self.push_variable(VariableKind::Type, VariableOrigin::Symbol(symbol));
        if let Some(type_id) = self.symbol_type_id(symbol) {
            let _ = self.bind_variable(variable, VariableValue::Type(type_id));
        }
        self.symbol_type_variables.insert(symbol, variable);

        variable
    }

    /// Return or create a static variable for one node.
    pub(in crate::check) fn node_static_variable(
        &mut self,
        node: dir::GlobalNodeIdAny,
    ) -> VariableId {
        if let Some(variable) = self.node_static_variables.get(&node).copied() {
            return variable;
        }

        let variable = self.push_variable(VariableKind::Static, VariableOrigin::Node(node));
        self.node_static_variables.insert(node, variable);

        variable
    }

    /// Return or create a static variable for one symbol.
    pub(in crate::check) fn symbol_static_variable(
        &mut self,
        symbol: dir::GlobalSymbolId,
    ) -> VariableId {
        if let Some(variable) = self.symbol_static_variables.get(&symbol).copied() {
            return variable;
        }

        let variable = self.push_variable(VariableKind::Static, VariableOrigin::Symbol(symbol));
        if let Some(static_id) = self.symbol_static_id(symbol) {
            let _ = self.bind_variable(variable, VariableValue::Static(static_id));
        }
        self.symbol_static_variables.insert(symbol, variable);

        variable
    }

    /// Return or create a layout variable for one local type.
    pub(in crate::check) fn layout_variable(&mut self, type_id: dir::LocalTypeId) -> VariableId {
        if let Some(variable) = self.layout_variables.get(&type_id).copied() {
            return variable;
        }

        let variable =
            self.push_variable(VariableKind::Layout, VariableOrigin::Layout { ty: type_id });
        self.layout_variables.insert(type_id, variable);

        variable
    }

    /// Add one constraint.
    pub(in crate::check) fn push_constraint(&mut self, constraint: Constraint) {
        self.constraints.push(constraint);
    }

    /// Return collected constraints.
    pub(in crate::check) fn constraints(&self) -> &[Constraint] {
        &self.constraints
    }

    /// Add one obligation.
    pub(in crate::check) fn push_obligation(&mut self, obligation: Obligation) {
        self.obligations.push(obligation);
    }

    /// Drain collected obligations.
    pub(in crate::check) fn take_obligations(&mut self) -> Vec<Obligation> {
        std::mem::take(&mut self.obligations)
    }

    /// Add one check failure.
    pub(in crate::check) fn push_failure(&mut self, failure: CheckFailure) {
        self.failures.push(failure);
    }

    /// Drain check failures.
    pub(in crate::check) fn take_failures(&mut self) -> Vec<CheckFailure> {
        std::mem::take(&mut self.failures)
    }

    /// Return the source anchor for one local node.
    pub(in crate::check) fn anchor_node(&self, node_id: dir::LocalNodeIdAny) -> DiagnosticAnchor {
        self.parsed
            .tree
            .get_span_by_id(node_id.id)
            .map(DiagnosticAnchor::from)
            .unwrap_or_else(|| DiagnosticAnchor::from(self.module))
    }

    /// Return the source anchor for one global node.
    pub(in crate::check) fn anchor_global_node(
        &self,
        node_id: dir::GlobalNodeIdAny,
    ) -> DiagnosticAnchor {
        if node_id.module_id == self.module {
            self.anchor_node(node_id.local_id)
        } else {
            self.anchor_module()
        }
    }

    /// Return the source anchor for the whole module.
    pub(in crate::check) fn anchor_module(&self) -> DiagnosticAnchor {
        DiagnosticAnchor::from(self.module)
    }

    /// Add one recoverable check diagnostic.
    pub(in crate::check) fn push_diagnostic(&mut self, diagnostic: CheckError) {
        self.diagnostics.push(diagnostic);
    }

    /// Drain recoverable check diagnostics.
    pub(in crate::check) fn take_diagnostics(&mut self) -> Vec<CheckError> {
        std::mem::take(&mut self.diagnostics)
    }

    /// Return the relation diagnostic for one type relation.
    pub(in crate::check) fn type_relation_diagnostic(
        &self,
        relation: TypeRelation,
        anchor: DiagnosticAnchor,
    ) -> CheckError {
        match relation {
            TypeRelation::Equal | TypeRelation::Assignable => CheckError::NotAssignable {
                anchor,
                module: self.module,
            },
            TypeRelation::Satisfies => CheckError::ConstraintNotSatisfied {
                anchor,
                module: self.module,
            },
            TypeRelation::Extends => CheckError::DoesNotExtend {
                anchor,
                module: self.module,
            },
            TypeRelation::Implements => CheckError::DoesNotImplement {
                anchor,
                module: self.module,
            },
        }
    }

    /// Finish check state into checked DIR.
    pub(in crate::check) fn finish(mut self) -> DirCheckedModule {
        // write solved node and symbol facts
        for variable in &self.variables {
            match (&variable.origin, &variable.value) {
                (VariableOrigin::Node(node), Some(VariableValue::Type(type_id))) => {
                    self.types.set_node_type(*node, *type_id);
                }
                (VariableOrigin::Symbol(symbol), Some(VariableValue::Type(type_id)))
                    if symbol.module_id == self.module =>
                {
                    self.types.set_symbol_type(*symbol, *type_id);
                }
                (VariableOrigin::Symbol(symbol), Some(VariableValue::Static(static_id)))
                    if symbol.module_id == self.module =>
                {
                    self.statics.set_symbol_static(*symbol, *static_id);
                }
                _ => {}
            }
        }

        DirCheckedModule {
            types: Arc::new(self.types),
            statics: Arc::new(self.statics),
            resolutions: Arc::new(self.resolutions),
            instances: Arc::new(self.instances),
            relations: Arc::new(self.relations),
            extensions: Arc::new(self.extensions),
            layouts: Arc::new(self.layouts),
            captures: Arc::new(self.captures),
        }
    }
}

/// Return a small vector with one variable.
pub(in crate::check) fn one_variable(variable: VariableId) -> SmallVec<[VariableId; 4]> {
    smallvec::smallvec![variable]
}
