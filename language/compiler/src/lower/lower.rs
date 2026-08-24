use std::hash::{DefaultHasher, Hash, Hasher};
use std::sync::Arc;

use destack_artifact::{DiagnosticLike, MirLowered};
use destack_core::{FxIndexMap, StringId, StringPool};
use destack_dir as dir;
use destack_mir as mir;
use destack_source::ModuleId;

use crate::lower::{
    FunctionDeclaration, FunctionLowerer, GenericInstanceKey, Implementer, LowerModuleState,
    NominalInstance, NominalState,
};
use crate::{CompilerError, CompilerResult};

/// One lowering outcome a body reads: the lowered value, or the banked diagnostic of the first
/// failure.
pub(in crate::lower) type Lowered<T> = Result<T, Arc<dyn DiagnosticLike>>;

/// Lowering state for one module.
///
/// Lowering reports failures in three ways.
/// `CompilerResult::Internal` reports a compiler bug.
/// `CompilerResult::Diagnostic` reports a user error where lowering raises it.
/// `Lowered` banks an outcome at first read and raises it for every body that reads it.
pub(crate) struct ModuleLowerer<'a> {
    /// The module being lowered.
    pub(in crate::lower) module: ModuleId,
    /// The source string pool.
    pub(in crate::lower) strings: &'a StringPool,
    /// The state of every reachable module.
    pub(in crate::lower) modules: FxIndexMap<ModuleId, LowerModuleState>,

    /// The declaration outcome for each callable instance key.
    pub(in crate::lower) functions: FxIndexMap<GenericInstanceKey, FunctionDeclaration>,
    /// The synthesized default constructors queued for body lowering.
    pub(in crate::lower) synthesized_constructors: Vec<(
        dir::GlobalSymbolId,
        Option<(ModuleId, dir::LocalInstanceId)>,
        mir::FunctionId,
    )>,
    /// The sema instance behind each closed selection, keyed by the arguments' structure.
    pub(in crate::lower) specializations:
        FxIndexMap<(dir::GlobalSymbolId, Vec<u64>), (ModuleId, dir::LocalInstanceId)>,
    /// The MIR representation behind each type the bodies read.
    pub(in crate::lower) representations:
        FxIndexMap<dir::GlobalTypeId, Lowered<mir::LocalNodeId<mir::Type>>>,
    /// The dispatch shape behind each constraint the bodies read.
    pub(in crate::lower) constraints:
        FxIndexMap<dir::GlobalTypeId, Lowered<mir::LocalNodeId<mir::Type>>>,
    /// The nominal instance behind each application type the bodies read.
    pub(in crate::lower) stored_nominals: FxIndexMap<dir::GlobalTypeId, Lowered<NominalInstance>>,
    /// The state of each nominal representation being lowered or already lowered.
    pub(in crate::lower) nominal_states: FxIndexMap<GenericInstanceKey, NominalState>,
    /// The global declared for each module constant.
    pub(in crate::lower) globals:
        FxIndexMap<dir::GlobalSymbolId, Lowered<mir::LocalNodeId<mir::Global>>>,
    /// Loaded language item symbols keyed by item.
    pub(in crate::lower) language_symbols: FxIndexMap<dir::LanguageItem, dir::GlobalSymbolId>,
    /// Loaded language items keyed by symbol.
    pub(in crate::lower) language_items: FxIndexMap<dir::GlobalSymbolId, dir::LanguageItem>,
    /// Canonical items and members keyed by lowered declarations.
    pub(in crate::lower) language: mir::LanguageTable,
    /// The authored drop hook member declared beside each Drop-conforming nominal.
    pub(in crate::lower) drop_hooks: FxIndexMap<dir::GlobalSymbolId, dir::GlobalSymbolId>,
    /// The immortal String object and value type per collected literal content.
    pub(in crate::lower) string_literals:
        FxIndexMap<StringId, Lowered<(mir::GlobalId, mir::LocalNodeId<mir::Type>)>>,
    /// The immortal BigInt object and value type per collected literal value.
    pub(in crate::lower) bigint_literals:
        FxIndexMap<i64, Lowered<(mir::GlobalId, mir::LocalNodeId<mir::Type>)>>,
    /// The runtime bindings stored by the module initializer, in order.
    pub(in crate::lower) initializers: Vec<(
        mir::LocalNodeId<mir::Global>,
        dir::LocalNodeId<dir::Expression>,
    )>,

    /// The dispatch shape registered for each lowered constraint.
    pub(in crate::lower) dynamic_shapes: FxIndexMap<mir::LocalNodeId<mir::Type>, mir::DynamicShape>,
    /// The implementer registered for each erased concrete type and constraint.
    pub(in crate::lower) implementers:
        FxIndexMap<(mir::LocalNodeId<mir::Type>, mir::LocalNodeId<mir::Type>), Implementer>,
}

impl<'a> ModuleLowerer<'a> {
    /// Create the lowering state over one materialized module.
    pub(crate) fn new(
        module: ModuleId,
        strings: &'a StringPool,
        modules: FxIndexMap<ModuleId, LowerModuleState>,
    ) -> Self {
        Self {
            module,
            strings,
            modules,
            functions: FxIndexMap::default(),
            specializations: FxIndexMap::default(),
            synthesized_constructors: Vec::new(),
            representations: FxIndexMap::default(),
            constraints: FxIndexMap::default(),
            stored_nominals: FxIndexMap::default(),
            nominal_states: FxIndexMap::default(),
            globals: FxIndexMap::default(),
            language_symbols: FxIndexMap::default(),
            language_items: FxIndexMap::default(),
            language: mir::LanguageTable::default(),
            drop_hooks: FxIndexMap::default(),
            string_literals: FxIndexMap::default(),
            bigint_literals: FxIndexMap::default(),
            initializers: Vec::new(),
            dynamic_shapes: FxIndexMap::default(),
            implementers: FxIndexMap::default(),
        }
    }

    /// Index the instances sema closed by their structural selection.
    fn index_specializations(&mut self) -> CompilerResult<()> {
        // every loaded module contributes its own closed instances
        let mut entries = Vec::new();
        for (module, state) in &self.modules {
            for (instance, entry) in state.generics.iter_instances() {
                let arguments: Vec<_> = entry
                    .selection
                    .arguments
                    .iter()
                    .map(|binding| binding.argument)
                    .collect();
                entries.push((entry.selection.symbol, arguments, *module, instance));
            }
        }

        // key each instance by the structure of its arguments
        let mut specializations = FxIndexMap::default();
        for (symbol, arguments, module, instance) in entries {
            let mut keys = Vec::with_capacity(arguments.len());
            for argument in arguments {
                keys.push(self.structural_type_key(argument)?);
            }
            specializations
                .entry((symbol, keys))
                .or_insert((module, instance));
        }
        self.specializations = specializations;

        Ok(())
    }

    /// Return the sema instance behind one closed selection.
    pub(in crate::lower) fn specialization_of(
        &self,
        symbol: dir::GlobalSymbolId,
        arguments: &[dir::GlobalTypeId],
    ) -> CompilerResult<Option<(ModuleId, dir::LocalInstanceId)>> {
        let mut keys = Vec::with_capacity(arguments.len());
        for argument in arguments {
            keys.push(self.structural_type_key(*argument)?);
        }

        Ok(self.specializations.get(&(symbol, keys)).copied())
    }

    /// Hash one type's structure, stable across module interning.
    pub(in crate::lower) fn structural_type_key(
        &self,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<u64> {
        let mut hasher = DefaultHasher::new();
        let mut visiting = Vec::new();
        self.hash_type_structure(ty, &mut hasher, &mut visiting)?;

        Ok(hasher.finish())
    }

    /// Hash one type's head and children into the running structural key.
    fn hash_type_structure(
        &self,
        ty: dir::GlobalTypeId,
        hasher: &mut impl Hasher,
        visiting: &mut Vec<dir::GlobalTypeId>,
    ) -> CompilerResult<()> {
        // close recursive structures at their first revisit
        if visiting.contains(&ty) {
            u8::MAX.hash(hasher);

            return Ok(());
        }

        // lifetime components never shape a specialization
        if self.type_is_lifetime(ty)? {
            0u8.hash(hasher);

            return Ok(());
        }
        visiting.push(ty);

        // hash the head and its non-type payload
        let kind = self.ty(ty)?;
        std::mem::discriminant(&kind).hash(hasher);
        match &kind {
            dir::Type::Primitive(primitive) => primitive.hash(hasher),
            dir::Type::Literal(literal) => literal.hash(hasher),
            dir::Type::Memory(memory) => memory.hash(hasher),
            dir::Type::Application(application) => application.symbol.hash(hasher),
            dir::Type::Reference(reference) => reference.symbol.hash(hasher),
            dir::Type::Parameter(parameter) => parameter.hash(hasher),
            dir::Type::FunctionSignature(signature) => {
                (ty.module_id, *signature).hash(hasher);
            }
            _ => {}
        }

        // hash the children in structural order
        let mut children = Vec::new();
        self.types(ty.module_id)?
            .for_each_child(&kind, |child| children.push(child));
        for child in children {
            self.hash_type_structure(child, hasher, visiting)?;
        }
        visiting.pop();

        Ok(())
    }

    /// Return whether one type denotes a lifetime.
    fn type_is_lifetime(&self, ty: dir::GlobalTypeId) -> CompilerResult<bool> {
        self.type_is_lifetime_guarded(ty, &mut Vec::new())
    }

    /// Judge one lifetime type with the visited unions tracked.
    fn type_is_lifetime_guarded(
        &self,
        ty: dir::GlobalTypeId,
        visiting: &mut Vec<dir::GlobalTypeId>,
    ) -> CompilerResult<bool> {
        Ok(match self.ty(ty)? {
            dir::Type::Memory(dir::MemoryLiteral::Lifetime(_)) => true,
            // written memory literals spell lifetimes as strings
            dir::Type::Literal(dir::Literal::String(name)) => {
                matches!(self.strings.get(name), "static" | "frame")
            }
            dir::Type::Parameter(parameter) => {
                let binding = self
                    .state(parameter.module_id)?
                    .generics
                    .get_parameter(parameter.local_id);

                binding.memory_parameter() == Some(dir::MemoryParameter::Lifetime)
            }
            dir::Type::Union(union) => {
                if visiting.contains(&ty) {
                    return Ok(false);
                }
                visiting.push(ty);
                let elements = self.types(ty.module_id)?.type_ids(union.elements).to_vec();
                for element in elements {
                    if !self.type_is_lifetime_guarded(element, visiting)? {
                        return Ok(false);
                    }
                }

                !union.elements.is_empty()
            }
            _ => false,
        })
    }

    /// Resolve one template type through its instance's materialized types.
    pub(in crate::lower) fn instance_type(
        &self,
        instance: Option<(ModuleId, dir::LocalInstanceId)>,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let Some((module, instance)) = instance else {
            return Ok(ty);
        };

        // overlay the moved types sparsely, keeping the written type on a miss
        match self.state(module)?.generics.instance_type(instance, ty) {
            Some(resolved) => Ok(resolved),
            None => Ok(ty),
        }
    }

    /// Index the drop hook member each loaded Drop-conforming nominal declares.
    ///
    /// Drop conformances bind in the nominal's own module: inline on the declaration,
    /// or on a same-module extension of it.
    fn index_drop_hooks(&mut self) -> CompilerResult<()> {
        let Some(interface) = self.language_symbols.get(&dir::LanguageItem::Drop).copied() else {
            return Ok(());
        };

        // collect the hook each loaded conformance selects, keyed by its nominal
        let mut hooks = FxIndexMap::default();
        for module in self.modules.keys().copied().collect::<Vec<_>>() {
            let state = self.state(module)?;
            for (symbol, definition) in state.definitions.iter_definitions() {
                let target = match definition {
                    dir::Definition::Extension(extension) => match extension.target {
                        dir::ExtensionTarget::Rooted {
                            root: dir::TypeRoot::Declaration(target),
                            ..
                        } if target.module_id == module => target,
                        _ => continue,
                    },
                    _ => symbol,
                };
                for conformance in definition.implementations() {
                    let conformance_symbol = self.ty(conformance.interface)?.symbol();
                    if conformance_symbol != Some(interface) {
                        continue;
                    }
                    let [member] = conformance.members.as_slice() else {
                        return Err(CompilerError::Internal {
                            message: "a Drop conformance did not select exactly one hook"
                                .to_string(),
                        });
                    };

                    hooks.insert(target, member.member);
                }
            }
        }
        self.drop_hooks = hooks;

        Ok(())
    }

    /// Return whether one nominal declares a Drop conformance.
    pub(in crate::lower) fn declares_drop(&self, symbol: dir::GlobalSymbolId) -> bool {
        self.drop_hooks.contains_key(&symbol)
    }

    /// Register each Drop-conforming nominal's authored hook beside its lowered storage.
    ///
    /// Sema closes hook instances beside their nominals, so the bodies arrive through
    /// the ordinary declaration pipeline; only imported hooks declare here.
    fn register_drop_hooks(
        &mut self,
        builder: &mut mir::ModuleBuilder,
        errors: &mut Vec<Box<dyn DiagnosticLike>>,
    ) -> CompilerResult<()> {
        if self.drop_hooks.is_empty() {
            return Ok(());
        }

        // pair each lowered nominal with the hook instance sharing its arguments
        let mut entries = Vec::new();
        for (key, state) in &self.nominal_states {
            let Some(member) = self.drop_hooks.get(&key.symbol).copied() else {
                continue;
            };
            let storage = match state {
                NominalState::Declared { storage, .. } => *storage,
                NominalState::Lowered(nominal) => nominal.storage,
            };
            // an unparameterized extension hook closes without the nominal's arguments
            let mut hook = GenericInstanceKey {
                symbol: member,
                arguments: key.arguments.clone(),
            };
            if !self.functions.contains_key(&hook) && !hook.arguments.is_empty() {
                hook = GenericInstanceKey {
                    symbol: member,
                    arguments: Vec::new(),
                };
            }
            entries.push((hook, storage));
        }

        for (key, storage_type) in entries {
            // declare an unreferenced imported hook
            if !self.functions.contains_key(&key) && key.arguments.is_empty() {
                match self.declare_imported_function(builder, key.symbol) {
                    Ok(()) => {}
                    Err(CompilerError::Diagnostic(diagnostic)) => {
                        errors.push(diagnostic);
                        continue;
                    }
                    Err(error) => return Err(error),
                }
            }

            let function = match self.functions.get(&key) {
                Some(FunctionDeclaration::Declared(function)) => *function,
                // declaration failures already reported their diagnostics
                Some(FunctionDeclaration::Failed) => continue,
                None => {
                    return Err(CompilerError::Internal {
                        message: format!("drop hook {:?} has no lowered instance", key.symbol),
                    });
                }
            };

            // register the hook for every storage its value can inhabit
            for storage in [
                mir::Storage::Frame,
                mir::Storage::Heap(mir::Space::Local),
                mir::Storage::Heap(mir::Space::Shared),
            ] {
                builder
                    .drops_mut()
                    .set_hook(storage_type, storage, function);
            }
        }

        Ok(())
    }

    /// Lower the module, returning the artifact and its diagnostics.
    pub(crate) fn lower(
        &mut self,
        target_layout: mir::TargetLayout,
    ) -> CompilerResult<(MirLowered, Vec<Box<dyn DiagnosticLike>>)> {
        let mut builder = mir::ModuleBuilder::new();
        builder.set_target_layout(target_layout);

        self.index_language_items()?;
        self.index_drop_hooks()?;

        // index the instances sema closed by their structural selection
        self.index_specializations()?;

        // declare identities: types, callable headers, globals, imports, instances
        let (bodies, mut errors) = self.declare_module(&mut builder)?;

        // lower every declared body, keeping failures isolated per function
        for body in bodies {
            match FunctionLowerer::lower(self, &mut builder, body) {
                Ok(()) => {}
                Err(CompilerError::Diagnostic(diagnostic)) => errors.push(diagnostic),
                Err(error) => return Err(error),
            }
        }

        // lower the synthesized default constructors to their initializer prologues
        let synthesized = std::mem::take(&mut self.synthesized_constructors);
        for (class, specialization, function) in synthesized {
            match FunctionLowerer::lower_default_constructor(
                self,
                &mut builder,
                class,
                specialization,
                function,
            ) {
                Ok(()) => {}
                Err(CompilerError::Diagnostic(diagnostic)) => errors.push(diagnostic),
                Err(error) => return Err(error),
            }
        }

        // register the authored drop hooks beside their lowered nominals
        match self.register_drop_hooks(&mut builder, &mut errors) {
            Ok(()) => {}
            Err(CompilerError::Diagnostic(diagnostic)) => errors.push(diagnostic),
            Err(error) => return Err(error),
        }

        // store the runtime bindings from the module initializer
        let mut initializer = None;
        match self.lower_module_initializer(&mut builder) {
            Ok(function) => initializer = function,
            Err(CompilerError::Diagnostic(diagnostic)) => errors.push(diagnostic),
            Err(error) => return Err(error),
        }

        // compute layouts for every represented type in the module
        let target = builder.target_layout();
        let (tree, layouts) = builder.tree_and_layouts_mut();
        let mut layouts = mir::LayoutBuilder::new(tree, layouts, target);
        layouts
            .layout_reachable_types()
            .map_err(|error| CompilerError::from((self.module, error)))?;

        // publish dynamic dispatch over the laid-out types
        self.build_dispatch_tables(&mut builder, &mut errors)?;

        // publish the lowered names into the shared pool
        let (tree, target, layouts, dispatch, drops, accesses, effects, profile, strings) =
            builder.finish();
        self.strings.ensure_all_from(&strings);

        // assemble the lowered module artifact
        let lowered = MirLowered {
            tree,
            target,
            layouts,
            language: std::mem::take(&mut self.language),
            dispatch,
            drops,
            accesses,
            effects,
            profile,
            initializer,
        };

        Ok((lowered, errors))
    }
}
