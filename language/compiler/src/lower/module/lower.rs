use std::collections::{HashMap, HashSet};

use destack_base::StringPool;
use destack_dir::{GlobalSymbolId, LocalNodeId};
use destack_source::ModuleId;
use destack_workspace::{Module, ProfileId, TargetId};
use indexmap::IndexSet;
use {destack_dir as dir, destack_mir as mir};

use crate::LowerResult;

use crate::lower::item::GlobalBinding;
use crate::lower::table::VtableGlobal;
use crate::lower::table::interface::InterfaceSlot;
use crate::lower::{BuiltinTypeLayouts, TypeLowerer};

/// Context for lowering a DIR module to MIR.
#[derive(Debug)]
#[allow(dead_code)]
pub(crate) struct ModuleLowerer<'a> {
    /// Provide access to the compiler for shared resources.
    pub(crate) compiler: &'a crate::Compiler,
    /// Identify the module being lowered.
    pub(crate) module_id: ModuleId,
    /// Identify the profile used for DIR access.
    pub(crate) profile: ProfileId,
    /// Provide access to the source module data.
    pub(crate) module: &'a Module,
    /// Provide access to the DIR tree for expression lookup.
    pub(crate) dir_tree: &'a dir::NodeTree,
    /// Provide access to the root expressions for the module.
    pub(crate) dir_roots: &'a [LocalNodeId<dir::Expression>],
    /// Provide access to symbol metadata for type resolution.
    pub(crate) symbols: &'a dir::SymbolTable,
    /// Provide access to inferred and declared types.
    pub(crate) types: &'a dir::TypeTable,
    /// Identify the target backend for lowering.
    pub(crate) target: &'a TargetId,

    /// Build MIR nodes for this module.
    pub(crate) builder: mir::ModuleBuilder,
    /// Map DIR symbols to MIR function ids.
    pub(crate) functions_by_symbol: HashMap<GlobalSymbolId, mir::LocalNodeId<mir::Function>>,
    /// Map MIR function ids to their signature types.
    pub(crate) function_signature_types:
        HashMap<mir::LocalNodeId<mir::Function>, mir::LocalNodeId<mir::Type>>,
    /// Map DIR symbols to MIR global bindings.
    pub(crate) globals_by_symbol: HashMap<GlobalSymbolId, GlobalBinding>,
    /// Lower and cache DIR types into MIR types.
    pub(crate) type_lowerer: TypeLowerer,
    /// Synthetic name for call signatures in dispatch tables.
    pub(crate) dispatch_call_name: destack_base::StringId,
    /// Synthetic name for construct signatures in dispatch tables.
    pub(crate) dispatch_construct_name: destack_base::StringId,
    /// Synthetic name for vtable header fields.
    pub(crate) vtable_field_name: destack_base::StringId,
    /// Track interface slot data for dispatch lowering.
    pub(crate) interface_slots_by_symbol: HashMap<GlobalSymbolId, Vec<InterfaceSlot>>,
    /// Track interface slot lowering in progress.
    pub(crate) interface_slots_in_progress: IndexSet<GlobalSymbolId>,
    /// Track nominal layout lowering by symbol.
    pub(crate) nominal_layouts_by_symbol: HashMap<GlobalSymbolId, mir::LocalNodeId<mir::Type>>,
    /// Track nominal layout lowering in progress.
    pub(crate) nominal_layouts_in_progress: IndexSet<GlobalSymbolId>,
    /// Track class symbols that require vtable headers.
    pub(crate) vtable_layout_symbols: Option<HashSet<GlobalSymbolId>>,
    /// Track vtables that have been lowered.
    pub(crate) vtable_by_symbol: HashMap<GlobalSymbolId, mir::DispatchTableId>,
    /// Track vtable lowering in progress.
    pub(crate) vtable_in_progress: IndexSet<GlobalSymbolId>,
    /// Precomputed dispatch table ids for class vtables.
    pub(crate) vtable_ids_by_symbol: HashMap<GlobalSymbolId, mir::DispatchTableId>,
    /// Track itabs that have been lowered.
    pub(crate) itab_by_pair: HashMap<(GlobalSymbolId, GlobalSymbolId), mir::DispatchTableId>,
    /// Track itab lowering in progress.
    pub(crate) itab_in_progress: IndexSet<(GlobalSymbolId, GlobalSymbolId)>,
    /// Track whether the dispatch registry is initialized.
    pub(crate) dispatch_registry_ready: bool,
    /// Virtual dispatch slot ids keyed by method symbol.
    pub(crate) virtual_method_slots_by_symbol: HashMap<GlobalSymbolId, u32>,
    /// Ordered list of class symbols that require vtables.
    pub(crate) vtable_class_symbols: Vec<GlobalSymbolId>,
    /// Predeclared vtable globals keyed by class symbol.
    pub(crate) vtable_globals_by_symbol: HashMap<GlobalSymbolId, VtableGlobal>,
    /// Ordered interface itab pairs for deterministic table ids.
    pub(crate) interface_itab_pairs: Vec<(GlobalSymbolId, GlobalSymbolId)>,
    /// Precomputed itab ids keyed by concrete and interface symbols.
    pub(crate) interface_itab_ids: HashMap<(GlobalSymbolId, GlobalSymbolId), mir::DispatchTableId>,
}

#[allow(clippy::too_many_arguments)]
impl<'a> ModuleLowerer<'a> {
    /// Create a new module lowering context.
    pub(crate) fn new(
        compiler: &'a crate::Compiler,
        module: &'a Module,
        profile: ProfileId,
        dir_tree: &'a dir::NodeTree,
        dir_roots: &'a [LocalNodeId<dir::Expression>],
        symbols: &'a dir::SymbolTable,
        types: &'a dir::TypeTable,
        target: &'a TargetId,
        pointer_bytes: u8,
    ) -> Self {
        // initialize the module builder
        let mut builder = mir::ModuleBuilder::new();

        // seed the mir string pool with program strings
        let strings = compiler.program.strings.as_ref().clone().into_immutable();
        builder.strings().copy_from_immutable(&strings);

        // create the type lowerer
        let type_lowerer = TypeLowerer::new(
            &mut builder,
            pointer_bytes,
            compiler.program.modules.clone(),
            compiler.program.packages.clone(),
        );
        let dispatch_call_name = builder.intern("@call");
        let dispatch_construct_name = builder.intern("@new");
        let vtable_field_name = builder.intern("@vtable");

        Self {
            compiler,
            module_id: module.id,
            profile,
            module,
            dir_tree,
            dir_roots,
            symbols,
            types,
            target,
            builder,
            functions_by_symbol: HashMap::new(),
            function_signature_types: HashMap::new(),
            globals_by_symbol: HashMap::new(),
            type_lowerer,
            dispatch_call_name,
            dispatch_construct_name,
            vtable_field_name,
            interface_slots_by_symbol: HashMap::new(),
            interface_slots_in_progress: IndexSet::new(),
            nominal_layouts_by_symbol: HashMap::new(),
            nominal_layouts_in_progress: IndexSet::new(),
            vtable_layout_symbols: None,
            vtable_by_symbol: HashMap::new(),
            vtable_in_progress: IndexSet::new(),
            vtable_ids_by_symbol: HashMap::new(),
            itab_by_pair: HashMap::new(),
            itab_in_progress: IndexSet::new(),
            dispatch_registry_ready: false,
            virtual_method_slots_by_symbol: HashMap::new(),
            vtable_class_symbols: Vec::new(),
            vtable_globals_by_symbol: HashMap::new(),
            interface_itab_pairs: Vec::new(),
            interface_itab_ids: HashMap::new(),
        }
    }

    /// Lower this entire DIR module to MIR (in-place).
    pub(crate) fn lower_module(&mut self) -> LowerResult<()> {
        // initialize builtin layouts
        self.initialize_string_type()?;

        // build dispatch registry
        self.lower_dispatch_registry()?;

        // lower external calls
        self.lower_external_calls()?;

        // lower root expressions
        for expression_id in self.dir_roots.iter().copied() {
            self.lower_root_expression(expression_id)?;
        }

        // finalize nominal types so layouts are cached
        self.finalize_declared_types()?;

        // emit dispatch tables (vtables, itabs)
        self.dispatch_tables()?;

        // finalize metadata for all cached types
        self.finalize_type_metadata()?;

        Ok(())
    }

    /// Ensure all nominal types are lowered into the type cache.
    fn finalize_declared_types(&mut self) -> LowerResult<()> {
        // track symbols we've already finalized
        let mut seen = HashSet::new();
        for (declaration_id, declaration) in self.dir_tree.iter_nodes_of_type::<dir::Declaration>()
        {
            let symbol = match declaration {
                dir::Declaration::Struct { descriptor, .. }
                | dir::Declaration::Class { descriptor, .. }
                | dir::Declaration::Enum { descriptor, .. }
                | dir::Declaration::Interface { descriptor, .. } => {
                    descriptor.symbol.into_global(self.module_id)
                }
                dir::Declaration::Type {
                    descriptor, kind, ..
                } => {
                    if !matches!(kind, dir::TypeKind::Nominal) {
                        continue;
                    }
                    descriptor.symbol.into_global(self.module_id)
                }
                _ => continue,
            };
            if !seen.insert(symbol) {
                continue;
            }

            let anchor = declaration_id
                .into_global_any(self.module_id)
                .into_anchored(Some(self.profile));
            if let Some(instance_type_id) = self.types.get_instance_type_id(symbol) {
                self.lower_type(instance_type_id, anchor)?;
            }

            if symbol.ty() == dir::SymbolType::Interface
                && let Some(reference_type_id) = self.nominal_reference_type_id_for_symbol(symbol)
            {
                self.lower_type(reference_type_id, anchor)?;
            }
        }

        Ok(())
    }

    /// Finalize metadata for all cached types.
    fn finalize_type_metadata(&mut self) -> LowerResult<()> {
        // collect cached types
        let mut cached_types = Vec::new();
        for (type_id, entry) in &self.type_lowerer.type_cache {
            let mir_type = match entry {
                crate::lower::TypeCacheEntry::Ready(mir_type) => *mir_type,
                crate::lower::TypeCacheEntry::InProgress => {
                    return Err(crate::LowerError::Internal {
                        module: self.module_id,
                        message: "type lowering cache left in progress".to_string(),
                    });
                }
            };
            cached_types.push((*type_id, mir_type));
        }

        // finalize metadata for each cached type
        for (type_id, mir_type) in cached_types {
            let anchor = self.type_anchor(type_id);
            self.metadata_name_for_type(type_id, mir_type, anchor)?;
            self.layout_metadata_for_type(mir_type)?;
            self.field_map_metadata_for_type(type_id, mir_type, anchor)?;
            if let Some(symbol) = self.types.symbol_for_instance_type(type_id) {
                self.lineage_metadata_for_symbol(symbol, mir_type, anchor)?;
            }
        }

        self.validate_metadata_names_assigned()?;

        Ok(())
    }

    /// Finish the module lowering process and return the resulting MIR tree and string pool.
    pub(crate) fn finish(self) -> (mir::NodeTree, StringPool) {
        self.builder.finish_mutable()
    }

    /// Initialize the canonical string type from builtin definitions.
    fn initialize_string_type(&mut self) -> LowerResult<()> {
        // resolve a stable anchor for type lowering
        let Some(anchor) = self
            .dir_roots
            .first()
            .copied()
            .map(|root| root.into_global_any(self.module_id))
            .map(|root| root.into_anchored(Some(self.profile)))
        else {
            return Ok(());
        };

        // ensure builtin layouts are installed
        let mut builtin_layouts = BuiltinTypeLayouts::new(
            self.compiler,
            self.profile,
            &mut self.builder,
            &mut self.type_lowerer,
        );
        builtin_layouts.string_type_for_builtin(anchor)?;

        Ok(())
    }
}
