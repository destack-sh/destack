use std::panic::Location;
use std::sync::Arc;

use rustc_hash::FxHashMap;
use smallvec::SmallVec;
use tspp_artifact::{DirResolved, EnvironmentBound, EnvironmentDeclared};
use tspp_core::{FxIndexMap, FxIndexSet, StringPool};
use tspp_dir as dir;
use tspp_dir::TypeFold;
use tspp_repository::{ArtifactAttemptRecorder, ArtifactReader, Environment, ProviderContext};
use tspp_source::{ModuleId, PackageId, ProfileId, StringId};

use crate::export::ExportResolver;
use crate::sema::auto::DecisionKey;
use crate::sema::{
    Answer, Cause, CauseId, CheckCounters, CheckModuleState, CheckTrace, CoroutineBody,
    DecoratorApplication, ExtensionHead, ExternalModuleTable, FieldInitializationObligation,
    FlowBranch, FlowState, Fulfillment, FunctionBody, GenericParameterId, GoalKey, HeritageReach,
    InferContext, NodeTable, Origin, OriginId, RelationKey, VarianceForm, VarianceState,
};
use crate::{Compiler, CompilerError, CompilerResult};

/// One solving pass over a module.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Pass {
    /// Declare the module's own interface from source.
    Declare,
    /// Flatten declared owners into stored member bindings.
    Elaborate,
    /// Infer the module's bodies.
    Check,
    /// Close the module's instances and evaluate their resolved types.
    Materialize,
    /// Project the memberships tooling reads over the settled module.
    Analyze,
}

/// One goal on the active decision path.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(in crate::sema) enum ActiveGoal {
    /// One auto interface derivation over a type, holding on re-entry.
    Derive(dir::GlobalTypeId, dir::AutoInterface),
    /// One extension matched against a subject, missing on re-entry.
    Extension(dir::GlobalSymbolId, dir::GlobalTypeId),
    /// One extension implementation goal, failing on re-entry.
    Implementation(dir::GlobalTypeId, dir::GlobalTypeId),
    /// One generic pattern matched against an actual, failing on re-entry.
    Match(dir::GlobalTypeId, dir::GlobalTypeId, u64),
}

/// State for checking one resolved module.
pub(in crate::sema) struct CheckState<'a> {
    // context
    /// The compiler running this check attempt.
    pub(in crate::sema) compiler: &'a Compiler,
    /// The provider context for artifact reads and diagnostics.
    pub(in crate::sema) context: &'a dyn ProviderContext,
    /// The provider's trace recorder, present on traced runs.
    pub(in crate::sema) recorder: Option<&'a ArtifactAttemptRecorder>,
    /// The provider-scoped artifact reader.
    pub(in crate::sema) artifacts: &'a ArtifactReader<'a>,
    /// The active profile.
    pub(in crate::sema) profile: ProfileId,
    /// The bound implicit environment: names, globals, and language items.
    pub(in crate::sema) environment_bound: Arc<EnvironmentBound>,
    /// The declared implicit environment, present while checking.
    pub(in crate::sema) environment_declared: Option<Arc<EnvironmentDeclared>>,
    /// The ambient environment captured by the current revision.
    pub(in crate::sema) environment: Arc<Environment>,
    /// The module being declared or checked.
    pub(in crate::sema) module_id: ModuleId,
    /// The pass this state solves.
    pub(in crate::sema) pass: Pass,
    /// The module's working state.
    pub(in crate::sema) module: CheckModuleState<'a>,
    /// Loaded external module states keyed by module id.
    pub(in crate::sema) external_modules: &'a ExternalModuleTable,
    /// Resolved import targets of external modules read for alias hops.
    pub(in crate::sema) external_resolutions: FxIndexMap<ModuleId, Arc<DirResolved>>,

    // solver
    /// The module's transient inference state.
    pub(in crate::sema) infer: InferContext,
    /// The fulfillment queue running pending work to verdicts.
    pub(in crate::sema) fulfill: Fulfillment,
    /// Flow cursor state for the pass's single body traversal.
    pub(in crate::sema) flow: FlowState,
    /// Named function bodies keyed by their declaration symbol.
    pub(in crate::sema) functions: FxIndexMap<dir::GlobalSymbolId, FunctionBody>,
    /// Lambda bodies keyed by their value expression.
    pub(in crate::sema) lambdas: FxIndexMap<dir::GlobalNodeIdAny, FunctionBody>,
    /// Coroutine bodies in discovery order, kept until their creation rows commit.
    pub(in crate::sema) coroutines: Vec<CoroutineBody>,
    /// Member block bodies discovered while checking, in discovery order.
    pub(in crate::sema) blocks: Vec<dir::GlobalNodeIdAny>,
    /// Resolved decorators in module walk order.
    pub(in crate::sema) decorators: Vec<DecoratorApplication>,

    // walk
    /// Declarations already walked, when canonicalized or in root order.
    pub(in crate::sema) walked_declarations: FxIndexSet<dir::GlobalNodeIdAny>,
    /// The authored decorators this pass's walk already visited.
    pub(in crate::sema) walked_decorators: FxIndexSet<dir::LocalNodeId<dir::Decorator>>,
    /// Declarations currently walking, innermost last.
    pub(in crate::sema) walking_declarations: Vec<dir::GlobalNodeIdAny>,
    /// The goals on the active decision path, each closing on re-entry by its own rule.
    pub(in crate::sema) active: FxIndexSet<ActiveGoal>,
    /// The count of conditional reductions nested on the stack.
    pub(in crate::sema) instantiation_depth: u32,

    // memos
    /// Export lookups reused by import suggestions.
    pub(in crate::sema) exports: ExportResolver,
    /// Remembered choices per decided goal over closed operands.
    pub(in crate::sema) answers: FxIndexMap<GoalKey, Answer>,
    /// Normalized heads per canonical type and assuming template.
    pub(in crate::sema) normalizations:
        FxIndexMap<(dir::GlobalTypeId, Option<dir::GlobalGenericTemplateId>), dir::GlobalTypeId>,
    /// Barrier-erased forms of closed contextual targets.
    pub(in crate::sema) erasures: FxIndexMap<dir::GlobalTypeId, dir::GlobalTypeId>,
    /// The closed substitutions applied so far, each keyed by its position.
    pub(in crate::sema) substitution_keys: FxIndexSet<(
        Option<dir::GlobalTypeId>,
        SmallVec<[dir::GenericArgumentBinding; 4]>,
    )>,
    /// The substituted graph of each type under each closed substitution.
    pub(in crate::sema) substituted: FxHashMap<(dir::GlobalTypeId, u32), dir::GlobalTypeId>,
    /// Memoized scalar families per closed type, none for types outside every family.
    pub(in crate::sema) scalar_families:
        FxIndexMap<dir::GlobalTypeId, Option<dir::ScalarFamilySet>>,
    /// Memoized aliasing per closed type.
    pub(in crate::sema) aliasing: FxIndexMap<dir::GlobalTypeId, bool>,
    /// The instantiations of each module read for its template bodies, by source node.
    pub(in crate::sema) instantiations:
        FxIndexMap<ModuleId, FxIndexMap<dir::GlobalNodeIdAny, Vec<dir::Instantiation>>>,
    /// The dependents collected this pass for foreign declarations recorded after their import.
    pub(in crate::sema) foreign_dependents: FxIndexMap<dir::GlobalSymbolId, Vec<dir::GlobalTypeId>>,
    /// Memoized canonical flat union members per closed union target.
    pub(in crate::sema) canonical_unions:
        FxIndexMap<dir::GlobalTypeId, Option<SmallVec<[dir::GlobalTypeId; 4]>>>,
    /// Decided relations over closed operands.
    pub(in crate::sema) decided_relations: FxIndexMap<RelationKey, bool>,
    /// Extension targets closed receivers failed to match, by declared target and receiver.
    pub(in crate::sema) unmatched_targets: FxIndexSet<(dir::GlobalTypeId, dir::GlobalTypeId)>,
    /// Const bindings initialized by a fresh value.
    pub(in crate::sema) fresh_consts: FxIndexSet<dir::GlobalSymbolId>,
    /// Storable representations decided this pass.
    pub(in crate::sema) storables:
        FxIndexSet<(dir::GlobalTypeId, Option<dir::GlobalGenericTemplateId>)>,
    /// Derived parameter variances per handle form, with in-flight marks.
    pub(in crate::sema) variances:
        FxIndexMap<(dir::GlobalGenericParameterId, VarianceForm), VarianceState>,
    /// Written argument ranks per generic parameter.
    pub(in crate::sema) argument_ranks:
        FxIndexMap<GenericParameterId, (usize, Option<dir::GlobalGenericTemplateId>, usize)>,
    /// Declarations reached by each declaration's heritage.
    pub(in crate::sema) heritages: FxIndexMap<dir::GlobalSymbolId, HeritageReach>,
    /// Canonical member bindings per owner and space.
    pub(in crate::sema) member_bindings:
        FxIndexMap<(dir::GlobalSymbolId, dir::MemberSpace), Option<Arc<Vec<dir::MemberBinding>>>>,
    /// The declaring owner and visibility per member symbol.
    pub(in crate::sema) member_visibilities:
        FxIndexMap<dir::GlobalSymbolId, Option<(dir::GlobalSymbolId, dir::Visibility)>>,
    /// Decided auto interface conformances per decision key.
    pub(in crate::sema) conformances: FxIndexMap<DecisionKey, bool>,
    /// Memoized drop hook members per nominal, none for nominals outside the Drop conformance.
    pub(in crate::sema) drop_hooks: FxIndexMap<dir::GlobalSymbolId, Option<dir::GlobalSymbolId>>,
    /// Extension symbols visible per looking module and target head.
    pub(in crate::sema) visible_extensions:
        FxIndexMap<(ModuleId, ExtensionHead), SmallVec<[dir::GlobalSymbolId; 4]>>,
    /// Member keys each blanket extension can expose.
    pub(in crate::sema) blanket_keys: FxIndexMap<dir::GlobalSymbolId, FxIndexSet<dir::StaticKey>>,
    /// Interface requirements each extension implements, keyed by member key.
    pub(in crate::sema) requirement_interfaces:
        FxIndexMap<dir::GlobalSymbolId, FxIndexMap<dir::StaticKey, dir::GlobalSymbolId>>,

    // outputs
    /// Stable declaration symbol types.
    pub(in crate::sema) declaration_types: FxIndexMap<dir::GlobalSymbolId, dir::GlobalTypeId>,
    /// Body-owned binding symbol types.
    pub(in crate::sema) binding_types: FxIndexMap<dir::GlobalSymbolId, dir::GlobalTypeId>,
    /// Checked source node occurrence types.
    pub(in crate::sema) node_types: NodeTable,
    /// The requirements taking a receiver per interface, derived once per symbol.
    pub(in crate::sema) receiver_requirements:
        FxHashMap<dir::GlobalSymbolId, SmallVec<[dir::GlobalTypeId; 4]>>,
    /// The implementations of each interface across the program, read once per interface.
    pub(in crate::sema) program_implementations: FxHashMap<
        dir::GlobalSymbolId,
        SmallVec<[(dir::GlobalSymbolId, Option<dir::GlobalSymbolId>); 4]>,
    >,
    /// Constructor exit branches per initialized class, each with its constructor, filled at check.
    pub(in crate::sema) constructor_branches:
        FxIndexMap<dir::GlobalSymbolId, Vec<(dir::GlobalSymbolId, FlowBranch)>>,
    /// Declarations required to initialize their fields, checked once the constructors are.
    pub(in crate::sema) field_initializations: Vec<FieldInitializationObligation>,

    // stats
    /// Work counters for the provider trace.
    pub(in crate::sema) counters: CheckCounters,
    /// Trace state kept only when tracing is requested.
    pub(in crate::sema) trace: Option<Box<CheckTrace>>,
}

impl<'a> CheckState<'a> {
    /// Return the shared repository string pool.
    pub(in crate::sema) fn strings(&self) -> &'a StringPool {
        self.compiler.repository.string_pool()
    }

    /// Create check state over one loaded module.
    pub(in crate::sema) fn new(
        compiler: &'a Compiler,
        context: &'a dyn ProviderContext,
        artifacts: &'a ArtifactReader<'a>,
        profile: ProfileId,
        environment_bound: Arc<EnvironmentBound>,
        environment_declared: Option<Arc<EnvironmentDeclared>>,
        environment: Arc<Environment>,
        module: CheckModuleState<'a>,
        externals: &'a ExternalModuleTable,
        pass: Pass,
        records_events: bool,
    ) -> Self {
        let module_id = module.module.id;

        Self {
            // context
            compiler,
            context,
            recorder: context.recorder(),
            artifacts,
            profile,
            environment_bound,
            environment_declared,
            environment,
            module_id,
            pass,
            module,
            external_modules: externals,
            external_resolutions: FxIndexMap::default(),
            // solver
            infer: InferContext::new(),
            fulfill: Fulfillment::new(),
            flow: FlowState::default(),
            functions: FxIndexMap::default(),
            lambdas: FxIndexMap::default(),
            coroutines: Vec::new(),
            blocks: Vec::new(),
            decorators: Vec::new(),
            walked_declarations: FxIndexSet::default(),
            walked_decorators: FxIndexSet::default(),
            walking_declarations: Vec::new(),
            active: FxIndexSet::default(),
            instantiation_depth: 0,
            // memos
            exports: ExportResolver::new(profile),
            answers: FxIndexMap::default(),
            normalizations: FxIndexMap::default(),
            erasures: FxIndexMap::default(),
            substitution_keys: FxIndexSet::default(),
            substituted: FxHashMap::default(),
            scalar_families: FxIndexMap::default(),
            instantiations: FxIndexMap::default(),
            foreign_dependents: FxIndexMap::default(),
            canonical_unions: FxIndexMap::default(),
            aliasing: FxIndexMap::default(),
            decided_relations: FxIndexMap::default(),
            unmatched_targets: FxIndexSet::default(),
            fresh_consts: FxIndexSet::default(),
            storables: FxIndexSet::default(),
            variances: FxIndexMap::default(),
            argument_ranks: FxIndexMap::default(),
            heritages: FxIndexMap::default(),
            member_bindings: FxIndexMap::default(),
            member_visibilities: FxIndexMap::default(),
            conformances: FxIndexMap::default(),
            drop_hooks: FxIndexMap::default(),
            visible_extensions: FxIndexMap::default(),
            blanket_keys: FxIndexMap::default(),
            requirement_interfaces: FxIndexMap::default(),
            // outputs
            declaration_types: FxIndexMap::default(),
            binding_types: FxIndexMap::default(),
            node_types: NodeTable::default(),
            receiver_requirements: FxHashMap::default(),
            program_implementations: FxHashMap::default(),
            constructor_branches: FxIndexMap::default(),
            field_initializations: Vec::new(),
            // stats
            counters: CheckCounters::default(),
            trace: CheckTrace::new(records_events),
        }
    }

    /// Return whether this check infers one module's bodies.
    pub(in crate::sema) fn is_inferred_module(&self, module: ModuleId) -> bool {
        self.pass == Pass::Check && module == self.module_id
    }

    /// Return whether this pass declares the module's own interface.
    pub(in crate::sema) fn is_declaring(&self) -> bool {
        self.pass == Pass::Declare
    }

    /// Return whether this pass infers the module's bodies.
    pub(in crate::sema) fn is_checking(&self) -> bool {
        self.pass == Pass::Check
    }

    /// Commit the error type to every exported binding whose derivation failed.
    pub(in crate::sema) fn commit_underivable_exports(&mut self) -> CompilerResult<()> {
        // view the module tree with its expansion patches
        let module = self.module_id;
        let (parsed, expanded) = self.patched_inputs(module);
        let tree = dir::View::new(&parsed.tree).patched(&expanded.patch);

        // collect every exported module-scope declarator
        let mut exported = Vec::new();
        for root in &expanded.roots {
            self.collect_exported_declarators(module, tree, *root, &mut exported);
        }

        // report and commit the error type where derivation failed
        for (declarator, symbol) in exported {
            // keep exports that already derived a type
            if self.symbol_type_maybe(symbol)?.is_some() {
                continue;
            }

            // skip statically absent exports
            if self.is_absent(declarator.into_global(module)) {
                continue;
            }

            // report the failure once while declaring
            if self.is_declaring() {
                self.report_export_type_not_derivable(module, declarator);
            }

            let error = self.intern_type(dir::Type::Error)?;
            self.commit_symbol_type(symbol, error)?;
        }

        Ok(())
    }

    /// Collect exported module-scope declarator symbols beneath one root.
    fn collect_exported_declarators(
        &self,
        module: ModuleId,
        tree: dir::View<'_>,
        root: dir::LocalNodeId<dir::Expression>,
        exported: &mut Vec<(dir::LocalNodeIdAny, dir::GlobalSymbolId)>,
    ) {
        // collect the exports each root statement declares
        match tree.get(root) {
            // recurse into nested module and global root statements
            dir::Expression::Declaration(declaration) => {
                let expressions = match tree.get(*declaration) {
                    dir::Declaration::Global(declaration) => declaration.expressions.clone(),
                    dir::Declaration::Module(declaration) => declaration.expressions.clone(),
                    _ => return,
                };
                for expression in expressions {
                    self.collect_exported_declarators(module, tree, expression, exported);
                }
            }

            // collect each exported declarator's declaration symbol
            dir::Expression::Let {
                export: Some(_),
                declarators,
                ..
            } => {
                for declarator in declarators.clone() {
                    let pattern = tree.get(declarator).pattern;
                    let Some(symbol) = self.module(module).declaration_symbol(pattern.into_any())
                    else {
                        continue;
                    };
                    exported.push((declarator.into_any(), symbol));
                }
            }

            // skip every other statement
            _ => {}
        }
    }

    /// Walk the loaded module's declarations for the declare pass.
    pub(in crate::sema) fn walk_declarations(&mut self) -> CompilerResult<()> {
        let module = self.module_id;

        // import external declared modules
        self.import_external_modules()?;

        // declare template identities before walking their bounds
        self.declare_module_templates(module)?;
        self.walk_module_templates(module)?;

        self.walk_module(module)
    }

    /// Walk the loaded module's bodies against the declared entries for the check pass.
    pub(in crate::sema) fn walk_bodies(&mut self) -> CompilerResult<()> {
        let module = self.module_id;

        // import external declared modules
        self.import_external_modules()?;

        // canonicalize this module's declared types once against its imports
        self.canonicalize_declared_types()?;

        self.walk_module_bodies(module)
    }

    /// Return one language symbol resolved for one module.
    pub(in crate::sema) fn language_symbol(
        &self,
        item: dir::LanguageItem,
    ) -> CompilerResult<dir::GlobalSymbolId> {
        self.environment_bound
            .language
            .symbol(item)
            .ok_or_else(|| CompilerError::Internal {
                message: format!("bound environment is missing language item {item}"),
            })
    }

    /// Return the package that declares the language items.
    pub(in crate::sema) fn language_package(&self) -> CompilerResult<PackageId> {
        self.environment_bound
            .language
            .package()
            .ok_or_else(|| CompilerError::Internal {
                message: "bound environment declares no language items".to_string(),
            })
    }

    /// Return the language item named by one resolved symbol.
    pub(in crate::sema) fn language_item(
        &self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<Option<dir::LanguageItem>> {
        Ok(self.environment_bound.language.item(symbol))
    }

    /// Return the nominal symbol named by one type head.
    pub(in crate::sema) fn type_symbol(
        &self,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<Option<dir::GlobalSymbolId>> {
        let symbol = match self.ty(ty)? {
            dir::Type::Application(instance) => Some(instance.symbol),
            _ => None,
        };

        Ok(symbol)
    }
}

impl<'a> CheckState<'a> {
    /// Return one type head from this module's open overlay or external tables.
    #[track_caller]
    pub(in crate::sema) fn ty(&self, id: dir::GlobalTypeId) -> CompilerResult<dir::Type> {
        let ty = self.ty_raw(id)?;

        // refuse a solved variable left unresolved, whose payloads belong to its solution
        if let dir::Type::Variable(variable) = ty
            && self.infer.solution(variable)?.is_some()
        {
            let caller = Location::caller();

            return Err(CompilerError::Internal {
                message: format!(
                    "solved variable {variable:?} read unresolved as type {id:?} at {caller}"
                ),
            });
        }

        Ok(ty)
    }

    /// Return one type head through its solution.
    pub(in crate::sema) fn resolved_ty(&self, id: dir::GlobalTypeId) -> CompilerResult<dir::Type> {
        let id = self.shallow_resolve(id)?;

        self.ty(id)
    }

    /// Reserved for callers that match variables explicitly, like resolution and write-back.
    pub(in crate::sema) fn ty_raw(&self, id: dir::GlobalTypeId) -> CompilerResult<dir::Type> {
        // read this module's open working types
        if self.is_own_module(id.module_id) {
            self.type_maybe(id.local_id)
                .ok_or_else(|| CompilerError::Internal {
                    message: format!("check type {id:?} is not allocated"),
                })
        }
        // read external committed tables
        else if let Some(external) = self.external(id.module_id)? {
            Ok(external.types().get_type(id.local_id))
        }
        // fail loudly on a module missing from this check
        else {
            Err(CompilerError::Internal {
                message: format!("check type {id:?} belongs to an unloaded module"),
            })
        }
    }

    /// Return whether any operand already reported an error.
    pub(in crate::sema) fn has_error_operand(
        &self,
        operands: &[dir::GlobalTypeId],
    ) -> CompilerResult<bool> {
        for operand in operands {
            let operand = self.shallow_resolve(*operand)?;
            if self.type_flags(operand)?.has_error() {
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Return one type's structural flags.
    pub(in crate::sema) fn type_flags(
        &self,
        id: dir::GlobalTypeId,
    ) -> CompilerResult<dir::TypeFlags> {
        // read this module's open working types
        if self.is_own_module(id.module_id) {
            self.type_flags_maybe(id.local_id)
                .ok_or_else(|| CompilerError::Internal {
                    message: format!("check type {id:?} is not allocated"),
                })
        }
        // read external committed tables
        else if let Some(external) = self.external(id.module_id)? {
            Ok(external.types().get_type_flags(id.local_id))
        }
        // fail loudly on a module missing from this check
        else {
            Err(CompilerError::Internal {
                message: format!("check type {id:?} belongs to an unloaded module"),
            })
        }
    }

    /// Return one own-module type, reading the open tail over the committed base.
    pub(in crate::sema) fn type_maybe(&self, type_id: dir::LocalTypeId) -> Option<dir::Type> {
        self.module
            .types_tail
            .get_type_maybe(type_id)
            .or_else(|| self.module.types.get_type_maybe(type_id))
    }

    /// Return one own-module type's structural flags.
    pub(in crate::sema) fn type_flags_maybe(
        &self,
        type_id: dir::LocalTypeId,
    ) -> Option<dir::TypeFlags> {
        if let Some(flags) = self.module.types_tail.get_type_flags_maybe(type_id) {
            return Some(flags);
        }

        // read the committed base where it holds the type
        self.module.types.get_type_maybe(type_id)?;

        Some(self.module.types.get_type_flags(type_id))
    }

    /// Return one own-module operation payload, reading the tail over the base.
    fn operation_maybe(&self, id: dir::TypeOperationId) -> Option<dir::TypeOperation> {
        self.module
            .types_tail
            .operation(id)
            .or_else(|| self.module.types.operation_maybe(id))
            .copied()
    }

    /// Return one own-module signature payload, reading the tail over the base.
    fn signature_maybe(&self, id: dir::FunctionSignatureId) -> Option<dir::FunctionSignatureType> {
        self.module
            .types_tail
            .signature(id)
            .or_else(|| self.module.types.signature_maybe(id))
            .copied()
    }

    /// Return one own-module member payload, reading the tail over the base.
    fn member_maybe(&self, id: dir::MemberTypeId) -> Option<dir::MemberType> {
        self.module
            .types_tail
            .member(id)
            .or_else(|| self.module.types.member_maybe(id))
            .copied()
    }

    /// Return one own-module refined payload, reading the tail over the base.
    fn refined_maybe(&self, id: dir::RefinedTypeId) -> Option<dir::RefinedType> {
        self.module
            .types_tail
            .refined(id)
            .or_else(|| self.module.types.refined_maybe(id))
            .copied()
    }

    /// Return one own-module borrow payload, reading the tail over the base.
    pub(in crate::sema) fn borrow_maybe(&self, id: dir::BorrowFormId) -> Option<dir::BorrowForm> {
        self.module
            .types_tail
            .borrow_form(id)
            .or_else(|| self.module.types.borrow_form_maybe(id))
            .copied()
    }

    /// Visit each direct child type id of one type owned by a module.
    pub(in crate::sema) fn for_each_type_child(
        &self,
        module: ModuleId,
        ty: &dir::Type,
        visit: impl FnMut(dir::GlobalTypeId),
    ) -> CompilerResult<()> {
        // resolve payload lists through the owning module's tables
        if self.is_own_module(module) {
            self.module
                .types
                .with_tail(&self.module.types_tail)
                .for_each_child(ty, visit);
        } else if let Some(external) = self.external(module)? {
            external.types().for_each_child(ty, visit);
        } else {
            return Err(CompilerError::Internal {
                message: format!("check type children belong to an unloaded module {module:?}"),
            });
        }

        Ok(())
    }

    /// Intern one type into a module's working segment.
    pub(in crate::sema) fn intern_type(
        &mut self,
        ty: dir::Type,
    ) -> CompilerResult<dir::GlobalTypeId> {
        // count this intern for the pass stats
        self.counters.interns += 1;
        let module = self.module_id;

        // memory forms intern in one canonical composition order
        let ty = self.canonical_form_type(ty)?;

        // an application short of its parameters interns as its completed application
        if let dir::Type::Application(application) = &ty
            && self.pass != Pass::Declare
            && let Some(filled) = self.fill_elided_application(module, application)?
        {
            return Ok(filled);
        }

        // join the structural flags of every child type
        let mut children = SmallVec::<[dir::GlobalTypeId; 8]>::new();
        self.for_each_type_child(module, &ty, |child| children.push(child))?;
        let mut child_flags = dir::TypeFlags::EMPTY;
        for child in &children {
            child_flags |= self.type_flags(*child)?;
        }

        // note the foreign modules this type's children mention
        for child in &children {
            if child.module_id != module {
                self.module.references.insert(child.module_id);
            }
        }

        // note the foreign modules the type value itself mentions
        let mut mentions = SmallVec::<[ModuleId; 2]>::new();
        ty.referenced_modules(&mut |mentioned| mentions.push(mentioned));
        for mentioned in mentions {
            if mentioned != module {
                self.module.references.insert(mentioned);
            }
        }

        // merge the flags of operation payloads
        if let dir::Type::Operation(operation) = ty {
            child_flags |= self.type_operation(module, operation)?.own_flags();
        }

        // mark a parameter head by its kind
        if let dir::Type::Parameter(parameter) | dir::Type::Erased(parameter) = &ty {
            child_flags |= self
                .generic_parameter(*parameter)?
                .map_or(dir::TypeFlags::HAS_TYPE_PARAMETER, |binding| {
                    binding.kind.parameter_flags()
                });
        }

        // an intrinsic alias application computes like an operation
        let alias_body = match &ty {
            dir::Type::Application(dir::GenericApplication { symbol, .. })
            | dir::Type::Reference(dir::TypeReference { symbol, .. }) => {
                match self.definition(*symbol)?.as_deref() {
                    Some(dir::Definition::TypeAlias(alias)) => Some(alias.value),
                    _ => None,
                }
            }
            _ => None,
        };
        let is_intrinsic = match alias_body {
            Some(body) => matches!(self.ty(body)?, dir::Type::Intrinsic),
            None => false,
        };
        if is_intrinsic {
            child_flags |= dir::TypeFlags::HAS_OPERATION;
        }

        // keep alias and collection applications intact
        let is_alias = alias_body.is_some() && !is_intrinsic;
        let is_written_alias = is_alias
            || match &ty {
                dir::Type::Application(instance) => matches!(
                    self.language_item(instance.symbol)?,
                    Some(
                        dir::LanguageItem::Array
                            | dir::LanguageItem::Slice
                            | dir::LanguageItem::FixedArray
                            | dir::LanguageItem::Dynamic
                    )
                ),
                _ => false,
            };

        // store the type in this module's working tail
        let (local, inserted) = self.module.types_tail.intern_type_inserted(ty, child_flags);
        let id = local.into_global(module);

        // normalize each newly born closed head once declarations can load
        if inserted
            && !child_flags.has_variable()
            && self.pass != Pass::Declare
            && !is_written_alias
        {
            return self.normalize_closed(id);
        }

        Ok(id)
    }

    /// Intern one work origin into the solver.
    pub(in crate::sema) fn intern_origin(&mut self, origin: Origin) -> OriginId {
        self.infer.intern_origin(origin)
    }

    /// Intern one constraint cause into the solver.
    pub(in crate::sema) fn intern_cause(&mut self, cause: Cause) -> CauseId {
        self.infer.intern_cause(cause)
    }

    /// Return one interned cause's origin.
    pub(in crate::sema) fn cause_origin(&self, id: CauseId) -> Origin {
        self.infer.cause(id).origin
    }

    /// Return one type operation payload by its interned id.
    pub(in crate::sema) fn type_operation(
        &self,
        module: ModuleId,
        id: dir::TypeOperationId,
    ) -> CompilerResult<dir::TypeOperation> {
        if self.is_own_module(module) {
            self.operation_maybe(id)
        } else if let Some(external) = self.external(module)? {
            external.types().operation_maybe(id).copied()
        } else {
            None
        }
        .ok_or_else(|| CompilerError::Internal {
            message: format!("check type operation {id:?} is not allocated in {module:?}"),
        })
    }

    /// Return one borrow form payload by its interned id.
    pub(in crate::sema) fn type_borrow(
        &self,
        module: ModuleId,
        id: dir::BorrowFormId,
    ) -> CompilerResult<dir::BorrowForm> {
        if self.is_own_module(module) {
            self.borrow_maybe(id)
        } else if let Some(external) = self.external(module)? {
            external.types().borrow_form_maybe(id).copied()
        } else {
            None
        }
        .ok_or_else(|| CompilerError::Internal {
            message: format!("check borrow form {id:?} is not allocated in {module:?}"),
        })
    }

    /// Intern one borrow form into a module's working segment.
    pub(in crate::sema) fn intern_borrow(
        &mut self,
        region: dir::GlobalTypeId,
        access: dir::GlobalTypeId,
    ) -> CompilerResult<dir::Form> {
        let id = self
            .module
            .types_tail
            .intern_borrow(dir::BorrowForm { region, access });

        Ok(dir::Form::Borrowed(id))
    }

    /// Intern one region pair into a module's working segment.
    pub(in crate::sema) fn intern_region(
        &mut self,
        extent: dir::GlobalTypeId,
        space: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        // name a region term's whole extent and space
        if self.memory_kind(space)? == Some(dir::MemoryParameter::Region) {
            return Ok(space);
        }

        self.intern_type(dir::Type::Region(dir::RegionType { extent, space }))
    }

    /// Intern the literal naming the local space.
    pub(in crate::sema) fn local_space(&mut self) -> CompilerResult<dir::GlobalTypeId> {
        self.space_literal(dir::Space::Local)
    }

    /// Intern the literal naming one space.
    pub(in crate::sema) fn space_literal(
        &mut self,
        space: dir::Space,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let value = self.strings().intern(space.text());

        self.intern_type(dir::Type::Literal(dir::Literal::String(value)))
    }

    /// Intern the canonical singleton naming one lifetime.
    pub(in crate::sema) fn lifetime_literal(
        &mut self,
        lifetime: dir::Lifetime,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let value = self.strings().intern(&lifetime.text());

        self.intern_type(dir::Type::Literal(dir::Literal::String(value)))
    }

    /// Intern the canonical singleton naming one access.
    pub(in crate::sema) fn access_literal(
        &mut self,
        access: dir::Access,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let value = self.strings().intern(access.text());

        self.intern_type(dir::Type::Literal(dir::Literal::String(value)))
    }

    /// Intern the union of every access a shared borrow may have.
    pub(in crate::sema) fn shared_accesses(&mut self) -> CompilerResult<dir::GlobalTypeId> {
        let readonly = self.access_literal(dir::Access::Readonly)?;
        let mutable = self.access_literal(dir::Access::Mutable)?;
        let immutable = self.access_literal(dir::Access::Immutable)?;

        self.normalized_union_type([readonly, mutable, immutable])
    }

    /// Adopt one memory form's module-local borrow entry into this module.
    pub(in crate::sema) fn adopt_form(
        &mut self,
        source: ModuleId,
        form: dir::Form,
    ) -> CompilerResult<dir::Form> {
        // rewrite a foreign borrow into this module's rows
        match form {
            dir::Form::Borrowed(id) if source != self.module_id => {
                let borrow = self.type_borrow(source, id)?;

                self.intern_borrow(borrow.region, borrow.access)
            }
            form => Ok(form),
        }
    }

    /// Return one member projection payload by its interned id.
    pub(in crate::sema) fn type_member(
        &self,
        module: ModuleId,
        id: dir::MemberTypeId,
    ) -> CompilerResult<dir::MemberType> {
        if self.is_own_module(module) {
            self.member_maybe(id)
        } else if let Some(external) = self.external(module)? {
            external.types().member_maybe(id).copied()
        } else {
            None
        }
        .ok_or_else(|| CompilerError::Internal {
            message: format!("check member type {id:?} is not allocated in {module:?}"),
        })
    }

    /// Return one type's member payload when its head is a member projection.
    pub(in crate::sema) fn member_head(
        &self,
        id: dir::GlobalTypeId,
    ) -> CompilerResult<Option<dir::MemberType>> {
        match self.ty(id)? {
            dir::Type::Member(member) => Ok(Some(self.type_member(id.module_id, member)?)),
            _ => Ok(None),
        }
    }

    /// Intern one member projection into a module's working segment.
    pub(in crate::sema) fn intern_member(
        &mut self,
        member: dir::MemberType,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let id = self.module.types_tail.intern_member(member);

        self.intern_type(dir::Type::Member(id))
    }

    /// Return one refined application payload by its interned id.
    pub(in crate::sema) fn type_refined(
        &self,
        module: ModuleId,
        id: dir::RefinedTypeId,
    ) -> CompilerResult<dir::RefinedType> {
        if self.is_own_module(module) {
            self.refined_maybe(id)
        } else if let Some(external) = self.external(module)? {
            external.types().refined_maybe(id).copied()
        } else {
            None
        }
        .ok_or_else(|| CompilerError::Internal {
            message: format!("check refined type {id:?} is not allocated in {module:?}"),
        })
    }

    /// Return one type's refined payload when its head is a refined application.
    pub(in crate::sema) fn refined_head(
        &self,
        id: dir::GlobalTypeId,
    ) -> CompilerResult<Option<dir::RefinedType>> {
        match self.ty(id)? {
            dir::Type::Refined(refined) => Ok(Some(self.type_refined(id.module_id, refined)?)),
            _ => Ok(None),
        }
    }

    /// Intern one refined application into a module's working segment.
    pub(in crate::sema) fn intern_refined(
        &mut self,
        refined: dir::RefinedType,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let id = self.module.types_tail.intern_refined(refined);

        self.intern_type(dir::Type::Refined(id))
    }

    /// Return one function signature payload by its interned id.
    pub(in crate::sema) fn type_signature(
        &self,
        module: ModuleId,
        id: dir::FunctionSignatureId,
    ) -> CompilerResult<dir::FunctionSignatureType> {
        if self.is_own_module(module) {
            self.signature_maybe(id)
        } else if let Some(external) = self.external(module)? {
            external.types().signature_maybe(id).copied()
        } else {
            None
        }
        .ok_or_else(|| CompilerError::Internal {
            message: format!("check function signature {id:?} is not allocated in {module:?}"),
        })
    }

    /// Return one type's signature payload when its head is a signature.
    pub(in crate::sema) fn signature_head(
        &self,
        id: dir::GlobalTypeId,
    ) -> CompilerResult<Option<dir::FunctionSignatureType>> {
        // read the signature each callable head carries
        match self.ty(id)? {
            dir::Type::FunctionSignature(signature) => {
                Ok(Some(self.type_signature(id.module_id, signature)?))
            }
            // fat callables and pointers carry their signature behind the value head
            dir::Type::Function(function) => self.signature_head(function.signature),
            dir::Type::FunctionPointer(function) => self.signature_head(function.signature),
            _ => Ok(None),
        }
    }

    /// Return the ordinary function value type of a signature.
    pub(in crate::sema) fn function_type(
        &mut self,
        signature: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let receiver = self.receiver_literal(dir::ReceiverMode::Borrowed {
            access: dir::Access::Readonly,
        })?;

        self.intern_type(dir::Type::Function(dir::FunctionType {
            signature,
            receiver,
        }))
    }

    /// Intern one function signature into a module's working segment.
    pub(in crate::sema) fn intern_signature(
        &mut self,
        signature: dir::FunctionSignatureType,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let id = self.module.types_tail.intern_signature(signature);

        self.intern_type(dir::Type::FunctionSignature(id))
    }

    /// Return one type's operation payload when its head is an operation.
    pub(in crate::sema) fn operation_head(
        &self,
        id: dir::GlobalTypeId,
    ) -> CompilerResult<Option<dir::TypeOperation>> {
        let id = self.shallow_resolve(id)?;

        // read the operation the head names
        match self.ty(id)? {
            dir::Type::Operation(operation) => {
                Ok(Some(self.type_operation(id.module_id, operation)?))
            }
            _ => Ok(None),
        }
    }

    /// Intern one type operation into a module's working segment.
    pub(in crate::sema) fn intern_operation(
        &mut self,
        operation: dir::TypeOperation,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let id = self.module.types_tail.intern_operation(operation);

        self.intern_type(dir::Type::Operation(id))
    }

    /// Return whether one tree tag names a lowercase builder row.
    pub(in crate::sema) fn is_intrinsic_tree_tag(&self, name: dir::StringId) -> bool {
        self.strings()
            .get(name)
            .starts_with(|letter: char| letter.is_ascii_lowercase())
    }

    /// Intern one type id list into a module's working segment.
    pub(in crate::sema) fn intern_type_ids(
        &mut self,
        values: &[dir::GlobalTypeId],
    ) -> CompilerResult<dir::TypeListId> {
        Ok(self.module.types_tail.intern_type_ids(values))
    }

    /// Intern one tuple element list into a module's working segment.
    pub(in crate::sema) fn intern_elements(
        &mut self,
        values: &[dir::TypeElement],
    ) -> CompilerResult<dir::TypeListId> {
        Ok(self.module.types_tail.intern_elements(values))
    }

    /// Intern one tuple type over its element rows.
    pub(in crate::sema) fn intern_tuple(
        &mut self,
        elements: &[dir::TypeElement],
    ) -> CompilerResult<dir::GlobalTypeId> {
        let elements = self.intern_elements(elements)?;

        // intern the elements as one tuple
        self.intern_type(dir::Type::Tuple(dir::TupleType {
            form: dir::TupleForm::Tuple,
            elements,
        }))
    }

    /// Intern one function parameter list into a module's working segment.
    pub(in crate::sema) fn intern_parameters(
        &mut self,
        values: &[dir::FunctionParameterType],
    ) -> CompilerResult<dir::TypeListId> {
        Ok(self.module.types_tail.intern_parameters(values))
    }

    /// Intern one generic argument binding list.
    pub(in crate::sema) fn intern_generic_arguments(
        &mut self,
        values: &[dir::GenericArgumentBinding],
    ) -> CompilerResult<dir::TypeListId> {
        Ok(self.module.types_tail.intern_generic_arguments(values))
    }

    /// Intern one index signature list into a module's working segment.
    pub(in crate::sema) fn intern_index_signatures(
        &mut self,
        values: &[dir::TypeIndexSignature],
    ) -> CompilerResult<dir::TypeListId> {
        Ok(self.module.types_tail.intern_index_signatures(values))
    }

    /// Intern one string list into a module's working segment.
    pub(in crate::sema) fn intern_strings(
        &mut self,
        values: &[StringId],
    ) -> CompilerResult<dir::TypeListId> {
        Ok(self.module.types_tail.intern_strings(values))
    }

    /// Return one type id list owned by a module.
    pub(in crate::sema) fn type_ids(
        &self,
        module: ModuleId,
        list: dir::TypeListId,
    ) -> CompilerResult<&'a [dir::GlobalTypeId]> {
        self.type_rows(
            module,
            list,
            |table| table.type_ids(list),
            |segment| segment.type_ids_maybe(list),
        )
    }

    /// Return one positional type id, or none past the list end.
    pub(in crate::sema) fn type_id_at(
        &self,
        module: ModuleId,
        list: dir::TypeListId,
        index: u32,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        Ok(self.type_ids(module, list)?.get(index as usize).copied())
    }

    /// Return one tuple element list owned by a module.
    pub(in crate::sema) fn tuple_elements(
        &self,
        module: ModuleId,
        list: dir::TypeListId,
    ) -> CompilerResult<&'a [dir::TypeElement]> {
        self.type_rows(
            module,
            list,
            |table| table.elements(list),
            |segment| segment.elements_maybe(list),
        )
    }

    /// Read the tuple element types, or none when an optional or rest element breaks positions.
    pub(in crate::sema) fn tuple_element_types(
        &self,
        value: dir::GlobalTypeId,
        list: dir::TypeListId,
    ) -> CompilerResult<Option<SmallVec<[dir::GlobalTypeId; 4]>>> {
        let mut types = SmallVec::new();
        for element in self.tuple_elements(value.module_id, list)? {
            if element.is_optional || element.is_rest {
                return Ok(None);
            }

            types.push(element.ty);
        }

        Ok(Some(types))
    }

    /// Return one function parameter list owned by a module.
    pub(in crate::sema) fn signature_parameters(
        &self,
        module: ModuleId,
        list: dir::TypeListId,
    ) -> CompilerResult<&'a [dir::FunctionParameterType]> {
        self.type_rows(
            module,
            list,
            |table| table.parameters(list),
            |segment| segment.parameters_maybe(list),
        )
    }

    /// Return one signature's applied generic arguments.
    pub(in crate::sema) fn signature_arguments(
        &self,
        module: ModuleId,
        list: dir::TypeListId,
    ) -> CompilerResult<&'a [dir::GenericArgumentBinding]> {
        self.type_rows(
            module,
            list,
            |table| table.generic_arguments(list),
            |segment| segment.generic_arguments_maybe(list),
        )
    }

    /// Return one index signature list owned by a module.
    pub(in crate::sema) fn object_index_signatures(
        &self,
        module: ModuleId,
        list: dir::TypeListId,
    ) -> CompilerResult<&'a [dir::TypeIndexSignature]> {
        self.type_rows(
            module,
            list,
            |table| table.index_signatures(list),
            |segment| segment.index_signatures_maybe(list),
        )
    }

    /// Return one string list owned by a module.
    pub(in crate::sema) fn template_strings(
        &self,
        module: ModuleId,
        list: dir::TypeListId,
    ) -> CompilerResult<&'a [StringId]> {
        self.type_rows(
            module,
            list,
            |table| table.strings(list),
            |segment| segment.strings_maybe(list),
        )
    }

    /// Settle one interned list through the owning module's tables.
    fn type_rows<T>(
        &self,
        module: ModuleId,
        list: dir::TypeListId,
        read_table: impl FnOnce(&'a dir::TypeTable<'static>) -> &'a [T],
        read_tail: impl FnOnce(&dir::TypeTail<'a>) -> Option<&'a [T]>,
    ) -> CompilerResult<&'a [T]> {
        // resolve overlay lists over the committed base table
        if self.is_own_module(module) {
            if list.is_empty() {
                return Ok(&[]);
            }

            if let Some(elements) = read_tail(&self.module.types_tail) {
                return Ok(elements);
            }

            return Ok(read_table(self.module.types));
        }

        // read external committed tables
        if let Some(external) = self.external(module)? {
            return Ok(read_table(external.types()));
        }

        Err(CompilerError::Internal {
            message: format!("check type list {list:?} belongs to an unloaded module {module:?}"),
        })
    }

    /// Intern one applied reference type for a language item.
    pub(in crate::sema) fn language_type(
        &mut self,
        item: dir::LanguageItem,
        arguments: &[dir::GlobalTypeId],
    ) -> CompilerResult<dir::GlobalTypeId> {
        let symbol = self.language_symbol(item)?;
        let arguments = self.intern_type_ids(arguments)?;
        let ty = dir::Type::Application(dir::GenericApplication { symbol, arguments });

        self.intern_type(ty)
    }

    /// Intern one singleton type for an exact property key.
    pub(in crate::sema) fn static_key_type(
        &mut self,
        key: dir::StaticKey,
    ) -> CompilerResult<dir::GlobalTypeId> {
        self.intern_type(dir::Type::Key(key))
    }

    /// Intern the type an optional member holds: its value or undefined.
    pub(in crate::sema) fn optional_type(
        &mut self,
        value: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let undefined = self.intern_type(dir::Type::Undefined)?;

        self.normalized_union_type([value, undefined])
    }

    /// Intern one open variable reference type in its origin module's working segment.
    pub(in crate::sema) fn variable_type(
        &mut self,
        variable: dir::TypeVariableId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        self.intern_type(dir::Type::Variable(variable))
    }

    /// Canonicalize every declared value symbol type onto the checked tail.
    pub(in crate::sema) fn canonicalize_declared_types(&mut self) -> CompilerResult<()> {
        let module = self.module_id;
        let Some(declared) = self.module(module).declared.clone() else {
            return Ok(());
        };

        // adopt each declared symbol type
        for (symbol, _) in declared.types.symbol_types() {
            // skip statically absent declarations
            if let Ok(source) = self.symbol_source(symbol)
                && self.is_absent(source)
            {
                continue;
            }

            self.adopt_symbol_type_maybe(symbol)?;
        }

        Ok(())
    }

    /// Return one member symbol's declaring owner and visibility.
    pub(in crate::sema) fn member_visibility(
        &mut self,
        member: dir::GlobalSymbolId,
    ) -> CompilerResult<Option<(dir::GlobalSymbolId, dir::Visibility)>> {
        // serve the memo
        if let Some(entry) = self.member_visibilities.get(&member) {
            return Ok(*entry);
        }

        // read the visibility off the declaring definition
        let mut entry = None;
        if let Some(owner) = self.member_owner(member)?
            && let Some(definition) = self.definition(owner)?
        {
            entry = definition
                .member_visibility(member)
                .map(|visibility| (owner, visibility));
        }
        self.member_visibilities.insert(member, entry);

        Ok(entry)
    }

    /// Return one definition, the checked module's working one or an external module's.
    pub(in crate::sema) fn definition(
        &self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<Option<Arc<dir::Definition>>> {
        // read the checked module's working definitions first
        if self.is_own_module(symbol.module_id) {
            return Ok(self.module.definition(symbol));
        }

        // read external committed definitions
        Ok(self
            .external(symbol.module_id)?
            .and_then(|external| external.definitions().definition_handle(symbol))
            .cloned())
    }

    /// Return the definition declaring one member symbol.
    pub(in crate::sema) fn member_owner(
        &self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<Option<dir::GlobalSymbolId>> {
        if self.is_own_module(symbol.module_id) {
            return Ok(self
                .module
                .definitions_tail
                .definition_by_member(symbol)
                .or_else(|| {
                    self.module
                        .definitions
                        .member(symbol)
                        .map(|(owner, _, _)| owner)
                }));
        }

        Ok(self
            .external(symbol.module_id)?
            .and_then(|external| external.definitions().member(symbol))
            .map(|(owner, _, _)| owner))
    }

    /// Return the members selected to satisfy one `implements` clause.
    pub(in crate::sema) fn conformance_members(
        &self,
        source: dir::GlobalNodeIdAny,
    ) -> CompilerResult<Vec<dir::MemberConformance>> {
        let members = match self.is_own_module(source.module_id) {
            true => self.module.conformance_members(source),
            false => self
                .external(source.module_id)?
                .and_then(|external| external.members().conformance_members(source)),
        };

        Ok(members.map(<[_]>::to_vec).unwrap_or_default())
    }

    /// Return the member one declaration selected for an interface requirement.
    pub(in crate::sema) fn conformance_member(
        &self,
        declaration: dir::GlobalSymbolId,
        requirement: dir::GlobalSymbolId,
    ) -> CompilerResult<Option<dir::GlobalSymbolId>> {
        let Some(definition) = self.definition(declaration)? else {
            return Ok(None);
        };
        for conformance in definition.implementations() {
            let selected = self
                .conformance_members(conformance.source)?
                .into_iter()
                .find(|selected| selected.requirement == requirement);
            if let Some(selected) = selected {
                return Ok(Some(selected.member));
            }
        }

        Ok(None)
    }

    /// Record the space one nominal declaration's instances live in, written or inherited.
    pub(in crate::sema) fn commit_nominal_space(
        &mut self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<()> {
        if let Some(space) = self.nominal_space(symbol)? {
            self.module_mut(symbol.module_id)
                .representations_tail
                .set_space(symbol, space);
        }

        Ok(())
    }

    /// Insert one checked definition into its module's working segment.
    pub(in crate::sema) fn insert_definition(
        &mut self,
        symbol: dir::GlobalSymbolId,
        source: dir::GlobalNodeIdAny,
        definition: dir::Definition,
    ) -> CompilerResult<()> {
        // keep checked segments appending, leaving unchanged declared entries layered
        if !self.is_declaring()
            && let Some(declared) = self
                .module_maybe(symbol.module_id)
                .and_then(|state| state.declared.as_ref())
            && declared.definitions.definition(symbol) == Some(&definition)
        {
            return Ok(());
        }

        // require the module the symbol declares in
        let working =
            self.module_maybe_mut(symbol.module_id)
                .ok_or_else(|| CompilerError::Internal {
                    message: format!(
                        "check module {:?} has no working definitions",
                        symbol.module_id
                    ),
                })?;

        // insert the definition into the pass segment
        working
            .definitions_tail
            .insert_definition(symbol, source, definition);

        Ok(())
    }

    /// Rebuild one type value by mapping every direct child type id.
    pub(in crate::sema) fn map_type_children(
        &mut self,
        source: ModuleId,
        ty: dir::Type,
        map: &mut impl FnMut(&mut Self, dir::GlobalTypeId) -> CompilerResult<dir::GlobalTypeId>,
    ) -> CompilerResult<dir::Type> {
        let ty = match ty {
            // leaf heads
            dir::Type::Variable(_)
            | dir::Type::Error
            | dir::Type::Never
            | dir::Type::Unknown
            | dir::Type::Void
            | dir::Type::Null
            | dir::Type::Undefined
            | dir::Type::Primitive(_)
            | dir::Type::Literal(_)
            | dir::Type::Key(_)
            | dir::Type::Static(_)
            | dir::Type::Intrinsic
            | dir::Type::Parameter(_)
            | dir::Type::Erased(_)
            | dir::Type::This
            | dir::Type::Range(_) => ty,

            // map explicit arguments of declaration references
            dir::Type::Reference(mut reference) => {
                reference.arguments = self.map_type_id_list(source, reference.arguments, map)?;

                dir::Type::Reference(reference)
            }

            // map the region extent and space
            dir::Type::Region(mut region) => {
                region.map_types(&mut |ty| map(self, ty))?;

                dir::Type::Region(region)
            }

            // declaration applications
            dir::Type::Application(mut instance) => {
                instance.arguments = self.map_type_id_list(source, instance.arguments, map)?;

                dir::Type::Application(instance)
            }
            dir::Type::Refined(refined) => {
                let mut refined = self.type_refined(source, refined)?;
                refined.map_types(&mut |ty| map(self, ty))?;
                let canonical =
                    self.intern_refinements(refined.base, &[(refined.key, refined.value)])?;

                self.ty(canonical)?
            }
            dir::Type::Member(member) => {
                let mut member = self.type_member(source, member)?;
                member.owner = map(self, member.owner)?;
                member.arguments = self.map_type_id_list(source, member.arguments, map)?;
                member.qualifier = member
                    .qualifier
                    .map(|qualifier| map(self, qualifier))
                    .transpose()?;
                let member = self.module.types_tail.intern_member(member);

                dir::Type::Member(member)
            }
            dir::Type::Variant(mut member) => {
                member.map_types(&mut |ty| map(self, ty))?;

                dir::Type::Variant(member)
            }

            // memory forms
            dir::Type::Form(mut form) => {
                form.value = map(self, form.value)?;
                match &mut form.form {
                    dir::Form::Borrowed(borrow) => {
                        let mut resolved = self.type_borrow(source, *borrow)?;
                        resolved.map_types(&mut |ty| map(self, ty))?;
                        *borrow = self.module.types_tail.intern_borrow(resolved);
                    }
                    dir::Form::Owned | dir::Form::Raw | dir::Form::Readonly => {}
                }
                dir::Type::Form(form)
            }
            dir::Type::Dynamic(mut dynamic) => {
                dynamic.map_types(&mut |ty| map(self, ty))?;

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
                        conditional.map_types(&mut |ty| map(self, ty))?;

                        dir::TypeOperation::Conditional(conditional)
                    }
                    dir::TypeOperation::Narrow(mut narrow) => {
                        narrow.map_types(&mut |ty| map(self, ty))?;

                        dir::TypeOperation::Narrow(narrow)
                    }
                    dir::TypeOperation::Mapped(mut mapped) => {
                        mapped.map_types(&mut |ty| map(self, ty))?;

                        dir::TypeOperation::Mapped(mapped)
                    }
                    dir::TypeOperation::Index(mut index) => {
                        index.map_types(&mut |ty| map(self, ty))?;

                        dir::TypeOperation::Index(index)
                    }
                    dir::TypeOperation::TemplateLiteral(mut template) => {
                        let strings = self.template_strings(source, template.strings)?;
                        template.strings = self.intern_strings(strings)?;
                        template.spans = self.map_type_id_list(source, template.spans, map)?;

                        dir::TypeOperation::TemplateLiteral(template)
                    }
                    dir::TypeOperation::Infer(mut infer) => {
                        infer.map_types(&mut |ty| map(self, ty))?;

                        dir::TypeOperation::Infer(infer)
                    }
                    dir::TypeOperation::TypeOf(query) => dir::TypeOperation::TypeOf(query),
                    dir::TypeOperation::Instantiation(mut application) => {
                        application.target = map(self, application.target)?;
                        application.arguments =
                            self.map_type_id_list(source, application.arguments, map)?;

                        dir::TypeOperation::Instantiation(application)
                    }
                    dir::TypeOperation::KeyOf(mut unary) => {
                        unary.map_types(&mut |ty| map(self, ty))?;

                        dir::TypeOperation::KeyOf(unary)
                    }
                    dir::TypeOperation::NoInfer(mut unary) => {
                        unary.map_types(&mut |ty| map(self, ty))?;

                        dir::TypeOperation::NoInfer(unary)
                    }
                    dir::TypeOperation::Awaited(mut unary) => {
                        unary.map_types(&mut |ty| map(self, ty))?;

                        dir::TypeOperation::Awaited(unary)
                    }
                    dir::TypeOperation::SpaceOf(mut unary) => {
                        unary.map_types(&mut |ty| map(self, ty))?;

                        dir::TypeOperation::SpaceOf(unary)
                    }
                    dir::TypeOperation::TryOutput { value } => dir::TypeOperation::TryOutput {
                        value: map(self, value)?,
                    },
                    dir::TypeOperation::TryResidual { value } => dir::TypeOperation::TryResidual {
                        value: map(self, value)?,
                    },
                    dir::TypeOperation::TryFailure { value } => dir::TypeOperation::TryFailure {
                        value: map(self, value)?,
                    },
                    dir::TypeOperation::StaticBinary(mut binary) => {
                        binary.map_types(&mut |ty| map(self, ty))?;

                        dir::TypeOperation::StaticBinary(binary)
                    }
                    dir::TypeOperation::StaticUnary(mut unary) => {
                        unary.map_types(&mut |ty| map(self, ty))?;

                        dir::TypeOperation::StaticUnary(unary)
                    }
                };
                let operation = self.module.types_tail.intern_operation(operation);

                dir::Type::Operation(operation)
            }

            // collections
            dir::Type::FixedArray(mut array) => {
                array.map_types(&mut |ty| map(self, ty))?;

                dir::Type::FixedArray(array)
            }
            dir::Type::Slice(mut slice) => {
                slice.map_types(&mut |ty| map(self, ty))?;

                dir::Type::Slice(slice)
            }
            dir::Type::Tuple(mut tuple) => {
                let mut elements = SmallVec::<[dir::TypeElement; 8]>::from_slice(
                    self.tuple_elements(source, tuple.elements)?,
                );
                elements.map_types(&mut |ty| map(self, ty))?;
                tuple.elements = self.intern_elements(&elements)?;

                dir::Type::Tuple(tuple)
            }

            // concrete object classes
            dir::Type::Object(shape) => {
                let shape = self.map_shape(source, shape, map)?;

                dir::Type::Object(shape)
            }
            dir::Type::FunctionSignature(function) => {
                let mut function = self.type_signature(source, function)?;
                let mut arguments: SmallVec<[_; 4]> =
                    self.signature_arguments(source, function.arguments)?.into();
                arguments.map_types(&mut |ty| map(self, ty))?;
                function.arguments = self.intern_generic_arguments(&arguments)?;

                if let Some(this_parameter) = &mut function.this_parameter {
                    *this_parameter = map(self, *this_parameter)?;
                }

                let mut parameters = SmallVec::<[dir::FunctionParameterType; 8]>::from_slice(
                    self.signature_parameters(source, function.parameters)?,
                );
                parameters.map_types(&mut |ty| map(self, ty))?;
                function.parameters = self.intern_parameters(&parameters)?;

                if let Some(return_type) = &mut function.return_type {
                    *return_type = map(self, *return_type)?;
                }
                let function = self.module.types_tail.intern_signature(function);

                dir::Type::FunctionSignature(function)
            }
            dir::Type::Function(mut function) => {
                function.map_types(&mut |ty| map(self, ty))?;

                dir::Type::Function(function)
            }
            dir::Type::FunctionPointer(mut function) => {
                function.map_types(&mut |ty| map(self, ty))?;

                dir::Type::FunctionPointer(function)
            }

            // algebraic composites
            dir::Type::Union(mut union) => {
                union.elements = self.map_type_id_list(source, union.elements, map)?;

                dir::Type::Union(union)
            }
            dir::Type::Intersection(mut intersection) => {
                intersection.elements =
                    self.map_type_id_list(source, intersection.elements, map)?;

                dir::Type::Intersection(intersection)
            }
        };

        Ok(ty)
    }

    /// Map one shape's embedded type ids from one module into another.
    fn map_shape(
        &mut self,
        source: ModuleId,
        mut shape: dir::ObjectType,
        map: &mut impl FnMut(&mut Self, dir::GlobalTypeId) -> CompilerResult<dir::GlobalTypeId>,
    ) -> CompilerResult<dir::ObjectType> {
        // map each property's read and write types
        let mut properties = SmallVec::<[dir::TypeProperty; 8]>::from_slice(
            self.object_properties(source, shape.properties)?,
        );
        properties.map_types(&mut |ty| map(self, ty))?;

        // map the signature lists as they stand
        shape.properties = self.intern_properties(&properties)?;
        shape.call_signatures = self.map_type_id_list(source, shape.call_signatures, map)?;
        shape.construct_signatures =
            self.map_type_id_list(source, shape.construct_signatures, map)?;

        // map each index signature's key and value types
        let mut signatures = SmallVec::<[dir::TypeIndexSignature; 2]>::from_slice(
            self.object_index_signatures(source, shape.index_signatures)?,
        );
        signatures.map_types(&mut |ty| map(self, ty))?;

        shape.index_signatures = self.intern_index_signatures(&signatures)?;

        Ok(shape)
    }

    /// Map one interned type id list from one module into another.
    fn map_type_id_list(
        &mut self,
        source: ModuleId,
        list: dir::TypeListId,
        map: &mut impl FnMut(&mut Self, dir::GlobalTypeId) -> CompilerResult<dir::GlobalTypeId>,
    ) -> CompilerResult<dir::TypeListId> {
        let mut ids = SmallVec::<[dir::GlobalTypeId; 8]>::from_slice(self.type_ids(source, list)?);
        ids.map_types(&mut |ty| map(self, ty))?;

        self.intern_type_ids(&ids)
    }

    /// Return the nominal application beneath refinements.
    pub(in crate::sema) fn nominal_application(
        &self,
        id: dir::GlobalTypeId,
    ) -> CompilerResult<(ModuleId, dir::GenericApplication)> {
        self.nominal_application_maybe(id)?
            .ok_or_else(|| CompilerError::Internal {
                message: format!("type {id:?} has no nominal application"),
            })
    }

    /// Return the nominal application beneath refinements, if present.
    pub(in crate::sema) fn nominal_application_maybe(
        &self,
        mut id: dir::GlobalTypeId,
    ) -> CompilerResult<Option<(ModuleId, dir::GenericApplication)>> {
        loop {
            match self.ty(id)? {
                dir::Type::Refined(refined) => {
                    id = self.type_refined(id.module_id, refined)?.base;
                }
                dir::Type::Application(application) => {
                    return Ok(Some((id.module_id, application)));
                }
                _ => return Ok(None),
            }
        }
    }

    /// Return one shape property list owned by a module.
    pub(in crate::sema) fn object_properties(
        &self,
        module: ModuleId,
        list: dir::TypeListId,
    ) -> CompilerResult<&'a [dir::TypeProperty]> {
        self.type_rows(
            module,
            list,
            |table| table.properties(list),
            |segment| segment.properties_maybe(list),
        )
    }

    /// Intern one shape property list into a module's working segment.
    pub(in crate::sema) fn intern_properties(
        &mut self,
        values: &[dir::TypeProperty],
    ) -> CompilerResult<dir::TypeListId> {
        Ok(self.module.types_tail.intern_properties(values))
    }

    /// Intern one anonymous object type over a property list.
    pub(in crate::sema) fn intern_object(
        &mut self,
        properties: &[dir::TypeProperty],
    ) -> CompilerResult<dir::GlobalTypeId> {
        let properties = self.intern_properties(properties)?;

        // intern the properties as one object shape
        self.intern_type(dir::Type::Object(dir::ObjectType {
            properties,
            call_signatures: dir::TypeListId::EMPTY,
            construct_signatures: dir::TypeListId::EMPTY,
            index_signatures: dir::TypeListId::EMPTY,
        }))
    }

    /// Intern associated bindings around one base type.
    pub(in crate::sema) fn intern_refinements(
        &mut self,
        base: dir::GlobalTypeId,
        new_bindings: &[(dir::StaticKey, dir::GlobalTypeId)],
    ) -> CompilerResult<dir::GlobalTypeId> {
        // merge the bindings over the base's own refinements, the new binding of a key winning
        let (base, mut bindings) = self.refinements(base)?;
        for (key, value) in new_bindings {
            bindings.retain(|(existing, _)| existing != key);
            bindings.push((*key, *value));
        }

        // sort the bindings, so equal refinement sets intern identically
        bindings.sort_by_key(|(key, _)| *key);
        let mut ty = base;

        // build one canonical refinement chain
        for (key, value) in bindings {
            ty = self.intern_refined(dir::RefinedType {
                base: ty,
                key,
                value,
            })?;
        }

        Ok(ty)
    }

    /// Return one refined type's base with its associated refinements.
    pub(in crate::sema) fn refinements(
        &self,
        mut id: dir::GlobalTypeId,
    ) -> CompilerResult<(
        dir::GlobalTypeId,
        SmallVec<[(dir::StaticKey, dir::GlobalTypeId); 2]>,
    )> {
        let mut bindings = SmallVec::new();

        // peel the canonical refinement chain
        while let dir::Type::Refined(refined) = self.ty(id)? {
            let refined = self.type_refined(id.module_id, refined)?;
            bindings.push((refined.key, refined.value));
            id = refined.base;
        }

        // sort the bindings into canonical key order
        bindings.sort_by_key(|(key, _)| *key);

        Ok((id, bindings))
    }
}
