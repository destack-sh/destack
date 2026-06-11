use std::sync::Arc;

use destack_artifact::{DirBound, DirExpanded, DirParsed, DirResolved, ProfileKey};
use destack_core::StringPool;
use destack_dir as dir;
use destack_repository::Module;
use destack_source::ModuleId;
use indexmap::{IndexMap, IndexSet};

use crate::check::{Capture, CheckError, CheckState, Condition};
use crate::{CompilerError, CompilerResult};

/// State owned by one module inside a checked component.
pub(in crate::check) struct CheckModuleState {
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

    /// Out-of-component modules visible from this module.
    pub(in crate::check) external_modules: IndexSet<ModuleId>,
    /// Captures discovered while walking this module.
    pub(in crate::check) captures: Vec<Capture>,
    /// Static availability of declarations in this module.
    pub(in crate::check) availability: IndexMap<dir::GlobalSymbolId, Condition>,
    /// Diagnostics reported while walking this module.
    pub(in crate::check) diagnostics: Vec<CheckError>,

    /// Next check-owned generic template id.
    next_generic_template_id: u32,
    /// Next check-owned generic parameter id.
    next_generic_parameter_id: u32,
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
        Self {
            module,
            profile,
            strings,
            parsed,
            bound,
            resolved,
            expanded,
            external_modules: IndexSet::new(),
            captures: Vec::new(),
            availability: IndexMap::new(),
            diagnostics: Vec::new(),
            next_generic_template_id: 0,
            next_generic_parameter_id: 0,
        }
    }

    /// Return the post-expansion DIR tree view visible to check.
    pub(in crate::check) fn view(&self) -> dir::View<'_> {
        dir::View::with_patches(
            &self.parsed.tree,
            std::slice::from_ref(&self.expanded.patch),
        )
    }

    /// Return the cumulative binding table visible to check.
    pub(in crate::check) fn binding_table(&self) -> dir::BindingTable<'static> {
        self.expanded.binding_table(&self.bound)
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
    pub(in crate::check) fn type_table(&self) -> dir::TypeTable<'static> {
        self.expanded.type_table(&self.bound)
    }

    /// Return the cumulative static table visible to check inputs.
    pub(in crate::check) fn static_table(&self) -> dir::StaticTable<'static> {
        self.expanded.static_table(&self.bound)
    }

    /// Return whether this module can read another module.
    pub(in crate::check) fn imports_module(&self, module: ModuleId) -> bool {
        self.external_modules.contains(&module)
    }

    /// Return one fresh generic template id owned by this module.
    pub(in crate::check) fn fresh_generic_template_id(&mut self) -> dir::GlobalGenericTemplateId {
        let local_id = dir::LocalGenericTemplateId::new(self.next_generic_template_id);
        self.next_generic_template_id += 1;
        local_id.into_global(self.module.id)
    }

    /// Return one local input type visible to check.
    pub(in crate::check) fn r#type(&self, type_id: dir::LocalTypeId) -> &dir::Type {
        if let Some(ty) = self.expanded.types.get_type_maybe(type_id) {
            return ty;
        }

        self.bound.types.get_type(type_id)
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

    /// Return one fresh generic parameter id owned by this module.
    pub(in crate::check) fn fresh_generic_parameter_id(&mut self) -> dir::GlobalGenericParameterId {
        let local_id = dir::LocalGenericParameterId::new(self.next_generic_parameter_id);

        self.next_generic_parameter_id += 1;

        local_id.into_global(self.module.id)
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
    pub(in crate::check) fn binding_table(&self, module: ModuleId) -> dir::BindingTable<'_> {
        if let Some(module) = self.modules.get(&module) {
            module.binding_table()
        } else {
            self.external_module(module).bindings.clone()
        }
    }

    /// Return one component or external type.
    pub(in crate::check) fn r#type(&self, ty: dir::GlobalTypeId) -> &dir::Type {
        if let Some(module) = self.modules.get(&ty.module_id) {
            module.r#type(ty.local_id)
        } else {
            self.external_module(ty.module_id)
                .types
                .get_type(ty.local_id)
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
