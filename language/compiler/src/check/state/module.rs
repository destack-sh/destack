use std::sync::Arc;

use destack_artifact::{DirBound, DirExpanded, DirParsed, DirResolved, ProfileKey};
use destack_core::StringPool;
use destack_dir as dir;
use destack_repository::Module;
use destack_source::{ModuleId, Span};
use indexmap::{IndexMap, IndexSet};

use crate::check::{Capture, CheckError, CheckState, CheckWarning, Condition};
use crate::{CompilerError, CompilerResult};

/// State owned by one module inside a checked component.
pub(in crate::check) struct CheckModuleState {
    /// The requested source module.
    pub(in crate::check) module: Arc<Module>,
    /// Out-of-component modules visible from this module.
    pub(in crate::check) external_modules: IndexSet<ModuleId>,
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
    /// The cumulative bound type table built once at load.
    pub(in crate::check) types: dir::TypeTable<'static>,
    /// Open check output accumulating for this module.
    pub(in crate::check) working: WorkingSegments,
    /// Captures discovered while walking this module.
    pub(in crate::check) captures: Vec<Capture>,
    /// Static availability of declarations in this module.
    pub(in crate::check) availability: IndexMap<dir::GlobalSymbolId, Condition>,
    /// Declarations whose guards decided statically false.
    pub(in crate::check) unavailable: IndexSet<dir::GlobalSymbolId>,
    /// Diagnostics reported while walking this module.
    pub(in crate::check) diagnostics: Vec<destack_artifact::DiagnosticBuilder<CheckError>>,
    /// Warnings reported while walking this module.
    pub(in crate::check) warnings: Vec<destack_artifact::DiagnosticBuilder<CheckWarning>>,
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
        // build the layered tables once, every lookup reuses them
        let bindings = expanded.binding_table(&bound);
        let types = expanded.type_table(&bound);
        let working = WorkingSegments::new(module.id, &expanded);

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
            working,
            external_modules: IndexSet::new(),
            captures: Vec::new(),
            availability: IndexMap::new(),
            unavailable: IndexSet::new(),
            diagnostics: Vec::new(),
            warnings: Vec::new(),
        }
    }

    /// Move this module's working segments out for commit.
    /// No read may follow the move; fresh empty segments replace them.
    pub(in crate::check) fn take_working(&mut self) -> WorkingSegments {
        std::mem::replace(
            &mut self.working,
            WorkingSegments::new(self.module.id, &self.expanded),
        )
    }

    /// Return the post-expansion DIR tree view visible to check.
    pub(in crate::check) fn view(&self) -> dir::View<'_> {
        dir::View::with_patches(
            &self.parsed.tree,
            std::slice::from_ref(&self.expanded.patch),
        )
    }

    /// Return the authored parse tree that source renders print.
    pub(in crate::check) fn parsed_tree(&self) -> &dir::Tree {
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

    /// Return the cumulative type table visible to check inputs.
    pub(in crate::check) fn type_table(&self) -> &dir::TypeTable<'static> {
        &self.types
    }

    /// Return one committed type when any bound segment carries it.
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

/// Working segments accumulating open check output for one module.
/// TODO #Cleanup: inline WorkingSegments
pub(in crate::check) struct WorkingSegments {
    /// Open types layered over the expanded table.
    pub(in crate::check) types: dir::TypeSegment,
    /// Checked declaration definitions.
    pub(in crate::check) definitions: dir::DefinitionSegment,
    /// Induced generic templates and parameters.
    pub(in crate::check) generics: dir::GenericSegment,
}

impl WorkingSegments {
    /// Create empty working segments over one expanded module.
    fn new(module: ModuleId, expanded: &DirExpanded) -> Self {
        Self {
            types: dir::TypeSegment::from_base(&expanded.types),
            definitions: dir::DefinitionSegment::new(module),
            generics: dir::GenericSegment::new(module),
        }
    }
}
