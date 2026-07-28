use std::slice::from_ref;
use std::sync::Arc;

use destack_artifact::{
    DiagnosticBuilder, DiagnosticControlTable, DirBound, DirDeclaredModule, DirExpanded, DirParsed,
    DirResolved, ProfileKey,
};
use destack_core::{FxIndexMap, FxIndexSet};
use destack_dir as dir;
use destack_repository::{Module, Package};
use destack_source::{ModuleId, Span};
use smallvec::SmallVec;

use crate::check::{
    Answer, Capture, Cause, CauseKind, CheckError, CheckState, CheckWarning, Constraint,
    Dependency, FlowPoint, FlowPointId, FlowSite, Origin, Relation, StaticGate,
};
use crate::{CompilerError, CompilerResult};

/// State owned by one module inside a checked component.
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
    /// The cumulative binding table built once at load.
    pub(in crate::check) bindings: dir::BindingTable<'static>,
    /// Checked symbols synthesized from resolved language features.
    pub(in crate::check) bindings_tail: dir::BindingSegment,
    /// Out-of-component modules visible from this module.
    pub(in crate::check) external_modules: FxIndexSet<ModuleId>,

    // open checked state owned by this module
    /// The committed base type table built once at load.
    pub(in crate::check) types: dir::TypeTable<'static>,
    /// Open inference types layered over the committed base.
    pub(in crate::check) types_tail: dir::TypeSegment,
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
    pub(in crate::check) static_presence: FxIndexMap<dir::LocalNodeIdAny, StaticGate>,
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
        declared: Option<&DirDeclaredModule>,
    ) -> Self {
        // create the inherited bindings and this check's open overlays
        let bindings = expanded.binding_table(&bound);
        let bindings_tail = match declared {
            Some(declared) => (*declared.bindings).clone(),
            None => dir::BindingSegment::from_table(&bindings),
        };
        let types = expanded.type_table(&bound);
        let types_tail = match declared {
            Some(declared) => (*declared.types).clone(),
            None => dir::TypeSegment::from_base(&expanded.types),
        };
        let generics = match declared {
            Some(declared) => (*declared.generics).clone(),
            None => dir::GenericSegment::new(module.id),
        };
        let statics = match declared {
            Some(declared) => (*declared.statics).clone(),
            None => dir::StaticSegment::from_base(&expanded.statics),
        };
        let decorators = match declared {
            Some(declared) => (*declared.decorators).clone(),
            None => dir::DecoratorSegment::new(module.id),
        };
        let definitions = dir::DefinitionSegment::new(module.id);
        let auto = dir::AutoSegment::new(module.id);
        let resolutions = dir::ResolutionSegment::new(module.id);
        let coercions = dir::CoercionSegment::new(module.id);
        let capture_segment = dir::CaptureSegment::new(module.id);
        let files = parsed.files.iter().map(|file| file.file_id).collect();
        let controls = DiagnosticControlTable::new(module.id, files);

        Self {
            module,
            package,
            profile,
            parsed,
            bound,
            resolved,
            expanded,
            bindings,
            bindings_tail,
            types,
            types_tail,
            definitions,
            auto,
            generics,
            statics,
            resolutions,
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

    /// Return one source symbol's declaration node when it declares locally.
    pub(in crate::check) fn symbol_declaration_node_maybe(
        &self,
        symbol: dir::LocalSymbolId,
    ) -> Option<dir::LocalNodeIdAny> {
        let bindings = self.binding_table();
        let declaration = bindings.get_symbol(symbol).declaration?;

        (declaration.module_id == self.module.id).then_some(declaration.local_id)
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

    /// Return one operation payload visible to check, reading the overlay over the base table.
    pub(in crate::check) fn operation_maybe(
        &self,
        id: dir::TypeOperationId,
    ) -> Option<dir::TypeOperation> {
        self.types_tail
            .operation(id)
            .or_else(|| self.types.operation_maybe(id))
            .copied()
    }

    /// Return one signature payload visible to check, reading the overlay over the base table.
    pub(in crate::check) fn signature_maybe(
        &self,
        id: dir::FunctionSignatureId,
    ) -> Option<dir::FunctionSignatureType> {
        self.types_tail
            .signature(id)
            .or_else(|| self.types.signature_maybe(id))
            .copied()
    }

    /// Return one member payload visible to check, reading the overlay over the base table.
    pub(in crate::check) fn member_maybe(&self, id: dir::MemberTypeId) -> Option<dir::MemberType> {
        self.types_tail
            .member(id)
            .or_else(|| self.types.member_maybe(id))
            .copied()
    }

    /// Return one refined payload visible to check, reading the overlay over the base table.
    pub(in crate::check) fn refined_maybe(
        &self,
        id: dir::RefinedTypeId,
    ) -> Option<dir::RefinedType> {
        self.types_tail
            .refined(id)
            .or_else(|| self.types.refined_maybe(id))
            .copied()
    }

    /// Return one borrow payload visible to check, reading the overlay over the base table.
    pub(in crate::check) fn borrow_maybe(&self, id: dir::BorrowFormId) -> Option<dir::BorrowForm> {
        self.types_tail
            .borrow_form(id)
            .or_else(|| self.types.borrow_form_maybe(id))
            .copied()
    }

    /// Return one type visible to check, reading the overlay over the base table.
    pub(in crate::check) fn type_maybe(&self, type_id: dir::LocalTypeId) -> Option<dir::Type> {
        self.types_tail
            .get_type_maybe(type_id)
            .or_else(|| self.types.get_type_maybe(type_id))
    }

    /// Return one type's structural flags, reading the overlay over the base table.
    pub(in crate::check) fn type_flags_maybe(
        &self,
        type_id: dir::LocalTypeId,
    ) -> Option<dir::TypeFlags> {
        if let Some(flags) = self.types_tail.get_type_flags_maybe(type_id) {
            return Some(flags);
        }

        self.types.get_type_maybe(type_id)?;

        Some(self.types.get_type_flags(type_id))
    }

    /// Return the cumulative type table visible to check.
    pub(in crate::check) fn type_table(&self) -> dir::TypeTable<'_> {
        self.types.with_tail(&self.types_tail)
    }

    /// Return one local input static visible to check.
    pub(in crate::check) fn r#static(&self, static_id: dir::LocalStaticId) -> &dir::StaticTerm {
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
    /// Return whether one module belongs to the active checked component.
    pub(in crate::check) fn is_component_module(&self, module: ModuleId) -> bool {
        self.modules.contains_key(&module)
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
        self.modules
            .get(&symbol.module_id)
            .is_some_and(|module| module.absent_symbols.contains(&symbol))
    }

    /// Return whether one source node is inside a statically absent subtree.
    pub(in crate::check) fn is_absent(&self, node: dir::GlobalNodeIdAny) -> bool {
        let module = self.module(node.module_id);
        let view = module.view();
        let mut current = Some(node.local_id);
        while let Some(local) = current {
            if module.statics.contains_absent_root(local) {
                return true;
            }
            current = view.get_parent_any(local);
        }

        false
    }

    /// Return loaded state for one in-component module.
    pub(in crate::check) fn module(&self, module: ModuleId) -> &CheckModuleState {
        match self.modules.get(&module) {
            Some(state) => state,
            None => unreachable!("check module {module:?} was not loaded"),
        }
    }

    /// Return loaded state for one in-component module mutably.
    pub(in crate::check) fn module_mut(&mut self, module: ModuleId) -> &mut CheckModuleState {
        match self.modules.get_mut(&module) {
            Some(state) => state,
            None => unreachable!("check module {module:?} was not loaded"),
        }
    }

    /// Move loaded state for one in-component module out of check state.
    pub(in crate::check) fn take_module(&mut self, module: ModuleId) -> CheckModuleState {
        self.modules
            .swap_remove(&module)
            .unwrap_or_else(|| unreachable!("check module {module:?} was not loaded"))
    }

    /// Return one source node type without flow narrowing, if present.
    pub(in crate::check) fn committed_node_type(
        &self,
        node: dir::GlobalNodeIdAny,
    ) -> Option<dir::GlobalTypeId> {
        self.node_types.get(&node).copied()
    }

    /// Return one source node type without flow narrowing.
    pub(in crate::check) fn node_type(
        &self,
        node: dir::GlobalNodeIdAny,
    ) -> CompilerResult<Answer<dir::GlobalTypeId>> {
        match self.committed_node_type(node) {
            Some(ty) => Ok(Answer::Ready(ty)),
            None => Ok(Answer::pending([Dependency::NodeType(node)])),
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
            let mut open = Vec::new();
            for index in 0..self.solver.variable_count() {
                let variable = dir::TypeVariableId(index as u32);
                if let Ok(record) = self.solver.variable(variable)
                    && record.state.is_open()
                {
                    let default = self
                        .solver
                        .variables
                        .variable_default(variable)
                        .map(|default| self.format_type(default));
                    open.push(format!(
                        "{variable:?} origin={:?} bounds={}/{} default={default:?}",
                        record.origin, record.lower.count, record.upper.count,
                    ));
                }
            }

            return Err(CompilerError::Internal {
                message: format!(
                    "required node has no checked type: {}; decision={:?}; open variables: {}",
                    self.node_label(node),
                    self.decisions.kind(node),
                    open.join("; "),
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
        if let Some(previous) = self.committed_node_type(node) {
            if previous == ty {
                return Ok(());
            }

            let previous = self.format_type(previous);
            let ty = self.format_type(ty);
            let node = self.node_label(node);

            return Err(CompilerError::Internal {
                message: format!("check node {node} received two types: {previous} and {ty}"),
            });
        }

        self.node_types.insert(node, ty);
        for waiter in self.solver.wake(Dependency::NodeType(node)) {
            self.queue_task(waiter);
        }

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

        let ty = self.intern_type(node.module_id, dir::Type::Error)?;
        self.commit_node_type(node, ty)?;

        Ok(ty)
    }

    /// Return one source node's recorded flow site.
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

    /// Return one component declaration type, if present.
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
        for waiter in self.solver.wake(Dependency::SymbolType(symbol)) {
            self.queue_task(waiter);
        }

        Ok(())
    }

    /// Return one component binding type, if present.
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
        for waiter in self.solver.wake(Dependency::SymbolType(symbol)) {
            self.queue_task(waiter);
        }

        Ok(())
    }

    /// Bind one symbol to an exact checked type.
    pub(in crate::check) fn bind_symbol_type(
        &mut self,
        symbol: dir::GlobalSymbolId,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let is_binding = self.symbol_kind(symbol) == dir::SymbolKind::Variable;
        let existing = if is_binding {
            self.binding_type_maybe(symbol)
        } else {
            self.declaration_type_maybe(symbol)
        };

        if let Some(existing) = existing {
            let origin = self.intern_origin(Origin::Symbol(symbol));
            let origin = self.solver.origin(origin);
            let cause = self.intern_cause(Cause::root(origin, CauseKind::Expression));
            self.push_constraint(Constraint::r#type(origin, Relation::Equal, existing, ty, cause));

            return Ok(existing);
        }

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
    ) -> CompilerResult<Answer<dir::GlobalTypeId>> {
        if !self.modules.contains_key(&symbol.module_id) {
            self.import_external_module(symbol.module_id)?;
        }

        if let Some(ty) = self.symbol_type_maybe(symbol) {
            return Ok(Answer::Ready(ty));
        }

        // external tables are sealed, so absence can never wake a waiter
        if !self.is_component_module(symbol.module_id) {
            return Err(CompilerError::Internal {
                message: format!(
                    "external symbol {} has no committed type\n{}",
                    self.format_symbol(symbol),
                    std::backtrace::Backtrace::force_capture(),
                ),
            });
        }

        Ok(Answer::pending([Dependency::SymbolType(symbol)]))
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
    ) -> CompilerResult<Answer<Option<dir::GlobalTypeId>>> {
        if let Some(symbol) = member.type_symbol() {
            return match self.symbol_type(symbol)? {
                Answer::Ready(ty) => Ok(Answer::Ready(Some(ty))),
                Answer::Pending(blockers) => Ok(Answer::Pending(blockers)),
            };
        }

        Ok(Answer::Ready(member.value_type()))
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

    /// Return the inferred static value of one source symbol.
    pub(in crate::check) fn static_value(
        &self,
        symbol: dir::GlobalSymbolId,
    ) -> Option<dir::GlobalTypeId> {
        self.modules
            .get(&symbol.module_id)
            .and_then(|module| module.static_values.get(&symbol).copied())
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
        if let Some(module) = self.modules.get(&module) {
            &module.resolved
        } else {
            &self.external_module(module).resolved
        }
    }

    /// Return one module's tree view, loaded or external.
    pub(in crate::check) fn module_view(&self, module: ModuleId) -> dir::View<'_> {
        if let Some(module) = self.modules.get(&module) {
            module.view()
        } else {
            self.external_module(module).view()
        }
    }

    /// Return one binding table by module.
    pub(in crate::check) fn binding_table(&self, module: ModuleId) -> dir::BindingTable<'_> {
        if let Some(module) = self.modules.get(&module) {
            module.binding_table()
        } else {
            self.external_module(module).bindings.clone()
        }
    }

    /// Return one component or external static.
    pub(in crate::check) fn r#static(&self, value: dir::GlobalStaticId) -> &dir::StaticTerm {
        if let Some(module) = self.modules.get(&value.module_id) {
            module.r#static(value.local_id)
        } else {
            self.external_module(value.module_id)
                .statics
                .get_static(value.local_id)
        }
    }

    /// Return the declaration kind for one symbol.
    pub(in crate::check) fn symbol_kind(&self, symbol: dir::GlobalSymbolId) -> dir::SymbolKind {
        let binding_table = self.binding_table(symbol.module_id);
        let symbol = binding_table.get_symbol(symbol.local_id);

        symbol.kind
    }
}
