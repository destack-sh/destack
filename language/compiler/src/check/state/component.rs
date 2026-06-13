use std::sync::Arc;

use destack_artifact::GlobalEnvironment;
use destack_dir as dir;
use destack_repository::{ArtifactReader, ProviderContext};
use destack_source::{ModuleId, ProfileId};
use indexmap::IndexMap;

use crate::check::{
    Assumption, CheckComponentKey, CheckEvent, CheckExternalModuleState, CheckModuleState,
    ConstraintTable, DecisionTable, GenericIndex, InputTable, Journal, Mutation, ObligationTable,
    Queue, RelationCache, VariableTable, VarianceEntry,
};
use crate::{Compiler, CompilerError, CompilerResult};

/// State for checking one resolved component.
///
/// Fields group by speculation behavior: the solver tables roll back
/// under probes through the journal, the memo tables only record
/// closed facts that stay valid across probe rollback, and everything
/// else is fixed once walking finishes.
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

    // walk-recorded facts, fixed once solving starts
    /// Inferred types and values keyed by source identity.
    pub(in crate::check) inputs: InputTable,

    // speculative solver state, journaled for probe rollback
    /// Open inference variables.
    pub(in crate::check) variables: VariableTable,
    /// Collected relation constraints.
    pub(in crate::check) constraints: ConstraintTable,
    /// Decided node meanings.
    pub(in crate::check) decisions: DecisionTable,
    /// Obligated checks collected while walking.
    pub(in crate::check) obligations: ObligationTable,
    /// Memoized relation verdicts with the in-progress cycle guard.
    pub(in crate::check) relations: RelationCache,
    /// Implicit coercions recorded at accepted value flows.
    pub(in crate::check) coercions: IndexMap<dir::GlobalNodeIdAny, dir::Coercion>,
    /// Scheduled solver work.
    pub(in crate::check) queue: Queue,
    /// Active static guard assumptions for the running task.
    pub(in crate::check) assumptions: Vec<Assumption>,
    /// Mutation log for speculative probes.
    pub(in crate::check) journal: Journal,

    // memoized closed facts, valid across probe rollback
    /// Memoized closed evaluations keyed by reduced root.
    pub(in crate::check) evaluations: IndexMap<dir::GlobalTypeId, dir::GlobalTypeId>,
    /// Generic instances, argument variables, and induction bookkeeping.
    pub(in crate::check) generics: GenericIndex,
    /// Memoized layout segments per module, component and external.
    pub(in crate::check) layouts: IndexMap<ModuleId, dir::LayoutSegment>,
    /// Generic parameter variance derivations.
    pub(in crate::check) variances: IndexMap<dir::GlobalGenericParameterId, VarianceEntry>,

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
            inputs: InputTable::new(),
            variables: VariableTable::new(),
            constraints: ConstraintTable::new(),
            decisions: DecisionTable::new(),
            relations: RelationCache::new(),
            evaluations: IndexMap::new(),
            coercions: IndexMap::new(),
            assumptions: Vec::new(),
            queue: Queue::new(),
            journal: Journal::new(),
            generics: GenericIndex::new(),
            obligations: ObligationTable::new(),
            layouts: IndexMap::new(),
            variances: IndexMap::new(),
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

        // declare component headers before any body can read them
        for module in modules.iter().copied() {
            self.declare_module_headers(module)?;
        }

        // walk modules in stable component order
        for module in modules.iter().copied() {
            self.walk_module(module)?;
        }

        Ok(())
    }

    /// Complete walk-time state before solving.
    pub(in crate::check) fn propagate(&mut self) -> CompilerResult<()> {
        let modules = self.modules.keys().copied().collect::<Vec<_>>();
        self.propagate_induced_generics(&modules)
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
        self.layouts
            .insert(module_id, dir::LayoutSegment::new(module_id));

        Ok(())
    }

    /// Return one language symbol resolved for one module.
    pub(in crate::check) fn language_symbol(&self, item: dir::LanguageItem) -> dir::GlobalSymbolId {
        self.environment.language.symbol(item).unwrap_or_else(|| {
            unreachable!("language item {item} is missing from the global environment")
        })
    }
}

impl CheckState<'_> {
    /// Return one type, reading open working types over committed tables.
    pub(in crate::check) fn ty(&self, id: dir::GlobalTypeId) -> CompilerResult<&dir::Type> {
        // read open working types over committed component tables
        if let Some(module) = self.modules.get(&id.module_id) {
            if let Some(ty) = module.working.types.get_type_maybe(id.local_id) {
                return Ok(ty);
            }

            module
                .type_maybe(id.local_id)
                .ok_or_else(|| CompilerError::Internal {
                    message: format!("check type {id:?} is not allocated in any segment"),
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

    /// Allocate one open type in a module's working segment.
    pub(in crate::check) fn push_type(
        &mut self,
        module: ModuleId,
        ty: dir::Type,
        source: dir::LocalNodeIdAny,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let working = self
            .modules
            .get_mut(&module)
            .ok_or_else(|| CompilerError::Internal {
                message: format!("check module {module:?} has no working types"),
            })?;
        let local = working.working.types.insert_type_from_any(ty, source);
        // probe rollback reclaims speculative allocations
        self.journal.record(Mutation::TypeAllocated { module });

        Ok(local.into_global(module))
    }

    /// Allocate one open variable reference type in a module's working segment.
    pub(in crate::check) fn push_variable_type(
        &mut self,
        variable: dir::TypeVariableId,
        source: dir::LocalNodeIdAny,
    ) -> CompilerResult<dir::GlobalTypeId> {
        self.push_type(variable.module_id, dir::Type::Variable(variable), source)
    }

    /// Return one definition, reading working segments over external tables.
    pub(in crate::check) fn definition(
        &self,
        symbol: dir::GlobalSymbolId,
    ) -> Option<&dir::Definition> {
        // read working component definitions first
        if let Some(module) = self.modules.get(&symbol.module_id)
            && let Some(definition) = module.working.definitions.definition(symbol)
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
            .working
            .definitions
            .insert_definition(symbol, source, definition);

        Ok(())
    }
}
