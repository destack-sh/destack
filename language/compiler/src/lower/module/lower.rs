use std::collections::VecDeque;
use std::sync::Arc;

use destack_artifact::{DiagnosticLike, MirLowered};
use destack_core::{FxIndexMap, StringId, StringPool};
use destack_dir as dir;
use destack_mir as mir;
use destack_source::ModuleId;

use crate::lower::{
    FunctionDeclaration, FunctionDefinition, FunctionLowerer, GenericInstanceKey, Implementer,
    LowerModuleState, NominalInstance, NominalState,
};
use crate::{CompilerError, CompilerResult};

/// One lowering outcome a body reads: the lowered value, or the first failure's diagnostic.
pub(in crate::lower) type Lowered<T> = Result<T, Arc<dyn DiagnosticLike>>;

/// The lowering state over one module and the modules it reads.
pub(crate) struct LowerState<'a> {
    // context
    /// The module being lowered.
    pub(in crate::lower) module: ModuleId,
    /// The target ABI layout this module lowers against.
    pub(in crate::lower) target_layout: mir::TargetLayout,
    /// The pointer width of the target, in bytes.
    pub(in crate::lower) pointer_bytes: u8,
    /// The source string pool.
    pub(in crate::lower) strings: &'a StringPool,
    /// The state of every reachable module.
    pub(in crate::lower) modules: FxIndexMap<ModuleId, LowerModuleState>,
    /// The recorded instance behind each closed application type.
    pub(in crate::lower) application_instances:
        FxIndexMap<dir::GlobalTypeId, (ModuleId, dir::LocalInstanceId)>,
    /// The materialized instance behind each closed selection, keyed by receiver and arguments.
    pub(in crate::lower) specializations: FxIndexMap<
        (
            dir::GlobalSymbolId,
            Option<dir::GlobalTypeId>,
            Vec<dir::GlobalTypeId>,
        ),
        (ModuleId, dir::LocalInstanceId),
    >,

    // queues
    /// The synthesized default constructors queued for body lowering.
    pub(in crate::lower) synthesized_constructors: Vec<(
        dir::GlobalSymbolId,
        Option<(ModuleId, dir::LocalInstanceId)>,
        mir::FunctionId,
    )>,
    /// The synthesized builtin clone bodies and their copied value types.
    pub(in crate::lower) synthesized_clones: Vec<(mir::FunctionId, mir::TypeId)>,
    /// The bodies declared while lowering, awaiting their own lowering.
    pub(in crate::lower) pending: Vec<FunctionDefinition>,

    // memos
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
    /// The loaded language item symbols, keyed by item.
    pub(in crate::lower) language_symbols: FxIndexMap<dir::LanguageItem, dir::GlobalSymbolId>,
    /// The loaded language items, keyed by symbol.
    pub(in crate::lower) language_items: FxIndexMap<dir::GlobalSymbolId, dir::LanguageItem>,
    /// The canonical items and members, keyed by lowered declaration.
    pub(in crate::lower) language: mir::LanguageTable,
    /// The authored drop hook member declared beside each Drop-conforming nominal.
    pub(in crate::lower) drop_hooks: FxIndexMap<dir::GlobalSymbolId, dir::GlobalSymbolId>,
    /// The constant String object and value type per collected literal content.
    pub(in crate::lower) string_literals:
        FxIndexMap<StringId, Lowered<(mir::GlobalId, mir::LocalNodeId<mir::Type>)>>,
    /// The constant BigInt object and value type per collected literal value.
    pub(in crate::lower) bigint_literals:
        FxIndexMap<i64, Lowered<(mir::GlobalId, mir::LocalNodeId<mir::Type>)>>,

    // outputs
    /// The declaration outcome for each callable instance key.
    pub(in crate::lower) functions: FxIndexMap<GenericInstanceKey, FunctionDeclaration>,
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

impl<'a> LowerState<'a> {
    /// Create the lowering state over one materialized module.
    pub(crate) fn new(
        module: ModuleId,
        strings: &'a StringPool,
        modules: FxIndexMap<ModuleId, LowerModuleState>,
        target_layout: mir::TargetLayout,
    ) -> Self {
        Self {
            // context
            module,
            target_layout,
            pointer_bytes: (target_layout.pointer_bits() / 8) as u8,
            strings,
            modules,
            application_instances: FxIndexMap::default(),
            specializations: FxIndexMap::default(),
            // queues
            synthesized_constructors: Vec::new(),
            synthesized_clones: Vec::new(),
            pending: Vec::new(),
            // memos
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
            // outputs
            functions: FxIndexMap::default(),
            initializers: Vec::new(),
            dynamic_shapes: FxIndexMap::default(),
            implementers: FxIndexMap::default(),
        }
    }

    /// Index the materialized instances by their recorded selection.
    fn index_specializations(&mut self) -> CompilerResult<()> {
        // collect the closed instances every loaded module contributes
        let mut specializations = FxIndexMap::default();
        let mut applications = FxIndexMap::default();
        for (module, state) in &self.modules {
            // key each instance by its symbol, receiver, and arguments
            for (instance, entry) in state.generics.iter_instances() {
                let arguments: Vec<_> = entry
                    .key
                    .arguments
                    .iter()
                    .map(|binding| binding.argument)
                    .collect();
                specializations
                    .entry((entry.key.symbol, entry.key.receiver, arguments))
                    .or_insert((*module, instance));
            }

            // key each application instance by its closed type
            for (ty, instance) in state.generics.iter_application_instances() {
                applications.entry(ty).or_insert((*module, instance));
            }
        }

        // publish both indexes over the loaded modules
        self.specializations = specializations;
        self.application_instances = applications;

        Ok(())
    }

    /// Return the recorded instance behind one closed application type.
    pub(in crate::lower) fn application_instance(
        &self,
        ty: dir::GlobalTypeId,
    ) -> Option<(ModuleId, dir::LocalInstanceId)> {
        self.application_instances.get(&ty).copied()
    }

    /// Return the materialized instance behind one closed selection.
    pub(in crate::lower) fn specialization_of(
        &self,
        symbol: dir::GlobalSymbolId,
        receiver: Option<dir::GlobalTypeId>,
        arguments: &[dir::GlobalTypeId],
    ) -> Option<(ModuleId, dir::LocalInstanceId)> {
        let key = (symbol, receiver, arguments.to_vec());

        self.specializations.get(&key).copied()
    }

    /// Return the recorded instance behind one applied type.
    pub(in crate::lower) fn application_specialization(
        &self,
        ty: dir::GlobalTypeId,
        application: &dir::GenericApplication,
    ) -> CompilerResult<Option<(ModuleId, dir::LocalInstanceId)>> {
        // read the direct binding when substitution moved the application whole
        if let Some(instance) = self.application_instance(ty) {
            return Ok(Some(instance));
        }

        // key a rebuilt application on its recorded symbol and arguments
        let arguments = self
            .types(ty.module_id)?
            .type_ids(application.arguments)
            .to_vec();

        Ok(self.specialization_of(application.symbol, None, &arguments))
    }

    /// Map one dir space to its mir space.
    pub(in crate::lower) fn mir_space(space: dir::Space) -> mir::Space {
        match space {
            dir::Space::Local => mir::Space::Local,
            dir::Space::Shared => mir::Space::Shared,
            dir::Space::Constant => mir::Space::Constant,
        }
    }

    /// Return whether one type denotes a lifetime.
    pub(in crate::lower) fn type_is_lifetime(&self, ty: dir::GlobalTypeId) -> CompilerResult<bool> {
        self.type_is_lifetime_guarded(ty, &mut Vec::new())
    }

    /// Return whether one type denotes a lifetime, tracking the visited unions.
    fn type_is_lifetime_guarded(
        &self,
        ty: dir::GlobalTypeId,
        visiting: &mut Vec<dir::GlobalTypeId>,
    ) -> CompilerResult<bool> {
        Ok(match self.ty(ty)? {
            // region terms name lifetimes directly
            dir::Type::Region(_) => true,
            // memory literals name lifetimes as strings
            dir::Type::Literal(dir::Literal::String(name)) => {
                matches!(self.strings.get(name), "static" | "frame")
            }
            // region parameters name lifetimes through their binding
            dir::Type::Parameter(parameter) => {
                let binding = self
                    .state(parameter.module_id)?
                    .generics
                    .get_parameter(parameter.local_id);

                binding.memory_parameter() == Some(dir::MemoryParameter::Region)
            }
            // a union names a lifetime once every element does
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
            // every other head denotes a value
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

        // resolve the ids substitution moved, leaving every other type as written
        match self.state(module)?.generics.instance_type(instance, ty) {
            Some(resolved) => Ok(resolved),
            None => Ok(ty),
        }
    }

    /// Resolve one symbol's materialized type under one instance.
    pub(in crate::lower) fn instance_symbol_type(
        &self,
        instance: Option<(ModuleId, dir::LocalInstanceId)>,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let Some((module, instance)) = instance else {
            return self.symbol_type(symbol);
        };

        // resolve the symbol through the materialized instance
        match self
            .state(module)?
            .generics
            .instance_symbol(instance, symbol)
        {
            Some(resolved) => Ok(resolved),
            None => Err(CompilerError::Internal {
                message: format!("a missing materialized type for the symbol {symbol:?}"),
            }),
        }
    }

    /// Return whether one closed nominal type committed an auto conformance.
    pub(in crate::lower) fn nominal_conformance(
        &self,
        ty: dir::GlobalTypeId,
        interface: dir::AutoInterface,
    ) -> CompilerResult<bool> {
        match self.ty(ty)? {
            // instantiation-invariant declarations read the committed definition conformances
            dir::Type::Application(application)
                if application.arguments.is_empty()
                    || self.template_is_memory_only(application.symbol)? =>
            {
                self.definition_conformance(application.symbol, interface)
            }
            // references read the conformances of their referent declaration
            dir::Type::Reference(reference) => {
                self.definition_conformance(reference.symbol, interface)
            }
            // generic applications read the committed instance conformances
            dir::Type::Application(application) => {
                let Some((module, instance)) = self.application_specialization(ty, &application)?
                else {
                    return Ok(false);
                };
                let conformances = &self
                    .state(module)?
                    .generics
                    .get_instance(instance)
                    .conformances;

                Ok(conformances.contains(interface))
            }

            _ => Ok(false),
        }
    }

    /// Return whether one declaration's template holds only memory parameters.
    fn template_is_memory_only(&self, symbol: dir::GlobalSymbolId) -> CompilerResult<bool> {
        let Some(template) = self
            .definition(symbol)?
            .and_then(|definition| definition.template())
        else {
            return Ok(true);
        };

        // reject the first parameter naming something beyond memory
        let state = self.state(symbol.module_id)?;
        let parameters = state.generics.get_template(template).parameters.clone();
        for parameter in parameters {
            let binding = state.generics.get_parameter(parameter);
            if binding.memory_parameter().is_none() {
                return Ok(false);
            }
        }

        Ok(true)
    }

    /// Return whether one declaration's committed conformances include an interface.
    fn definition_conformance(
        &self,
        symbol: dir::GlobalSymbolId,
        interface: dir::AutoInterface,
    ) -> CompilerResult<bool> {
        let conformances = self
            .definition(symbol)?
            .and_then(|definition| definition.conformances());

        Ok(conformances.is_some_and(|conformances| conformances.contains(interface)))
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
                // take the extension target, or the declaration itself
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

                // record the hook each Drop conformance selects
                for conformance in definition.implementations() {
                    let conformance_symbol = self.ty(conformance.interface)?.symbol();
                    if conformance_symbol != Some(interface) {
                        continue;
                    }
                    let [member] = conformance.members.as_slice() else {
                        return Err(CompilerError::Internal {
                            message: "a Drop conformance without exactly one hook".to_string(),
                        });
                    };

                    hooks.insert(target, member.member);
                }
            }
        }

        // publish the index over the loaded modules
        self.drop_hooks = hooks;

        Ok(())
    }

    /// Register each Drop-conforming nominal's authored hook beside its lowered storage.
    ///
    /// Local hook bodies arrive through the ordinary declarations, so this pass
    /// declares only the imported hooks.
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
            let (storage, value) = match state {
                NominalState::Declared { storage, value } => (*storage, *value),
                NominalState::Lowered(nominal) => (nominal.storage, nominal.value),
            };

            let hook = GenericInstanceKey {
                symbol: member,
                receiver: None,
                arguments: key.arguments.clone(),
            };
            entries.push((hook, storage, value));
        }

        // register each paired hook against its lowered storage
        for (mut key, storage_type, value_type) in entries {
            // fall back to the unparameterized key of an extension hook
            if !self.functions.contains_key(&key) && !key.arguments.is_empty() {
                let unparameterized = GenericInstanceKey {
                    symbol: key.symbol,
                    receiver: None,
                    arguments: Vec::new(),
                };
                if self.functions.contains_key(&unparameterized) {
                    key = unparameterized;
                }
            }

            // import the foreign hook instance missing from the local declarations
            if !self.functions.contains_key(&key) {
                match self.import_drop_hook(builder.tree_mut(), &key, storage_type, value_type) {
                    Ok(()) => {}
                    Err(CompilerError::Diagnostic(diagnostic)) => {
                        errors.push(diagnostic);
                        continue;
                    }
                    Err(error) => return Err(error),
                }
            }

            // read the lowered instance behind the hook
            let function = match self.functions.get(&key) {
                Some(FunctionDeclaration::Declared(function)) => *function,
                // cascade from declarations that already reported their diagnostics
                Some(FunctionDeclaration::Failed) => continue,
                None => {
                    return Err(CompilerError::Internal {
                        message: format!(
                            "a missing lowered instance for the drop hook {:?}",
                            key.symbol
                        ),
                    });
                }
            };

            // register the hook for every storage its value can inhabit
            for storage in [
                mir::Storage::Frame,
                mir::Storage::LocalHeap,
                mir::Storage::SharedHeap,
            ] {
                builder
                    .drops_mut()
                    .set_hook(storage_type, storage, function);
            }
        }

        Ok(())
    }

    /// Lower the module, returning the artifact and its diagnostics.
    pub(crate) fn lower(&mut self) -> CompilerResult<(MirLowered, Vec<Box<dyn DiagnosticLike>>)> {
        // build the module against the target layout
        let mut builder = mir::ModuleBuilder::new();
        builder.set_target_layout(self.target_layout);

        // index the language items and the drop hooks the loaded modules declare
        self.index_language_items()?;
        self.index_drop_hooks()?;

        // index the materialized instances by their recorded selection
        self.index_specializations()?;

        // declare identities: types, callable headers, globals, imports, instances
        let (bodies, mut errors) = self.declare_module(builder.tree_mut())?;

        // lower every declared body
        let mut queue = VecDeque::from(bodies);
        loop {
            // drain the bodies declared while lowering
            queue.extend(std::mem::take(&mut self.pending));

            // lower the next queued body
            if let Some(body) = queue.pop_front() {
                match FunctionLowerer::lower(self, &mut builder, body) {
                    Ok(()) => {}
                    Err(CompilerError::Diagnostic(diagnostic)) => errors.push(diagnostic),
                    Err(error) => return Err(error),
                }

                continue;
            }

            // lower the synthesized default constructors to their initializer prologues
            if !self.synthesized_constructors.is_empty() {
                let (class, specialization, function) = self.synthesized_constructors.remove(0);
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

                continue;
            }

            // lower the synthesized builtin clones to receiver copies
            if !self.synthesized_clones.is_empty() {
                let (function, result) = self.synthesized_clones.remove(0);
                match FunctionLowerer::lower_builtin_clone(self, &mut builder, function, result) {
                    Ok(()) => {}
                    Err(CompilerError::Diagnostic(diagnostic)) => errors.push(diagnostic),
                    Err(error) => return Err(error),
                }

                continue;
            }

            break;
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
