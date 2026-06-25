use std::sync::Arc;

use destack_artifact::{
    DiagnosticBuilder, DirBound, DirExpanded, DirParsed, DirResolved, ProfileKey,
};
use destack_core::StringPool;
use destack_dir as dir;
use destack_repository::Module;
use destack_source::{ModuleId, Span};
use indexmap::{IndexMap, IndexSet};
use smallvec::SmallVec;

use crate::check::{
    Answer, Capture, CheckError, CheckState, CheckWarning, Condition, Constraint, ConstraintRole,
    Dependency, Origin, Relation,
};
use crate::{CompilerError, CompilerResult};

/// State owned by one module inside a checked component.
pub(in crate::check) struct CheckModuleState {
    // inherited inputs from upstream phases, read-only
    /// The requested source module.
    pub(in crate::check) module: Arc<Module>,
    /// The active semantic profile.
    pub(in crate::check) profile: ProfileKey,
    /// The shared string pool.
    pub(in crate::check) strings: Arc<StringPool>,
    /// The parsed DIR input.
    pub(in crate::check) parsed: Arc<DirParsed>,
    /// The bound DIR input.
    pub(in crate::check) bound: Arc<DirBound>,
    /// The resolved DIR input.
    pub(in crate::check) resolved: Arc<DirResolved>,
    /// The expanded DIR input.
    pub(in crate::check) expanded: Arc<DirExpanded>,
    /// The cumulative binding table built once at load.
    pub(in crate::check) bindings: dir::BindingTable<'static>,
    /// Out-of-component modules visible from this module.
    pub(in crate::check) external_modules: IndexSet<ModuleId>,

    // open checked state owned by this module
    /// Open inference types layered over the expanded base.
    pub(in crate::check) types: dir::TypeSegment,
    /// Checked declaration definitions.
    pub(in crate::check) definitions: dir::DefinitionSegment,
    /// Induced generic templates and parameters.
    pub(in crate::check) generics: dir::GenericSegment,
    /// Checked static values.
    pub(in crate::check) statics: dir::StaticSegment,
    /// Checked node resolutions.
    pub(in crate::check) resolutions: dir::ResolutionSegment,
    /// Checked implicit coercions.
    pub(in crate::check) coercions: dir::CoercionSegment,
    /// Checked layout facts.
    pub(in crate::check) layouts: dir::LayoutSegment,
    /// Checked capture facts.
    pub(in crate::check) capture_segment: dir::CaptureSegment,
    /// Checked annotations.
    pub(in crate::check) annotations: dir::AnnotationSegment,
    /// Inferred static symbol values, materialized to statics during write.
    pub(in crate::check) static_values: IndexMap<dir::GlobalSymbolId, dir::GlobalTypeId>,
    /// Captures discovered while walking this module.
    pub(in crate::check) captures: Vec<Capture>,

    // walk-recorded selection context, consumed during select
    /// Active static `@if` guard predicates keyed by guarded node.
    pub(in crate::check) node_conditions:
        IndexMap<dir::GlobalNodeIdAny, SmallVec<[dir::GlobalTypeId; 2]>>,
    /// Active static `@if` guard predicates keyed by guarded declaration.
    pub(in crate::check) symbol_conditions:
        IndexMap<dir::GlobalSymbolId, SmallVec<[dir::GlobalTypeId; 2]>>,
    /// Declarations whose guards decided statically false.
    pub(in crate::check) unavailable: IndexSet<dir::GlobalSymbolId>,
    // diagnostics drained during write
    /// Diagnostics reported while walking this module.
    pub(in crate::check) diagnostics: Vec<DiagnosticBuilder<CheckError>>,
    /// Warnings reported while walking this module.
    pub(in crate::check) warnings: Vec<DiagnosticBuilder<CheckWarning>>,
}

impl CheckModuleState {
    /// Create module state from loaded inputs and empty working state.
    pub(in crate::check) fn new(
        module: Arc<Module>,
        profile: ProfileKey,
        strings: Arc<StringPool>,
        parsed: Arc<DirParsed>,
        bound: Arc<DirBound>,
        resolved: Arc<DirResolved>,
        expanded: Arc<DirExpanded>,
    ) -> Self {
        // create the inherited bindings and this check's open overlays
        let bindings = expanded.binding_table(&bound);
        let types = dir::TypeSegment::from_base(&expanded.types);
        let definitions = dir::DefinitionSegment::new(module.id);
        let generics = dir::GenericSegment::new(module.id);
        let statics = dir::StaticSegment::from_base(&expanded.statics);
        let resolutions = dir::ResolutionSegment::new(module.id);
        let coercions = dir::CoercionSegment::new(module.id);
        let layouts = dir::LayoutSegment::new(module.id);
        let capture_segment = dir::CaptureSegment::new(module.id);
        let annotations = dir::AnnotationSegment::new(module.id);

        Self {
            module,
            profile,
            strings,
            parsed,
            bound,
            resolved,
            expanded,
            bindings,
            types,
            definitions,
            generics,
            statics,
            resolutions,
            coercions,
            layouts,
            capture_segment,
            annotations,
            static_values: IndexMap::new(),
            node_conditions: IndexMap::new(),
            symbol_conditions: IndexMap::new(),
            unavailable: IndexSet::new(),
            external_modules: IndexSet::new(),
            captures: Vec::new(),
            diagnostics: Vec::new(),
            warnings: Vec::new(),
        }
    }

    /// Return the post-expansion DIR tree view visible to check.
    pub(in crate::check) fn view(&self) -> dir::View<'_> {
        dir::View::with_patches(
            &self.parsed.tree,
            std::slice::from_ref(&self.expanded.patch),
        )
    }

    /// Return the authored source tree used for source rendering.
    pub(in crate::check) fn source_tree(&self) -> &dir::Tree {
        &self.parsed.tree
    }

    /// Return the authored source span of one node.
    pub(in crate::check) fn authored_span(&self, node: dir::LocalNodeIdAny) -> Span {
        self.parsed.tree.source_index.get_main_or_enclosing(node.id)
    }

    /// Return the authored diagnostic span of one visible node.
    pub(in crate::check) fn diagnostic_span(&self, node: dir::LocalNodeIdAny) -> Option<Span> {
        let view = self.view();
        let source = view.get_source_any(node);

        // prefer the authored node that produced the visible node
        if let Some(span) = self.parsed.tree.get_main_span_by_id(source) {
            return Some(span);
        }
        if let Some(span) = self.parsed.tree.get_span_by_id(source) {
            return Some(span);
        }

        view.get_span_by_id(node.id)
    }

    /// Return the cumulative binding table visible to check.
    pub(in crate::check) fn binding_table(&self) -> &dir::BindingTable<'static> {
        &self.bindings
    }

    /// Return the symbol introduced by a source declaration node.
    pub(in crate::check) fn declaration_symbol(
        &self,
        node: dir::LocalNodeIdAny,
    ) -> Option<dir::GlobalSymbolId> {
        let symbol = self
            .binding_table()
            .declaration_symbol(node.into_global(self.module.id))?;

        Some(symbol.into_global(self.module.id))
    }

    /// Return the implicit receiver symbol introduced for one member node.
    pub(in crate::check) fn implicit_receiver_symbol(
        &self,
        node: dir::LocalNodeIdAny,
    ) -> Option<dir::GlobalSymbolId> {
        let symbol = self
            .binding_table()
            .implicit_receiver_symbol(node.into_global(self.module.id))?;

        Some(symbol.into_global(self.module.id))
    }

    /// Return one source symbol's declaration node.
    pub(in crate::check) fn symbol_declaration_node(
        &self,
        symbol: dir::LocalSymbolId,
    ) -> CompilerResult<dir::LocalNodeIdAny> {
        let bindings = self.binding_table();
        let binding = bindings.get_symbol(symbol);

        // require source symbols to have local declaration nodes
        let Some(declaration) = binding.declaration else {
            return Err(CompilerError::Internal {
                message: format!("source symbol {symbol:?} has no declaration node"),
            });
        };
        if declaration.module_id != self.module.id {
            return Err(CompilerError::Internal {
                message: format!("source symbol {symbol:?} declaration points outside its module"),
            });
        }

        Ok(declaration.local_id)
    }

    /// Return one open working type when this module's overlay carries it.
    pub(in crate::check) fn type_maybe(&self, type_id: dir::LocalTypeId) -> Option<&dir::Type> {
        self.types.get_type_maybe(type_id)
    }

    /// Return one local input static visible to check.
    pub(in crate::check) fn r#static(&self, static_id: dir::LocalStaticId) -> &dir::StaticTerm {
        if let Some(value) = self.expanded.statics.get_static_maybe(static_id) {
            return value;
        }

        self.bound.statics.get_static(static_id)
    }

    /// Return whether one local symbol is an imported alias.
    pub(in crate::check) fn is_import_alias(&self, symbol: dir::LocalSymbolId) -> bool {
        self.resolved.imports.symbol_target(symbol).is_some()
    }
}

impl CheckState<'_> {
    /// Return whether one module belongs to the active checked component.
    pub(in crate::check) fn is_component_module(&self, module: ModuleId) -> bool {
        self.modules.contains_key(&module)
    }

    /// Return the symbols whose static guards did not decide false.
    pub(in crate::check) fn available_symbols(
        &self,
        symbols: &[dir::GlobalSymbolId],
    ) -> SmallVec<[dir::GlobalSymbolId; 4]> {
        symbols
            .iter()
            .copied()
            .filter(|symbol| !self.is_unavailable_symbol(*symbol))
            .collect()
    }

    /// Return whether one symbol's guard decided statically false.
    pub(in crate::check) fn is_unavailable_symbol(&self, symbol: dir::GlobalSymbolId) -> bool {
        self.modules
            .get(&symbol.module_id)
            .is_some_and(|module| module.unavailable.contains(&symbol))
    }

    /// Return loaded state for one module.
    pub(in crate::check) fn module(&self, module: ModuleId) -> &CheckModuleState {
        match self.modules.get(&module) {
            Some(state) => state,
            None => unreachable!("check module {module:?} was not loaded"),
        }
    }

    /// Return loaded state for one module mutably.
    pub(in crate::check) fn module_mut(&mut self, module: ModuleId) -> &mut CheckModuleState {
        match self.modules.get_mut(&module) {
            Some(state) => state,
            None => unreachable!("check module {module:?} was not loaded"),
        }
    }

    /// Move loaded state for one module out of check state.
    pub(in crate::check) fn take_module(&mut self, module: ModuleId) -> CheckModuleState {
        self.modules
            .swap_remove(&module)
            .unwrap_or_else(|| unreachable!("check module {module:?} was not loaded"))
    }

    /// Return the checked type of one source node, if present.
    pub(in crate::check) fn node_type_maybe(
        &self,
        node: dir::GlobalNodeIdAny,
    ) -> Option<dir::GlobalTypeId> {
        self.solver.node_type(node)
    }

    /// Return the checked type answer for one source node.
    pub(in crate::check) fn node_type_answer(
        &self,
        node: dir::GlobalNodeIdAny,
    ) -> CompilerResult<Answer<dir::GlobalTypeId>> {
        if let Some(ty) = self.node_type_maybe(node) {
            return Ok(Answer::Ready(ty));
        }

        Ok(Answer::pending([Dependency::Decision(node)]))
    }

    /// Return the checked type required for one source node.
    pub(in crate::check) fn require_node_type(
        &self,
        node: dir::GlobalNodeIdAny,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let Some(ty) = self.node_type_maybe(node) else {
            return Err(CompilerError::Internal {
                message: format!("node {node:?} has no checked type"),
            });
        };

        Ok(ty)
    }

    /// Bind one node to the type selected by a solver task.
    pub(in crate::check) fn bind_node_type(
        &mut self,
        node: dir::GlobalNodeIdAny,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<()> {
        let Some(existing) = self.node_type_maybe(node) else {
            self.set_node_type(node, ty)?;

            return Ok(());
        };

        if let Some(variable) = self.root_variable(existing)? {
            self.push_lower_bound(variable, ty)?;
        } else if existing != ty {
            self.push_constraint(Constraint {
                relation: Relation::Equal,
                left: existing,
                right: ty,
                origin: Origin::Node(node),
                condition: Condition::Always,
                role: ConstraintRole::Check,
            });
        }

        Ok(())
    }

    /// Record the inferred type of one source node.
    pub(in crate::check) fn set_node_type(
        &mut self,
        node: dir::GlobalNodeIdAny,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<()> {
        self.solver.set_node_type(node, ty)
    }

    /// Return one component declaration type, if present.
    pub(in crate::check) fn declaration_type_maybe(
        &self,
        symbol: dir::GlobalSymbolId,
    ) -> Option<dir::GlobalTypeId> {
        self.declaration_types.get(&symbol).copied()
    }

    /// Record one declaration symbol type.
    pub(in crate::check) fn set_declaration_type(
        &mut self,
        symbol: dir::GlobalSymbolId,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<()> {
        if let Some(previous) = self.declaration_type_maybe(symbol)
            && previous != ty
        {
            return Err(CompilerError::Internal {
                message: format!(
                    "declaration symbol {symbol:?} already has type {previous:?}, got {ty:?}"
                ),
            });
        }

        self.declaration_types.insert(symbol, ty);

        Ok(())
    }

    /// Return one component binding type, if present.
    pub(in crate::check) fn binding_type_maybe(
        &self,
        symbol: dir::GlobalSymbolId,
    ) -> Option<dir::GlobalTypeId> {
        self.binding_types.get(&symbol).copied()
    }

    /// Record one binding symbol type.
    pub(in crate::check) fn set_binding_type(
        &mut self,
        symbol: dir::GlobalSymbolId,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<()> {
        let previous = self.binding_type_maybe(symbol);
        if previous.is_some_and(|previous| previous != ty) {
            return Err(CompilerError::Internal {
                message: format!(
                    "binding symbol {symbol:?} already has type {previous:?}, got {ty:?}"
                ),
            });
        }

        self.binding_types.insert(symbol, ty);

        Ok(())
    }

    /// Return one loaded symbol's checked type, if present.
    pub(in crate::check) fn symbol_type_maybe(
        &self,
        symbol: dir::GlobalSymbolId,
    ) -> Option<dir::GlobalTypeId> {
        // prefer body-owned bindings over stable declarations
        if let Some(ty) = self.binding_type_maybe(symbol) {
            return Some(ty);
        }
        if let Some(ty) = self.declaration_type_maybe(symbol) {
            return Some(ty);
        }

        // read external committed symbol types
        if let Some(external) = self.external_modules.get(&symbol.module_id) {
            return external.types.get_symbol_type_id(symbol);
        }

        None
    }

    /// Return the inferred static value of one source symbol.
    pub(in crate::check) fn static_value(
        &self,
        symbol: dir::GlobalSymbolId,
    ) -> Option<dir::GlobalTypeId> {
        self.modules
            .get(&symbol.module_id)
            .and_then(|module| module.static_values.get(&symbol).copied())
    }

    /// Record the inferred static value of one source symbol as a singleton type.
    pub(in crate::check) fn set_static_value(
        &mut self,
        symbol: dir::GlobalSymbolId,
        value: dir::GlobalTypeId,
    ) -> CompilerResult<()> {
        let static_values = &mut self.module_mut(symbol.module_id).static_values;
        if static_values
            .insert(symbol, value)
            .is_some_and(|previous| previous != value)
        {
            return Err(CompilerError::Internal {
                message: format!("check symbol {symbol:?} received two static values"),
            });
        }

        Ok(())
    }

    /// Return the active static `@if` guard predicates of one source node.
    pub(in crate::check) fn node_condition(
        &self,
        node: dir::GlobalNodeIdAny,
    ) -> &[dir::GlobalTypeId] {
        self.modules
            .get(&node.module_id)
            .and_then(|module| module.node_conditions.get(&node))
            .map_or(&[], |predicates| predicates.as_slice())
    }

    /// Return the active static condition of one source node.
    pub(in crate::check) fn node_static_condition(&self, node: dir::GlobalNodeIdAny) -> Condition {
        let predicates = self.node_condition(node);
        if predicates.is_empty() {
            Condition::Always
        } else {
            Condition::When(predicates.iter().copied().collect())
        }
    }

    /// Record the active static `@if` guard predicates of one source node.
    pub(in crate::check) fn set_node_condition(
        &mut self,
        node: dir::GlobalNodeIdAny,
        predicates: SmallVec<[dir::GlobalTypeId; 2]>,
    ) {
        self.module_mut(node.module_id)
            .node_conditions
            .insert(node, predicates);
    }

    /// Return the active static `@if` guard predicates of one declaration.
    pub(in crate::check) fn symbol_condition(
        &self,
        symbol: dir::GlobalSymbolId,
    ) -> &[dir::GlobalTypeId] {
        self.modules
            .get(&symbol.module_id)
            .and_then(|module| module.symbol_conditions.get(&symbol))
            .map_or(&[], |predicates| predicates.as_slice())
    }

    /// Record the active static `@if` guard predicates of one declaration.
    pub(in crate::check) fn set_symbol_condition(
        &mut self,
        symbol: dir::GlobalSymbolId,
        predicates: SmallVec<[dir::GlobalTypeId; 2]>,
    ) {
        self.module_mut(symbol.module_id)
            .symbol_conditions
            .insert(symbol, predicates);
    }

    /// Return one binding table by module.
    pub(in crate::check) fn binding_table(&self, module: ModuleId) -> &dir::BindingTable<'static> {
        if let Some(module) = self.modules.get(&module) {
            module.binding_table()
        } else {
            &self.external_module(module).bindings
        }
    }

    /// Return one component or external static.
    pub(in crate::check) fn r#static(&self, value: dir::GlobalStaticId) -> &dir::StaticTerm {
        if let Some(module) = self.modules.get(&value.module_id) {
            module.r#static(value.local_id)
        } else {
            self.external_module(value.module_id)
                .statics
                .get_static(value.local_id)
        }
    }

    /// Return the declaration kind for one symbol.
    pub(in crate::check) fn symbol_kind(&self, symbol: dir::GlobalSymbolId) -> dir::SymbolKind {
        let binding_table = self.binding_table(symbol.module_id);
        let symbol = binding_table.get_symbol(symbol.local_id);

        symbol.kind
    }
}
