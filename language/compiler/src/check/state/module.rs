use std::slice::from_ref;
use std::sync::Arc;

use destack_artifact::{
    DiagnosticBuilder, DiagnosticControlTable, DirBound, DirDeclared, DirElaborated, DirExpanded,
    DirParsed, DirResolved, ProfileKey,
};
use destack_core::{FxIndexMap, FxIndexSet};
use destack_dir as dir;
use destack_repository::{Module, Package};
use destack_source::{ModuleId, Span};
use smallvec::SmallVec;

use crate::check::{
    ApparentInstance, Capture, Cause, CauseKind, CheckError, CheckState, CheckWarning, Constraint,
    FlowPoint, FlowPointId, FlowSite, Origin, Relation, StaticPresence, VariableRole, Widening,
};
use crate::{CompilerError, CompilerResult};

/// Working state owned by one checked module.
pub(in crate::check) struct CheckModuleState {
    // inherited inputs from upstream phases, read-only
    /// The requested source module.
    pub(in crate::check) module: Arc<Module>,
    /// The package containing the requested module.
    pub(in crate::check) package: Arc<Package>,
    /// The active target profile.
    pub(in crate::check) profile: ProfileKey,
    /// The parsed DIR input.
    pub(in crate::check) parsed: Arc<DirParsed>,
    /// The bound DIR input.
    pub(in crate::check) bound: Arc<DirBound>,
    /// The resolved DIR input.
    pub(in crate::check) resolved: Arc<DirResolved>,
    /// The expanded DIR input.
    pub(in crate::check) expanded: Arc<DirExpanded>,
    /// The foreign modules the stored rows mention, accumulated at store time.
    pub(in crate::check) references: FxIndexSet<ModuleId>,
    /// The declared DIR artifact seeding this check, absent while declaring.
    pub(in crate::check) declared: Option<Arc<DirDeclared>>,
    /// The elaborated stage backing this check, when elaborating is done.
    pub(in crate::check) elaborated: Option<Arc<DirElaborated>>,
    /// The cumulative binding table built once at load.
    pub(in crate::check) bindings: dir::BindingTable<'static>,
    /// Checked symbols synthesized from resolved language features.
    pub(in crate::check) bindings_tail: dir::BindingSegment,
    /// External modules visible from this module.
    pub(in crate::check) external_modules: FxIndexSet<ModuleId>,

    // open checked state owned by this module
    /// The committed base type table built once at load.
    pub(in crate::check) types: dir::TypeTable<'static>,
    /// The committed base static table built once at load.
    pub(in crate::check) statics_base: dir::StaticTable<'static>,
    /// Open inference types layered over the committed base.
    pub(in crate::check) types_tail: dir::TypeTail,
    /// Checked declaration definitions.
    pub(in crate::check) definitions: dir::DefinitionSegment,
    /// Auto-derived implementations.
    pub(in crate::check) auto: dir::AutoSegment,
    /// Induced generic templates and parameters.
    pub(in crate::check) generics: dir::GenericSegment,
    /// Checked static values.
    pub(in crate::check) statics: dir::StaticSegment,
    /// Checked node resolutions.
    pub(in crate::check) resolutions: dir::ResolutionSegment,
    /// Decisions inference made this pass.
    pub(in crate::check) decisions: dir::DecisionSegment,
    /// Checked member availability.
    pub(in crate::check) members: dir::MemberSegment,
    /// Checked implicit coercions.
    pub(in crate::check) coercions: dir::CoercionSegment,
    /// Checked captures.
    pub(in crate::check) capture_segment: dir::CaptureSegment,
    /// Checked decorators.
    pub(in crate::check) decorators: dir::DecoratorSegment,
    /// Checked diagnostic controls.
    pub(in crate::check) controls: DiagnosticControlTable,
    /// Inferred static symbol values written to statics during output.
    pub(in crate::check) static_values: FxIndexMap<dir::GlobalSymbolId, dir::GlobalTypeId>,
    /// Captures discovered while walking this module.
    pub(in crate::check) captures: Vec<Capture>,
    /// Durable flow states discovered while walking this module.
    pub(in crate::check) flows: Vec<FlowPoint>,
    /// Entry flow point for each walked source node occurrence.
    pub(in crate::check) node_flows: FxIndexMap<dir::GlobalNodeIdAny, FlowPointId>,
    /// Generic template assumed by each checked source node.
    pub(in crate::check) node_scopes:
        FxIndexMap<dir::GlobalNodeIdAny, Option<dir::GlobalGenericTemplateId>>,
    /// Source nodes whose end no control path reaches.
    pub(in crate::check) unreachable_ends: FxIndexSet<dir::LocalNodeIdAny>,

    // statically false gates
    /// Presence decisions for decorated source nodes.
    pub(in crate::check) static_presence: FxIndexMap<dir::LocalNodeIdAny, StaticPresence>,
    /// Declarations whose guards decided statically false.
    pub(in crate::check) absent_symbols: FxIndexSet<dir::GlobalSymbolId>,

    // diagnostics drained during write
    /// Diagnostics reported while walking this module.
    pub(in crate::check) diagnostics: Vec<DiagnosticBuilder<CheckError>>,
    /// Warnings reported while walking this module.
    pub(in crate::check) warnings: Vec<DiagnosticBuilder<CheckWarning>>,
}

impl CheckModuleState {
    /// Create module state from loaded inputs and empty checked state.
    pub(in crate::check) fn new(
        module: Arc<Module>,
        package: Arc<Package>,
        profile: ProfileKey,
        parsed: Arc<DirParsed>,
        bound: Arc<DirBound>,
        resolved: Arc<DirResolved>,
        expanded: Arc<DirExpanded>,
        declared: Option<Arc<DirDeclared>>,
        elaborated: Option<Arc<DirElaborated>>,
    ) -> Self {
        // stack this check's overlays over the declared segments, else the expanded base
        let (bindings, types, statics_base, types_tail, generics, statics, decorators) =
            match &declared {
                Some(declared) => (
                    match &elaborated {
                        Some(elaborated) => elaborated.binding_table(&bound, &expanded, declared),
                        None => declared.binding_table(&bound, &expanded),
                    },
                    match &elaborated {
                        Some(elaborated) => elaborated.type_table(&bound, &expanded, declared),
                        None => declared.type_table(&bound, &expanded),
                    },
                    match &elaborated {
                        Some(elaborated) => elaborated.static_table(&bound, &expanded, declared),
                        None => declared.static_table(&bound, &expanded),
                    },
                    match &elaborated {
                        Some(elaborated) => dir::TypeTail::over(vec![
                            Arc::clone(&declared.types),
                            Arc::clone(&elaborated.types),
                        ]),
                        None => dir::TypeTail::over_base(Arc::clone(&declared.types)),
                    },
                    match &elaborated {
                        Some(elaborated) => dir::GenericSegment::from_base(&elaborated.generics),
                        None => dir::GenericSegment::from_base(&declared.generics),
                    },
                    match &elaborated {
                        Some(elaborated) => dir::StaticSegment::from_base(&elaborated.statics),
                        None => dir::StaticSegment::from_base(&declared.statics),
                    },
                    match &elaborated {
                        Some(elaborated) => {
                            dir::DecoratorSegment::from_base(&elaborated.decorators)
                        }
                        None => dir::DecoratorSegment::from_base(&declared.decorators),
                    },
                ),
                None => (
                    expanded.binding_table(&bound),
                    expanded.type_table(&bound),
                    expanded.static_table(&bound),
                    dir::TypeTail::from_base(&expanded.types),
                    dir::GenericSegment::new(module.id),
                    dir::StaticSegment::from_base(&expanded.statics),
                    dir::DecoratorSegment::new(module.id),
                ),
            };
        let bindings_tail = dir::BindingSegment::from_table(&bindings);

        // adopt the elaborated definitions whole, they key by symbol
        let definitions = match (&elaborated, &declared) {
            (Some(elaborated), _) => (*elaborated.definitions).clone(),
            (None, Some(declared)) => (*declared.definitions).clone(),
            (None, None) => dir::DefinitionSegment::new(module.id),
        };

        // open the remaining segments and this module's diagnostic controls
        let auto = dir::AutoSegment::new(module.id);
        let resolutions = dir::ResolutionSegment::new(module.id);
        let decisions = dir::DecisionSegment::new(module.id);
        let members = match (&elaborated, &declared) {
            (Some(elaborated), _) => (*elaborated.members).clone(),
            (None, Some(declared)) => (*declared.members).clone(),
            (None, None) => dir::MemberSegment::new(module.id),
        };
        let coercions = dir::CoercionSegment::new(module.id);
        let capture_segment = dir::CaptureSegment::new(module.id);
        let controls = match &elaborated {
            Some(elaborated) => (*elaborated.controls).clone(),
            None => {
                let files = parsed.files.iter().map(|file| file.file_id).collect();

                DiagnosticControlTable::new(module.id, files)
            }
        };

        Self {
            module,
            package,
            profile,
            parsed,
            bound,
            resolved,
            expanded,
            references: FxIndexSet::default(),
            declared,
            elaborated,
            bindings,
            bindings_tail,
            types,
            statics_base,
            types_tail,
            definitions,
            auto,
            generics,
            statics,
            resolutions,
            decisions,
            members,
            coercions,
            capture_segment,
            decorators,
            controls,
            static_values: FxIndexMap::default(),
            static_presence: FxIndexMap::default(),
            absent_symbols: FxIndexSet::default(),
            external_modules: FxIndexSet::default(),
            captures: Vec::new(),
            flows: Vec::new(),
            node_flows: FxIndexMap::default(),
            node_scopes: FxIndexMap::default(),
            unreachable_ends: FxIndexSet::default(),
            diagnostics: Vec::new(),
            warnings: Vec::new(),
        }
    }

    /// Return the post-expansion DIR tree view visible to check.
    pub(in crate::check) fn view(&self) -> dir::View<'_> {
        dir::View::with_patches(&self.parsed.tree, from_ref(&self.expanded.patch))
    }

    /// Return the authored source tree used for source rendering.
    pub(in crate::check) fn source_tree(&self) -> &dir::Tree {
        &self.parsed.tree
    }

    /// Return the authored source span of one node.
    pub(in crate::check) fn authored_span(&self, node: dir::LocalNodeIdAny) -> Span {
        self.parsed.tree.source_index.get_main_or_enclosing(node.id)
    }

    /// Return the full source span of one visible node's authored origin.
    pub(in crate::check) fn source_span(&self, node: dir::LocalNodeIdAny) -> Option<Span> {
        let source = self.view().get_source_any(node);

        self.parsed.tree.get_span_by_id(source)
    }

    /// Return the authored diagnostic span of one visible node.
    pub(in crate::check) fn diagnostic_span(&self, node: dir::LocalNodeIdAny) -> Option<Span> {
        let view = self.view();
        let source = view.get_source_any(node);

        // prefer the authored node that produced the visible node
        if let Some(span) = self.parsed.tree.get_main_span_by_id(source) {
            return Some(span);
        }
        if let Some(span) = self.parsed.tree.get_span_by_id(source) {
            return Some(span);
        }

        view.get_span_by_id(node.id)
    }

    /// Return the cumulative binding table visible to check.
    pub(in crate::check) fn binding_table(&self) -> dir::BindingTable<'_> {
        self.bindings.with_tail(&self.bindings_tail)
    }

    /// Return the symbol introduced by a source declaration node.
    pub(in crate::check) fn declaration_symbol(
        &self,
        node: dir::LocalNodeIdAny,
    ) -> Option<dir::GlobalSymbolId> {
        let symbol = self
            .binding_table()
            .declaration_symbol(node.into_global(self.module.id))?;

        Some(symbol.into_global(self.module.id))
    }

    /// Return the implicit receiver symbol introduced for one member node.
    pub(in crate::check) fn implicit_receiver_symbol(
        &self,
        node: dir::LocalNodeIdAny,
    ) -> Option<dir::GlobalSymbolId> {
        let symbol = self
            .binding_table()
            .implicit_receiver_symbol(node.into_global(self.module.id))?;

        Some(symbol.into_global(self.module.id))
    }

    /// Return one source symbol's declaration node.
    pub(in crate::check) fn symbol_declaration_node(
        &self,
        symbol: dir::LocalSymbolId,
    ) -> CompilerResult<dir::LocalNodeIdAny> {
        let bindings = self.binding_table();
        let binding = bindings.get_symbol(symbol);

        // require source symbols to have local declaration nodes
        let Some(declaration) = binding.declaration else {
            return Err(CompilerError::Internal {
                message: format!("source symbol {symbol:?} has no declaration node"),
            });
        };
        if declaration.module_id != self.module.id {
            return Err(CompilerError::Internal {
                message: format!("source symbol {symbol:?} declaration points outside its module"),
            });
        }

        Ok(declaration.local_id)
    }

    /// Return one local input static visible to check.
    pub(in crate::check) fn r#static(&self, static_id: dir::LocalStaticId) -> &dir::StaticTerm {
        // read this pass's own terms, then the expanded and bound bases
        if let Some(value) = self.statics.get_static_maybe(static_id) {
            return value;
        }

        if let Some(value) = self.expanded.statics.get_static_maybe(static_id) {
            return value;
        }

        self.bound.statics.get_static(static_id)
    }

    /// Return whether one local symbol is an imported alias.
    pub(in crate::check) fn is_import_alias(&self, symbol: dir::LocalSymbolId) -> bool {
        self.resolved.imports.symbol_resolution(symbol).is_some()
    }
}

impl CheckState<'_> {
    /// Return whether one module is the checked module.
    pub(in crate::check) fn is_own_module(&self, module: ModuleId) -> bool {
        module == self.module_id
    }

    /// Return whether one module's declared tables are readable.
    pub(in crate::check) fn is_loaded_module(&self, module: ModuleId) -> bool {
        self.is_own_module(module) || self.external_modules.contains_key(&module)
    }

    /// Return the module's working state when it is the checked module.
    pub(in crate::check) fn module_maybe(&self, module: ModuleId) -> Option<&CheckModuleState> {
        (module == self.module_id).then_some(&self.module)
    }

    /// Return the mutable working state when it is the checked module.
    pub(in crate::check) fn module_maybe_mut(
        &mut self,
        module: ModuleId,
    ) -> Option<&mut CheckModuleState> {
        (module == self.module_id).then_some(&mut self.module)
    }

    /// Return the symbols whose static guards did not decide false.
    pub(in crate::check) fn present_symbols(
        &self,
        symbols: &[dir::GlobalSymbolId],
    ) -> SmallVec<[dir::GlobalSymbolId; 4]> {
        symbols
            .iter()
            .copied()
            .filter(|symbol| !self.is_absent_symbol(*symbol))
            .collect()
    }

    /// Return the unique live symbol referenced by one source node.
    pub(in crate::check) fn reference_symbol(
        &self,
        source: dir::GlobalNodeIdAny,
    ) -> Option<dir::GlobalSymbolId> {
        let reference = self
            .module_resolved(source.module_id)
            .references
            .get(source)?;
        let dir::Reference::Bound(symbols) = reference else {
            return None;
        };
        let symbols = self.present_symbols(symbols);
        let [symbol] = symbols.as_slice() else {
            return None;
        };

        Some(*symbol)
    }

    /// Return whether one symbol's guard decided statically false.
    pub(in crate::check) fn is_absent_symbol(&self, symbol: dir::GlobalSymbolId) -> bool {
        self.module_maybe(symbol.module_id)
            .is_some_and(|module| module.absent_symbols.contains(&symbol))
    }

    /// Return whether one source node is inside a statically absent subtree.
    pub(in crate::check) fn is_absent(&self, node: dir::GlobalNodeIdAny) -> bool {
        let module = self.module(node.module_id);
        let view = module.view();
        let mut current = Some(node.local_id);
        while let Some(local) = current {
            if module.statics.contains_absent_root(local)
                || module.statics_base.contains_absent_root(local)
            {
                return true;
            }
            current = view.get_parent_any(local);
        }

        false
    }

    /// Return the checked module's working state.
    pub(in crate::check) fn module(&self, module: ModuleId) -> &CheckModuleState {
        match self.module_maybe(module) {
            Some(state) => state,
            None => unreachable!("check module {module:?} was not loaded"),
        }
    }

    /// Return the checked module's working state mutably.
    pub(in crate::check) fn module_mut(&mut self, module: ModuleId) -> &mut CheckModuleState {
        match self.module_maybe_mut(module) {
            Some(state) => state,
            None => unreachable!("check module {module:?} was not loaded"),
        }
    }

    /// Return one source node type without flow narrowing, if present.
    pub(in crate::check) fn committed_node_type(
        &self,
        node: dir::GlobalNodeIdAny,
    ) -> Option<dir::GlobalTypeId> {
        // read this pass's own commits first
        if let Some(ty) = self.node_types.get(&node) {
            return Some(ty);
        }

        // read own declared-stage node types
        if let Some(module) = self.module_maybe(node.module_id) {
            return module.types.get_node_type_id(node);
        }

        None
    }

    /// Return one source node type without flow narrowing.
    pub(in crate::check) fn node_type(
        &self,
        node: dir::GlobalNodeIdAny,
    ) -> CompilerResult<dir::GlobalTypeId> {
        match self.committed_node_type(node) {
            Some(ty) => Ok(ty),
            None => Err(CompilerError::Internal {
                message: format!("node type read before checking {node:?}"),
            }),
        }
    }

    /// Return an invariant label for one source node.
    pub(in crate::check) fn node_label(&self, node: dir::GlobalNodeIdAny) -> String {
        let module = self.module(node.module_id);
        let uri = module.module.uri.as_ref();

        // include source position without inspecting the node's syntax variant
        if let Some(span) = module.diagnostic_span(node.local_id) {
            format!("node {node:?} in {uri} at {span:?}")
        } else {
            format!("node {node:?} in {uri}")
        }
    }

    /// Return one source node type without flow narrowing or fail on an invariant break.
    pub(in crate::check) fn require_node_type(
        &self,
        node: dir::GlobalNodeIdAny,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let Some(ty) = self.committed_node_type(node) else {
            return Err(CompilerError::Internal {
                message: format!(
                    "required node has no checked type: {}; decision={:?}",
                    self.node_label(node),
                    self.decision(node),
                ),
            });
        };

        Ok(ty)
    }

    /// Commit one source node type.
    pub(in crate::check) fn commit_node_type(
        &mut self,
        node: dir::GlobalNodeIdAny,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<()> {
        // compare only against this pass's own commits
        if let Some(previous) = self.node_types.get(&node) {
            if previous == ty {
                return Ok(());
            }

            // solve a committed hole, or require re-derivations to agree
            if let dir::Type::Variable(variable) = self.ty(previous)? {
                if self.infer.variable(variable)?.state.is_open() {
                    // an open derivation equates as a bound until it
                    //  closes: hole solutions are closed types
                    if self.type_variables(ty)?.is_empty() {
                        self.commit_solution(variable, ty)?;
                    } else if let Some(origin) = self.node_origin_maybe(node) {
                        let cause = self.intern_cause(Cause::root(origin, CauseKind::Expression));
                        self.constrain_type(origin, cause, Relation::Equal, previous, ty)?;
                    } else {
                        self.commit_solution(variable, ty)?;
                    }

                    return Ok(());
                }
                if self.shallow_resolve(previous)? == self.shallow_resolve(ty)? {
                    return Ok(());
                }

                // a solved hole and its re-derivation agree through the
                //  solver, once every operand variable settles
                if let Some(origin) = self.node_origin_maybe(node) {
                    let cause = self.intern_cause(Cause::root(origin, CauseKind::Expression));
                    self.register_constraint(Constraint::r#type(
                        origin,
                        Relation::Equal,
                        ty,
                        previous,
                        cause,
                    ));

                    return Ok(());
                }
            }

            let previous = self.format_type(previous);
            let ty = self.format_type(ty);
            let node = self.node_label(node);

            return Err(CompilerError::Internal {
                message: format!("check node {node} received two types: {previous} and {ty}"),
            });
        }

        self.node_types.insert(node, ty);

        Ok(())
    }

    /// Commit the error type for one rejected source node.
    pub(in crate::check) fn commit_error_node(
        &mut self,
        node: dir::GlobalNodeIdAny,
    ) -> CompilerResult<dir::GlobalTypeId> {
        if let Some(ty) = self.committed_node_type(node) {
            return Ok(ty);
        }

        let ty = self.intern_type(dir::Type::Error)?;
        self.commit_node_type(node, ty)?;

        Ok(ty)
    }

    /// Return one source node's recorded flow site.
    /// Enter one source node at the live cursor, recording its site.
    ///
    /// The single body traversal mints each node's site as it reaches
    /// it; deferred work re-reads the recorded maps through node_site.
    pub(in crate::check) fn visit_site(
        &mut self,
        node: dir::GlobalNodeIdAny,
    ) -> CompilerResult<FlowSite> {
        // reuse a site the walk already recorded for this node
        if let Some(flow) = self.module(node.module_id).node_flows.get(&node).copied() {
            let scope = self
                .module(node.module_id)
                .node_scopes
                .get(&node)
                .copied()
                .flatten();

            return Ok(FlowSite { node, flow, scope });
        }

        let flow = self.flow.point();
        let scope = self.flow.template_scope();
        let state = self.module_mut(node.module_id);
        state.node_flows.insert(node, flow);
        state.node_scopes.insert(node, scope);

        Ok(FlowSite { node, flow, scope })
    }

    pub(in crate::check) fn node_site(
        &self,
        node: dir::GlobalNodeIdAny,
    ) -> CompilerResult<FlowSite> {
        let Some(flow) = self.module(node.module_id).node_flows.get(&node).copied() else {
            let node = self.node_label(node);

            return Err(CompilerError::Internal {
                message: format!("check node {node} has no recorded runtime flow"),
            });
        };
        let origin = self.node_origin(node)?;
        let Origin::Node(_, scope) = origin else {
            unreachable!("node origin must name a source node")
        };

        Ok(FlowSite { node, flow, scope })
    }

    /// Return the work origin for one walked source node, or none.
    pub(in crate::check) fn node_origin_maybe(&self, node: dir::GlobalNodeIdAny) -> Option<Origin> {
        let scope = self
            .module(node.module_id)
            .node_scopes
            .get(&node)
            .copied()?;

        Some(Origin::Node(node, scope))
    }

    /// Return the work origin for one checked source node.
    pub(in crate::check) fn node_origin(
        &self,
        node: dir::GlobalNodeIdAny,
    ) -> CompilerResult<Origin> {
        let Some(scope) = self.module(node.module_id).node_scopes.get(&node).copied() else {
            let node = self.node_label(node);

            return Err(CompilerError::Internal {
                message: format!("check node {node} has no recorded origin"),
            });
        };

        Ok(Origin::Node(node, scope))
    }

    /// Return one declaration type committed this pass, if present.
    pub(in crate::check) fn declaration_type_maybe(
        &self,
        symbol: dir::GlobalSymbolId,
    ) -> Option<dir::GlobalTypeId> {
        self.declaration_types.get(&symbol).copied()
    }

    /// Commit one declaration symbol type.
    pub(in crate::check) fn commit_declaration_type(
        &mut self,
        symbol: dir::GlobalSymbolId,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<()> {
        if let Some(previous) = self.declaration_type_maybe(symbol) {
            if previous == ty {
                return Ok(());
            }

            return Err(CompilerError::Internal {
                message: format!(
                    "declaration symbol {symbol:?} already has type {previous:?}, got {ty:?}"
                ),
            });
        }

        self.declaration_types.insert(symbol, ty);
        self.solve_symbol_variable(symbol, ty)?;

        Ok(())
    }

    /// Return one binding type committed this pass, if present.
    pub(in crate::check) fn binding_type_maybe(
        &self,
        symbol: dir::GlobalSymbolId,
    ) -> Option<dir::GlobalTypeId> {
        self.binding_types.get(&symbol).copied()
    }

    /// Commit one binding symbol type.
    pub(in crate::check) fn commit_binding_type(
        &mut self,
        symbol: dir::GlobalSymbolId,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<()> {
        if let Some(previous) = self.binding_type_maybe(symbol) {
            if previous == ty {
                return Ok(());
            }

            return Err(CompilerError::Internal {
                message: format!(
                    "binding symbol {symbol:?} already has type {previous:?}, got {ty:?}"
                ),
            });
        }

        self.binding_types.insert(symbol, ty);
        self.solve_symbol_variable(symbol, ty)?;

        Ok(())
    }

    /// Solve one symbol's variable with its committed type.
    fn solve_symbol_variable(
        &mut self,
        symbol: dir::GlobalSymbolId,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<()> {
        let Some(variable) = self.infer.symbol_variables.get(&symbol).copied() else {
            return Ok(());
        };
        if !self.infer.variable(variable)?.state.is_open() {
            return Ok(());
        }

        // solve closed types; open types settle through their own inference
        if self.type_variables(ty)?.is_empty() {
            self.commit_solution(variable, ty)?;
        }

        Ok(())
    }

    /// Bind one symbol to an exact checked type.
    pub(in crate::check) fn bind_symbol_type(
        &mut self,
        symbol: dir::GlobalSymbolId,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        // read the type this symbol already committed, if any
        let is_binding = self.symbol_kind(symbol)?.is_binding();
        let existing = if is_binding {
            self.binding_type_maybe(symbol)
        } else {
            self.declaration_type_maybe(symbol)
        };

        // require a second derivation to agree with the committed type
        if let Some(existing) = existing {
            let origin = self.intern_origin(Origin::Symbol(symbol));
            let origin = self.infer.origin(origin);
            let cause = self.intern_cause(Cause::root(origin, CauseKind::Expression));
            self.push_constraint(Constraint::r#type(
                origin,
                Relation::Equal,
                existing,
                ty,
                cause,
            ))?;

            return Ok(existing);
        }

        // commit into the table matching the symbol's kind
        if is_binding {
            self.commit_binding_type(symbol, ty)?;
        } else {
            self.commit_declaration_type(symbol, ty)?;
        }

        Ok(ty)
    }

    /// Return one loaded symbol's checked type, if present.
    pub(in crate::check) fn symbol_type_maybe(
        &self,
        symbol: dir::GlobalSymbolId,
    ) -> Option<dir::GlobalTypeId> {
        // prefer body-owned bindings over stable declarations
        if let Some(ty) = self.binding_type_maybe(symbol) {
            return Some(ty);
        }
        if let Some(ty) = self.declaration_type_maybe(symbol) {
            return Some(ty);
        }

        // read own declared-stage symbol types, a persisted type
        //  that still carries open holes reads as absent
        if let Some(module) = self.module_maybe(symbol.module_id)
            && let Some(ty) = module.types.get_symbol_type_id(symbol)
            && !self.type_flags(ty).is_ok_and(|flags| flags.has_variable())
        {
            return Some(ty);
        }

        // read external committed symbol types
        if let Some(external) = self.external_modules.get(&symbol.module_id) {
            return external.types.get_symbol_type_id(symbol);
        }

        None
    }

    /// Return one symbol's checked type, importing its module on demand.
    pub(in crate::check) fn symbol_type(
        &mut self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        if !self.is_own_module(symbol.module_id) {
            self.import_external_module(symbol.module_id)?;
        }

        if let Some(ty) = self.canonical_symbol_type_maybe(symbol)? {
            return Ok(ty);
        }

        // external tables are settled, so an absent type is always a failure
        if !self.is_own_module(symbol.module_id) {
            return Err(CompilerError::Internal {
                message: format!(
                    "external symbol {} has no committed type",
                    self.format_symbol(symbol),
                ),
            });
        }

        // local symbols read through their inference hole until commitment
        let variable = self.symbol_variable(symbol);

        self.variable_type(variable)
    }

    /// Return the variable standing for one local symbol's type, allocating it once.
    pub(in crate::check) fn symbol_variable(
        &mut self,
        symbol: dir::GlobalSymbolId,
    ) -> dir::TypeVariableId {
        if let Some(variable) = self.infer.symbol_variables.get(&symbol) {
            return *variable;
        }

        let variable = self.allocate_variable(
            Origin::Symbol(symbol),
            Widening::Never,
            VariableRole::Symbol { symbol },
        );
        self.infer.symbol_variables.insert(symbol, variable);

        variable
    }

    /// Return one symbol's type, canonicalizing written types on read.
    pub(in crate::check) fn canonical_symbol_type_maybe(
        &mut self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        let Some(ty) = self.symbol_type_maybe(symbol) else {
            return Ok(None);
        };

        // canonicalize foreign written types once per module
        if !self.is_own_module(symbol.module_id) {
            // read written types uncanonicalized in declare mode
            if self.is_declaration() {
                return Ok(Some(ty));
            }
            if let Some(canonical) = self.imported_types.get(&symbol) {
                return Ok(Some(*canonical));
            }
            let canonical = self.canonical_foreign_type(symbol, ty)?;
            self.imported_types.insert(symbol, canonical);

            return Ok(Some(canonical));
        }

        // canonicalize own declared-stage values once, types stay written
        if self.binding_type_maybe(symbol).is_none()
            && self.declaration_type_maybe(symbol).is_none()
            && !self.symbol_kind(symbol)?.is_type_definition()
        {
            let canonical = self.canonical_foreign_type(symbol, ty)?;
            self.bind_symbol_type(symbol, canonical)?;

            return Ok(Some(canonical));
        }

        Ok(Some(ty))
    }

    /// Return the checked type required for one loaded symbol.
    pub(in crate::check) fn require_symbol_type(
        &self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let Some(ty) = self.symbol_type_maybe(symbol) else {
            return Err(CompilerError::Internal {
                message: format!("symbol {symbol:?} has no checked type"),
            });
        };

        Ok(ty)
    }

    /// Return one definition member's checked value type.
    pub(in crate::check) fn definition_member_type(
        &mut self,
        member: &dir::DefinitionMember,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        if let Some(symbol) = member.type_symbol() {
            let ty = self.symbol_type(symbol)?;

            return Ok(Some(ty));
        }

        Ok(member.value_type())
    }

    /// Return the checked value type required for one definition member.
    pub(in crate::check) fn require_definition_member_type(
        &self,
        member: &dir::DefinitionMember,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        if let Some(symbol) = member.type_symbol() {
            return Ok(Some(self.require_symbol_type(symbol)?));
        }

        Ok(member.value_type())
    }

    /// Return one symbol's settled static id, if declared.
    pub(in crate::check) fn symbol_static_id(
        &self,
        symbol: dir::GlobalSymbolId,
    ) -> Option<dir::GlobalStaticId> {
        if let Some(module) = self.module_maybe(symbol.module_id) {
            let table = module.statics_base.with_tail(&module.statics);

            table.get_symbol_static_id(symbol)
        } else {
            let external = self.external_modules.get(&symbol.module_id)?;

            external.statics.get_symbol_static_id(symbol)
        }
    }

    /// Return the inferred static value of one source symbol.
    pub(in crate::check) fn static_value(
        &mut self,
        symbol: dir::GlobalSymbolId,
    ) -> Option<dir::GlobalTypeId> {
        // read this pass's own committed values first
        if let Some(value) = self
            .module_maybe(symbol.module_id)
            .and_then(|module| module.static_values.get(&symbol).copied())
        {
            return Some(value);
        }

        // read own settled terms through the static table stack
        if let Some(module) = self.module_maybe(symbol.module_id) {
            let table = module.statics_base.with_tail(&module.statics);
            let id = table.get_symbol_static_id(symbol)?;
            let term = table.get_static_maybe(id.local_id)?.clone();

            return self.static_term_type(&term);
        }

        // read foreign settled terms through the loaded external tables
        let external = self.external_modules.get(&symbol.module_id)?;
        let id = external.statics.get_symbol_static_id(symbol)?;
        let term = external.statics.get_static_maybe(id.local_id)?.clone();

        self.static_term_type(&term)
    }

    /// Return the singleton type of one settled static term.
    fn static_term_type(&mut self, term: &dir::StaticTerm) -> Option<dir::GlobalTypeId> {
        match term {
            dir::StaticTerm::Type { ty } => Some(*ty),
            dir::StaticTerm::ScalarLiteral { value } => {
                self.intern_type(dir::Type::Literal(*value)).ok()
            }
            _ => None,
        }
    }

    /// Commit the inferred static value of one source symbol as a singleton type.
    pub(in crate::check) fn commit_static_value(
        &mut self,
        symbol: dir::GlobalSymbolId,
        value: dir::GlobalTypeId,
    ) -> CompilerResult<()> {
        let static_values = &mut self.module_mut(symbol.module_id).static_values;
        if static_values
            .insert(symbol, value)
            .is_some_and(|previous| previous != value)
        {
            return Err(CompilerError::Internal {
                message: format!("check symbol {symbol:?} received two static values"),
            });
        }

        Ok(())
    }

    /// Return one module's resolved names, loaded or external.
    pub(in crate::check) fn module_resolved(&self, module: ModuleId) -> &DirResolved {
        if let Some(module) = self.module_maybe(module) {
            &module.resolved
        } else {
            &self.external_module(module).resolved
        }
    }

    /// Return one module's tree view.
    pub(in crate::check) fn module_view(&self, module: ModuleId) -> dir::View<'_> {
        match self.module_maybe(module) {
            Some(module) => module.view(),
            None => unreachable!("foreign trees are not checking inputs: {module:?}"),
        }
    }

    /// Return one binding table by module, loaded or read from artifacts.
    pub(in crate::check) fn binding_table(&self, module: ModuleId) -> dir::BindingTable<'_> {
        if let Some(module) = self.module_maybe(module) {
            return module.binding_table();
        }
        if let Some(external) = self.external_modules.get(&module) {
            return external.bindings.clone();
        }

        // a referenced module's declared artifact depends on its bound tables
        let bound = self
            .artifacts
            .dir_bound_content(module, self.profile)
            .unwrap_or_else(|error| {
                unreachable!("referenced module {module:?} has no bound artifact: {error}")
            });
        let expanded = self
            .artifacts
            .dir_expanded_content(module, self.profile)
            .unwrap_or_else(|error| {
                unreachable!("referenced module {module:?} has no expanded artifact: {error}")
            });

        expanded.binding_table(bound.as_ref())
    }

    /// Return one own-module or external static.
    pub(in crate::check) fn r#static(&self, value: dir::GlobalStaticId) -> &dir::StaticTerm {
        if let Some(module) = self.module_maybe(value.module_id) {
            module.r#static(value.local_id)
        } else {
            self.external_module(value.module_id)
                .statics
                .get_static(value.local_id)
        }
    }

    /// Return one own-module symbol's kind without loading anything.
    pub(in crate::check) fn own_symbol_kind(
        &self,
        symbol: dir::GlobalSymbolId,
    ) -> Option<dir::SymbolKind> {
        if !self.is_own_module(symbol.module_id) {
            return None;
        }

        Some(
            self.binding_table(symbol.module_id)
                .get_symbol(symbol.local_id)
                .kind,
        )
    }

    /// Return one symbol's kind when its declaring module is readable.
    pub(in crate::check) fn symbol_kind_maybe(
        &mut self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<Option<dir::SymbolKind>> {
        Ok(Some(self.symbol_kind(symbol)?))
    }

    /// Return the declaration kind for one symbol.
    pub(in crate::check) fn symbol_kind(
        &mut self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<dir::SymbolKind> {
        // load the foreign module the classification reads
        if !self.is_own_module(symbol.module_id) {
            self.import_external_module(symbol.module_id)?;
        }

        let binding_table = self.binding_table(symbol.module_id);
        let symbol = binding_table.get_symbol(symbol.local_id);

        Ok(symbol.kind)
    }

    /// Flatten every declared nominal owner's member bindings into the module tail.
    pub(in crate::check) fn flatten_declared_owners(&mut self) -> CompilerResult<()> {
        // collect the module's declared nominal owners
        let module = self.module_id;
        let mut owners = Vec::new();
        for (symbol, definition) in self.module(module).definitions.iter_definitions() {
            let is_nominal = matches!(
                definition,
                dir::Definition::Struct(_)
                    | dir::Definition::Class(_)
                    | dir::Definition::Enum(_)
                    | dir::Definition::Newtype(_)
                    | dir::Definition::Interface(_)
            );
            if is_nominal {
                owners.push(symbol);
            }
        }

        // flatten each owner's bindings in both member spaces
        for symbol in owners {
            let origin = Origin::Symbol(symbol);

            // derive each declared parameter's variance at its own context
            if let Some(template) = self.symbol_template(symbol)? {
                for parameter in self.generic_template_parameters(template)? {
                    let form = self.parameter_variance_form(parameter)?;
                    let _ = self.parameter_variance(parameter, form)?;
                }
            }
            let application = self.declaration_instance(symbol)?;
            let arguments = self.type_ids(module, application.arguments)?.to_vec();
            let instance = ApparentInstance {
                symbol: application.symbol,
                arguments: arguments.iter().copied().collect(),
            };
            let Some(canonical) = self
                .module(module)
                .types
                .get_symbol_type_id(symbol)
                .or_else(|| self.module(module).types_tail.get_symbol_type_id(symbol))
            else {
                continue;
            };

            for space in [dir::MemberSpace::Instance, dir::MemberSpace::Static] {
                let bindings = self
                    .body()
                    .canonical_member_bindings(origin, &instance, space)?;
                let Some(bindings) = bindings else {
                    continue;
                };
                let subject = dir::MemberSubject::new(canonical, canonical, space);
                self.module_mut(module)
                    .members
                    .set_bindings(subject, bindings);
            }
        }

        Ok(())
    }
}
