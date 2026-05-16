use std::collections::{BTreeSet, HashMap, HashSet, VecDeque};
use std::sync::Arc;
use {destack_dir as dir, destack_mir as mir};

use destack_artifact::{
    DiagnosticAnchor, DirBound, DirParsed, GlobalEnvironment, LanguageIntrinsics,
};
use destack_core::StringPool;
use destack_source::{ModuleId, TargetId};
use destack_workspace::{CheckFailurePolicy, Module, ProfileId, ProviderContext, Target};
use indexmap::IndexSet;

use crate::{Compiler, CompilerError, CompilerResult, LowerError, LowerResult};

use crate::lower::{
    BuiltinTypeLayouts, DispatchTableGlobal, FunctionEnvironmentLayout, GlobalBinding, InstanceKey,
    InterfaceEntry, MethodKey, RUNTIME_CHECK_MESSAGES, RuntimeCheckConfig, RuntimeStatusLayout,
    TypeCacheEntry, TypeLowerer,
};

/// Context for lowering a DIR module to MIR.
#[allow(dead_code)]
pub(crate) struct ModuleLowerer<'a> {
    /// Provide access to the compiler for shared resources.
    pub(crate) compiler: &'a Compiler,
    /// Provide access to the pinned revision for cross-module reads.
    pub(crate) context: &'a dyn ProviderContext,
    /// Identify the module being lowered.
    pub(crate) module_id: ModuleId,
    /// Identify the profile used for DIR access.
    pub(crate) profile: ProfileId,
    /// Provide access to the source module data.
    pub(crate) module: &'a Module,
    /// Provide access to the DIR tree for expression lookup.
    pub(crate) dir_tree: &'a dir::Tree,
    /// Provide access to the root expressions for the module.
    pub(crate) dir_roots: &'a [dir::LocalNodeId<dir::Expression>],
    /// Provide access to declared DIR strings.
    pub(crate) strings: &'a StringPool,
    /// Stable module-level node for generated module state.
    pub(crate) module_node: dir::LocalNodeIdAny,
    /// Provide access to symbol metadata for type resolution.
    pub(crate) symbols: &'a dir::BindingTable<'a>,
    /// Provide access to inferred and declared types.
    pub(crate) types: &'a dir::TypeTable<'a>,
    /// Elaborated type guard entries.
    pub(crate) guards: &'a dir::GuardTable,
    /// Provide access to capture metadata for closures.
    pub(crate) captures: &'a dir::CaptureTable<'a>,
    /// Runtime check configuration for this target.
    pub(crate) runtime_checks: RuntimeCheckConfig,
    /// Language intrinsic bindings for this profile.
    pub(crate) language_intrinsics: Option<LanguageIntrinsics>,

    /// Build MIR nodes for this module.
    pub(crate) builder: mir::ModuleBuilder,
    /// Map lowered instances to MIR function ids.
    pub(crate) functions_by_instance: HashMap<InstanceKey, mir::LocalNodeId<mir::Function>>,
    /// Map MIR function ids to their signature types.
    pub(crate) function_signature_types:
        HashMap<mir::LocalNodeId<mir::Function>, mir::LocalNodeId<mir::Type>>,
    /// Map DIR symbols to MIR global bindings.
    pub(crate) globals_by_symbol: HashMap<dir::GlobalSymbolId, GlobalBinding>,
    /// Map string literal contents to MIR globals.
    pub(crate) string_literal_globals: HashMap<dir::StringId, mir::LocalNodeId<mir::Global>>,
    /// Map function environment layouts by function symbol.
    pub(crate) function_environment_layouts:
        HashMap<dir::GlobalSymbolId, FunctionEnvironmentLayout>,
    /// Cached empty function environment type.
    pub(crate) empty_function_environment_type: Option<mir::LocalNodeId<mir::Type>>,
    /// Cached empty function environment pointer type.
    pub(crate) empty_function_environment_pointer_type: Option<mir::LocalNodeId<mir::Type>>,
    /// Lower and cache DIR types into MIR types.
    pub(crate) type_lowerer: TypeLowerer<'a>,
    /// Synthetic name for call signatures in dispatch tables.
    pub(crate) dispatch_call_name: dir::StringId,
    /// Synthetic name for construct signatures in dispatch tables.
    pub(crate) dispatch_construct_name: dir::StringId,
    /// Synthetic name for vtable header fields.
    pub(crate) vtable_field_name: dir::StringId,

    /// Track interface slot data for dispatch lowering.
    pub(crate) interface_slots_by_symbol: HashMap<dir::GlobalSymbolId, Vec<InterfaceEntry>>,
    /// Track canonical interface dispatch field nodes by interface member.
    pub(crate) interface_dispatch_fields_by_member: HashMap<u32, mir::LocalNodeId<mir::Field>>,
    /// Track interface slot lowering in progress.
    pub(crate) interface_slots_in_progress: IndexSet<dir::GlobalSymbolId>,

    /// Track nominal layout lowering by symbol.
    pub(crate) nominal_layouts_by_symbol: HashMap<dir::GlobalSymbolId, mir::LocalNodeId<mir::Type>>,
    /// Track nominal layout lowering in progress.
    pub(crate) nominal_layouts_in_progress: IndexSet<dir::GlobalSymbolId>,

    /// Track class symbols that require vtable headers.
    pub(crate) vtable_layout_symbols: Option<HashSet<dir::GlobalSymbolId>>,
    /// Vtables that have been lowered.
    pub(crate) lowered_vtables: HashSet<dir::GlobalSymbolId>,
    /// Track vtable lowering in progress.
    pub(crate) vtable_in_progress: IndexSet<dir::GlobalSymbolId>,

    /// The interface tables that have been lowered.
    pub(crate) lowered_interface_tables: HashSet<(dir::GlobalSymbolId, dir::GlobalSymbolId)>,
    /// Track interface table lowering in progress.
    pub(crate) interface_table_in_progress: IndexSet<(dir::GlobalSymbolId, dir::GlobalSymbolId)>,

    /// Track whether dispatch declarations are initialized.
    pub(crate) dispatch_declared: bool,
    /// Class dispatch slots keyed by method symbol.
    pub(crate) virtual_method_slots_by_key: HashMap<(dir::GlobalSymbolId, MethodKey), u32>,

    /// Ordered list of class symbols that require vtables.
    pub(crate) vtable_class_symbols: Vec<dir::GlobalSymbolId>,
    /// Predeclared vtable globals keyed by class symbol.
    pub(crate) vtable_globals_by_symbol: HashMap<dir::GlobalSymbolId, DispatchTableGlobal>,

    /// Ordered interface table pairs for deterministic lowering.
    pub(crate) interface_table_pairs: Vec<(dir::GlobalSymbolId, dir::GlobalSymbolId)>,
    /// Predeclared interface table globals keyed by concrete and interface symbols.
    pub(crate) interface_table_globals_by_pair:
        HashMap<(dir::GlobalSymbolId, dir::GlobalSymbolId), DispatchTableGlobal>,

    /// Set of symbols marked as bindings.
    pub(crate) binding_symbols: HashSet<dir::GlobalSymbolId>,
    /// Binding ABI lowering toggle.
    pub(crate) binding_abi_lowering: bool,
    /// Cached runtime status layout for ABI lowering.
    pub(crate) runtime_status_layout: Option<RuntimeStatusLayout>,
    /// Cached binding function for taking runtime errors.
    pub(crate) take_platform_error_function: Option<mir::LocalNodeId<mir::Function>>,
    /// Function declarations waiting for body lowering.
    pub(crate) pending_function_bodies: VecDeque<dir::LocalNodeId<dir::Declaration>>,
    /// Function declarations already queued for body lowering.
    pub(crate) queued_function_bodies: HashSet<u32>,
}

#[allow(clippy::too_many_arguments)]
impl<'a> ModuleLowerer<'a> {
    /// Create a new module lowering context.
    pub(crate) fn new(
        compiler: &'a Compiler,
        context: &'a dyn ProviderContext,
        module: &'a Module,
        profile: ProfileId,
        dir_tree: &'a dir::Tree,
        dir_roots: &'a [dir::LocalNodeId<dir::Expression>],
        strings: &'a StringPool,
        module_node: dir::LocalNodeIdAny,
        symbols: &'a dir::BindingTable<'a>,
        types: &'a dir::TypeTable<'a>,
        guards: &'a dir::GuardTable,
        captures: &'a dir::CaptureTable<'a>,
        target: &'a TargetId,
        pointer_bytes: u8,
    ) -> CompilerResult<Self> {
        // initialize the module builder
        let mut builder = mir::ModuleBuilder::new();
        builder.set_pointer_bytes(pointer_bytes);

        // seed mir strings with the shared module pool
        builder.strings().ensure_all_from(strings);

        // resolve vector builtin symbols for vector lowering
        let vector_symbol =
            Self::language_item_for(compiler, context, profile, dir::LanguageItem::Vector)?;

        // create the type lowerer
        let type_lowerer = TypeLowerer::new(
            &mut builder,
            pointer_bytes,
            compiler,
            context,
            strings,
            profile,
            dir_tree,
            symbols,
            vector_symbol,
        );
        let dispatch_call_name = builder.intern("@call");
        let dispatch_construct_name = builder.intern("@new");
        let vtable_field_name = builder.intern("vtable");

        // resolve the target configuration
        let target_config = Self::target_config_for_module(compiler, context, module, target)?;

        // resolve runtime check policies
        let debug = compiler.profile(context.revision(), profile).env.debug;
        let runtime_checks = RuntimeCheckConfig::from_target(&target_config, debug);
        let binding_abi_lowering = target_config.emit.is_native();

        let language_intrinsics = Self::language_intrinsics(compiler, context, profile)?;

        Ok(Self {
            compiler,
            context,
            module_id: module.id,
            profile,
            module,
            dir_tree,
            dir_roots,
            strings,
            module_node,
            symbols,
            types,
            guards,
            captures,
            runtime_checks,
            language_intrinsics,
            builder,
            functions_by_instance: HashMap::new(),
            function_signature_types: HashMap::new(),
            globals_by_symbol: HashMap::new(),
            string_literal_globals: HashMap::new(),
            function_environment_layouts: HashMap::new(),
            empty_function_environment_type: None,
            empty_function_environment_pointer_type: None,
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
            lowered_vtables: HashSet::new(),
            vtable_in_progress: IndexSet::new(),
            lowered_interface_tables: HashSet::new(),
            interface_table_in_progress: IndexSet::new(),
            dispatch_declared: false,
            virtual_method_slots_by_key: HashMap::new(),
            vtable_class_symbols: Vec::new(),
            vtable_globals_by_symbol: HashMap::new(),
            interface_table_pairs: Vec::new(),
            interface_table_globals_by_pair: HashMap::new(),
            binding_symbols: HashSet::new(),
            binding_abi_lowering,
            runtime_status_layout: None,
            take_platform_error_function: None,
            pending_function_bodies: VecDeque::new(),
            queued_function_bodies: HashSet::new(),
        })
    }

    /// Build the intrinsic binding registry for the active profile.
    fn language_intrinsics(
        compiler: &Compiler,
        context: &dyn ProviderContext,
        profile: ProfileId,
    ) -> CompilerResult<Option<LanguageIntrinsics>> {
        let environment = compiler
            .global_environment(context, profile)
            .map_err(CompilerError::from)?;

        let mut intrinsics = LanguageIntrinsics::new();
        for module_id in &environment.modules {
            let parsed = compiler
                .dir_parsed(context, *module_id)
                .map_err(CompilerError::from)?;
            let bound = compiler
                .dir_bound(context, *module_id, profile)
                .map_err(CompilerError::from)?;
            Self::collect_intrinsic_bindings(
                *module_id,
                &parsed.tree,
                bound.as_ref(),
                compiler.repository.string_pool().as_ref(),
                &mut intrinsics,
            );
        }

        Ok(Some(intrinsics))
    }

    /// Read the global environment used by this lowerer.
    pub(crate) fn global_environment(&self) -> CompilerResult<Arc<GlobalEnvironment>> {
        self.compiler
            .global_environment(self.context, self.profile)
            .map_err(CompilerError::from)
    }

    /// Resolve one declared global symbol in this lowerer.
    pub(crate) fn declared_global_symbol(
        &self,
        name: &str,
    ) -> CompilerResult<Option<dir::GlobalSymbolId>> {
        let environment = self.global_environment()?;

        Ok(environment.language.symbol(name))
    }

    /// Resolve one compiler language symbol in this lowerer.
    pub(crate) fn language_item(
        &self,
        symbol: dir::LanguageItem,
    ) -> CompilerResult<Option<dir::GlobalSymbolId>> {
        let environment = self.global_environment()?;

        Ok(environment.language.item(symbol))
    }

    /// Resolve one language item before the lowerer has been constructed.
    fn language_item_for(
        compiler: &Compiler,
        context: &dyn ProviderContext,
        profile: ProfileId,
        symbol: dir::LanguageItem,
    ) -> CompilerResult<Option<dir::GlobalSymbolId>> {
        let environment = compiler
            .global_environment(context, profile)
            .map_err(CompilerError::from)?;

        Ok(environment.language.item(symbol))
    }

    /// Collect intrinsic bindings from one bound DIR module.
    fn collect_intrinsic_bindings(
        module_id: ModuleId,
        tree: &dir::Tree,
        bound: &DirBound,
        strings: &StringPool,
        intrinsics: &mut LanguageIntrinsics,
    ) {
        for symbol_id in bound.bindings.symbol_ids() {
            let symbol = bound.bindings.get_symbol(symbol_id);
            let Some(declaration) = symbol.declaration else {
                continue;
            };
            if declaration.module_id != module_id {
                continue;
            }

            let Some(name) =
                Self::intrinsic_name_for_declaration(tree, bound, strings, declaration.local_id)
            else {
                continue;
            };

            let symbol = symbol_id.into_global(module_id);
            intrinsics.names_by_symbol.insert(symbol, name.clone());
            intrinsics.symbols_by_name.insert(name, symbol);
        }
    }

    /// Resolve the intrinsic binding name attached to one declaration.
    fn intrinsic_name_for_declaration(
        tree: &dir::Tree,
        bound: &DirBound,
        strings: &StringPool,
        declaration: dir::LocalNodeIdAny,
    ) -> Option<String> {
        for decorator_id in tree.get_decorators(declaration.id) {
            let decorator = tree.get(decorator_id);
            let Some(name) = Self::intrinsic_name_for_expression(
                tree,
                bound,
                strings,
                decorator.expression,
                declaration.id,
            ) else {
                continue;
            };

            return Some(name);
        }

        None
    }

    /// Resolve an intrinsic decorator expression to its binding name.
    fn intrinsic_name_for_expression(
        tree: &dir::Tree,
        bound: &DirBound,
        strings: &StringPool,
        expression_id: dir::LocalNodeId<dir::Expression>,
        declaration_id: u32,
    ) -> Option<String> {
        let expression = tree.get(expression_id);

        match expression {
            dir::Expression::Call {
                left, arguments, ..
            } if Self::is_intrinsic_decorator_name(tree, *left) => {
                Self::intrinsic_name_for_arguments(tree, strings, arguments)
            }
            _ if Self::is_intrinsic_decorator_name(tree, expression_id) => {
                Self::default_intrinsic_name(bound, strings, declaration_id)
            }
            _ => None,
        }
    }

    /// Check whether an expression names the intrinsic decorator.
    fn is_intrinsic_decorator_name(
        tree: &dir::Tree,
        expression_id: dir::LocalNodeId<dir::Expression>,
    ) -> bool {
        let expression = tree.get(expression_id);
        let name = match expression {
            dir::Expression::Identifier { name } => Some(*name),
            dir::Expression::QualifiedReference { path, .. } => path.last_segment(),
            _ => None,
        };

        name == Some(dir::StringId::for_text("intrinsic"))
    }

    /// Resolve the explicit intrinsic binding name from decorator arguments.
    fn intrinsic_name_for_arguments(
        tree: &dir::Tree,
        strings: &StringPool,
        arguments: &[dir::LocalNodeId<dir::Argument>],
    ) -> Option<String> {
        let [argument_id] = arguments else {
            return None;
        };
        let argument = tree.get(*argument_id);
        let value = argument.value()?;
        let expression = tree.get(value);

        match expression {
            dir::Expression::ScalarLiteral(dir::ScalarLiteral::String(name)) => {
                Some(strings.get(*name).to_string())
            }
            _ => None,
        }
    }

    /// Resolve the default intrinsic binding name from the decorated declaration.
    fn default_intrinsic_name(
        bound: &DirBound,
        strings: &StringPool,
        declaration_id: u32,
    ) -> Option<String> {
        let symbol = bound
            .bindings
            .symbol_ids()
            .map(|symbol_id| bound.bindings.get_symbol(symbol_id))
            .find(|symbol| {
                symbol
                    .declaration
                    .is_some_and(|declaration| declaration.local_id.id == declaration_id)
            })?;

        let dir::StaticKey::Name(name) = symbol.key? else {
            return None;
        };

        Some(strings.get(name).to_string())
    }

    /// Read one committed bound DIR snapshot for a module when available.
    pub(crate) fn dir_bound_if_present(&self, module_id: ModuleId) -> Option<Arc<DirBound>> {
        self.compiler
            .dir_bound(self.context, module_id, self.profile)
            .ok()
    }

    /// Read one committed parsed DIR snapshot for a module when available.
    pub(crate) fn dir_parsed_if_present(&self, module_id: ModuleId) -> Option<Arc<DirParsed>> {
        self.compiler.dir_parsed(self.context, module_id).ok()
    }

    /// Resolve the target configuration for a module.
    fn target_config_for_module(
        compiler: &Compiler,
        context: &dyn ProviderContext,
        module: &Module,
        target: &TargetId,
    ) -> LowerResult<Target> {
        compiler
            .target_or_builtin(context, *target)
            .ok_or_else(|| LowerError::Internal {
                anchor: (module.id).into(),
                module: module.id,
                message: format!(
                    "missing target config for module {:?} with target {target:?}",
                    module.id
                ),
            })
    }

    /// Resolve allocation mode for a symbol based on decorators and profile flags.
    pub(crate) fn allocation_mode_for_symbol(
        &self,
        symbol: dir::GlobalSymbolId,
    ) -> mir::AllocationMode {
        let symbol = self.symbols.get_symbol(symbol.local_id);

        // prefer no heap when explicitly requested
        if self.symbol_has_language_decorator(symbol, dir::LanguageItem::NoHeap) {
            return mir::AllocationMode::NoHeap;
        }

        // apply no managed only when requested explicitly
        if self.symbol_has_language_decorator(symbol, dir::LanguageItem::NoManaged) {
            return mir::AllocationMode::NoManaged;
        }

        mir::AllocationMode::Any
    }

    /// Return whether one symbol has a compiler-known decorator.
    fn symbol_has_language_decorator(
        &self,
        symbol: &dir::Symbol,
        language_item: dir::LanguageItem,
    ) -> bool {
        let Some(declaration) = symbol.declaration else {
            return false;
        };
        let Ok(Some(target_symbol)) = self.language_item(language_item) else {
            return false;
        };

        // inspect attached decorator expressions
        for decorator_id in self.dir_tree.get_decorators(declaration.local_id.id) {
            let decorator = self.dir_tree.get(decorator_id);
            let expression = self.dir_tree.get(decorator.expression);
            let decorator_node = decorator.expression.into_global_any(self.module_id);
            if self
                .types
                .symbol_resolution(decorator_node)
                .is_some_and(|symbol| symbol == target_symbol)
            {
                return true;
            }
            if expression_is_unqualified_name(expression, language_item.export_name()) {
                return true;
            }
        }

        false
    }

    /// Create a MissingType error for a node.
    pub(crate) fn missing_type_error(&self, node_id: dir::GlobalNodeIdAny) -> LowerError {
        let node = node_id.into_anchored(Some(self.profile));

        LowerError::MissingType {
            anchor: self.diagnostic_anchor(node),
        }
    }

    /// Return the diagnostic anchor for one DIR node.
    pub(crate) fn diagnostic_anchor(&self, node: dir::AnchoredGlobalNodeId) -> DiagnosticAnchor {
        assert_eq!(
            self.module_id,
            node.module_id(),
            "lower diagnostic node belongs to a different module"
        );

        let span = self
            .dir_tree
            .get_span_by_id(node.local_id().id)
            .expect("lower diagnostic node is missing a source span");

        DiagnosticAnchor::Span(span)
    }

    /// Resolve a declared or inferred type id for a node or return MissingType.
    pub(crate) fn declared_or_inferred_type_id_for_node_or_error(
        &self,
        node_id: dir::GlobalNodeIdAny,
    ) -> LowerResult<dir::LocalTypeId> {
        self.types
            .get_declared_or_inferred_type_id(node_id)
            .ok_or_else(|| self.missing_type_error(node_id))
    }

    /// Resolve a symbol type id or return MissingType.
    pub(crate) fn type_id_for_symbol_or_error(
        &self,
        symbol: dir::GlobalSymbolId,
        anchor: dir::AnchoredGlobalNodeId,
    ) -> LowerResult<dir::LocalTypeId> {
        self.types
            .symbol_type_id(self.symbols, symbol)
            .ok_or_else(|| LowerError::MissingType {
                anchor: self.diagnostic_anchor(anchor),
            })
    }

    /// Insert a global binding for a symbol.
    pub(crate) fn insert_global_binding(
        &mut self,
        symbol: dir::GlobalSymbolId,
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
    pub(crate) fn symbol_instance_key(&self, symbol: dir::GlobalSymbolId) -> InstanceKey {
        InstanceKey::symbol(symbol)
    }

    /// Return the lowered function id for a symbol-backed instance.
    pub(crate) fn function_for_symbol(
        &self,
        symbol: dir::GlobalSymbolId,
    ) -> Option<mir::LocalNodeId<mir::Function>> {
        let instance = self.symbol_instance_key(symbol);
        self.functions_by_instance.get(&instance).copied()
    }

    /// Register a function binding and signature type for a symbol.
    pub(crate) fn register_function_binding_for_symbol(
        &mut self,
        symbol: dir::GlobalSymbolId,
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
        symbol: dir::GlobalSymbolId,
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
        symbol: dir::GlobalSymbolId,
        vtable: DispatchTableGlobal,
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

    /// Record a lowered vtable for a class symbol.
    pub(crate) fn record_vtable(&mut self, symbol: dir::GlobalSymbolId) -> LowerResult<()> {
        // record the lowered vtable once
        if !self.lowered_vtables.insert(symbol) {
            return Err(LowerError::Internal {
                anchor: (self.module_id).into(),
                module: self.module_id,
                message: format!("duplicate vtable for class {symbol:?}"),
            }
            .into());
        }

        Ok(())
    }

    /// Insert a class method slot for a method symbol.
    pub(crate) fn insert_virtual_method_slot(
        &mut self,
        class_symbol: dir::GlobalSymbolId,
        key: MethodKey,
        slot: u32,
        member_id: dir::LocalNodeId<dir::Member>,
    ) -> LowerResult<()> {
        let slot_key = (class_symbol, key);
        if self.virtual_method_slots_by_key.contains_key(&slot_key) {
            return Err(LowerError::UnsupportedConstruct {
                anchor: self.diagnostic_anchor(
                    member_id
                        .into_global_any(self.module_id)
                        .into_anchored(Some(self.profile)),
                ),
                message: format!("duplicate class method slot for {class_symbol:?} {key:?}"),
            }
            .into());
        }

        self.virtual_method_slots_by_key.insert(slot_key, slot);
        Ok(())
    }

    /// Insert an interface table global for an interface pair.
    pub(crate) fn insert_interface_table_global(
        &mut self,
        pair: (dir::GlobalSymbolId, dir::GlobalSymbolId),
        interface_table: DispatchTableGlobal,
    ) -> LowerResult<()> {
        // record the interface table global once
        Self::insert_unique_entry(
            self.module_id,
            &mut self.interface_table_globals_by_pair,
            pair,
            interface_table,
            "interface table global",
        )
    }

    /// Record a lowered interface table for an interface pair.
    pub(crate) fn record_interface_table(
        &mut self,
        pair: (dir::GlobalSymbolId, dir::GlobalSymbolId),
    ) -> LowerResult<()> {
        // record the lowered interface table once
        if !self.lowered_interface_tables.insert(pair) {
            return Err(LowerError::Internal {
                anchor: (self.module_id).into(),
                module: self.module_id,
                message: "duplicate interface table for interface pair".to_string(),
            }
            .into());
        }

        Ok(())
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
                anchor: (module_id).into(),
                module: module_id,
                message: format!("duplicate {label} for {key:?}"),
            }
            .into());
        }

        // insert the new entry
        map.insert(key, value);
        Ok(())
    }

    /// Lower this entire DIR module to MIR (in-place).
    pub(crate) fn lower_module(&mut self) -> CompilerResult<()> {
        // declare module level artifacts and initial lowering state
        self.declare()?;

        // lower reachable bodies and lazily realize types to fixpoint
        self.lower()?;

        // finish deferred tables and metadata
        self.finish_lowering()?;

        Ok(())
    }

    /// Declare module-level artifacts before body lowering.
    fn declare(&mut self) -> CompilerResult<()> {
        // declare runtime string globals
        self.declare_string_literal_globals()?;

        // declare dispatch ids and slots
        self.declare_dispatch()?;

        // declare nominal types before body lowering
        self.lower_declared_types()?;
        self.declare_nominal_aliases()?;

        // declare module storage before functions can reference it
        self.declare_static_member_fields()?;

        // declare function shells before body lowering
        self.declare_functions()?;

        Ok(())
    }

    /// Lower root and queued function bodies.
    fn lower(&mut self) -> CompilerResult<()> {
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
        // emit final dispatch tables and type metadata
        self.emit_dispatch()?;
        self.emit_type_metadata()?;

        Ok(())
    }

    /// Predeclare globals for string literals used in this module.
    fn declare_string_literal_globals(&mut self) -> CompilerResult<()> {
        // collect literal values and a representative anchor
        let mut literals = BTreeSet::new();
        let mut anchor = None;

        // collect string literals from expressions
        for (expression_id, expression) in self.dir_tree.iter_nodes_of_type::<dir::Expression>() {
            if anchor.is_none() {
                anchor = Some(
                    expression_id
                        .into_global_any(self.module_id)
                        .into_anchored(Some(self.profile)),
                );
            }

            let dir::Expression::ScalarLiteral(dir::ScalarLiteral::String(string_id)) = expression
            else {
                continue;
            };

            literals.insert(*string_id);
        }

        // add runtime check messages when panic uses literals
        if matches!(self.runtime_checks.failure, CheckFailurePolicy::Panic) {
            if self.runtime_checks.bounds {
                let literal_id = dir::StringId::for_text(RUNTIME_CHECK_MESSAGES.bounds_check);
                literals.insert(literal_id);
            }

            if self.runtime_checks.null {
                let literal_id = dir::StringId::for_text(RUNTIME_CHECK_MESSAGES.null_check);
                literals.insert(literal_id);
            }

            if self.runtime_checks.division {
                let zero_id = dir::StringId::for_text(RUNTIME_CHECK_MESSAGES.division_by_zero);
                literals.insert(zero_id);
                let overflow_id = dir::StringId::for_text(RUNTIME_CHECK_MESSAGES.division_overflow);
                literals.insert(overflow_id);
            }

            if self.runtime_checks.overflow {
                let literal_id = dir::StringId::for_text(RUNTIME_CHECK_MESSAGES.integer_overflow);
                literals.insert(literal_id);
            }

            if self.runtime_checks.shift {
                let literal_id = dir::StringId::for_text(RUNTIME_CHECK_MESSAGES.shift_out_of_range);
                literals.insert(literal_id);
            }
        }

        // skip when no string literals are present
        if literals.is_empty() {
            return Ok(());
        }

        // require the language item string layout
        let Some(anchor) = anchor else {
            return Err(LowerError::Internal {
                anchor: (self.module_id).into(),
                module: self.module_id,
                message: "missing string literal anchor".to_string(),
            }
            .into());
        };
        let string_type = if let Some(string_type) = self.type_lowerer.string_type() {
            string_type
        } else {
            let mut builtin_layouts = BuiltinTypeLayouts::new(
                self.compiler,
                self.context,
                self.profile,
                &mut self.builder,
                &mut self.type_lowerer,
            );
            builtin_layouts
                .string_type_for_builtin(anchor)?
                .ok_or_else(|| LowerError::UnsupportedConstruct {
                    anchor: self.diagnostic_anchor(anchor),
                    message: "missing language item String layout (load core)".to_string(),
                })
                .map_err(CompilerError::from)?
        };

        // create globals in deterministic order
        let mut ordered: Vec<_> = literals.into_iter().collect();
        ordered.sort_by(|left, right| {
            let left_value = self.strings.get(*left);
            let right_value = self.strings.get(*right);
            left_value.cmp(right_value)
        });
        for literal_id in ordered {
            let literal = self.strings.get(literal_id);
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

    /// Ensure all nominal types are lowered into the primitive type cache.
    fn lower_declared_types(&mut self) -> LowerResult<()> {
        // track symbols we've already finalized
        let mut seen = HashSet::new();
        for (declaration_id, declaration) in self.dir_tree.iter_nodes_of_type::<dir::Declaration>()
        {
            let symbol = match declaration {
                dir::Declaration::Struct(_)
                | dir::Declaration::Class(_)
                | dir::Declaration::Enum(_)
                | dir::Declaration::Interface(_) => self.require_symbol_for_node(declaration_id)?,
                dir::Declaration::Type(declaration) => {
                    if !declaration.is_nominal {
                        continue;
                    }
                    self.require_symbol_for_node(declaration_id)?
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

            if matches!(
                self.symbol_form(symbol),
                Some(dir::SymbolForm::Class | dir::SymbolForm::Interface)
            ) && let Some(reference_type_id) = self.nominal_reference_type_id_for_symbol(symbol)
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
                        anchor: (self.module_id).into(),
                        module: self.module_id,
                        message: "type lowering cache left in progress".to_string(),
                    }
                    .into());
                }
            };
            cached_types.push((*type_id, mir_type));
        }

        // collect all MIR types so synthetic aggregate layouts get metadata too
        let all_mir_types: Vec<_> = self
            .builder
            .tree()
            .iter_nodes::<mir::Type>()
            .map(|(ty, _)| ty)
            .collect();
        let synthetic_anchor = self
            .module_node
            .into_global(self.module_id)
            .into_anchored(Some(self.profile));

        // finalize metadata for each cached type
        for (type_id, mir_type) in cached_types {
            let anchor = self.type_anchor(type_id);
            self.metadata_name_for_type(type_id, mir_type, anchor)?;
            self.layout_metadata_for_type(type_id, mir_type, anchor)?;
            if let Some(symbol) = self.types.symbol_for_instance_type(type_id) {
                self.lineage_metadata_for_symbol(symbol, mir_type, anchor)?;
            }
        }

        // finish raw layout metadata for MIR-only aggregate repr types
        for mir_type in all_mir_types {
            self.layout_metadata_for_mir_type(mir_type, synthetic_anchor)?;
        }

        Ok(())
    }

    /// Finish the module lowering process and return the resulting MIR tree and string pool.
    pub(crate) fn finish(mut self) -> (mir::Tree, StringPool) {
        // copy final DIR source spans into MIR
        for node_id in 0..self.builder.tree().node_count() as u32 {
            let Some(source_id) = self.builder.tree().get_source(node_id) else {
                continue;
            };
            let span = self
                .dir_tree
                .get_span_by_id(source_id)
                .expect("lowered DIR source is missing a source span");
            self.builder.tree_mut().set_span_by_id(node_id, span);
        }

        self.builder.finish_mutable()
    }

    /// Declare nominal aliases in the MIR tree.
    fn declare_nominal_aliases(&mut self) -> LowerResult<()> {
        // collect existing aliases by name
        let mut existing_aliases = HashSet::new();
        for (_, alias) in self.builder.tree().iter_nodes::<mir::TypeAlias>() {
            existing_aliases.insert(alias.name);
        }

        // emit aliases for each lowered nominal type
        for symbol_id in 0..self.symbols.symbol_count() {
            // skip non nominal symbols
            let symbol = self.symbols.get_symbol_by_id(symbol_id);
            if !matches!(
                symbol.form,
                dir::SymbolForm::Newtype | dir::SymbolForm::Enum
            ) {
                continue;
            }

            // resolve the instance type and its lowered mir type
            let local_id = dir::LocalSymbolId::new(symbol_id);
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
                    anchor: self.diagnostic_anchor(anchor),
                    message: "nominal alias missing symbol name".to_string(),
                }
                .into());
            };

            // insert the alias if it is not already present
            let name_id = self.builder.intern(&name);
            if existing_aliases.contains(&name_id) {
                continue;
            }

            self.builder.tree_mut().insert(mir::TypeAlias {
                name: name_id,
                ty: mir_type.into(),
            });
            existing_aliases.insert(name_id);
        }

        Ok(())
    }
}

/// Return whether one expression is an unqualified reference to a name.
fn expression_is_unqualified_name(expression: &dir::Expression, name: &str) -> bool {
    let name = dir::StringId::for_text(name);
    match expression {
        dir::Expression::Identifier { name: actual } => *actual == name,
        dir::Expression::QualifiedReference { path, .. } => {
            path.segments.len() == 1 && path.segments.first().copied() == Some(name)
        }
        _ => false,
    }
}
