use std::sync::Arc;

use destack_artifact::GlobalEnvironment;
use destack_dir as dir;
use destack_repository::{ArtifactReader, ProviderContext};
use destack_source::{ComponentId, ModuleId, ProfileId};
use indexmap::IndexMap;

use crate::check::{
    CheckEvent, CheckExternalModuleState, CheckModuleState, DecisionTable, GenericIndex, Origin,
    Solver, VarianceEntry,
};
use crate::{CheckError, Compiler, CompilerError, CompilerResult};

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

    // checked symbol state
    /// Stable declaration symbol types.
    pub(in crate::check) declaration_types: IndexMap<dir::GlobalSymbolId, dir::GlobalTypeId>,
    /// Body-owned binding symbol types.
    pub(in crate::check) binding_types: IndexMap<dir::GlobalSymbolId, dir::GlobalTypeId>,
    /// Stable source node types.
    pub(in crate::check) node_types: IndexMap<dir::GlobalNodeIdAny, dir::GlobalTypeId>,
    /// Stable source node decisions.
    pub(in crate::check) decisions: DecisionTable,

    // solver state
    /// Active component solver state.
    pub(in crate::check) solver: Solver,

    // memoized closed facts, valid across rejected probes
    /// Memoized closed type reductions keyed by original type.
    pub(in crate::check) reduced_types: IndexMap<dir::GlobalTypeId, dir::GlobalTypeId>,
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
            declaration_types: IndexMap::new(),
            binding_types: IndexMap::new(),
            node_types: IndexMap::new(),
            decisions: DecisionTable::new(),
            solver: Solver::new(),
            reduced_types: IndexMap::new(),
            generics: GenericIndex::new(),
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

        // walk component headers before any body can read them
        for module in modules.iter().copied() {
            self.walk_module_headers(module)?;
        }

        // walk modules in stable component order
        for module in modules.iter().copied() {
            self.walk_module(module)?;
        }

        Ok(())
    }

    /// Complete walk-time state before solving.
    pub(in crate::check) fn propagate(&mut self) -> CompilerResult<()> {
        self.propagate_induced_generics()
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
    /// Return one type from this component's open overlay or external tables.
    pub(in crate::check) fn ty(&self, id: dir::GlobalTypeId) -> CompilerResult<&dir::Type> {
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
        let local = working.types.insert_type_from_any(ty, source);

        Ok(local.into_global(module))
    }

    /// Allocate one reference type for a language item.
    pub(in crate::check) fn push_language_type(
        &mut self,
        module: ModuleId,
        source: dir::LocalNodeIdAny,
        item: dir::LanguageItem,
        arguments: Vec<dir::GlobalTypeId>,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let symbol = self.language_symbol(item);
        let ty = dir::Type::Instance(dir::GenericInstance { symbol, arguments });

        self.push_type(module, ty, source)
    }

    /// Allocate one singleton type for an exact property key.
    pub(in crate::check) fn push_static_key_type(
        &mut self,
        module: ModuleId,
        source: dir::LocalNodeIdAny,
        key: dir::StaticKey,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let ty =
            match key {
                dir::StaticKey::Name(name) => dir::Type::Literal(dir::ScalarLiteral::String(name)),
                dir::StaticKey::Index(index) => match i64::try_from(index) {
                    Ok(index) => dir::Type::Literal(dir::ScalarLiteral::Integer(index)),
                    Err(_) => dir::Type::Primitive(dir::PrimitiveType::Integer(
                        dir::IntegerType::Pointer { is_signed: false },
                    )),
                },
                dir::StaticKey::Symbol(dir::SymbolKey::Unique(symbol)) => {
                    dir::Type::Instance(dir::GenericInstance {
                        symbol,
                        arguments: Vec::new(),
                    })
                }
                dir::StaticKey::Symbol(dir::SymbolKey::Registry(_)) => {
                    dir::Type::Primitive(dir::PrimitiveType::Symbol)
                }
            };

        self.push_type(module, ty, source)
    }

    /// Allocate the type read from an optional index signature.
    pub(in crate::check) fn push_index_signature_read_type(
        &mut self,
        origin: Origin,
        value: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let module = origin.module();
        let source = self.origin_source_node(origin)?;
        let undefined = self.push_type(module, dir::Type::Undefined, source)?;

        self.normalized_union_type(module, [value, undefined], source)
    }

    /// Allocate one open type at the source carried by an origin.
    pub(in crate::check) fn push_type_at_origin(
        &mut self,
        origin: Origin,
        ty: dir::Type,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let source = self.origin_source_node(origin)?;

        self.push_type(origin.module(), ty, source)
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
                    let (module, anchor) = self.source_anchor(member.source());
                    let member = self.format_static_key(&key);
                    let error = CheckError::DuplicateMember {
                        anchor,
                        module,
                        member,
                    };

                    self.module_mut(module).diagnostics.push(error.into());
                }
            } else {
                seen.insert(entry, is_overloadable);
            }
        }
    }
}
