use std::collections::VecDeque;
use std::sync::Arc;

use destack_artifact::{DiagnosticLike, EnvironmentBound, MirDeclared, MirLowered};
use destack_core::{FxIndexMap, StringId, StringPool};
use destack_dir as dir;
use destack_mir as mir;
use destack_repository::{ArtifactReader, ProfileId, ProviderContext};
use destack_source::{ModuleId, TargetId};

use crate::lower::{
    DeclaredModule, DirModule, FunctionDeclaration, FunctionDefinition, FunctionLowerer,
    GenericInstanceKey, GenericScope, LowerPhase, NominalInstance, NominalState,
};
use crate::{Compiler, CompilerError, CompilerResult};

/// The function one witness selects and the arguments it applies, none at its open indices.
pub(in crate::lower) struct WitnessEntry {
    /// The closed instance, or the template when an argument stays open.
    pub(in crate::lower) function: mir::FunctionId,
    /// The template's arguments with one hole per open argument, empty for a closed instance.
    pub(in crate::lower) arguments: Vec<Option<mir::GenericArgument>>,
}

impl WitnessEntry {
    /// Fill each hole from the domain its template generic ranges over.
    pub(in crate::lower) fn fill(
        &self,
        tree: &mir::Tree,
        mut fill: impl FnMut(&mir::GenericParameterDomain) -> Option<mir::GenericArgument>,
    ) -> Option<Vec<mir::GenericArgument>> {
        let generics = &tree.get(self.function).generics;
        let mut arguments = Vec::with_capacity(self.arguments.len());
        for (index, argument) in self.arguments.iter().enumerate() {
            arguments.push(match argument {
                Some(argument) => argument.clone(),
                None => fill(&generics.get(index)?.domain)?,
            });
        }

        Some(arguments)
    }
}

/// One outcome the bodies read by key: the lowered value, or the first failure's diagnostic.
pub(in crate::lower) type Memo<K, T> = FxIndexMap<K, Result<T, Arc<dyn DiagnosticLike>>>;

/// One step the module initializer runs, in source order.
pub(crate) enum ModuleInitializer {
    /// Store a module binding's runtime value into its global.
    Binding {
        /// The global holding the binding.
        global: mir::LocalNodeId<mir::Global>,
        /// The binding's initializer expression.
        value: dir::LocalNodeId<dir::Expression>,
    },
    /// Run one module-level statement.
    Statement(dir::LocalNodeId<dir::Expression>),
}

/// The lowering state over one module and the modules it reads.
pub(crate) struct ModuleLowerer<'a> {
    // context
    /// The module being lowered.
    pub(in crate::lower) module: ModuleId,
    /// The artifact being built.
    pub(in crate::lower) phase: LowerPhase,
    /// The target this module lowers for.
    pub(in crate::lower) target: TargetId,
    /// The target ABI layout this module lowers against.
    pub(in crate::lower) target_layout: mir::TargetLayout,
    /// The pointer width of the target, in bytes.
    pub(in crate::lower) pointer_bytes: u8,
    /// The source string pool.
    pub(in crate::lower) strings: &'a StringPool,
    /// The compiler reading the modules.
    pub(in crate::lower) compiler: &'a Compiler,
    /// The provider context of the lowering attempt.
    pub(in crate::lower) context: &'a dyn ProviderContext,
    /// The artifact reader the module states load through.
    pub(in crate::lower) artifacts: ArtifactReader<'a>,
    /// The profile the modules are read under.
    pub(in crate::lower) profile: ProfileId,
    /// The bound environment naming the language items.
    pub(in crate::lower) environment: Arc<EnvironmentBound>,
    /// The state of each module read so far.
    pub(in crate::lower) modules: FxIndexMap<ModuleId, DirModule>,
    /// The declared MIR of every module read for its declarations.
    pub(in crate::lower) declared: FxIndexMap<ModuleId, DeclaredModule>,
    /// The witnesses this module records, keyed by the lowered, lifetime-erased type answering.
    pub(in crate::lower) lowered_witnesses:
        FxIndexMap<mir::TypeId, Vec<(dir::GlobalTypeId, dir::Witness)>>,

    // queues
    /// The bodies declared while lowering, awaiting their own lowering.
    pub(in crate::lower) pending: Vec<FunctionDefinition>,

    // memos
    /// The MIR representation behind each type the bodies read.
    pub(in crate::lower) representations: Memo<dir::GlobalTypeId, mir::TypeId>,
    /// The dispatch shape behind each constraint the bodies read.
    pub(in crate::lower) constraints: Memo<dir::GlobalTypeId, mir::TypeId>,
    /// The nominal instance behind each application type the bodies read.
    pub(in crate::lower) stored_nominals: Memo<dir::GlobalTypeId, NominalInstance>,
    /// The state of each nominal representation being lowered or already lowered.
    pub(in crate::lower) nominal_states: FxIndexMap<GenericInstanceKey, NominalState>,
    /// The global declared for each module constant.
    pub(in crate::lower) globals: Memo<dir::GlobalSymbolId, mir::LocalNodeId<mir::Global>>,
    /// The constant String object and value type per collected literal content.
    pub(in crate::lower) string_literals: Memo<StringId, (mir::GlobalId, mir::TypeId)>,
    /// The constant BigInt object and value type per collected literal value.
    pub(in crate::lower) bigint_literals: Memo<i64, (mir::GlobalId, mir::TypeId)>,

    // outputs
    /// The declaration outcome for each callable instance key.
    pub(in crate::lower) functions: FxIndexMap<GenericInstanceKey, FunctionDeclaration>,
    /// The allocating entry for each generated constructor declaration.
    pub(in crate::lower) constructors: FxIndexMap<mir::Symbol, mir::FunctionId>,
    /// The steps the module initializer runs, in source order.
    pub(in crate::lower) initializers: Vec<ModuleInitializer>,
    /// The dispatch shape registered for each lowered constraint.
    pub(in crate::lower) dynamic_shapes: FxIndexMap<mir::TypeId, mir::DynamicShape>,
}

impl<'a> ModuleLowerer<'a> {
    /// Create the lowering state over one materialized module.
    pub(crate) fn new(
        module: ModuleId,
        phase: LowerPhase,
        strings: &'a StringPool,
        target: TargetId,
        target_layout: mir::TargetLayout,
        compiler: &'a Compiler,
        context: &'a dyn ProviderContext,
        artifacts: ArtifactReader<'a>,
        profile: ProfileId,
        environment: Arc<EnvironmentBound>,
    ) -> Self {
        // intern the language item attribute name the trees key by text
        strings.intern("languageItem");

        Self {
            // context
            module,
            phase,
            target,
            target_layout,
            pointer_bytes: (target_layout.pointer_bits() / 8) as u8,
            strings,
            compiler,
            context,
            artifacts,
            profile,
            environment,
            modules: FxIndexMap::default(),
            declared: FxIndexMap::default(),
            lowered_witnesses: FxIndexMap::default(),

            // queues
            pending: Vec::new(),
            // memos
            representations: FxIndexMap::default(),
            constraints: FxIndexMap::default(),
            stored_nominals: FxIndexMap::default(),
            nominal_states: FxIndexMap::default(),
            globals: FxIndexMap::default(),
            string_literals: FxIndexMap::default(),
            bigint_literals: FxIndexMap::default(),
            // outputs
            functions: FxIndexMap::default(),
            constructors: FxIndexMap::default(),
            initializers: Vec::new(),
            dynamic_shapes: FxIndexMap::default(),
        }
    }

    /// Return the values one application binds its dependent parameters to, in template order.
    pub(in crate::lower) fn dependent_arguments(
        &mut self,
        ty: dir::GlobalTypeId,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<Vec<dir::GlobalTypeId>> {
        let dependents = self.declared_dependents(symbol)?;
        if dependents.is_empty() {
            return Ok(Vec::new());
        }

        // read the bound values, the template's own application closing at its dependents
        let state = self.state(ty.module_id)?;
        match state.generics.application_instance(ty) {
            Some(instance) => Ok(state.generics.get_instance(instance).key.dependents.clone()),
            None if self.is_identity_application(ty, symbol)? => Ok(dependents),
            None => Err(CompilerError::Internal {
                message: format!(
                    "application {:?} of '{}' has no instance",
                    self.ty(ty)?,
                    self.symbol_path(symbol)?
                ),
            }),
        }
    }

    /// Return the dependents one declaration's template declares.
    pub(in crate::lower) fn declared_dependents(
        &mut self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<Vec<dir::GlobalTypeId>> {
        let state = self.state(symbol.module_id)?;

        // read the dependents the declaration recorded
        match state.generics.symbol_dependents(symbol.local_id) {
            Some(dependents) => Ok(dependents.to_vec()),
            None => Err(CompilerError::Internal {
                message: format!(
                    "a declaration '{}' without its recorded dependents",
                    self.symbol_path(symbol)?
                ),
            }),
        }
    }

    /// Return whether one application applies a template at its own written parameters.
    fn is_identity_application(
        &mut self,
        ty: dir::GlobalTypeId,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<bool> {
        let dir::Type::Application(application) = self.ty(ty)? else {
            return Ok(false);
        };
        let Some(template) = self
            .definition(symbol)?
            .and_then(|definition| definition.template())
        else {
            return Ok(false);
        };
        let arguments = self
            .types(ty.module_id)?
            .type_ids(application.arguments)
            .to_vec();
        let parameters: Vec<_> = {
            let state = self.state(symbol.module_id)?;
            let template = state.generics.get_template(template);
            template
                .parameters
                .iter()
                .map(|parameter| (*parameter, state.generics.get_parameter(*parameter).origin))
                .collect()
        };

        // pair every written parameter with the argument naming it
        let mut written = arguments.iter();
        for (parameter, origin) in parameters {
            if origin == dir::GenericParameterOrigin::Receiver {
                continue;
            }
            let Some(argument) = written.next() else {
                return Ok(false);
            };
            let names_parameter = matches!(
                self.ty(*argument)?,
                dir::Type::Parameter(named) if named == parameter.into_global(symbol.module_id)
            );
            if !names_parameter {
                return Ok(false);
            }
        }

        Ok(written.next().is_none())
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
    pub(in crate::lower) fn type_is_lifetime(
        &mut self,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<bool> {
        self.type_is_lifetime_guarded(ty, &mut Vec::new())
    }

    /// Return whether one type denotes a lifetime, tracking the visited unions.
    fn type_is_lifetime_guarded(
        &mut self,
        ty: dir::GlobalTypeId,
        visiting: &mut Vec<dir::GlobalTypeId>,
    ) -> CompilerResult<bool> {
        Ok(match self.ty(ty)? {
            // region terms name lifetimes directly
            dir::Type::Region(_) => true,
            // memory literals name lifetimes as strings
            dir::Type::Literal(dir::Literal::String(name)) => {
                dir::Lifetime::parse(self.strings.get(name)).is_some()
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

    /// Return whether one nominal type's declaration derives Copy.
    pub(in crate::lower) fn nominal_copies(
        &mut self,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<bool> {
        let Some(symbol) = self.nominal_symbol(ty)? else {
            return Ok(false);
        };

        self.state(symbol.module_id)?
            .representations
            .derives_copy(symbol)
            .ok_or_else(|| CompilerError::Internal {
                message: format!("a nominal declaration {symbol:?} without a copy derivation"),
            })
    }

    /// Return the space one nominal declaration's instances live in.
    pub(in crate::lower) fn nominal_space(
        &mut self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<Option<dir::Space>> {
        Ok(self.state(symbol.module_id)?.representations.space(symbol))
    }

    /// Register the hook each Drop witness names beside the storage of the type it answers for.
    fn register_drop_hooks(
        &mut self,
        builder: &mut mir::ModuleBuilder,
        errors: &mut Vec<Box<dyn DiagnosticLike>>,
    ) -> CompilerResult<()> {
        let Some(drop) = self.environment.language.symbol(dir::LanguageItem::Drop) else {
            return Ok(());
        };

        // pair each Drop witness with the type it answers for
        let mut entries = Vec::new();
        let witnesses: Vec<_> = self
            .state(self.module)?
            .generics
            .iter_witnesses()
            .map(|(ty, interface, witness)| {
                (
                    ty,
                    interface,
                    witness
                        .functions
                        .first()
                        .map(|function| function.function.clone()),
                )
            })
            .collect();
        for (ty, interface, hook) in witnesses {
            if self.ty(interface)?.symbol() != Some(drop) {
                continue;
            }
            let Some(hook) = hook else {
                return Err(CompilerError::Internal {
                    message: "a Drop witness without its hook".to_string(),
                });
            };
            entries.push((ty, hook));
        }

        // declare each hook once and register it beside the storage of its type
        for (ty, hook) in entries {
            let nominal = self
                .type_lowerer(builder.tree_mut(), &GenericScope::default().erased())
                .lower_nominal(ty)?;
            let storage = nominal.storage;
            let entry = match self.witness_function(builder.tree_mut(), &hook) {
                Ok(entry) => entry,
                Err(CompilerError::Diagnostic(diagnostic)) => {
                    errors.push(diagnostic);
                    continue;
                }
                Err(error) => return Err(error),
            };
            let function = if entry.arguments.is_empty() {
                entry.function
            } else {
                // borrow the dropped object for as long as it lives
                let arguments = entry.fill(builder.tree(), |domain| match domain {
                    mir::GenericParameterDomain::Region { .. } => {
                        Some(mir::GenericArgument::Region(mir::Lifetime::managed()))
                    }
                    _ => None,
                });
                let Some(arguments) = arguments else {
                    return Err(CompilerError::Internal {
                        message: "a drop hook with unbound arguments its storage cannot fill"
                            .to_string(),
                    });
                };
                let key = GenericInstanceKey {
                    symbol: hook.symbol,
                    receiver: None,
                    arguments: arguments.clone(),
                };
                let chain = self.symbol_scope(hook.symbol)?;
                self.declare_specialization(
                    builder.tree_mut(),
                    &key,
                    hook.symbol,
                    entry.function,
                    arguments,
                    &chain,
                )?
            };
            builder.drops_mut().set_hook(storage, function);
        }

        Ok(())
    }

    /// Declare the function a witness selects, a template leaving its open arguments unbound.
    pub(in crate::lower) fn witness_function(
        &mut self,
        tree: &mut mir::Tree,
        key: &dir::InstanceKey,
    ) -> CompilerResult<WitnessEntry> {
        let bindings = self.instance_bindings(&key.arguments)?;
        let arguments: Vec<_> = bindings.iter().map(|binding| binding.argument).collect();
        let caller = GenericScope::default().erased();
        let instance_key = self.type_lowerer(tree, &caller).generic_instance_key(
            key.symbol,
            key.receiver,
            &arguments,
        )?;

        // declare a closed callable as the function itself
        let chain = self.symbol_scope(key.symbol)?;
        if chain.count() == 0 {
            let function = self
                .declare_instance(
                    tree,
                    &instance_key,
                    key.symbol,
                    key.receiver,
                    &bindings,
                    &key.dependents,
                    &caller,
                )?
                .function()?;

            return Ok(WitnessEntry {
                function,
                arguments: Vec::new(),
            });
        }

        // place the template arguments
        let template = self.template_function(tree, key.symbol, key.receiver)?;
        let placed = self.instance_arguments(
            tree,
            template,
            key.symbol,
            key.receiver,
            &bindings,
            &key.dependents,
            &chain,
        )?;

        // require each owner parameter but induced memory, leaving the implementer's places open
        for (parameter, index) in &chain.parameters {
            let is_induced = self
                .state(parameter.module_id)?
                .generics
                .get_parameter(parameter.local_id)
                .induced_memory_parameter()
                .is_some();
            if placed[*index as usize].is_none() && *index < chain.owner_count && !is_induced {
                return Err(self.unplaced_argument(key.symbol, &chain, *index)?);
            }
        }

        // lower the placed arguments, then keep the template while one stays open
        let mut lower = self.type_lowerer(tree, &caller);
        let mut lowered = Vec::with_capacity(placed.len());
        for argument in placed {
            lowered.push(match argument {
                Some(argument) => Some(lower.lower_generic_argument(argument)?),
                None => None,
            });
        }
        if lowered.iter().any(Option::is_none) {
            return Ok(WitnessEntry {
                function: template,
                arguments: lowered,
            });
        }
        let arguments = lowered.into_iter().flatten().collect();
        let function = self.declare_specialization(
            tree,
            &instance_key,
            key.symbol,
            template,
            arguments,
            &chain,
        )?;

        Ok(WitnessEntry {
            function,
            arguments: Vec::new(),
        })
    }

    /// Return the dispatch shape one constraint registered.
    pub(in crate::lower) fn dynamic_shape(
        &self,
        tree: &mir::Tree,
        constraint: mir::TypeId,
    ) -> Option<(&mir::DynamicShape, Vec<mir::GenericArgument>)> {
        let (base, arguments) = match tree.get(constraint) {
            mir::Type::Application {
                base, arguments, ..
            } if !arguments.is_empty() => (*base, arguments.clone()),
            _ => (constraint, Vec::new()),
        };

        self.dynamic_shapes
            .get(&base)
            .map(|shape| (shape, arguments))
    }

    /// Record the witnesses this module closes into the MIR witness table.
    fn lower_witness_table(&mut self, tree: &mut mir::Tree) -> CompilerResult<mir::WitnessTable> {
        let entries: Vec<_> = self
            .state(self.module)?
            .generics
            .iter_witnesses()
            .map(|(ty, interface, witness)| (ty, interface, witness.clone()))
            .collect();
        let caller = GenericScope::default().erased();
        let mut table = mir::WitnessTable::default();
        for (ty, interface, witness) in entries {
            // key the witness by the erased concrete type and the interface's constraint type
            if self.is_open_argument(ty)? {
                continue;
            }
            let concrete = self.type_lowerer(tree, &caller).lower(ty)?;
            let concrete = mir::erase_lifetimes(tree, concrete);
            self.lowered_witnesses
                .entry(concrete)
                .or_default()
                .push((interface, witness.clone()));
            let constraint = self
                .type_lowerer(tree, &caller)
                .lower_nominal(interface)?
                .storage;

            // declare the implementer behind each requirement
            let mut functions = Vec::with_capacity(witness.functions.len());
            for function in &witness.functions {
                let Some(member) = self.symbol_name(function.member)? else {
                    return Err(CompilerError::Internal {
                        message: "a witness member without a name".to_string(),
                    });
                };
                let requirement = self.template_function(tree, function.member, None)?;
                let entry = self.witness_function(tree, &function.function)?;
                functions.push(mir::WitnessFunction {
                    member,
                    requirement,
                    function: entry.function,
                    arguments: entry.arguments,
                });
            }

            // lower the type behind each associated type
            let mut types = Vec::with_capacity(witness.types.len());
            for witness_type in &witness.types {
                let dir::StaticKey::Name(member) = witness_type.member else {
                    return Err(CompilerError::Internal {
                        message: "an associated type under an indexed key".to_string(),
                    });
                };
                let lowered = self.type_lowerer(tree, &caller).lower(witness_type.ty)?;
                types.push(mir::WitnessType {
                    member,
                    ty: mir::erase_lifetimes(tree, lowered),
                });
            }

            // read the global behind each associated const
            let mut constants = Vec::with_capacity(witness.constants.len());
            for constant in &witness.constants {
                let dir::StaticKey::Name(member) = constant.member else {
                    return Err(CompilerError::Internal {
                        message: "an associated const under an indexed key".to_string(),
                    });
                };
                let Some(global) = self.constant_global(tree, constant.value)? else {
                    return Err(CompilerError::Internal {
                        message: format!(
                            "an associated const '{}' without its global",
                            self.symbol_path(constant.value)?
                        ),
                    });
                };
                constants.push(mir::WitnessConst { member, global });
            }

            table.insert(mir::Witness {
                concrete,
                constraint: mir::erase_lifetimes(tree, constraint),
                functions,
                types,
                constants,
            });
        }

        Ok(table)
    }

    /// Declare the module's types, callable headers, and globals, foreign ones reserved.
    pub(crate) fn declare(
        &mut self,
    ) -> CompilerResult<(MirDeclared, Vec<Box<dyn DiagnosticLike>>)> {
        let mut builder = mir::ModuleBuilder::new(self.module);
        builder.set_target_layout(self.target_layout);
        self.state(self.module)?;
        let (_, errors) = self.declare_module(builder.tree_mut())?;

        // publish the declared names into the shared pool
        let target = builder.target_layout();
        let (tree, strings) = builder.finish_tree();
        self.strings.ensure_all_from(&strings);
        let declared = MirDeclared {
            tree: Arc::new(tree),
            target,
        };

        Ok((declared, errors))
    }

    /// Lower the module, returning the artifact and its diagnostics.
    pub(crate) fn lower(&mut self) -> CompilerResult<(MirLowered, Vec<Box<dyn DiagnosticLike>>)> {
        // build the module against the target layout
        let mut builder = mir::ModuleBuilder::new(self.module);
        builder.set_target_layout(self.target_layout);

        // read the module being lowered and every module its rows mention
        self.state(self.module)?;

        // declare identities: types, callable headers, globals, imports, instances
        let (bodies, mut errors) = self.declare_module(builder.tree_mut())?;

        // record the witnesses this module closes, declaring the implementers they name
        let witnesses = self.lower_witness_table(builder.tree_mut())?;

        // lower every declared body
        let mut queue = VecDeque::from(bodies);
        loop {
            // drain the bodies declared while lowering
            queue.extend(std::mem::take(&mut self.pending));

            // lower the next queued body
            let Some(body) = queue.pop_front() else {
                break;
            };
            match FunctionLowerer::lower(self, &mut builder, body) {
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

        // lay out every represented type, reporting a layout diagnostic beside the lowering errors
        let target = builder.target_layout();
        let (tree, layouts) = builder.tree_and_layouts_mut();
        let mut layouts = mir::LayoutBuilder::new(tree, layouts, target).witnesses(&witnesses);
        match layouts
            .layout_reachable_types()
            .map_err(|error| CompilerError::from((self.module, error)))
        {
            Ok(()) => {}
            Err(CompilerError::Diagnostic(diagnostic)) => errors.push(diagnostic),
            Err(error) => return Err(error),
        }

        // store the witness table once the layouts read it
        *builder.witnesses_mut() = witnesses;

        // publish the dispatch shapes, their tables built over the instance layouts
        self.publish_dispatch_shapes(&mut builder);

        // publish the lowered names into the shared pool
        let (tree, target, layouts, dispatch, drops, witnesses, effects, profile, strings) =
            builder.finish();
        self.strings.ensure_all_from(&strings);

        // assemble the lowered module artifact
        let lowered = MirLowered {
            tree: Arc::new(tree),
            target,
            layouts,
            dispatch,
            drops,
            witnesses,
            effects,
            profile,
            initializer,
        };

        Ok((lowered, errors))
    }
}
