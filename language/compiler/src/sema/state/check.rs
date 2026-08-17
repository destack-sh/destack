use std::sync::Arc;

use destack_artifact::{
    DirBound, DirChecked, DirDeclared, DirElaborated, DirExpanded, DirParsed, DirResolved,
    EnvironmentBound, EnvironmentDeclared,
};
use destack_core::{FxIndexMap, FxIndexSet, StringPool};
use destack_dir as dir;
use destack_repository::{ArtifactAttemptRecorder, ArtifactReader, Environment, ProviderContext};
use destack_source::{ModuleId, ProfileId, StringId};
use rustc_hash::FxHashMap;
use smallvec::SmallVec;

use crate::sema::{
    Cause, CauseId, CheckCounters, CheckModuleState, CheckTrace, DecoratorApplication,
    ExternalModuleTable, FlowBranch, FlowState, Fulfillment, FunctionBody, GenericParameterId,
    HeritageReach, InducedParameterSite, InferContext, MemberSubject, MemberTable, Origin,
    OriginId, Relation, RelationKey, Scope, Selection, SelectionKey, VarianceForm, VarianceState,
    Verdict, should_stream_check_events,
};
use crate::{Compiler, CompilerError, CompilerResult};

/// One solving pass over a module.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::sema) enum Pass {
    /// Declare the module's own interface from source.
    Declare,
    /// Flatten declared owners into stored member bindings.
    Elaborate,
    /// Infer the module's bodies.
    Check,
    /// Close the module's instances and evaluate settled types.
    Materialize,
}

/// Committed node types in dense module columns, rolled back under open probes.
#[derive(Debug, Default)]
pub(in crate::sema) struct NodeTable {
    /// One dense column per module, indexed by tree-global node id.
    columns: FxHashMap<ModuleId, Vec<Option<(dir::NodeType, dir::GlobalTypeId)>>>,
    /// Entries replaced while a probe journals, with their priors.
    journal: Vec<(dir::GlobalNodeIdAny, Option<dir::GlobalTypeId>)>,
    /// The open probe depth: positive depths journal inserts.
    probes: usize,
}

impl NodeTable {
    /// Return one committed node type.
    pub(in crate::sema) fn get(&self, node: &dir::GlobalNodeIdAny) -> Option<dir::GlobalTypeId> {
        let column = self.columns.get(&node.module_id)?;
        let (tag, ty) = column.get(node.local_id.id as usize).copied().flatten()?;

        (tag == node.local_id.ty).then_some(ty)
    }

    /// Return whether one node committed a type.
    pub(in crate::sema) fn contains(&self, node: dir::GlobalNodeIdAny) -> bool {
        self.get(&node).is_some()
    }

    /// Commit one node type, journaling the prior under open probes.
    pub(in crate::sema) fn insert(&mut self, node: dir::GlobalNodeIdAny, ty: dir::GlobalTypeId) {
        let column = self.columns.entry(node.module_id).or_default();
        let index = node.local_id.id as usize;
        if column.len() <= index {
            column.resize_with(index + 1, || None);
        }

        // journal the replaced entry while a probe may roll back
        if self.probes > 0 {
            let prior = column[index].and_then(|(tag, ty)| (tag == node.local_id.ty).then_some(ty));
            self.journal.push((node, prior));
        }

        column[index] = Some((node.local_id.ty, ty));
    }

    /// Collect every committed node id.
    pub(in crate::sema) fn nodes(&self) -> Vec<dir::GlobalNodeIdAny> {
        let mut nodes = Vec::new();
        for (module, column) in &self.columns {
            for (index, entry) in column.iter().enumerate() {
                if let Some((tag, _)) = entry {
                    nodes.push(dir::GlobalNodeIdAny {
                        module_id: *module,
                        local_id: dir::LocalNodeIdAny::new(index as u32, *tag),
                    });
                }
            }
        }

        nodes
    }

    /// Mark the probe journal and start journaling inserts.
    pub(in crate::sema) fn open_probe(&mut self) -> usize {
        self.probes += 1;

        self.journal.len()
    }

    /// Roll journaled inserts back to one probe mark.
    pub(in crate::sema) fn close_probe(&mut self, mark: usize) {
        while self.journal.len() > mark {
            let (node, prior) = self.journal.pop().expect("journaled probe entry");
            let column = self
                .columns
                .get_mut(&node.module_id)
                .expect("journaled column");
            column[node.local_id.id as usize] = prior.map(|ty| (node.local_id.ty, ty));
        }

        self.probes -= 1;
    }
}

/// State for checking one resolved module.
pub(in crate::sema) struct CheckState<'a> {
    // context
    /// The compiler running this check attempt.
    pub(in crate::sema) compiler: &'a Compiler,
    /// The provider context that owns artifact reads and diagnostics.
    pub(in crate::sema) context: &'a dyn ProviderContext,
    /// The provider's trace recorder, absent outside traced runs.
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

    // self
    /// The module being declared or checked.
    pub(in crate::sema) module_id: ModuleId,
    /// The module's working state.
    pub(in crate::sema) module: CheckModuleState,
    /// The pass this state solves.
    pub(in crate::sema) pass: Pass,

    // externals
    /// Loaded external module states keyed by module id.
    pub(in crate::sema) external_modules: ExternalModuleTable,
    /// Resolved import targets of external modules read for alias hops.
    pub(in crate::sema) external_resolved: FxIndexMap<ModuleId, Arc<DirResolved>>,
    /// Canonical own-module forms of imported symbol types.
    pub(in crate::sema) imported_types: FxIndexMap<dir::GlobalSymbolId, dir::GlobalTypeId>,

    // decisions
    /// Decided relations between closed type pairs.
    pub(in crate::sema) relates: FxIndexMap<RelationKey, bool>,
    /// Canonical member bindings per owner and space.
    pub(in crate::sema) bindings:
        FxIndexMap<(dir::GlobalSymbolId, dir::MemberSpace), Option<Arc<Vec<dir::MemberBinding>>>>,
    /// Extension selections of settled goals, replaying the winning implementation on later hits.
    pub(in crate::sema) extensions: FxIndexMap<
        (Relation, dir::GlobalTypeId, dir::GlobalTypeId, Scope),
        (Verdict, Option<dir::GlobalSymbolId>),
    >,
    /// Work counters for the stats sidecar.
    pub(in crate::sema) counters: CheckCounters,
    /// Normalized heads of closed types.
    pub(in crate::sema) normalizations: FxIndexMap<(dir::GlobalTypeId, Scope), dir::GlobalTypeId>,
    /// Barrier-erased forms of closed contextual targets.
    pub(in crate::sema) erasures: FxIndexMap<dir::GlobalTypeId, dir::GlobalTypeId>,
    /// Extension member tables of closed subjects, grouped by key.
    pub(in crate::sema) members: FxIndexMap<MemberSubject, MemberTable>,
    /// Whether writeback is settling declared-form member bindings.
    pub(in crate::sema) settling: bool,
    /// Decided auto interface conformances of closed types.
    pub(in crate::sema) conforms: FxIndexMap<(dir::GlobalTypeId, dir::AutoInterface, Scope), bool>,
    /// Declarations reached by each declaration's heritage.
    pub(in crate::sema) heritages: FxIndexMap<dir::GlobalSymbolId, HeritageReach>,
    /// Active derivability goals closed coinductively on re-entry.
    pub(in crate::sema) deriving: FxIndexSet<(dir::GlobalTypeId, dir::AutoInterface)>,
    /// Active extension member lookups closed coinductively on re-entry.
    pub(in crate::sema) extending: FxIndexSet<(dir::GlobalSymbolId, dir::GlobalTypeId)>,
    /// Storable representations proved this pass, keyed by assuming scope.
    pub(in crate::sema) storable: FxIndexSet<(dir::GlobalTypeId, Scope)>,
    /// Selected and instantiated callables keyed by callee and operand types.
    pub(in crate::sema) selections: FxIndexMap<SelectionKey, Selection>,
    /// Derived parameter variances per handle form, with in-flight marks.
    pub(in crate::sema) variances:
        FxIndexMap<(dir::GlobalGenericParameterId, VarianceForm), VarianceState>,

    /// The module's transient inference state.
    pub(in crate::sema) infer: InferContext,
    /// The fulfillment queue driving pending work to verdicts.
    pub(in crate::sema) fulfill: Fulfillment,

    // walk state
    /// Resolved decorators in module walk order.
    pub(in crate::sema) decorators: Vec<DecoratorApplication>,
    /// The authored decorators this pass's walk already visited.
    pub(in crate::sema) walked_decorators: FxIndexSet<dir::LocalNodeId<dir::Decorator>>,
    /// Declaration types scanned for induced memory variables.
    pub(in crate::sema) induced_parameter_sites: Vec<InducedParameterSite>,
    /// Induced parameters already rebound to their sites this run.
    pub(in crate::sema) claimed_induced: FxIndexSet<GenericParameterId>,

    // checked state
    /// Stable declaration symbol types.
    pub(in crate::sema) declaration_types: FxIndexMap<dir::GlobalSymbolId, dir::GlobalTypeId>,
    /// Body-owned binding symbol types.
    pub(in crate::sema) binding_types: FxIndexMap<dir::GlobalSymbolId, dir::GlobalTypeId>,
    /// Checked source node occurrence types.
    pub(in crate::sema) node_types: NodeTable,
    /// Flow cursor state for the pass's single body traversal.
    pub(in crate::sema) flow: FlowState,
    /// Constructor exit branches per initialized class, filled at check.
    pub(in crate::sema) constructor_branches: FxIndexMap<dir::GlobalSymbolId, Vec<FlowBranch>>,
    /// Contextual expected types by source node occurrence.
    pub(in crate::sema) expected_types: NodeTable,

    // walk scheduling
    /// Declarations already walked, when asked or in root order.
    pub(in crate::sema) walked_declarations: FxIndexSet<dir::GlobalNodeIdAny>,
    /// Declarations currently walking, innermost last.
    pub(in crate::sema) walking_declarations: Vec<dir::GlobalNodeIdAny>,
    /// Extension applicability goals currently deciding, for cycle breaking.
    pub(in crate::sema) deciding_extensions:
        FxIndexSet<(Relation, dir::GlobalTypeId, dir::GlobalTypeId)>,

    // body driver state
    /// Named function bodies keyed by their declaration symbol.
    pub(in crate::sema) functions: FxIndexMap<dir::GlobalSymbolId, FunctionBody>,
    /// Member block bodies discovered while checking, in discovery order.
    pub(in crate::sema) blocks: Vec<dir::GlobalNodeIdAny>,
    /// Lambda bodies keyed by their value expression.
    pub(in crate::sema) lambdas: FxIndexMap<dir::GlobalNodeIdAny, FunctionBody>,

    // tracing
    /// Retained trace state, present only when tracing is requested.
    pub(in crate::sema) trace: Option<Box<CheckTrace>>,
}

impl<'a> CheckState<'a> {
    /// Return the shared repository string pool.
    pub(in crate::sema) fn strings(&self) -> &'a StringPool {
        self.compiler.repository.string_pool()
    }

    /// Create a module check state.
    pub(in crate::sema) fn new(
        compiler: &'a Compiler,
        context: &'a dyn ProviderContext,
        artifacts: &'a ArtifactReader<'a>,
        profile: ProfileId,
        environment_bound: Arc<EnvironmentBound>,
        environment_declared: Option<Arc<EnvironmentDeclared>>,
        environment: Arc<Environment>,
        module_id: ModuleId,
        pass: Pass,
        emit_events: bool,
    ) -> CompilerResult<Self> {
        // load the module's working state from its committed artifacts
        let profile_key = compiler.profile(context.revision(), profile)?.key;
        let repository_module = compiler.module(context.revision(), module_id)?;
        let package = compiler.package(context.revision(), repository_module.package_id)?;
        let parsed = artifacts
            .read::<DirParsed>(module_id)
            .map_err(CompilerError::from)?;
        let bound = artifacts
            .read::<DirBound>((module_id, profile))
            .map_err(CompilerError::from)?;
        let resolved = artifacts
            .read::<DirResolved>((module_id, profile))
            .map_err(CompilerError::from)?;
        let expanded = artifacts
            .read::<DirExpanded>((module_id, profile))
            .map_err(CompilerError::from)?;

        // seed later passes from the module's own committed artifacts
        let declared = (pass != Pass::Declare)
            .then(|| artifacts.read::<DirDeclared>((module_id, profile)))
            .transpose()
            .map_err(CompilerError::from)?;
        let elaborated = matches!(pass, Pass::Check | Pass::Materialize)
            .then(|| artifacts.read::<DirElaborated>((module_id, profile)))
            .transpose()
            .map_err(CompilerError::from)?;
        let checked = (pass == Pass::Materialize)
            .then(|| artifacts.read::<DirChecked>((module_id, profile)))
            .transpose()
            .map_err(CompilerError::from)?;

        // assemble the module's working tables over those artifacts
        let module = CheckModuleState::new(
            repository_module,
            package,
            profile_key,
            parsed,
            bound,
            resolved,
            Arc::clone(&expanded),
            declared,
            elaborated,
            checked,
        );

        // open every decision, inference, and trace table empty
        let state = Self {
            compiler,
            context,
            recorder: context.recorder(),
            artifacts,
            profile,
            environment_bound,
            environment_declared,
            environment,
            module_id,
            module,
            external_modules: ExternalModuleTable::default(),
            external_resolved: FxIndexMap::default(),
            pass,
            relates: FxIndexMap::default(),
            bindings: FxIndexMap::default(),
            extensions: FxIndexMap::default(),
            counters: CheckCounters::default(),
            normalizations: FxIndexMap::default(),
            erasures: FxIndexMap::default(),
            members: FxIndexMap::default(),
            settling: false,
            conforms: FxIndexMap::default(),
            heritages: FxIndexMap::default(),
            deriving: FxIndexSet::default(),
            extending: FxIndexSet::default(),
            storable: FxIndexSet::default(),
            selections: FxIndexMap::default(),
            variances: FxIndexMap::default(),
            imported_types: FxIndexMap::default(),
            infer: InferContext::new(),
            fulfill: Fulfillment::new(),
            decorators: Vec::new(),
            walked_decorators: FxIndexSet::default(),
            induced_parameter_sites: Vec::new(),
            claimed_induced: FxIndexSet::default(),
            declaration_types: FxIndexMap::default(),
            binding_types: FxIndexMap::default(),
            node_types: NodeTable::default(),
            flow: FlowState::default(),
            constructor_branches: FxIndexMap::default(),
            expected_types: NodeTable::default(),
            walked_declarations: FxIndexSet::default(),
            walking_declarations: Vec::new(),
            deciding_extensions: FxIndexSet::default(),
            functions: FxIndexMap::default(),
            blocks: Vec::new(),
            lambdas: FxIndexMap::default(),
            trace: CheckTrace::new(emit_events, should_stream_check_events()),
        };

        Ok(state)
    }

    /// Return whether this check infers one member's bodies.
    pub(in crate::sema) fn infers_module(&self, module: ModuleId) -> bool {
        self.pass == Pass::Check && module == self.module_id
    }

    /// Return whether this pass declares the module's own interface.
    pub(in crate::sema) fn is_declaration(&self) -> bool {
        self.pass == Pass::Declare
    }

    /// Return whether this pass infers the module's bodies.
    pub(in crate::sema) fn is_checking(&self) -> bool {
        self.pass == Pass::Check
    }

    /// Bind the error type to every exported binding without a derived type.
    pub(in crate::sema) fn bind_underivable_exports(&mut self) -> CompilerResult<()> {
        // view the module tree with its expansion patches
        let module = self.module_id;
        let input = self.module(module);
        let parsed = input.parsed.clone();
        let expanded = input.expanded.clone();
        let tree = dir::View::with_patches(&parsed.tree, std::slice::from_ref(&expanded.patch));

        // collect every exported module-scope declarator
        let mut exported = Vec::new();
        for root in &expanded.roots {
            self.collect_exported_declarators(module, tree, *root, &mut exported);
        }

        // report and bind the error type where derivation failed
        for (declarator, symbol) in exported {
            if self.symbol_type_maybe(symbol).is_some() {
                continue;
            }

            // skip statically absent exports
            if self.is_absent(declarator.into_global(module)) {
                continue;
            }

            // report the failure once, while declaring
            if self.is_declaration() {
                self.report_export_type_not_derivable(module, declarator);
            }

            let error = self.intern_type(dir::Type::Error)?;
            self.bind_symbol_type(symbol, error)?;
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

            // skip statements without exports
            _ => {}
        }
    }

    /// Walk the loaded module.
    pub(in crate::sema) fn walk(&mut self) -> CompilerResult<()> {
        let module = self.module_id;

        // import external declared modules
        self.import_external_modules()?;

        // checking walks bodies against the declared entries
        if self.is_checking() {
            self.canonicalize_declared_types()?;

            return self.walk_module_bodies(module);
        }

        // declare template identities before walking their bounds
        self.declare_module_templates(module)?;
        self.walk_module_templates(module)?;

        self.walk_module(module)
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

    /// Return the language item named by one resolved symbol.
    pub(in crate::sema) fn language_item(
        &mut self,
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
            dir::Type::Reference(reference) => Some(reference.symbol),
            dir::Type::Application(instance) => Some(instance.symbol),
            _ => None,
        };

        Ok(symbol)
    }
}

impl CheckState<'_> {
    /// Return one type from this module's open overlay or external tables.
    pub(in crate::sema) fn ty(&self, id: dir::GlobalTypeId) -> CompilerResult<dir::Type> {
        // read this module's open working types
        if self.is_own_module(id.module_id) {
            self.type_maybe(id.local_id)
                .ok_or_else(|| CompilerError::Internal {
                    message: format!("check type {id:?} is not allocated"),
                })
        }
        // read external committed tables
        else if let Some(external) = self.external_modules.get(&id.module_id) {
            Ok(external.types.get_type(id.local_id))
        }
        // fail loudly on a module this check never loaded
        else {
            Err(CompilerError::Internal {
                message: format!("check type {id:?} belongs to an unloaded module"),
            })
        }
    }

    /// Return whether any operand already reported an error.
    pub(in crate::sema) fn any_error_operand(
        &self,
        operands: &[dir::GlobalTypeId],
    ) -> CompilerResult<bool> {
        for operand in operands {
            if self.type_flags(*operand)?.has_error() {
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
        else if let Some(external) = self.external_modules.get(&id.module_id) {
            Ok(external.types.get_type_flags(id.local_id))
        }
        // fail loudly on a module this check never loaded
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
    pub(in crate::sema) fn intern_type(
        &mut self,
        ty: dir::Type,
    ) -> CompilerResult<dir::GlobalTypeId> {
        self.counters.interns += 1;
        let module = self.module_id;

        // memory forms intern in one canonical composition order
        let ty = self.canonical_form_type(ty)?;

        // join the structural flags of every child type
        let mut children = SmallVec::<[dir::GlobalTypeId; 8]>::new();
        self.for_each_type_child(module, &ty, |child| children.push(child))?;
        let mut child_flags = dir::TypeFlags::EMPTY;
        for child in &children {
            child_flags |= self.type_flags(*child)?;
        }

        // record the foreign modules this type mentions as it stores
        for child in &children {
            if child.module_id != module {
                self.module.references.insert(child.module_id);
            }
        }

        let mut mentions = SmallVec::<[ModuleId; 2]>::new();
        ty.referenced_modules(&mut |mentioned| mentions.push(mentioned));
        for mentioned in mentions {
            if mentioned != module {
                self.module.references.insert(mentioned);
            }
        }

        // operation payloads contribute their own symbolic flags
        if let dir::Type::Operation(operation) = ty {
            child_flags |= self.type_operation(module, operation)?.own_flags();
        }

        // keep written alias and collection applications intact, normalizing them lazily
        let is_written_alias = match &ty {
            dir::Type::Application(instance) => {
                matches!(
                    self.definition(instance.symbol)?,
                    Some(dir::Definition::TypeAlias(_))
                ) || matches!(
                    self.language_item(instance.symbol)?,
                    Some(
                        dir::LanguageItem::Array
                            | dir::LanguageItem::Slice
                            | dir::LanguageItem::FixedArray
                            | dir::LanguageItem::Dynamic
                    )
                )
            }
            _ => false,
        };

        // store the type in this module's working tail
        let (local, inserted) = self.module.types_tail.intern_type_inserted(ty, child_flags);
        let id = local.into_global(module);

        // settle each newly born closed head once declarations can load
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
    pub(in crate::sema) fn type_borrow(
        &self,
        module: ModuleId,
        id: dir::BorrowFormId,
    ) -> CompilerResult<dir::BorrowForm> {
        if self.is_own_module(module) {
            self.borrow_maybe(id)
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
    pub(in crate::sema) fn intern_borrow(
        &mut self,
        lifetime: dir::GlobalTypeId,
        access: dir::GlobalTypeId,
    ) -> CompilerResult<dir::Form> {
        let id = self
            .module
            .types_tail
            .intern_borrow(dir::BorrowForm { lifetime, access });

        Ok(dir::Form::Borrowed(id))
    }

    /// Adopt one memory form's module-local borrow entry into this module.
    pub(in crate::sema) fn adopt_form(
        &mut self,
        source: ModuleId,
        form: dir::Form,
    ) -> CompilerResult<dir::Form> {
        match form {
            dir::Form::Borrowed(id) if source != self.module_id => {
                let borrow = self.type_borrow(source, id)?;

                self.intern_borrow(borrow.lifetime, borrow.access)
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
    pub(in crate::sema) fn signature_head(
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

    /// Intern one function parameter list into a module's working segment.
    pub(in crate::sema) fn intern_parameters(
        &mut self,
        values: &[dir::FunctionParameterType],
    ) -> CompilerResult<dir::TypeListId> {
        Ok(self.module.types_tail.intern_parameters(values))
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
    ) -> CompilerResult<&[dir::GlobalTypeId]> {
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
    ) -> CompilerResult<&[dir::TypeElement]> {
        self.type_rows(
            module,
            list,
            |table| table.elements(list),
            |segment| segment.elements_maybe(list),
        )
    }

    /// Return one function parameter list owned by a module.
    pub(in crate::sema) fn signature_parameters(
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
    pub(in crate::sema) fn shape_index_signatures(
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
    pub(in crate::sema) fn template_strings(
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
        if self.is_own_module(module) {
            if list.is_empty() {
                return Ok(&[]);
            }

            if let Some(elements) = read_segment(&self.module.types_tail) {
                return Ok(elements);
            }

            return Ok(read_table(&self.module.types));
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

    /// Intern the type read from an optional index signature.
    pub(in crate::sema) fn index_signature_read_type(
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

        for (symbol, _) in declared.types.symbol_types() {
            // skip statically absent declarations
            if let Ok(source) = self.symbol_source(symbol)
                && self.is_absent(source)
            {
                continue;
            }

            self.canonical_symbol_type_maybe(symbol)?;
        }

        Ok(())
    }

    /// Return one definition, importing the symbol's module as needed.
    pub(in crate::sema) fn definition(
        &mut self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<Option<&dir::Definition>> {
        // unloaded foreign definitions stay symbolic
        if !self.is_own_module(symbol.module_id) {
            self.import_external_module(symbol.module_id)?;
        }

        Ok(self.definition_maybe(symbol))
    }

    /// Return one already loaded definition, without importing.
    pub(in crate::sema) fn definition_maybe(
        &self,
        symbol: dir::GlobalSymbolId,
    ) -> Option<&dir::Definition> {
        // read the checked module's working definitions first
        if let Some(module) = self.module_maybe(symbol.module_id)
            && let Some(definition) = module.definition(symbol)
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
    pub(in crate::sema) fn insert_definition(
        &mut self,
        symbol: dir::GlobalSymbolId,
        source: dir::GlobalNodeIdAny,
        definition: dir::Definition,
    ) -> CompilerResult<()> {
        self.report_duplicate_definition_members(&definition);

        // keep checked segments append-grow: unchanged declared entries stay layered
        if !self.is_declaration()
            && let Some(declared) = self
                .module_maybe(symbol.module_id)
                .and_then(|state| state.declared.as_ref())
            && declared.definitions.definition(symbol) == Some(&definition)
        {
            return Ok(());
        }

        let working =
            self.module_maybe_mut(symbol.module_id)
                .ok_or_else(|| CompilerError::Internal {
                    message: format!(
                        "check module {:?} has no working definitions",
                        symbol.module_id
                    ),
                })?;

        working
            .definitions_tail
            .insert_definition(symbol, source, definition);

        Ok(())
    }

    /// Return one definition for mutation.
    pub(in crate::sema) fn definition_mut(
        &mut self,
        symbol: dir::GlobalSymbolId,
    ) -> Option<&mut dir::Definition> {
        self.module_maybe_mut(symbol.module_id)?
            .definition_mut(symbol)
    }

    /// Commit one nominal declaration's solved space.
    pub(in crate::sema) fn commit_nominal_space(
        &mut self,
        symbol: dir::GlobalSymbolId,
        space: dir::Space,
    ) -> CompilerResult<()> {
        let module =
            self.module_maybe_mut(symbol.module_id)
                .ok_or_else(|| CompilerError::Internal {
                    message: format!("nominal declaration {symbol:?} is not in the checked module"),
                })?;
        let definition = module
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
    pub(in crate::sema) fn map_type_children(
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
                let refined = self.module.types_tail.intern_refined(refined);

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
                let member = self.module.types_tail.intern_member(member);

                dir::Type::Member(member)
            }
            dir::Type::Variant(mut member) => {
                member.owner = map(self, member.owner)?;

                dir::Type::Variant(member)
            }

            // memory forms
            dir::Type::Form(mut form) => {
                form.value = map(self, form.value)?;
                match &mut form.form {
                    dir::Form::Borrowed(borrow) => {
                        let mut resolved = self.type_borrow(source, *borrow)?;
                        resolved.lifetime = map(self, resolved.lifetime)?;
                        resolved.access = map(self, resolved.access)?;
                        *borrow = self.module.types_tail.intern_borrow(resolved);
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
                        template.strings = self.intern_strings(&strings)?;
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
                let operation = self.module.types_tail.intern_operation(operation);

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
                tuple.elements = self.intern_elements(&elements)?;

                dir::Type::Tuple(tuple)
            }

            // concrete object classes
            dir::Type::Object(shape) => {
                let shape = self.map_shape(source, target, shape, map)?;

                dir::Type::Object(shape)
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
                function.parameters = self.intern_parameters(&parameters)?;

                if let Some(return_type) = &mut function.return_type {
                    *return_type = map(self, *return_type)?;
                }
                let function = self.module.types_tail.intern_signature(function);

                dir::Type::FunctionSignature(function)
            }
            dir::Type::Function(mut function) => {
                function.signature = map(self, function.signature)?;

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

    /// Map one shape's embedded type ids from one module into another.
    fn map_shape(
        &mut self,
        source: ModuleId,
        target: ModuleId,
        mut shape: dir::ShapeType,
        map: &mut impl FnMut(&mut Self, dir::GlobalTypeId) -> CompilerResult<dir::GlobalTypeId>,
    ) -> CompilerResult<dir::ShapeType> {
        // map each property's read and write types
        let mut properties = SmallVec::<[dir::TypeProperty; 8]>::from_slice(
            self.shape_properties(source, shape.properties)?,
        );
        for property in &mut properties {
            property.access = match property.access {
                dir::PropertyAccess::Read(ty) => dir::PropertyAccess::Read(map(self, ty)?),
                dir::PropertyAccess::Write(ty) => dir::PropertyAccess::Write(map(self, ty)?),
                dir::PropertyAccess::ReadWrite { read, write } => dir::PropertyAccess::ReadWrite {
                    read: map(self, read)?,
                    write: map(self, write)?,
                },
            };
        }

        // map the signature lists as they stand
        shape.properties = self.intern_properties(&properties)?;
        shape.call_signatures =
            self.map_type_id_list(source, target, shape.call_signatures, map)?;
        shape.construct_signatures =
            self.map_type_id_list(source, target, shape.construct_signatures, map)?;

        // map each index signature's key and value types
        let mut signatures = SmallVec::<[dir::TypeIndexSignature; 2]>::from_slice(
            self.shape_index_signatures(source, shape.index_signatures)?,
        );
        for signature in &mut signatures {
            signature.key_type = map(self, signature.key_type)?;
            signature.value_type = map(self, signature.value_type)?;
        }

        shape.index_signatures = self.intern_index_signatures(&signatures)?;

        Ok(shape)
    }

    /// Map one interned type id list from one module into another.
    fn map_type_id_list(
        &mut self,
        source: ModuleId,
        _target: ModuleId,
        list: dir::TypeListId,
        map: &mut impl FnMut(&mut Self, dir::GlobalTypeId) -> CompilerResult<dir::GlobalTypeId>,
    ) -> CompilerResult<dir::TypeListId> {
        let mut ids = SmallVec::<[dir::GlobalTypeId; 8]>::from_slice(self.type_ids(source, list)?);
        for id in &mut ids {
            *id = map(self, *id)?;
        }

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

    /// Return one written application's arguments with elided slots filled.
    pub(in crate::sema) fn filled_application_arguments(
        &mut self,
        module: ModuleId,
        instance: &dir::GenericApplication,
    ) -> CompilerResult<Vec<dir::GlobalTypeId>> {
        let filled = self.fill_elided_application(module, instance)?;
        if let Some(filled) = filled
            && let dir::Type::Application(application) = self.ty(filled)?
        {
            // read the arguments from the module the filled type interned into
            return Ok(self
                .type_ids(filled.module_id, application.arguments)?
                .to_vec());
        }

        Ok(self.type_ids(module, instance.arguments)?.to_vec())
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
    pub(in crate::sema) fn shape_properties(
        &self,
        module: ModuleId,
        list: dir::TypeListId,
    ) -> CompilerResult<&[dir::TypeProperty]> {
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

        self.intern_type(dir::Type::Object(dir::ShapeType {
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
        bindings: &[(dir::StaticKey, dir::GlobalTypeId)],
    ) -> CompilerResult<dir::GlobalTypeId> {
        // sort the bindings, so equal refinement sets intern identically
        let mut bindings = SmallVec::<[_; 2]>::from_slice(bindings);
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

    /// Return one refined type's base and associated bindings.
    pub(in crate::sema) fn refinement_bindings(
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

        bindings.sort_by_key(|(key, _)| *key);

        Ok((id, bindings))
    }
}
