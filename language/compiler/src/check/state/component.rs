use std::sync::Arc;

use destack_artifact::{DirDeclaredComponent, DirDeclaredModule, GlobalEnvironment};
use destack_core::{FxIndexMap, FxIndexSet, StringPool};
use destack_dir as dir;
use destack_repository::{ArtifactReader, Environment, ProviderContext};
use destack_source::{ComponentId, ModuleId, ProfileId, StringId};
use smallvec::SmallVec;

use crate::check::{
    Cause, CauseId, CheckEvent, CheckExternalModuleState, CheckModuleState, DecisionTable,
    DecoratorApplication, FunctionBody, GenericIndex, GenericScope, GenericTemplateId, Origin,
    OriginId, Solver, TryPropagationTarget, VarianceContext, VarianceState,
    should_stream_check_events,
};
use crate::{Compiler, CompilerError, CompilerResult};

/// The artifact one external module's committed tables load from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(in crate::check) enum CheckExternalComponent {
    /// One inference component's checked artifact.
    Checked(ComponentId),
    /// One reference component's declared artifact.
    Declared(ComponentId),
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
    /// The active global language and module environment.
    pub(in crate::check) global: Arc<GlobalEnvironment>,
    /// The ambient environment captured by the current revision.
    pub(in crate::check) environment: Arc<Environment>,

    // loaded modules
    /// Loaded component modules keyed by module id.
    pub(in crate::check) modules: FxIndexMap<ModuleId, CheckModuleState>,
    /// Loaded out-of-component modules keyed by module id.
    pub(in crate::check) external_modules: FxIndexMap<ModuleId, CheckExternalModuleState>,
    /// Members whose bodies this check infers.
    pub(in crate::check) inference_modules: FxIndexSet<ModuleId>,
    /// Modules with an active walk state, innermost last.
    pub(in crate::check) active_walks: FxIndexSet<ModuleId>,
    /// Whether every member template is declared and walked.
    pub(in crate::check) templates_ready: bool,
    /// Sealed artifact containing each external module's committed tables.
    pub(in crate::check) external_components: FxIndexMap<ModuleId, CheckExternalComponent>,
    /// Inherent extension modules awaiting their first extension lookup.
    pub(in crate::check) inherent_externals: Option<FxIndexSet<ModuleId>>,
    /// Inherent extension symbols carried by the component graph.
    pub(in crate::check) inherent_extensions: Vec<dir::GlobalSymbolId>,

    // walk state
    /// Resolved decorators in component walk order.
    pub(in crate::check) decorators: Vec<DecoratorApplication>,

    // checked state
    /// Stable declaration symbol types.
    pub(in crate::check) declaration_types: FxIndexMap<dir::GlobalSymbolId, dir::GlobalTypeId>,
    /// Body-owned binding symbol types.
    pub(in crate::check) binding_types: FxIndexMap<dir::GlobalSymbolId, dir::GlobalTypeId>,
    /// Checked source node occurrence types.
    pub(in crate::check) node_types: FxIndexMap<dir::GlobalNodeIdAny, dir::GlobalTypeId>,
    /// Stable source node decisions.
    pub(in crate::check) decisions: DecisionTable,

    // generic state
    /// Generic instances, argument variables, and induction bookkeeping.
    pub(in crate::check) generics: GenericIndex,
    /// Flattened generic scopes computed once per template.
    pub(in crate::check) scopes: FxIndexMap<GenericTemplateId, Arc<GenericScope>>,
    /// Generic parameter variance derivations per handle context.
    pub(in crate::check) variances:
        FxIndexMap<(dir::GlobalGenericParameterId, VarianceContext), VarianceState>,
    /// Declarations already walked, on demand or in root order.
    pub(in crate::check) walked_declarations: FxIndexSet<dir::GlobalNodeIdAny>,
    /// Declarations currently walking, innermost last.
    pub(in crate::check) walking_declarations: Vec<dir::GlobalNodeIdAny>,
    /// Declarations applied while still walking, keyed to their referents.
    pub(in crate::check) cyclic_inductions: FxIndexMap<dir::GlobalNodeIdAny, dir::GlobalNodeIdAny>,

    // reduction state
    /// Memoized closed reduced types keyed by source type.
    pub(in crate::check) reduced_types:
        FxIndexMap<(dir::GlobalTypeId, Option<dir::GlobalGenericTemplateId>), dir::GlobalTypeId>,
    /// Memoized closed reduced type graphs keyed by source type.
    pub(in crate::check) reduced_type_graphs:
        FxIndexMap<(dir::GlobalTypeId, Option<dir::GlobalGenericTemplateId>), dir::GlobalTypeId>,
    /// Memoized common places for closed contextual types.
    pub(in crate::check) contextual_places: FxIndexMap<
        (dir::GlobalTypeId, Option<dir::GlobalGenericTemplateId>),
        Option<dir::GlobalTypeId>,
    >,

    // solver state
    /// Active component solver state.
    pub(in crate::check) solver: Solver,
    /// Obligation steps run so far, for trace numbering.
    pub(in crate::check) solve_steps: usize,
    /// Named function bodies keyed by their declaration symbol.
    pub(in crate::check) functions: FxIndexMap<dir::GlobalSymbolId, FunctionBody>,
    /// Lambda bodies keyed by their value expression.
    pub(in crate::check) lambdas: FxIndexMap<dir::GlobalNodeIdAny, FunctionBody>,
    /// Catch result holes keyed by their catch node.
    pub(in crate::check) catch_results: FxIndexMap<dir::GlobalNodeIdAny, dir::GlobalTypeId>,
    /// Try propagation targets keyed by their fallible source node.
    pub(in crate::check) try_propagations: FxIndexMap<dir::GlobalNodeIdAny, TryPropagationTarget>,
    /// Control output holes keyed by their loop, label, and break-value nodes.
    pub(in crate::check) control_results: FxIndexMap<dir::GlobalNodeIdAny, dir::GlobalTypeId>,

    // tracing
    /// Trace events recorded while checking.
    pub(in crate::check) events: Vec<CheckEvent>,
    /// Whether check events should be kept for artifact output.
    pub(in crate::check) emit_events: bool,
    /// Whether check events should print as they are recorded.
    pub(in crate::check) stream_events: bool,
}

impl<'a> CheckState<'a> {
    /// Return the shared repository string pool.
    pub(in crate::check) fn strings(&self) -> &'a StringPool {
        self.compiler.repository.string_pool()
    }

    /// Create a component check state.
    pub(in crate::check) fn new(
        compiler: &'a Compiler,
        context: &'a dyn ProviderContext,
        artifacts: &'a ArtifactReader<'a>,
        profile: ProfileId,
        global: Arc<GlobalEnvironment>,
        environment: Arc<Environment>,
        external_components: FxIndexMap<ModuleId, CheckExternalComponent>,
        inherent_externals: FxIndexSet<ModuleId>,
        inherent_extensions: Vec<dir::GlobalSymbolId>,
        inference_modules: FxIndexSet<ModuleId>,
        emit_events: bool,
    ) -> Self {
        Self {
            compiler,
            context,
            artifacts,
            profile,
            global,
            environment,
            modules: FxIndexMap::default(),
            external_modules: FxIndexMap::default(),
            inference_modules,
            active_walks: FxIndexSet::default(),
            templates_ready: false,
            external_components,
            inherent_externals: Some(inherent_externals),
            inherent_extensions,
            decorators: Vec::new(),
            declaration_types: FxIndexMap::default(),
            binding_types: FxIndexMap::default(),
            node_types: FxIndexMap::default(),
            decisions: DecisionTable::new(),
            generics: GenericIndex::new(),
            scopes: FxIndexMap::default(),
            variances: FxIndexMap::default(),
            walked_declarations: FxIndexSet::default(),
            walking_declarations: Vec::new(),
            cyclic_inductions: FxIndexMap::default(),
            reduced_types: FxIndexMap::default(),
            reduced_type_graphs: FxIndexMap::default(),
            contextual_places: FxIndexMap::default(),
            solver: Solver::new(),
            solve_steps: 0,
            functions: FxIndexMap::default(),
            lambdas: FxIndexMap::default(),
            catch_results: FxIndexMap::default(),
            try_propagations: FxIndexMap::default(),
            control_results: FxIndexMap::default(),
            events: Vec::new(),
            emit_events,
            stream_events: should_stream_check_events(),
        }
    }

    /// Return whether this check infers one member's bodies.
    pub(in crate::check) fn infers_module(&self, module: ModuleId) -> bool {
        self.inference_modules.contains(&module)
    }

    /// Return whether this check declares one reference component.
    pub(in crate::check) fn is_declaration(&self) -> bool {
        self.inference_modules.is_empty()
    }

    /// Declare one reference component.
    pub(in crate::check) fn declare(&mut self, modules: &[ModuleId]) -> CompilerResult<()> {
        self.load_declared_modules(modules)?;
        self.walk()?;
        self.propagate_induced_parameters()?;
        self.check_decorators()?;

        self.settle()
    }

    /// Check one inference component over its declared reference component.
    pub(in crate::check) fn check(
        &mut self,
        modules: &[ModuleId],
        declared: Option<&DirDeclaredComponent>,
    ) -> CompilerResult<()> {
        self.load_checked_modules(modules, declared)?;
        self.walk()?;
        self.propagate_induced_parameters()?;
        self.check_decorators()?;

        self.settle()
    }

    /// Load source modules for declaration.
    fn load_declared_modules(&mut self, modules: &[ModuleId]) -> CompilerResult<()> {
        for module in modules {
            self.load_module(*module, None)?;
        }

        Ok(())
    }

    /// Load source modules over their committed declaration prefixes.
    fn load_checked_modules(
        &mut self,
        modules: &[ModuleId],
        declared: Option<&DirDeclaredComponent>,
    ) -> CompilerResult<()> {
        // load modules in stable component order
        for module in modules {
            let module_declaration = declared
                .map(|declared| {
                    declared
                        .module(*module)
                        .ok_or_else(|| CompilerError::Internal {
                            message: format!(
                                "declared component {} misses module {module:?}",
                                declared.component
                            ),
                        })
                })
                .transpose()?;
            self.load_module(*module, module_declaration)?;
        }

        // index declared generic identities so re-derivations reuse their ids
        if declared.is_some() {
            for module in modules {
                self.index_declared_generics(*module)?;
            }
        }

        Ok(())
    }

    /// Walk every loaded module.
    pub(in crate::check) fn walk(&mut self) -> CompilerResult<()> {
        let modules = self.modules.keys().copied().collect::<Vec<_>>();

        // import external checked artifacts
        self.import_component_external_modules()?;

        // declare template identities, then walk their bounds, so
        //  declarations resolve in any order across module cycles
        for module in modules.iter().copied() {
            self.declare_module_templates(module)?;
        }
        for module in modules.iter().copied() {
            self.walk_module_templates(module)?;
        }
        self.templates_ready = true;

        // walk modules in stable component order
        for module in modules.iter().copied() {
            self.walk_module(module)?;
        }

        Ok(())
    }

    /// Load one module into component state.
    fn load_module(
        &mut self,
        module_id: ModuleId,
        declared: Option<&DirDeclaredModule>,
    ) -> CompilerResult<()> {
        if self.is_component_module(module_id) {
            return Ok(());
        }

        let profile = self
            .compiler
            .profile(self.context.revision(), self.profile)?
            .key;
        let module = self.compiler.module(self.context.revision(), module_id)?;
        let package = self
            .compiler
            .package(self.context.revision(), module.package_id)?;
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
        let module = CheckModuleState::new(
            module,
            package,
            profile,
            parsed,
            bound,
            resolved,
            Arc::clone(&expanded),
            declared,
        );

        self.modules.insert(module_id, module);

        Ok(())
    }

    /// Return one language symbol resolved for one module.
    pub(in crate::check) fn language_symbol(
        &self,
        item: dir::LanguageItem,
    ) -> CompilerResult<dir::GlobalSymbolId> {
        self.global
            .language
            .symbol(item)
            .ok_or_else(|| CompilerError::Internal {
                message: format!("global environment is missing language item {item}"),
            })
    }

    /// Return the language item named by one resolved symbol.
    pub(in crate::check) fn language_item(
        &mut self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<Option<dir::LanguageItem>> {
        let symbol = self.resolve_symbol_alias(symbol)?;

        Ok(self.global.language.item(symbol))
    }

    /// Return the nominal symbol named by one type head.
    pub(in crate::check) fn type_symbol(
        &self,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<Option<dir::GlobalSymbolId>> {
        let symbol = match self.ty(ty)? {
            dir::Type::Reference(reference) => Some(reference.symbol),
            dir::Type::Application(instance) => Some(instance.symbol),
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
        if let Some(module) = self.modules.get(&module) {
            module.type_table().for_each_child(ty, visit);
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
    pub(in crate::check) fn intern_type(
        &mut self,
        module: ModuleId,
        ty: dir::Type,
    ) -> CompilerResult<dir::GlobalTypeId> {
        // memory forms intern in one canonical composition order
        let ty = self.canonical_form_type(module, ty)?;

        // join the structural flags of every child type
        let mut children = SmallVec::<[dir::GlobalTypeId; 8]>::new();
        self.for_each_type_child(module, &ty, |child| children.push(child))?;
        let mut child_flags = dir::TypeFlags::EMPTY;
        for child in children {
            child_flags |= self.type_flags(child)?;
        }

        // operation payloads contribute their own symbolic flags
        if let dir::Type::Operation(operation) = ty {
            child_flags |= self.type_operation(module, operation)?.own_flags();
        }

        let local = self
            .module_mut(module)
            .types_tail
            .intern_type(ty, child_flags);

        Ok(local.into_global(module))
    }

    /// Intern one work origin into the solver.
    pub(in crate::check) fn intern_origin(&mut self, origin: Origin) -> OriginId {
        self.solver.intern_origin(origin)
    }

    /// Intern one constraint cause into the solver.
    pub(in crate::check) fn intern_cause(&mut self, cause: Cause) -> CauseId {
        self.solver.intern_cause(cause)
    }

    /// Return one interned cause's origin.
    pub(in crate::check) fn cause_origin(&self, id: CauseId) -> Origin {
        self.solver.cause(id).origin
    }

    /// Return one type operation payload by its interned id.
    pub(in crate::check) fn type_operation(
        &self,
        module: ModuleId,
        id: dir::TypeOperationId,
    ) -> CompilerResult<dir::TypeOperation> {
        if let Some(state) = self.modules.get(&module) {
            state.operation_maybe(id)
        } else if let Some(external) = self.external_modules.get(&module) {
            external.types.operation_maybe(id).copied()
        } else {
            None
        }
        .ok_or_else(|| CompilerError::Internal {
            message: format!("check type operation {id:?} is not allocated in {module:?}"),
        })
    }

    /// Return one borrow form payload by its interned id.
    pub(in crate::check) fn type_borrow(
        &self,
        module: ModuleId,
        id: dir::BorrowFormId,
    ) -> CompilerResult<dir::BorrowForm> {
        if let Some(state) = self.modules.get(&module) {
            state.borrow_maybe(id)
        } else if let Some(external) = self.external_modules.get(&module) {
            external.types.borrow_form_maybe(id).copied()
        } else {
            None
        }
        .ok_or_else(|| CompilerError::Internal {
            message: format!("check borrow form {id:?} is not allocated in {module:?}"),
        })
    }

    /// Intern one borrow form into a module's working segment.
    pub(in crate::check) fn intern_borrow(
        &mut self,
        module: ModuleId,
        lifetime: dir::GlobalTypeId,
        access: dir::GlobalTypeId,
    ) -> CompilerResult<dir::Form> {
        let id = self
            .module_mut(module)
            .types_tail
            .intern_borrow(dir::BorrowForm { lifetime, access });

        Ok(dir::Form::Borrowed(id))
    }

    /// Return one member projection payload by its interned id.
    pub(in crate::check) fn type_member(
        &self,
        module: ModuleId,
        id: dir::MemberTypeId,
    ) -> CompilerResult<dir::MemberType> {
        if let Some(state) = self.modules.get(&module) {
            state.member_maybe(id)
        } else if let Some(external) = self.external_modules.get(&module) {
            external.types.member_maybe(id).copied()
        } else {
            None
        }
        .ok_or_else(|| CompilerError::Internal {
            message: format!("check member type {id:?} is not allocated in {module:?}"),
        })
    }

    /// Return one type's member payload when its head is a member projection.
    pub(in crate::check) fn member_head(
        &self,
        id: dir::GlobalTypeId,
    ) -> CompilerResult<Option<dir::MemberType>> {
        match self.ty(id)? {
            dir::Type::Member(member) => Ok(Some(self.type_member(id.module_id, member)?)),
            _ => Ok(None),
        }
    }

    /// Intern one member projection into a module's working segment.
    pub(in crate::check) fn intern_member(
        &mut self,
        module: ModuleId,
        member: dir::MemberType,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let id = self.module_mut(module).types_tail.intern_member(member);

        self.intern_type(module, dir::Type::Member(id))
    }

    /// Return one refined application payload by its interned id.
    pub(in crate::check) fn type_refined(
        &self,
        module: ModuleId,
        id: dir::RefinedTypeId,
    ) -> CompilerResult<dir::RefinedType> {
        if let Some(state) = self.modules.get(&module) {
            state.refined_maybe(id)
        } else if let Some(external) = self.external_modules.get(&module) {
            external.types.refined_maybe(id).copied()
        } else {
            None
        }
        .ok_or_else(|| CompilerError::Internal {
            message: format!("check refined type {id:?} is not allocated in {module:?}"),
        })
    }

    /// Return one type's refined payload when its head is a refined application.
    pub(in crate::check) fn refined_head(
        &self,
        id: dir::GlobalTypeId,
    ) -> CompilerResult<Option<dir::RefinedType>> {
        match self.ty(id)? {
            dir::Type::Refined(refined) => Ok(Some(self.type_refined(id.module_id, refined)?)),
            _ => Ok(None),
        }
    }

    /// Intern one refined application into a module's working segment.
    pub(in crate::check) fn intern_refined(
        &mut self,
        module: ModuleId,
        refined: dir::RefinedType,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let id = self.module_mut(module).types_tail.intern_refined(refined);

        self.intern_type(module, dir::Type::Refined(id))
    }

    /// Return one function signature payload by its interned id.
    pub(in crate::check) fn type_signature(
        &self,
        module: ModuleId,
        id: dir::FunctionSignatureId,
    ) -> CompilerResult<dir::FunctionSignatureType> {
        if let Some(state) = self.modules.get(&module) {
            state.signature_maybe(id)
        } else if let Some(external) = self.external_modules.get(&module) {
            external.types.signature_maybe(id).copied()
        } else {
            None
        }
        .ok_or_else(|| CompilerError::Internal {
            message: format!("check function signature {id:?} is not allocated in {module:?}"),
        })
    }

    /// Return one type's signature payload when its head is a signature.
    pub(in crate::check) fn signature_head(
        &self,
        id: dir::GlobalTypeId,
    ) -> CompilerResult<Option<dir::FunctionSignatureType>> {
        match self.ty(id)? {
            dir::Type::FunctionSignature(signature) => {
                Ok(Some(self.type_signature(id.module_id, signature)?))
            }
            _ => Ok(None),
        }
    }

    /// Intern one function signature into a module's working segment.
    pub(in crate::check) fn intern_signature(
        &mut self,
        module: ModuleId,
        signature: dir::FunctionSignatureType,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let id = self
            .module_mut(module)
            .types_tail
            .intern_signature(signature);

        self.intern_type(module, dir::Type::FunctionSignature(id))
    }

    /// Return one type's operation payload when its head is an operation.
    pub(in crate::check) fn operation_head(
        &self,
        id: dir::GlobalTypeId,
    ) -> CompilerResult<Option<dir::TypeOperation>> {
        match self.ty(id)? {
            dir::Type::Operation(operation) => {
                Ok(Some(self.type_operation(id.module_id, operation)?))
            }
            _ => Ok(None),
        }
    }

    /// Intern one type operation into a module's working segment.
    pub(in crate::check) fn intern_operation(
        &mut self,
        module: ModuleId,
        operation: dir::TypeOperation,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let id = self
            .module_mut(module)
            .types_tail
            .intern_operation(operation);

        self.intern_type(module, dir::Type::Operation(id))
    }

    /// Intern one type id list into a module's working segment.
    pub(in crate::check) fn intern_type_ids(
        &mut self,
        module: ModuleId,
        values: &[dir::GlobalTypeId],
    ) -> CompilerResult<dir::TypeListId> {
        Ok(self.module_mut(module).types_tail.intern_type_ids(values))
    }

    /// Intern one tuple element list into a module's working segment.
    pub(in crate::check) fn intern_elements(
        &mut self,
        module: ModuleId,
        values: &[dir::TypeElement],
    ) -> CompilerResult<dir::TypeListId> {
        Ok(self.module_mut(module).types_tail.intern_elements(values))
    }

    /// Intern one shape field list into a module's working segment.
    pub(in crate::check) fn intern_fields(
        &mut self,
        module: ModuleId,
        values: &[dir::TypeField],
    ) -> CompilerResult<dir::TypeListId> {
        Ok(self.module_mut(module).types_tail.intern_fields(values))
    }

    /// Intern one function parameter list into a module's working segment.
    pub(in crate::check) fn intern_parameters(
        &mut self,
        module: ModuleId,
        values: &[dir::FunctionParameterType],
    ) -> CompilerResult<dir::TypeListId> {
        Ok(self.module_mut(module).types_tail.intern_parameters(values))
    }

    /// Intern one index signature list into a module's working segment.
    pub(in crate::check) fn intern_index_signatures(
        &mut self,
        module: ModuleId,
        values: &[dir::TypeIndexSignature],
    ) -> CompilerResult<dir::TypeListId> {
        Ok(self
            .module_mut(module)
            .types_tail
            .intern_index_signatures(values))
    }

    /// Intern one string list into a module's working segment.
    pub(in crate::check) fn intern_strings(
        &mut self,
        module: ModuleId,
        values: &[StringId],
    ) -> CompilerResult<dir::TypeListId> {
        Ok(self.module_mut(module).types_tail.intern_strings(values))
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
    ) -> CompilerResult<&[StringId]> {
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

    /// Intern one applied reference type for a language item.
    pub(in crate::check) fn language_type(
        &mut self,
        module: ModuleId,
        item: dir::LanguageItem,
        arguments: &[dir::GlobalTypeId],
    ) -> CompilerResult<dir::GlobalTypeId> {
        let symbol = self.language_symbol(item)?;
        let arguments = self.intern_type_ids(module, arguments)?;
        let ty = dir::Type::Application(dir::GenericApplication { symbol, arguments });

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
        let origin = self.solver.variable(variable)?.origin;
        let module = self.solver.origin(origin).module();

        self.intern_type(module, dir::Type::Variable(variable))
    }

    /// Return one definition, importing the symbol's module on demand.
    pub(in crate::check) fn definition(
        &mut self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<Option<&dir::Definition>> {
        if !self.modules.contains_key(&symbol.module_id) {
            self.import_external_module(symbol.module_id)?;
        }

        Ok(self.loaded_definition(symbol))
    }

    /// Return one already loaded definition, without importing.
    pub(in crate::check) fn loaded_definition(
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

    /// Commit one nominal declaration's solved space.
    pub(in crate::check) fn commit_nominal_space(
        &mut self,
        symbol: dir::GlobalSymbolId,
        space: dir::Space,
    ) -> CompilerResult<()> {
        let module =
            self.modules
                .get_mut(&symbol.module_id)
                .ok_or_else(|| CompilerError::Internal {
                    message: format!("nominal declaration {symbol:?} is not in a component module"),
                })?;
        let definition =
            module
                .definitions
                .definition_mut(symbol)
                .ok_or_else(|| CompilerError::Internal {
                    message: format!("nominal declaration {symbol:?} has no definition"),
                })?;
        if !definition.set_space(space) {
            return Err(CompilerError::Internal {
                message: format!("definition {symbol:?} cannot carry nominal placement"),
            });
        }

        Ok(())
    }

    /// Report duplicate non-overload member keys in one definition.
    fn report_duplicate_definition_members(&mut self, definition: &dir::Definition) {
        let mut seen = FxIndexMap::<(dir::MemberSpace, dir::StaticKey), bool>::default();

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
            dir::Type::Application(mut instance) => {
                instance.arguments =
                    self.map_type_id_list(source, target, instance.arguments, map)?;

                dir::Type::Application(instance)
            }
            dir::Type::Refined(refined) => {
                let mut refined = self.type_refined(source, refined)?;
                refined.base = map(self, refined.base)?;
                refined.value = map(self, refined.value)?;
                let refined = self.module_mut(target).types_tail.intern_refined(refined);

                dir::Type::Refined(refined)
            }
            dir::Type::Member(member) => {
                let mut member = self.type_member(source, member)?;
                member.owner = map(self, member.owner)?;
                member.arguments = self.map_type_id_list(source, target, member.arguments, map)?;
                member.qualifier = member
                    .qualifier
                    .map(|qualifier| map(self, qualifier))
                    .transpose()?;
                let member = self.module_mut(target).types_tail.intern_member(member);

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
                    dir::Form::Borrowed(borrow) => {
                        let mut resolved = self.type_borrow(source, *borrow)?;
                        resolved.lifetime = map(self, resolved.lifetime)?;
                        resolved.access = map(self, resolved.access)?;
                        *borrow = self.module_mut(target).types_tail.intern_borrow(resolved);
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
                let operation = match self.type_operation(source, operation)? {
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
                        if let Some(modifiers_type) = &mut mapped.parameter.modifiers_type {
                            *modifiers_type = map(self, *modifiers_type)?;
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
                        let strings = self.template_strings(source, template.strings)?.to_vec();
                        template.strings = self.intern_strings(target, &strings)?;
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
                let operation = self
                    .module_mut(target)
                    .types_tail
                    .intern_operation(operation);

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
            dir::Type::FunctionSignature(function) => {
                let mut function = self.type_signature(source, function)?;
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
                let function = self
                    .module_mut(target)
                    .types_tail
                    .intern_signature(function);

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
