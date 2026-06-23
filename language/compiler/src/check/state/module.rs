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

use crate::check::{Capture, CheckError, CheckState, CheckWarning, Condition, PlaceAccess};
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

    // open check output owned by this module
    /// Open inference types layered over the expanded base.
    pub(in crate::check) types: dir::TypeSegment,
    /// Checked declaration definitions.
    pub(in crate::check) definitions: dir::DefinitionSegment,
    /// Induced generic templates and parameters.
    pub(in crate::check) generics: dir::GenericSegment,
    /// Inferred static symbol values, materialized to statics at finish.
    pub(in crate::check) values: IndexMap<dir::GlobalSymbolId, dir::GlobalTypeId>,
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
    /// Place accesses keyed by written place node.
    pub(in crate::check) accesses: IndexMap<dir::GlobalNodeIdAny, PlaceAccess>,

    // diagnostics drained at finish
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
            values: IndexMap::new(),
            node_conditions: IndexMap::new(),
            symbol_conditions: IndexMap::new(),
            unavailable: IndexSet::new(),
            accesses: IndexMap::new(),
            external_modules: IndexSet::new(),
            captures: Vec::new(),
            diagnostics: Vec::new(),
            warnings: Vec::new(),
        }
    }

    /// Move this module's open type overlay out for finish, leaving it empty.
    pub(in crate::check) fn take_types(&mut self) -> dir::TypeSegment {
        std::mem::replace(
            &mut self.types,
            dir::TypeSegment::from_base(&self.expanded.types),
        )
    }

    /// Move this module's checked definitions out for finish, leaving them empty.
    pub(in crate::check) fn take_definitions(&mut self) -> dir::DefinitionSegment {
        std::mem::replace(
            &mut self.definitions,
            dir::DefinitionSegment::new(self.module.id),
        )
    }

    /// Move this module's induced generics out for finish, leaving them empty.
    pub(in crate::check) fn take_generics(&mut self) -> dir::GenericSegment {
        std::mem::replace(&mut self.generics, dir::GenericSegment::new(self.module.id))
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

    /// Return whether one checked node was authored in source.
    pub(in crate::check) fn is_authored(&self, node: dir::LocalNodeIdAny) -> bool {
        self.parsed.tree.has_node_id(node.id)
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

    /// Return the checked type of one source node, if present.
    pub(in crate::check) fn node_type_maybe(
        &self,
        node: dir::GlobalNodeIdAny,
    ) -> Option<dir::GlobalTypeId> {
        self.modules
            .get(&node.module_id)
            .and_then(|module| module.types.get_node_type_id(node))
    }

    /// Return the checked type of one source node.
    pub(in crate::check) fn node_type(
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

    /// Record the inferred type of one source node.
    pub(in crate::check) fn set_node_type(
        &mut self,
        node: dir::GlobalNodeIdAny,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<()> {
        let types = &mut self.module_mut(node.module_id).types;
        if types
            .get_node_type_id(node)
            .is_some_and(|previous| previous != ty)
        {
            return Err(CompilerError::Internal {
                message: format!("check node {node:?} received two types"),
            });
        }
        types.set_node_type(node, ty);

        Ok(())
    }

    /// Return one component symbol's checked type, if present.
    pub(in crate::check) fn component_symbol_type_maybe(
        &self,
        symbol: dir::GlobalSymbolId,
    ) -> Option<dir::GlobalTypeId> {
        self.modules
            .get(&symbol.module_id)
            .and_then(|module| module.types.get_symbol_type_id(symbol))
    }

    /// Return one loaded symbol's checked type, if present.
    pub(in crate::check) fn symbol_type_maybe(
        &self,
        symbol: dir::GlobalSymbolId,
    ) -> Option<dir::GlobalTypeId> {
        // prefer the component overlay over committed tables
        if let Some(ty) = self.component_symbol_type_maybe(symbol) {
            return Some(ty);
        }

        // read external committed symbol types
        if let Some(external) = self.external_modules.get(&symbol.module_id) {
            return external.types.get_symbol_type_id(symbol);
        }

        None
    }

    /// Record the inferred type of one source symbol.
    pub(in crate::check) fn set_symbol_type(
        &mut self,
        symbol: dir::GlobalSymbolId,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<()> {
        let types = &mut self.module_mut(symbol.module_id).types;
        if types
            .get_symbol_type_id(symbol)
            .is_some_and(|previous| previous != ty)
        {
            return Err(CompilerError::Internal {
                message: format!("check symbol {symbol:?} received two types"),
            });
        }
        types.set_symbol_type(symbol, ty);

        Ok(())
    }

    /// Return the inferred static value of one source symbol.
    pub(in crate::check) fn symbol_value(
        &self,
        symbol: dir::GlobalSymbolId,
    ) -> Option<dir::GlobalTypeId> {
        self.modules
            .get(&symbol.module_id)
            .and_then(|module| module.values.get(&symbol).copied())
    }

    /// Record the inferred static value of one source symbol as a singleton type.
    pub(in crate::check) fn set_symbol_value(
        &mut self,
        symbol: dir::GlobalSymbolId,
        value: dir::GlobalTypeId,
    ) -> CompilerResult<()> {
        let values = &mut self.module_mut(symbol.module_id).values;
        if values
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

    /// Return how syntax accesses one place expression.
    pub(in crate::check) fn place_access(&self, node: dir::GlobalNodeIdAny) -> PlaceAccess {
        self.modules
            .get(&node.module_id)
            .and_then(|module| module.accesses.get(&node).copied())
            .unwrap_or(PlaceAccess::Read)
    }

    /// Record how syntax accesses one place expression.
    pub(in crate::check) fn set_place_access(
        &mut self,
        node: dir::GlobalNodeIdAny,
        access: PlaceAccess,
    ) {
        self.module_mut(node.module_id)
            .accesses
            .insert(node, access);
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
