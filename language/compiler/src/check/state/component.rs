use std::sync::Arc;

use destack_artifact::GlobalEnvironment;
use destack_dir as dir;
use destack_repository::{ArtifactReader, ProviderContext};
use destack_source::{ComponentId, ModuleId, ProfileId};
use indexmap::IndexMap;
use smallvec::SmallVec;

use crate::check::{
    CheckEvent, CheckExternalModuleState, CheckModuleState, DecisionTable, GenericIndex,
    GenericScope, GenericTemplateId, Origin, Solver, VarianceEntry,
};
use crate::{Compiler, CompilerError, CompilerResult};

/// Artifact coordinates for one checked component.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(in crate::check) struct CheckComponentKey {
    /// The checked component entry module.
    pub entry: ModuleId,
    /// The checked component id.
    pub component: ComponentId,
}

/// State for checking one resolved component.
pub(in crate::check) struct CheckState<'a> {
    // the provider attempt running this check
    /// The compiler running this check attempt.
    pub(in crate::check) compiler: &'a Compiler,
    /// The provider context that owns artifact reads and diagnostics.
    pub(in crate::check) context: &'a dyn ProviderContext,
    /// The provider-scoped artifact reader.
    pub(in crate::check) artifacts: &'a ArtifactReader<'a>,
    /// The active profile.
    pub(in crate::check) profile: ProfileId,
    /// The active global environment.
    pub(in crate::check) environment: Arc<GlobalEnvironment>,

    // loaded modules
    /// Loaded component modules keyed by module id.
    pub(in crate::check) modules: IndexMap<ModuleId, CheckModuleState>,
    /// Loaded out-of-component modules keyed by module id.
    pub(in crate::check) external_modules: IndexMap<ModuleId, CheckExternalModuleState>,
    /// Checked component artifact containing each external module.
    pub(in crate::check) external_components: IndexMap<ModuleId, CheckComponentKey>,

    // checked state
    /// Stable declaration symbol types.
    pub(in crate::check) declaration_types: IndexMap<dir::GlobalSymbolId, dir::GlobalTypeId>,
    /// Body-owned binding symbol types.
    pub(in crate::check) binding_types: IndexMap<dir::GlobalSymbolId, dir::GlobalTypeId>,
    /// Checked source node occurrence types.
    pub(in crate::check) node_types: IndexMap<dir::GlobalNodeIdAny, dir::GlobalTypeId>,
    /// Stable source node decisions.
    pub(in crate::check) decisions: DecisionTable,

    // generic state
    /// Generic instances, argument variables, and induction bookkeeping.
    pub(in crate::check) generics: GenericIndex,
    /// Flattened generic scopes computed once per template.
    pub(in crate::check) scopes: IndexMap<GenericTemplateId, Arc<GenericScope>>,
    /// Generic parameter variance derivations.
    pub(in crate::check) variances: IndexMap<dir::GlobalGenericParameterId, VarianceEntry>,

    // reduction state
    /// Memoized closed reduced types keyed by source type.
    /// Entries are recorded outside probes only, and parameter reductions key by their assuming scope.
    pub(in crate::check) reduced_types:
        IndexMap<(dir::GlobalTypeId, Option<dir::GlobalGenericTemplateId>), dir::GlobalTypeId>,

    // solver state
    /// Active component solver state.
    pub(in crate::check) solver: Solver,

    // tracing
    /// Trace events recorded while checking.
    pub(in crate::check) events: Vec<CheckEvent>,
    /// Whether check events should print as they are recorded in debug builds.
    pub(in crate::check) emit_events: bool,
}

impl<'a> CheckState<'a> {
    /// Create a component check state.
    pub(in crate::check) fn new(
        compiler: &'a Compiler,
        context: &'a dyn ProviderContext,
        artifacts: &'a ArtifactReader<'a>,
        profile: ProfileId,
        environment: Arc<GlobalEnvironment>,
        external_components: IndexMap<ModuleId, CheckComponentKey>,
        emit_events: bool,
    ) -> Self {
        Self {
            compiler,
            context,
            artifacts,
            profile,
            environment,
            modules: IndexMap::new(),
            external_modules: IndexMap::new(),
            external_components,
            declaration_types: IndexMap::new(),
            binding_types: IndexMap::new(),
            node_types: IndexMap::new(),
            decisions: DecisionTable::new(),
            generics: GenericIndex::new(),
            scopes: IndexMap::new(),
            variances: IndexMap::new(),
            reduced_types: IndexMap::new(),
            solver: Solver::new(),
            events: Vec::new(),
            emit_events,
        }
    }

    /// Load all modules in one check component.
    pub(in crate::check) fn load(&mut self, modules: &[ModuleId]) -> CompilerResult<()> {
        // load modules in stable component order
        for module in modules {
            self.load_module(*module)?;
        }

        Ok(())
    }

    /// Walk every loaded module.
    pub(in crate::check) fn walk(&mut self) -> CompilerResult<()> {
        let modules = self.modules.keys().copied().collect::<Vec<_>>();

        // import external checked artifacts
        self.import_component_external_modules()?;

        // declare template identities, then walk their bounds, so
        // declarations resolve in any order across module cycles
        for module in modules.iter().copied() {
            self.declare_module_templates(module)?;
        }
        for module in modules.iter().copied() {
            self.walk_module_templates(module)?;
        }

        // walk modules in stable component order
        for module in modules.iter().copied() {
            self.walk_module(module)?;
        }

        Ok(())
    }

    /// Complete walk-time state before solving.
    pub(in crate::check) fn propagate(&mut self) -> CompilerResult<()> {
        self.propagate_induced_lifetimes()
    }

    /// Load one module into component state.
    fn load_module(&mut self, module_id: ModuleId) -> CompilerResult<()> {
        if self.is_component_module(module_id) {
            return Ok(());
        }

        let profile = self
            .compiler
            .profile(self.context.revision(), self.profile)?
            .key;
        let module = self.compiler.module(self.context.revision(), module_id)?;
        let parsed = self
            .artifacts
            .dir_parsed(module_id)
            .map_err(CompilerError::from)?;
        let bound = self
            .artifacts
            .dir_bound(module_id, self.profile)
            .map_err(CompilerError::from)?;
        let resolved = self
            .artifacts
            .dir_resolved(module_id, self.profile)
            .map_err(CompilerError::from)?;
        let expanded = self
            .artifacts
            .dir_expanded(module_id, self.profile)
            .map_err(CompilerError::from)?;
        let strings = Arc::clone(self.compiler.repository.string_pool());

        let module = CheckModuleState::new(
            module,
            profile,
            strings,
            parsed,
            bound,
            resolved,
            Arc::clone(&expanded),
        );

        self.modules.insert(module_id, module);

        Ok(())
    }

    /// Return one language symbol resolved for one module.
    pub(in crate::check) fn language_symbol(&self, item: dir::LanguageItem) -> dir::GlobalSymbolId {
        self.environment.language.symbol(item).unwrap_or_else(|| {
            unreachable!("language item {item} is missing from the global environment")
        })
    }

    /// Return the language item named by one resolved symbol.
    pub(in crate::check) fn language_item(
        &mut self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<Option<dir::LanguageItem>> {
        let symbol = self.resolve_symbol_alias(symbol)?;

        Ok(self.environment.language.item(symbol))
    }

    /// Return the nominal symbol named by one type head.
    pub(in crate::check) fn type_symbol(
        &self,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<Option<dir::GlobalSymbolId>> {
        let symbol = match self.ty(ty)? {
            dir::Type::Reference(reference) => Some(reference.symbol),
            dir::Type::Instance(instance) => Some(instance.symbol),
            _ => None,
        };

        Ok(symbol)
    }
}

impl CheckState<'_> {
    /// Return one type from this component's open overlay or external tables.
    pub(in crate::check) fn ty(&self, id: dir::GlobalTypeId) -> CompilerResult<dir::Type> {
        // read this component's open working types
        if let Some(module) = self.modules.get(&id.module_id) {
            module
                .type_maybe(id.local_id)
                .ok_or_else(|| CompilerError::Internal {
                    message: format!("check type {id:?} is not allocated"),
                })
        }
        // read external committed tables
        else if let Some(external) = self.external_modules.get(&id.module_id) {
            Ok(external.types.get_type(id.local_id))
        }
        // should never happen
        else {
            Err(CompilerError::Internal {
                message: format!("check type {id:?} belongs to an unloaded module"),
            })
        }
    }

    /// Return one type's structural flags.
    pub(in crate::check) fn type_flags(
        &self,
        id: dir::GlobalTypeId,
    ) -> CompilerResult<dir::TypeFlags> {
        // read this component's open working types
        if let Some(module) = self.modules.get(&id.module_id) {
            module
                .type_flags_maybe(id.local_id)
                .ok_or_else(|| CompilerError::Internal {
                    message: format!("check type {id:?} is not allocated"),
                })
        }
        // read external committed tables
        else if let Some(external) = self.external_modules.get(&id.module_id) {
            Ok(external.types.get_type_flags(id.local_id))
        }
        // should never happen
        else {
            Err(CompilerError::Internal {
                message: format!("check type {id:?} belongs to an unloaded module"),
            })
        }
    }

    /// Visit each direct child type id of one type owned by a module.
    pub(in crate::check) fn for_each_type_child(
        &self,
        module: ModuleId,
        ty: &dir::Type,
        visit: impl FnMut(dir::GlobalTypeId),
    ) -> CompilerResult<()> {
        // resolve payload lists through the owning module's tables
        if let Some(working) = self.modules.get(&module) {
            working.type_table().for_each_child(ty, visit);
        } else if let Some(external) = self.external_modules.get(&module) {
            external.types.for_each_child(ty, visit);
        } else {
            return Err(CompilerError::Internal {
                message: format!("check type children belong to an unloaded module {module:?}"),
            });
        }

        Ok(())
    }

    /// Intern one type into a module's working segment.
    /// Payload lists must already be interned into the same module.
    pub(in crate::check) fn intern_type(
        &mut self,
        module: ModuleId,
        ty: dir::Type,
    ) -> CompilerResult<dir::GlobalTypeId> {
        // join the structural flags of every child type
        let mut children = SmallVec::<[dir::GlobalTypeId; 8]>::new();
        self.for_each_type_child(module, &ty, |child| children.push(child))?;
        let mut child_flags = dir::TypeFlags::EMPTY;
        for child in children {
            child_flags |= self.type_flags(child)?;
        }

        let local = self
            .working_module_mut(module)?
            .types_tail
            .intern_type(ty, child_flags);

        Ok(local.into_global(module))
    }

    /// Intern one type id list into a module's working segment.
    pub(in crate::check) fn intern_type_ids(
        &mut self,
        module: ModuleId,
        values: &[dir::GlobalTypeId],
    ) -> CompilerResult<dir::TypeListId> {
        Ok(self
            .working_module_mut(module)?
            .types_tail
            .intern_type_ids(values))
    }

    /// Intern one tuple element list into a module's working segment.
    pub(in crate::check) fn intern_elements(
        &mut self,
        module: ModuleId,
        values: &[dir::TypeElement],
    ) -> CompilerResult<dir::TypeListId> {
        Ok(self
            .working_module_mut(module)?
            .types_tail
            .intern_elements(values))
    }

    /// Intern one shape field list into a module's working segment.
    pub(in crate::check) fn intern_fields(
        &mut self,
        module: ModuleId,
        values: &[dir::TypeField],
    ) -> CompilerResult<dir::TypeListId> {
        Ok(self
            .working_module_mut(module)?
            .types_tail
            .intern_fields(values))
    }

    /// Intern one function parameter list into a module's working segment.
    pub(in crate::check) fn intern_parameters(
        &mut self,
        module: ModuleId,
        values: &[dir::FunctionParameterType],
    ) -> CompilerResult<dir::TypeListId> {
        Ok(self
            .working_module_mut(module)?
            .types_tail
            .intern_parameters(values))
    }

    /// Intern one index signature list into a module's working segment.
    pub(in crate::check) fn intern_index_signatures(
        &mut self,
        module: ModuleId,
        values: &[dir::TypeIndexSignature],
    ) -> CompilerResult<dir::TypeListId> {
        Ok(self
            .working_module_mut(module)?
            .types_tail
            .intern_index_signatures(values))
    }

    /// Intern one string list into a module's working segment.
    pub(in crate::check) fn intern_strings(
        &mut self,
        module: ModuleId,
        values: &[destack_source::StringId],
    ) -> CompilerResult<dir::TypeListId> {
        Ok(self
            .working_module_mut(module)?
            .types_tail
            .intern_strings(values))
    }

    /// Return one type id list owned by a module.
    pub(in crate::check) fn type_ids(
        &self,
        module: ModuleId,
        list: dir::TypeListId,
    ) -> CompilerResult<&[dir::GlobalTypeId]> {
        self.type_rows(
            module,
            list,
            |table| table.type_ids(list),
            |segment| segment.type_ids_maybe(list),
        )
    }

    /// Return one positional type id, or none past the list end.
    pub(in crate::check) fn type_id_at(
        &self,
        module: ModuleId,
        list: dir::TypeListId,
        index: u32,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        Ok(self.type_ids(module, list)?.get(index as usize).copied())
    }

    /// Return one tuple element list owned by a module.
    pub(in crate::check) fn tuple_elements(
        &self,
        module: ModuleId,
        list: dir::TypeListId,
    ) -> CompilerResult<&[dir::TypeElement]> {
        self.type_rows(
            module,
            list,
            |table| table.elements(list),
            |segment| segment.elements_maybe(list),
        )
    }

    /// Return one shape field list owned by a module.
    pub(in crate::check) fn shape_fields(
        &self,
        module: ModuleId,
        list: dir::TypeListId,
    ) -> CompilerResult<&[dir::TypeField]> {
        self.type_rows(
            module,
            list,
            |table| table.fields(list),
            |segment| segment.fields_maybe(list),
        )
    }

    /// Return one function parameter list owned by a module.
    pub(in crate::check) fn signature_parameters(
        &self,
        module: ModuleId,
        list: dir::TypeListId,
    ) -> CompilerResult<&[dir::FunctionParameterType]> {
        self.type_rows(
            module,
            list,
            |table| table.parameters(list),
            |segment| segment.parameters_maybe(list),
        )
    }

    /// Return one index signature list owned by a module.
    pub(in crate::check) fn shape_index_signatures(
        &self,
        module: ModuleId,
        list: dir::TypeListId,
    ) -> CompilerResult<&[dir::TypeIndexSignature]> {
        self.type_rows(
            module,
            list,
            |table| table.index_signatures(list),
            |segment| segment.index_signatures_maybe(list),
        )
    }

    /// Return one string list owned by a module.
    pub(in crate::check) fn template_strings(
        &self,
        module: ModuleId,
        list: dir::TypeListId,
    ) -> CompilerResult<&[destack_source::StringId]> {
        self.type_rows(
            module,
            list,
            |table| table.strings(list),
            |segment| segment.strings_maybe(list),
        )
    }

    /// Resolve one interned list through the owning module's tables.
    fn type_rows<'s, T>(
        &'s self,
        module: ModuleId,
        list: dir::TypeListId,
        read_table: impl FnOnce(&'s dir::TypeTable<'static>) -> &'s [T],
        read_segment: impl FnOnce(&'s dir::TypeSegment) -> Option<&'s [T]>,
    ) -> CompilerResult<&'s [T]> {
        // resolve overlay lists over the committed base table
        if let Some(working) = self.modules.get(&module) {
            if list.is_empty() {
                return Ok(&[]);
            }
            if let Some(elements) = read_segment(&working.types_tail) {
                return Ok(elements);
            }

            return Ok(read_table(&working.types));
        }

        // read external committed tables
        if let Some(external) = self.external_modules.get(&module) {
            return Ok(read_table(&external.types));
        }

        Err(CompilerError::Internal {
            message: format!("check type list {list:?} belongs to an unloaded module {module:?}"),
        })
    }

    /// Return one loaded working module mutably.
    fn working_module_mut(&mut self, module: ModuleId) -> CompilerResult<&mut CheckModuleState> {
        self.modules
            .get_mut(&module)
            .ok_or_else(|| CompilerError::Internal {
                message: format!("check module {module:?} has no working types"),
            })
    }

    /// Intern one applied reference type for a language item.
    pub(in crate::check) fn language_type(
        &mut self,
        module: ModuleId,
        item: dir::LanguageItem,
        arguments: &[dir::GlobalTypeId],
    ) -> CompilerResult<dir::GlobalTypeId> {
        let symbol = self.language_symbol(item);
        let arguments = self.intern_type_ids(module, arguments)?;
        let ty = dir::Type::Instance(dir::GenericInstance { symbol, arguments });

        self.intern_type(module, ty)
    }

    /// Intern one singleton type for an exact property key.
    pub(in crate::check) fn static_key_type(
        &mut self,
        module: ModuleId,
        key: dir::StaticKey,
    ) -> CompilerResult<dir::GlobalTypeId> {
        self.intern_type(module, dir::Type::Key(key))
    }

    /// Intern the type read from an optional index signature.
    pub(in crate::check) fn index_signature_read_type(
        &mut self,
        origin: Origin,
        value: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let module = origin.module();
        let undefined = self.intern_type(module, dir::Type::Undefined)?;

        self.normalized_union_type(module, [value, undefined])
    }

    /// Intern one open variable reference type in its origin module's working segment.
    pub(in crate::check) fn variable_type(
        &mut self,
        variable: dir::TypeVariableId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let module = self.solver.variable(variable)?.origin.module();

        self.intern_type(module, dir::Type::Variable(variable))
    }

    /// Return one definition, reading working segments over external tables.
    pub(in crate::check) fn definition(
        &self,
        symbol: dir::GlobalSymbolId,
    ) -> Option<&dir::Definition> {
        // read working component definitions first
        if let Some(module) = self.modules.get(&symbol.module_id)
            && let Some(definition) = module.definitions.definition(symbol)
        {
            return Some(definition);
        }

        // read external committed definitions
        if let Some(external) = self.external_modules.get(&symbol.module_id) {
            return external.definitions.definition(symbol);
        }

        None
    }

    /// Insert one checked definition into its module's working segment.
    pub(in crate::check) fn insert_definition(
        &mut self,
        symbol: dir::GlobalSymbolId,
        source: dir::GlobalNodeIdAny,
        definition: dir::Definition,
    ) -> CompilerResult<()> {
        self.report_duplicate_definition_members(&definition);

        let working =
            self.modules
                .get_mut(&symbol.module_id)
                .ok_or_else(|| CompilerError::Internal {
                    message: format!(
                        "check module {:?} has no working definitions",
                        symbol.module_id
                    ),
                })?;

        working
            .definitions
            .insert_definition(symbol, source, definition);

        Ok(())
    }

    /// Report duplicate non-overload member keys in one definition.
    fn report_duplicate_definition_members(&mut self, definition: &dir::Definition) {
        let mut seen = IndexMap::<(dir::MemberSpace, dir::StaticKey), bool>::new();

        for member in definition.members() {
            let Some(key) = member.key() else {
                continue;
            };

            let entry = (member.space(), key);
            let is_overloadable = member.is_overloadable();
            if let Some(previous_is_overloadable) = seen.get(&entry) {
                if !*previous_is_overloadable || !is_overloadable {
                    self.report_duplicate_definition_member(member.source(), &key);
                }
            } else {
                seen.insert(entry, is_overloadable);
            }
        }
    }

    /// Rebuild one type value by mapping every direct child type id.
    ///
    /// Payload lists read from the source module that owns the value and
    /// rebuilt lists intern into the target module that owns the rebuild.
    pub(in crate::check) fn map_type_children(
        &mut self,
        source: ModuleId,
        target: ModuleId,
        ty: dir::Type,
        map: &mut impl FnMut(&mut Self, dir::GlobalTypeId) -> CompilerResult<dir::GlobalTypeId>,
    ) -> CompilerResult<dir::Type> {
        let ty = match ty {
            // leaves without child types
            dir::Type::Variable(_)
            | dir::Type::Error
            | dir::Type::Never
            | dir::Type::Any
            | dir::Type::Unknown
            | dir::Type::Void
            | dir::Type::Null
            | dir::Type::Undefined
            | dir::Type::Object
            | dir::Type::Primitive(_)
            | dir::Type::Literal(_)
            | dir::Type::Key(_)
            | dir::Type::Memory(_)
            | dir::Type::Static(_)
            | dir::Type::Intrinsic
            | dir::Type::Parameter(_)
            | dir::Type::Erased(_)
            | dir::Type::This
            | dir::Type::Range(_)
            | dir::Type::Reference(_) => ty,

            // declaration applications
            dir::Type::Instance(mut instance) => {
                instance.arguments =
                    self.map_type_id_list(source, target, instance.arguments, map)?;

                dir::Type::Instance(instance)
            }
            dir::Type::Member(mut member) => {
                member.owner = map(self, member.owner)?;
                member.arguments = self.map_type_id_list(source, target, member.arguments, map)?;
                member.qualifier = member
                    .qualifier
                    .map(|qualifier| map(self, qualifier))
                    .transpose()?;

                dir::Type::Member(member)
            }
            dir::Type::EnumMember(mut member) => {
                member.owner = map(self, member.owner)?;

                dir::Type::EnumMember(member)
            }

            // memory forms
            dir::Type::Form(mut form) => {
                form.value = map(self, form.value)?;
                match &mut form.form {
                    dir::Form::Borrowed { lifetime, access } => {
                        *lifetime = map(self, *lifetime)?;
                        *access = map(self, *access)?;
                    }
                    dir::Form::Placed { place } => *place = map(self, *place)?,
                    dir::Form::Managed
                    | dir::Form::Owned
                    | dir::Form::Raw
                    | dir::Form::Readonly => {}
                }

                dir::Type::Form(form)
            }
            dir::Type::Dynamic(mut dynamic) => {
                dynamic.constraint = map(self, dynamic.constraint)?;

                dir::Type::Dynamic(dynamic)
            }

            // type operations
            dir::Type::Operation(operation) => {
                let operation = match operation {
                    dir::TypeOperation::StringMapping { mapping, target } => {
                        dir::TypeOperation::StringMapping {
                            mapping,
                            target: map(self, target)?,
                        }
                    }
                    dir::TypeOperation::Conditional(mut conditional) => {
                        conditional.left = map(self, conditional.left)?;
                        conditional.right = map(self, conditional.right)?;
                        conditional.then_type = map(self, conditional.then_type)?;
                        conditional.else_type = map(self, conditional.else_type)?;

                        dir::TypeOperation::Conditional(conditional)
                    }
                    dir::TypeOperation::Narrow(mut narrow) => {
                        narrow.source = map(self, narrow.source)?;
                        narrow.target = map(self, narrow.target)?;

                        dir::TypeOperation::Narrow(narrow)
                    }
                    dir::TypeOperation::Mapped(mut mapped) => {
                        mapped.parameter.constraint = map(self, mapped.parameter.constraint)?;
                        if let Some(key_remap) = &mut mapped.parameter.key_remap {
                            *key_remap = map(self, *key_remap)?;
                        }
                        mapped.value = map(self, mapped.value)?;

                        dir::TypeOperation::Mapped(mapped)
                    }
                    dir::TypeOperation::Index(mut index) => {
                        index.left = map(self, index.left)?;
                        index.index = map(self, index.index)?;

                        dir::TypeOperation::Index(index)
                    }
                    dir::TypeOperation::TemplateLiteral(mut template) => {
                        template.spans =
                            self.map_type_id_list(source, target, template.spans, map)?;

                        dir::TypeOperation::TemplateLiteral(template)
                    }
                    dir::TypeOperation::Infer(mut infer) => {
                        if let Some(constraint) = &mut infer.constraint {
                            *constraint = map(self, *constraint)?;
                        }

                        dir::TypeOperation::Infer(infer)
                    }
                    dir::TypeOperation::TypeOf(query) => dir::TypeOperation::TypeOf(query),
                    dir::TypeOperation::KeyOf(mut unary) => {
                        unary.target = map(self, unary.target)?;

                        dir::TypeOperation::KeyOf(unary)
                    }
                    dir::TypeOperation::NoInfer(mut unary) => {
                        unary.target = map(self, unary.target)?;

                        dir::TypeOperation::NoInfer(unary)
                    }
                    dir::TypeOperation::Awaited(mut unary) => {
                        unary.target = map(self, unary.target)?;

                        dir::TypeOperation::Awaited(unary)
                    }
                    dir::TypeOperation::TryOutput { value } => dir::TypeOperation::TryOutput {
                        value: map(self, value)?,
                    },
                    dir::TypeOperation::TryResidual { value } => dir::TypeOperation::TryResidual {
                        value: map(self, value)?,
                    },
                    dir::TypeOperation::StaticBinary(mut binary) => {
                        binary.left = map(self, binary.left)?;
                        binary.right = map(self, binary.right)?;

                        dir::TypeOperation::StaticBinary(binary)
                    }
                    dir::TypeOperation::StaticUnary(mut unary) => {
                        unary.target = map(self, unary.target)?;

                        dir::TypeOperation::StaticUnary(unary)
                    }
                };

                dir::Type::Operation(operation)
            }

            // collections
            dir::Type::Array(mut array) => {
                array.element = map(self, array.element)?;

                dir::Type::Array(array)
            }
            dir::Type::FixedArray(mut array) => {
                array.element = map(self, array.element)?;
                array.count = map(self, array.count)?;

                dir::Type::FixedArray(array)
            }
            dir::Type::Slice(mut slice) => {
                slice.element = map(self, slice.element)?;

                dir::Type::Slice(slice)
            }
            dir::Type::Tuple(mut tuple) => {
                let mut elements = SmallVec::<[dir::TypeElement; 8]>::from_slice(
                    self.tuple_elements(source, tuple.elements)?,
                );
                for element in &mut elements {
                    element.ty = map(self, element.ty)?;
                }
                tuple.elements = self.intern_elements(target, &elements)?;

                dir::Type::Tuple(tuple)
            }

            // structural shapes
            dir::Type::Shape(mut shape) => {
                let mut fields = SmallVec::<[dir::TypeField; 8]>::from_slice(
                    self.shape_fields(source, shape.fields)?,
                );
                for field in &mut fields {
                    field.ty = map(self, field.ty)?;
                }
                shape.fields = self.intern_fields(target, &fields)?;
                shape.call_signatures =
                    self.map_type_id_list(source, target, shape.call_signatures, map)?;
                shape.construct_signatures =
                    self.map_type_id_list(source, target, shape.construct_signatures, map)?;

                let mut signatures = SmallVec::<[dir::TypeIndexSignature; 2]>::from_slice(
                    self.shape_index_signatures(source, shape.index_signatures)?,
                );
                for signature in &mut signatures {
                    signature.key_type = map(self, signature.key_type)?;
                    signature.value_type = map(self, signature.value_type)?;
                }
                shape.index_signatures = self.intern_index_signatures(target, &signatures)?;

                dir::Type::Shape(shape)
            }
            dir::Type::FunctionSignature(mut function) => {
                if let Some(this_parameter) = &mut function.this_parameter {
                    *this_parameter = map(self, *this_parameter)?;
                }

                let mut parameters = SmallVec::<[dir::FunctionParameterType; 8]>::from_slice(
                    self.signature_parameters(source, function.parameters)?,
                );
                for parameter in &mut parameters {
                    parameter.ty = map(self, parameter.ty)?;
                }
                function.parameters = self.intern_parameters(target, &parameters)?;

                if let Some(return_type) = &mut function.return_type {
                    *return_type = map(self, *return_type)?;
                }

                dir::Type::FunctionSignature(function)
            }
            dir::Type::Function(mut function) => {
                function.signature = map(self, function.signature)?;
                function.environment = map(self, function.environment)?;

                dir::Type::Function(function)
            }
            dir::Type::FunctionPointer(mut function) => {
                function.signature = map(self, function.signature)?;

                dir::Type::FunctionPointer(function)
            }

            // algebraic composites
            dir::Type::Union(mut union) => {
                union.elements = self.map_type_id_list(source, target, union.elements, map)?;

                dir::Type::Union(union)
            }
            dir::Type::Intersection(mut intersection) => {
                intersection.elements =
                    self.map_type_id_list(source, target, intersection.elements, map)?;

                dir::Type::Intersection(intersection)
            }
        };

        Ok(ty)
    }

    /// Rebuild one type id list by mapping every id.
    fn map_type_id_list(
        &mut self,
        source: ModuleId,
        target: ModuleId,
        list: dir::TypeListId,
        map: &mut impl FnMut(&mut Self, dir::GlobalTypeId) -> CompilerResult<dir::GlobalTypeId>,
    ) -> CompilerResult<dir::TypeListId> {
        let mut ids = SmallVec::<[dir::GlobalTypeId; 8]>::from_slice(self.type_ids(source, list)?);
        for id in &mut ids {
            *id = map(self, *id)?;
        }

        self.intern_type_ids(target, &ids)
    }
}
