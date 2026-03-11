use std::collections::{BTreeSet, HashMap, HashSet, VecDeque};

use destack_ast::StringId;
use destack_core::StringPool;
use destack_dir::{GlobalNodeIdAny, GlobalSymbolId, LocalNodeId};
use destack_source::ModuleId;
use destack_workspace::{
    CheckFailurePolicy, Module, ProfileId, Target, TargetId, WellKnownIntrinsics,
};
use indexmap::IndexSet;
use {destack_dir as dir, destack_mir as mir};

use crate::{Compiler, LowerError, LowerResult};

use crate::lower::{
    BuiltinTypeLayouts, ClosureEnvLayout, GlobalBinding, InstanceKey, InterfaceEntry, MethodKey,
    RUNTIME_CHECK_MESSAGES, RuntimeCheckConfig, RuntimeStatusLayout, TypeCacheEntry, TypeLowerer,
    VtableGlobal,
};

/// Context for lowering a DIR module to MIR.
#[derive(Debug)]
#[allow(dead_code)]
pub(crate) struct ModuleLowerer<'a> {
    /// Provide access to the compiler for shared resources.
    pub(crate) compiler: &'a Compiler,
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
    /// Provide access to capture metadata for closures.
    pub(crate) captures: &'a dir::CaptureTable,
    /// Runtime check configuration for this target.
    pub(crate) runtime_checks: RuntimeCheckConfig,
    /// Well-known intrinsic bindings for this profile.
    pub(crate) well_known_intrinsics: Option<WellKnownIntrinsics>,

    /// Build MIR nodes for this module.
    pub(crate) builder: mir::ModuleBuilder,
    /// Map lowered instances to MIR function ids.
    pub(crate) functions_by_instance: HashMap<InstanceKey, mir::LocalNodeId<mir::Function>>,
    /// Map MIR function ids to their signature types.
    pub(crate) function_signature_types:
        HashMap<mir::LocalNodeId<mir::Function>, mir::LocalNodeId<mir::Type>>,
    /// Map DIR symbols to MIR global bindings.
    pub(crate) globals_by_symbol: HashMap<GlobalSymbolId, GlobalBinding>,
    /// Map string literal contents to MIR globals.
    pub(crate) string_literal_globals: HashMap<StringId, mir::LocalNodeId<mir::Global>>,
    /// Map closure environment layouts by function symbol.
    pub(crate) closure_env_layouts: HashMap<GlobalSymbolId, ClosureEnvLayout>,
    /// Cached empty closure environment type.
    pub(crate) empty_closure_env_type: Option<mir::LocalNodeId<mir::Type>>,
    /// Cached empty closure environment pointer type.
    pub(crate) empty_closure_env_pointer_type: Option<mir::LocalNodeId<mir::Type>>,
    /// Lower and cache DIR types into MIR types.
    pub(crate) type_lowerer: TypeLowerer,
    /// Synthetic name for call signatures in dispatch tables.
    pub(crate) dispatch_call_name: StringId,
    /// Synthetic name for construct signatures in dispatch tables.
    pub(crate) dispatch_construct_name: StringId,
    /// Synthetic name for vtable header fields.
    pub(crate) vtable_field_name: StringId,

    /// Track interface slot data for dispatch lowering.
    pub(crate) interface_slots_by_symbol: HashMap<GlobalSymbolId, Vec<InterfaceEntry>>,
    /// Track canonical interface dispatch field nodes by interface member.
    pub(crate) interface_dispatch_fields_by_member: HashMap<u32, mir::LocalNodeId<mir::Field>>,
    /// Track interface slot lowering in progress.
    pub(crate) interface_slots_in_progress: IndexSet<GlobalSymbolId>,

    /// Track nominal layout lowering by symbol.
    pub(crate) nominal_layouts_by_symbol: HashMap<GlobalSymbolId, mir::LocalNodeId<mir::Type>>,
    /// Track nominal layout lowering in progress.
    pub(crate) nominal_layouts_in_progress: IndexSet<GlobalSymbolId>,

    /// Track class symbols that require vtable headers.
    pub(crate) vtable_layout_symbols: Option<HashSet<GlobalSymbolId>>,
    /// Track vtables that have been lowered.
    pub(crate) vtable_by_symbol: HashMap<GlobalSymbolId, mir::VtableId>,
    /// Track vtable lowering in progress.
    pub(crate) vtable_in_progress: IndexSet<GlobalSymbolId>,
    /// Precomputed dispatch table ids for class vtables.
    pub(crate) vtable_ids_by_symbol: HashMap<GlobalSymbolId, mir::VtableId>,

    /// Track itabs that have been lowered.
    pub(crate) itab_by_pair: HashMap<(GlobalSymbolId, GlobalSymbolId), mir::ItabId>,
    /// Track itab lowering in progress.
    pub(crate) itab_in_progress: IndexSet<(GlobalSymbolId, GlobalSymbolId)>,

    /// Track whether dispatch declarations are initialized.
    pub(crate) dispatch_declared: bool,
    /// Virtual dispatch slot ids keyed by method symbol.
    pub(crate) virtual_method_slots_by_key: HashMap<(GlobalSymbolId, MethodKey), u32>,

    /// Ordered list of class symbols that require vtables.
    pub(crate) vtable_class_symbols: Vec<GlobalSymbolId>,
    /// Predeclared vtable globals keyed by class symbol.
    pub(crate) vtable_globals_by_symbol: HashMap<GlobalSymbolId, VtableGlobal>,

    /// Ordered interface itab pairs for deterministic table ids.
    pub(crate) interface_itab_pairs: Vec<(GlobalSymbolId, GlobalSymbolId)>,
    /// Precomputed itab ids keyed by concrete and interface symbols.
    pub(crate) interface_itab_ids: HashMap<(GlobalSymbolId, GlobalSymbolId), mir::ItabId>,

    /// Set of symbols marked as bindings.
    pub(crate) binding_symbols: HashSet<GlobalSymbolId>,
    /// Binding ABI lowering toggle.
    pub(crate) binding_abi_lowering: bool,
    /// Cached runtime status layout for ABI lowering.
    pub(crate) runtime_status_layout: Option<RuntimeStatusLayout>,
    /// Cached binding function for taking runtime errors.
    pub(crate) take_platform_error_function: Option<mir::LocalNodeId<mir::Function>>,
    /// Function declarations waiting for body lowering.
    pub(crate) pending_function_bodies: VecDeque<LocalNodeId<dir::Declaration>>,
    /// Function declarations already queued for body lowering.
    pub(crate) queued_function_bodies: HashSet<u32>,
}

#[allow(clippy::too_many_arguments)]
impl<'a> ModuleLowerer<'a> {
    /// Create a new module lowering context.
    pub(crate) fn new(
        compiler: &'a Compiler,
        module: &'a Module,
        profile: ProfileId,
        dir_tree: &'a dir::NodeTree,
        dir_roots: &'a [LocalNodeId<dir::Expression>],
        symbols: &'a dir::SymbolTable,
        types: &'a dir::TypeTable,
        captures: &'a dir::CaptureTable,
        target: &'a TargetId,
        pointer_bytes: u8,
    ) -> Self {
        // initialize the module builder
        let mut builder = mir::ModuleBuilder::new_with_verify(compiler.options.verify_mir);
        builder.set_pointer_bytes(pointer_bytes);

        // seed the mir string pool with program strings
        let strings = compiler.program.strings.as_ref().clone().into_immutable();
        builder.strings().copy_from_immutable(&strings);

        // resolve vector builtin symbols for SIMD lowering
        let vector_symbol = compiler.get_well_known_symbol_from(
            profile,
            dir::WellKnownSymbol::Vector,
            dir::SymbolSpaceOrder::TypeThenValue,
        );

        // create the type lowerer
        let type_lowerer = TypeLowerer::new(
            &mut builder,
            pointer_bytes,
            compiler.program.modules.clone(),
            compiler.program.packages.clone(),
            vector_symbol,
        );
        let dispatch_call_name = builder.intern("@call");
        let dispatch_construct_name = builder.intern("@new");
        let vtable_field_name = builder.intern("@vtable");

        // resolve the target configuration
        let target_config = Self::target_config_for_module(compiler, module, target);

        // resolve runtime check policies
        let debug = compiler.program.profile(profile).key.debug;
        let runtime_checks = RuntimeCheckConfig::from_target(&target_config, debug);
        let binding_abi_lowering = target_config.output.is_native();

        let well_known_intrinsics = compiler
            .program
            .artifacts
            .intrinsic_environment(profile)
            .map(|environment| environment.intrinsics.clone());

        Self {
            compiler,
            module_id: module.id,
            profile,
            module,
            dir_tree,
            dir_roots,
            symbols,
            types,
            captures,
            runtime_checks,
            well_known_intrinsics,
            builder,
            functions_by_instance: HashMap::new(),
            function_signature_types: HashMap::new(),
            globals_by_symbol: HashMap::new(),
            string_literal_globals: HashMap::new(),
            closure_env_layouts: HashMap::new(),
            empty_closure_env_type: None,
            empty_closure_env_pointer_type: None,
            type_lowerer,
            dispatch_call_name,
            dispatch_construct_name,
            vtable_field_name,
            interface_slots_by_symbol: HashMap::new(),
            interface_dispatch_fields_by_member: HashMap::new(),
            interface_slots_in_progress: IndexSet::new(),
            nominal_layouts_by_symbol: HashMap::new(),
            nominal_layouts_in_progress: IndexSet::new(),
            vtable_layout_symbols: None,
            vtable_by_symbol: HashMap::new(),
            vtable_in_progress: IndexSet::new(),
            vtable_ids_by_symbol: HashMap::new(),
            itab_by_pair: HashMap::new(),
            itab_in_progress: IndexSet::new(),
            dispatch_declared: false,
            virtual_method_slots_by_key: HashMap::new(),
            vtable_class_symbols: Vec::new(),
            vtable_globals_by_symbol: HashMap::new(),
            interface_itab_pairs: Vec::new(),
            interface_itab_ids: HashMap::new(),
            binding_symbols: HashSet::new(),
            binding_abi_lowering,
            runtime_status_layout: None,
            take_platform_error_function: None,
            pending_function_bodies: VecDeque::new(),
            queued_function_bodies: HashSet::new(),
        }
    }

    /// Resolve the target configuration for a module.
    fn target_config_for_module(compiler: &Compiler, module: &Module, target: &TargetId) -> Target {
        // load the package configuration
        let package = compiler.program.packages.get(module.package_id);
        let package = package.read();

        // use the configured target when present
        if let Some(target_config) = package.targets.get(target) {
            return target_config.clone();
        }

        // fall back to implicit targets for tests and synthetic builds
        Target::implicit_for_name(&target.name).unwrap_or_else(|| {
            panic!("missing target config for module {module:?} with target {target:?}")
        })
    }

    /// Resolve allocation mode for a symbol based on decorators and profile flags.
    pub(crate) fn allocation_mode_for_symbol(&self, symbol: GlobalSymbolId) -> mir::AllocationMode {
        // load symbol decorators
        let symbol = self.symbols.get_symbol(symbol.local_id);
        let decorators = &symbol.decorators;

        // prefer stack only when explicitly requested
        if decorators.is_stack_only {
            return mir::AllocationMode::StackOnly;
        }

        // apply no managed only when requested explicitly
        if decorators.is_no_managed {
            return mir::AllocationMode::NoManaged;
        }

        mir::AllocationMode::Any
    }

    /// Create a MissingType error for a node.
    pub(crate) fn missing_type_error(&self, node_id: GlobalNodeIdAny) -> LowerError {
        LowerError::MissingType {
            node: node_id.into_anchored(Some(self.profile)),
        }
    }

    /// Resolve a declared or inferred type id for a node or return MissingType.
    pub(crate) fn declared_or_inferred_type_id_for_node_or_error(
        &self,
        node_id: GlobalNodeIdAny,
    ) -> LowerResult<dir::LocalTypeId> {
        self.types
            .get_declared_or_inferred_type_id(node_id)
            .ok_or_else(|| self.missing_type_error(node_id))
    }

    /// Resolve a symbol type id or return MissingType.
    pub(crate) fn type_id_for_symbol_or_error(
        &self,
        symbol: GlobalSymbolId,
        anchor: dir::AnchoredGlobalNodeId,
    ) -> LowerResult<dir::LocalTypeId> {
        self.types
            .get_type_id_for_symbol(self.symbols, symbol)
            .ok_or(LowerError::MissingType { node: anchor })
    }

    /// Insert a global binding for a symbol.
    pub(crate) fn insert_global_binding(
        &mut self,
        symbol: GlobalSymbolId,
        binding: GlobalBinding,
    ) -> LowerResult<()> {
        // record the global binding once
        Self::insert_unique_entry(
            self.module_id,
            &mut self.globals_by_symbol,
            symbol,
            binding,
            "global binding",
        )
    }

    /// Return the canonical instance key for a symbol-backed item.
    pub(crate) fn symbol_instance_key(&self, symbol: GlobalSymbolId) -> InstanceKey {
        InstanceKey::symbol(symbol)
    }

    /// Return the lowered function id for a symbol-backed instance.
    pub(crate) fn function_for_symbol(
        &self,
        symbol: GlobalSymbolId,
    ) -> Option<mir::LocalNodeId<mir::Function>> {
        let instance = self.symbol_instance_key(symbol);
        self.functions_by_instance.get(&instance).copied()
    }

    /// Register a function binding and signature type for a symbol.
    pub(crate) fn register_function_binding_for_symbol(
        &mut self,
        symbol: GlobalSymbolId,
        function_id: mir::LocalNodeId<mir::Function>,
        signature_type: mir::LocalNodeId<mir::Type>,
    ) -> LowerResult<()> {
        let instance = self.symbol_instance_key(symbol);
        Self::register_function_binding_in_maps(
            self.module_id,
            &mut self.functions_by_instance,
            &mut self.function_signature_types,
            instance,
            function_id,
            signature_type,
        )
    }

    /// Register a function binding and signature type for an instance.
    pub(crate) fn register_function_binding_in_maps(
        module_id: ModuleId,
        functions_by_instance: &mut HashMap<InstanceKey, mir::LocalNodeId<mir::Function>>,
        function_signature_types: &mut HashMap<
            mir::LocalNodeId<mir::Function>,
            mir::LocalNodeId<mir::Type>,
        >,
        instance: InstanceKey,
        function_id: mir::LocalNodeId<mir::Function>,
        signature_type: mir::LocalNodeId<mir::Type>,
    ) -> LowerResult<()> {
        // register the function binding
        Self::insert_unique_entry(
            module_id,
            functions_by_instance,
            instance,
            function_id,
            "function instance",
        )?;

        // register the function signature type
        Self::insert_unique_entry(
            module_id,
            function_signature_types,
            function_id,
            signature_type,
            "function signature type",
        )
    }

    /// Insert interface slots for a symbol.
    pub(crate) fn insert_interface_slots(
        &mut self,
        symbol: GlobalSymbolId,
        slots: Vec<InterfaceEntry>,
    ) -> LowerResult<()> {
        // record interface slots once
        Self::insert_unique_entry(
            self.module_id,
            &mut self.interface_slots_by_symbol,
            symbol,
            slots,
            "interface slots",
        )
    }

    /// Insert a vtable global for a class symbol.
    pub(crate) fn insert_vtable_global(
        &mut self,
        symbol: GlobalSymbolId,
        vtable: VtableGlobal,
    ) -> LowerResult<()> {
        // record the vtable global once
        Self::insert_unique_entry(
            self.module_id,
            &mut self.vtable_globals_by_symbol,
            symbol,
            vtable,
            "vtable global",
        )
    }

    /// Insert a vtable id for a class symbol.
    pub(crate) fn insert_vtable_id(
        &mut self,
        symbol: GlobalSymbolId,
        table_id: mir::VtableId,
    ) -> LowerResult<()> {
        // record the vtable id once
        Self::insert_unique_entry(
            self.module_id,
            &mut self.vtable_ids_by_symbol,
            symbol,
            table_id,
            "vtable id",
        )
    }

    /// Insert a vtable table id for a class symbol.
    pub(crate) fn insert_vtable_table(
        &mut self,
        symbol: GlobalSymbolId,
        table_id: mir::VtableId,
    ) -> LowerResult<()> {
        // record the lowered vtable table once
        Self::insert_unique_entry(
            self.module_id,
            &mut self.vtable_by_symbol,
            symbol,
            table_id,
            "vtable table",
        )
    }

    /// Insert a virtual method slot id for a method symbol.
    pub(crate) fn insert_virtual_method_slot(
        &mut self,
        class_symbol: GlobalSymbolId,
        key: MethodKey,
        slot_id: u32,
        member_id: LocalNodeId<dir::Member>,
    ) -> LowerResult<()> {
        let slot_key = (class_symbol, key);
        if self.virtual_method_slots_by_key.contains_key(&slot_key) {
            return Err(LowerError::UnsupportedConstruct {
                node: member_id
                    .into_global_any(self.module_id)
                    .into_anchored(Some(self.profile)),
                message: format!("duplicate virtual method slot for {class_symbol:?} {key:?}"),
            });
        }

        self.virtual_method_slots_by_key.insert(slot_key, slot_id);
        Ok(())
    }

    /// Insert a precomputed itab id for an interface pair.
    pub(crate) fn insert_interface_itab_id(
        &mut self,
        pair: (GlobalSymbolId, GlobalSymbolId),
        table_id: mir::ItabId,
    ) -> LowerResult<()> {
        // record the interface itab id once
        Self::insert_unique_entry(
            self.module_id,
            &mut self.interface_itab_ids,
            pair,
            table_id,
            "interface itab id",
        )
    }

    /// Require a precomputed vtable id for a class symbol.
    pub(crate) fn require_vtable_id(&self, symbol: GlobalSymbolId) -> LowerResult<mir::VtableId> {
        self.vtable_ids_by_symbol
            .get(&symbol)
            .copied()
            .ok_or_else(|| LowerError::Internal {
                module: self.module_id,
                message: format!("missing vtable id for class {symbol:?}"),
            })
    }

    /// Require a precomputed itab id for a concrete/interface pair.
    pub(crate) fn require_itab_id(
        &self,
        pair: (GlobalSymbolId, GlobalSymbolId),
    ) -> LowerResult<mir::ItabId> {
        self.interface_itab_ids
            .get(&pair)
            .copied()
            .ok_or_else(|| LowerError::Internal {
                module: self.module_id,
                message: "missing itab id for interface pair".to_string(),
            })
    }

    /// Insert a lowered itab table id for an interface pair.
    pub(crate) fn insert_itab_table(
        &mut self,
        pair: (GlobalSymbolId, GlobalSymbolId),
        table_id: mir::ItabId,
    ) -> LowerResult<()> {
        // record the lowered itab table once
        Self::insert_unique_entry(
            self.module_id,
            &mut self.itab_by_pair,
            pair,
            table_id,
            "itab table",
        )
    }

    /// Insert a key into a map and reject duplicates.
    pub(crate) fn insert_unique_entry<K, V>(
        module_id: ModuleId,
        map: &mut HashMap<K, V>,
        key: K,
        value: V,
        label: &str,
    ) -> LowerResult<()>
    where
        K: std::fmt::Debug + std::hash::Hash + Eq,
    {
        // reject duplicate entries
        if map.contains_key(&key) {
            return Err(LowerError::Internal {
                module: module_id,
                message: format!("duplicate {label} for {key:?}"),
            });
        }

        // insert the new entry
        map.insert(key, value);
        Ok(())
    }

    /// Lower this entire DIR module to MIR (in-place).
    pub(crate) fn lower_module(&mut self) -> LowerResult<()> {
        // declare module level artifacts and initial lowering state
        self.declare()?;

        // lower reachable bodies and lazily realize types to fixpoint
        self.lower()?;

        // finish deferred tables and metadata
        self.finish_lowering()?;

        Ok(())
    }

    /// Declare module-level artifacts before body lowering.
    fn declare(&mut self) -> LowerResult<()> {
        // initialize builtin layouts
        self.initialize_string_type()?;

        // declare runtime string globals and aliases
        self.declare_string_literal_globals()?;
        self.declare_string_type_alias()?;

        // declare dispatch ids and slots
        self.declare_dispatch()?;

        // declare function shells before body lowering
        self.declare_functions()?;

        Ok(())
    }

    /// Lower root and queued function bodies.
    fn lower(&mut self) -> LowerResult<()> {
        // lower root expressions first
        for expression_id in self.dir_roots.iter().copied() {
            self.lower_root_expression(expression_id)?;
        }

        // lower queued nested functions and lambdas
        self.lower_pending_functions()?;

        Ok(())
    }

    /// Finish deferred module artifacts after body lowering.
    fn finish_lowering(&mut self) -> LowerResult<()> {
        // realize nominal types so layouts are cached
        self.lower_declared_types()?;
        self.declare_newtype_aliases()?;

        // emit final dispatch tables and type metadata
        self.emit_dispatch()?;
        self.emit_type_metadata()?;

        Ok(())
    }

    /// Predeclare globals for string literals used in this module.
    fn declare_string_literal_globals(&mut self) -> LowerResult<()> {
        // collect literal values and a representative anchor
        let mut literals = BTreeSet::new();
        let mut anchor = None;

        // collect string literals from expressions
        for (expression_id, expression) in self.dir_tree.iter_nodes_of_type::<dir::Expression>() {
            let dir::Expression::ScalarLiteral {
                value: dir::ScalarLiteral::String(string_id),
            } = expression
            else {
                continue;
            };

            literals.insert(*string_id);
            if anchor.is_none() {
                anchor = Some(
                    expression_id
                        .into_global_any(self.module_id)
                        .into_anchored(Some(self.profile)),
                );
            }
        }

        // collect string literal types
        for type_id in self.types.iter_type_ids() {
            let dir::Type::TypeLiteral {
                value: dir::TypeLiteral::ScalarLiteral(dir::ScalarLiteral::String(string_id)),
            } = self.types.get_type(type_id)
            else {
                continue;
            };

            literals.insert(*string_id);
            if anchor.is_none() {
                let source = self.types.get_type_source(type_id);
                anchor = Some(source.into_anchored(self.module_id, Some(self.profile)));
            }
        }

        // add runtime check messages when panic uses literals
        if matches!(self.runtime_checks.failure, CheckFailurePolicy::Panic) {
            if self.runtime_checks.bounds {
                let literal_id = self
                    .compiler
                    .program
                    .strings
                    .intern(RUNTIME_CHECK_MESSAGES.bounds_check);
                literals.insert(literal_id);
            }

            if self.runtime_checks.null {
                let literal_id = self
                    .compiler
                    .program
                    .strings
                    .intern(RUNTIME_CHECK_MESSAGES.null_check);
                literals.insert(literal_id);
            }

            if self.runtime_checks.division {
                let zero_id = self
                    .compiler
                    .program
                    .strings
                    .intern(RUNTIME_CHECK_MESSAGES.division_by_zero);
                literals.insert(zero_id);
                let overflow_id = self
                    .compiler
                    .program
                    .strings
                    .intern(RUNTIME_CHECK_MESSAGES.division_overflow);
                literals.insert(overflow_id);
            }

            if self.runtime_checks.overflow {
                let literal_id = self
                    .compiler
                    .program
                    .strings
                    .intern(RUNTIME_CHECK_MESSAGES.integer_overflow);
                literals.insert(literal_id);
            }

            if self.runtime_checks.shift {
                let literal_id = self
                    .compiler
                    .program
                    .strings
                    .intern(RUNTIME_CHECK_MESSAGES.shift_out_of_range);
                literals.insert(literal_id);
            }
        }

        // skip when no string literals are present
        if literals.is_empty() {
            return Ok(());
        }

        // require the builtin string layout
        let Some(string_type) = self.type_lowerer.string_type() else {
            if let Some(anchor) = anchor {
                return Err(LowerError::UnsupportedConstruct {
                    node: anchor,
                    message: "missing builtin String layout (load lib/native)".to_string(),
                });
            }
            return Err(LowerError::Internal {
                module: self.module_id,
                message: "missing builtin String layout (load lib/native)".to_string(),
            });
        };

        // create globals in deterministic order
        let mut ordered: Vec<_> = literals.into_iter().collect();
        ordered.sort_by(|left, right| {
            let left_value = self.compiler.program.strings.get(*left);
            let right_value = self.compiler.program.strings.get(*right);
            left_value.as_ref().cmp(right_value.as_ref())
        });
        for literal_id in ordered {
            let literal = self.compiler.program.strings.get(literal_id);
            let name = self.string_literal_global_name(literal_id);
            let global = self.builder.global_constant(
                &name,
                string_type,
                mir::GlobalInitializer::string(literal.as_ref()),
            );
            Self::insert_unique_entry(
                self.module_id,
                &mut self.string_literal_globals,
                literal_id,
                global,
                "string literal global",
            )?;
        }

        Ok(())
    }

    /// Ensure all nominal types are lowered into the type cache.
    fn lower_declared_types(&mut self) -> LowerResult<()> {
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
    fn emit_type_metadata(&mut self) -> LowerResult<()> {
        // collect cached types
        let mut cached_types = Vec::new();
        for (type_id, entry) in &self.type_lowerer.type_cache {
            let mir_type = match entry {
                TypeCacheEntry::Ready(mir_type) => *mir_type,
                TypeCacheEntry::InProgress => {
                    return Err(LowerError::Internal {
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
            self.layout_metadata_for_type(type_id, mir_type, anchor)?;
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
        // get some anchor for error reporting
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

    /// Declare the canonical string type alias in the MIR tree.
    fn declare_string_type_alias(&mut self) -> LowerResult<()> {
        // skip when no string literal globals exist
        if self.string_literal_globals.is_empty() {
            return Ok(());
        }

        // resolve the canonical string type id
        let Some(string_type) = self.type_lowerer.string_type() else {
            return Ok(());
        };

        // ensure the string type refers to a struct layout
        let string_layout = match self.builder.tree().get(string_type) {
            mir::Type::Reference { pointee, .. } => *pointee,
            _ => {
                return Err(LowerError::Internal {
                    module: self.module_id,
                    message: "string type is not a reference".to_string(),
                });
            }
        };

        // build the alias name for the string layout
        let alias_id = self.builder.intern("String");

        // skip when the alias already exists
        let alias_exists = self
            .builder
            .tree()
            .iter_nodes::<mir::TypeAlias>()
            .any(|(_, alias)| alias.name == alias_id);
        if alias_exists {
            return Ok(());
        }

        // register the alias in the tree
        self.builder.tree_mut().insert(mir::TypeAlias {
            name: alias_id,
            ty: string_layout,
        });

        Ok(())
    }

    /// Declare newtype aliases in the MIR tree.
    fn declare_newtype_aliases(&mut self) -> LowerResult<()> {
        // collect existing aliases by name
        let mut existing_aliases = HashSet::new();
        for (_, alias) in self.builder.tree().iter_nodes::<mir::TypeAlias>() {
            existing_aliases.insert(alias.name);
        }

        // emit aliases for each lowered newtype
        for symbol_id in 0..self.symbols.symbol_count() {
            // skip non newtype symbols
            let symbol = self.symbols.get_symbol_by_id(symbol_id);
            if symbol.ty != dir::SymbolType::Newtype {
                continue;
            }

            // resolve the instance type and its lowered mir type
            let local_id = dir::LocalSymbolId::new_typed(symbol_id, symbol.ty);
            let global_id = local_id.into_global(self.module_id);
            let Some(instance_type_id) = self.types.get_instance_type_id(global_id) else {
                continue;
            };
            let Some(mir_type) = self.type_lowerer.cached_type(instance_type_id) else {
                continue;
            };

            // resolve the qualified alias name
            let Some(name) = self.qualified_symbol_name(global_id) else {
                let anchor = self.type_anchor(instance_type_id);
                return Err(LowerError::UnsupportedConstruct {
                    node: anchor,
                    message: "newtype alias missing symbol name".to_string(),
                });
            };

            // insert the alias if it is not already present
            let name_id = self.builder.intern(&name);
            if existing_aliases.contains(&name_id) {
                continue;
            }

            self.builder.tree_mut().insert(mir::TypeAlias {
                name: name_id,
                ty: mir_type,
            });
            existing_aliases.insert(name_id);
        }

        Ok(())
    }
}
