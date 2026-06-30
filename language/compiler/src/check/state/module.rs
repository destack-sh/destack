use std::sync::Arc;

use destack_artifact::{
    DiagnosticBuilder, DirBound, DirExpanded, DirParsed, DirResolved, ProfileKey,
};
use destack_core::StringPool;
use destack_dir as dir;
use destack_repository::Module;
use destack_source::{ModuleId, Span};
use indexmap::{IndexMap, IndexSet};
use smallvec::SmallVec;

use crate::check::{
    Answer, Capture, CheckError, CheckState, CheckWarning, Constraint, Dependency, FlowPoint,
    FlowSite, Origin, Relation, ValueUse, answer,
};
use crate::{CompilerError, CompilerResult};

/// State owned by one module inside a checked component.
pub(in crate::check) struct CheckModuleState {
    // inherited inputs from upstream phases, read-only
    /// The requested source module.
    pub(in crate::check) module: Arc<Module>,
    /// The active target profile.
    pub(in crate::check) profile: ProfileKey,
    /// The shared string pool.
    pub(in crate::check) strings: Arc<StringPool>,
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
    pub(in crate::check) external_modules: IndexSet<ModuleId>,

    // open checked state owned by this module
    /// Open inference types layered over the expanded base.
    pub(in crate::check) types: dir::TypeSegment,
    /// Checked declaration definitions.
    pub(in crate::check) definitions: dir::DefinitionSegment,
    /// Induced generic templates and parameters.
    pub(in crate::check) generics: dir::GenericSegment,
    /// Checked static values.
    pub(in crate::check) statics: dir::StaticSegment,
    /// Checked node resolutions.
    pub(in crate::check) resolutions: dir::ResolutionSegment,
    /// Checked implicit coercions.
    pub(in crate::check) coercions: dir::CoercionSegment,
    /// Checked layout derivations.
    pub(in crate::check) layouts: dir::LayoutSegment,
    /// Checked captures.
    pub(in crate::check) capture_segment: dir::CaptureSegment,
    /// Checked annotations.
    pub(in crate::check) annotations: dir::AnnotationSegment,
    /// Inferred static symbol values, materialized to statics during write.
    pub(in crate::check) static_values: IndexMap<dir::GlobalSymbolId, dir::GlobalTypeId>,
    /// Captures discovered while walking this module.
    pub(in crate::check) captures: Vec<Capture>,
    /// Durable flow states discovered while walking this module.
    pub(in crate::check) flows: Vec<FlowPoint>,

    // statically false gates
    /// Presence decisions for decorated source nodes.
    pub(in crate::check) static_presence: IndexMap<dir::GlobalNodeIdAny, bool>,
    /// Declarations whose guards decided statically false.
    pub(in crate::check) absent_symbols: IndexSet<dir::GlobalSymbolId>,
    // diagnostics drained during write
    /// Diagnostics reported while walking this module.
    pub(in crate::check) diagnostics: Vec<DiagnosticBuilder<CheckError>>,
    /// Warnings reported while walking this module.
    pub(in crate::check) warnings: Vec<DiagnosticBuilder<CheckWarning>>,
}

impl CheckModuleState {
    /// Create module state from loaded inputs and empty working state.
    pub(in crate::check) fn new(
        module: Arc<Module>,
        profile: ProfileKey,
        strings: Arc<StringPool>,
        parsed: Arc<DirParsed>,
        bound: Arc<DirBound>,
        resolved: Arc<DirResolved>,
        expanded: Arc<DirExpanded>,
    ) -> Self {
        // create the inherited bindings and this check's open overlays
        let bindings = expanded.binding_table(&bound);
        let bindings_tail = dir::BindingSegment::from_table(&bindings);
        let types = dir::TypeSegment::from_base(&expanded.types);
        let definitions = dir::DefinitionSegment::new(module.id);
        let generics = dir::GenericSegment::new(module.id);
        let statics = dir::StaticSegment::from_base(&expanded.statics);
        let resolutions = dir::ResolutionSegment::new(module.id);
        let coercions = dir::CoercionSegment::new(module.id);
        let layouts = dir::LayoutSegment::new(module.id);
        let capture_segment = dir::CaptureSegment::new(module.id);
        let annotations = dir::AnnotationSegment::new(module.id);

        Self {
            module,
            profile,
            strings,
            parsed,
            bound,
            resolved,
            expanded,
            bindings,
            bindings_tail,
            types,
            definitions,
            generics,
            statics,
            resolutions,
            coercions,
            layouts,
            capture_segment,
            annotations,
            static_values: IndexMap::new(),
            static_presence: IndexMap::new(),
            absent_symbols: IndexSet::new(),
            external_modules: IndexSet::new(),
            captures: Vec::new(),
            flows: Vec::new(),
            diagnostics: Vec::new(),
            warnings: Vec::new(),
        }
    }

    /// Return the post-expansion DIR tree view visible to check.
    pub(in crate::check) fn view(&self) -> dir::View<'_> {
        dir::View::with_patches(
            &self.parsed.tree,
            std::slice::from_ref(&self.expanded.patch),
        )
    }

    /// Return the authored source tree used for source rendering.
    pub(in crate::check) fn source_tree(&self) -> &dir::Tree {
        &self.parsed.tree
    }

    /// Return the authored source span of one node.
    pub(in crate::check) fn authored_span(&self, node: dir::LocalNodeIdAny) -> Span {
        self.parsed.tree.source_index.get_main_or_enclosing(node.id)
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

    /// Return one open working type when this module's overlay carries it.
    pub(in crate::check) fn type_maybe(&self, type_id: dir::LocalTypeId) -> Option<&dir::Type> {
        self.types.get_type_maybe(type_id)
    }

    /// Return one local input static visible to check.
    pub(in crate::check) fn r#static(&self, static_id: dir::LocalStaticId) -> &dir::StaticTerm {
        if let Some(value) = self.expanded.statics.get_static_maybe(static_id) {
            return value;
        }

        self.bound.statics.get_static(static_id)
    }

    /// Return whether one local symbol is an imported alias.
    pub(in crate::check) fn is_import_alias(&self, symbol: dir::LocalSymbolId) -> bool {
        self.resolved.imports.symbol_target(symbol).is_some()
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

    /// Return whether one symbol's guard decided statically false.
    pub(in crate::check) fn is_absent_symbol(&self, symbol: dir::GlobalSymbolId) -> bool {
        self.modules
            .get(&symbol.module_id)
            .is_some_and(|module| module.absent_symbols.contains(&symbol))
    }

    /// Return loaded state for one module.
    pub(in crate::check) fn module(&self, module: ModuleId) -> &CheckModuleState {
        match self.modules.get(&module) {
            Some(state) => state,
            None => unreachable!("check module {module:?} was not loaded"),
        }
    }

    /// Return loaded state for one module mutably.
    pub(in crate::check) fn module_mut(&mut self, module: ModuleId) -> &mut CheckModuleState {
        match self.modules.get_mut(&module) {
            Some(state) => state,
            None => unreachable!("check module {module:?} was not loaded"),
        }
    }

    /// Move loaded state for one module out of check state.
    pub(in crate::check) fn take_module(&mut self, module: ModuleId) -> CheckModuleState {
        self.modules
            .swap_remove(&module)
            .unwrap_or_else(|| unreachable!("check module {module:?} was not loaded"))
    }

    /// Return one committed source-node type, if present.
    pub(in crate::check) fn committed_node_type_maybe(
        &self,
        node: dir::GlobalNodeIdAny,
    ) -> Option<dir::GlobalTypeId> {
        self.node_types.get(&node).copied()
    }

    /// Return one committed source-node type.
    pub(in crate::check) fn committed_node_type(
        &self,
        node: dir::GlobalNodeIdAny,
    ) -> CompilerResult<Answer<dir::GlobalTypeId>> {
        if let Some(ty) = self.committed_node_type_maybe(node) {
            Ok(Answer::Ready(ty))
        } else {
            Ok(Answer::pending([Dependency::NodeType(node)]))
        }
    }

    /// Return an invariant message for one source node.
    pub(in crate::check) fn node_message(&self, node: dir::GlobalNodeIdAny) -> String {
        let module = self.module(node.module_id);
        let view = module.view();
        let detail = match node.local_id.ty {
            dir::NodeType::Expression => {
                let id = node.into_typed::<dir::Expression>().local_id;
                format!("{:?}", view.get(id))
            }
            dir::NodeType::Pattern => {
                let id = node.into_typed::<dir::Pattern>().local_id;
                format!("{:?}", view.get(id))
            }
            dir::NodeType::AssignPattern => {
                let id = node.into_typed::<dir::AssignPattern>().local_id;
                format!("{:?}", view.get(id))
            }
            dir::NodeType::TypeExpression => {
                let id = node.into_typed::<dir::TypeExpression>().local_id;
                format!("{:?}", view.get(id))
            }
            _ => format!("{:?}", node.local_id.ty),
        };

        format!("node {node:?}: {detail}")
    }

    /// Return one committed source-node type or fail on an internal invariant break.
    pub(in crate::check) fn require_committed_node_type(
        &self,
        node: dir::GlobalNodeIdAny,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let Some(ty) = self.committed_node_type_maybe(node) else {
            return Err(CompilerError::Internal {
                message: format!(
                    "required node has no checked type: {}",
                    self.node_message(node)
                ),
            });
        };

        Ok(ty)
    }

    /// Commit one source-node type.
    pub(in crate::check) fn commit_node_type(
        &mut self,
        node: dir::GlobalNodeIdAny,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<()> {
        if let Some(previous) = self.committed_node_type_maybe(node) {
            if previous == ty {
                return Ok(());
            }

            let previous = self.format_type(previous);
            let ty = self.format_type(ty);
            let node = self.node_message(node);

            return Err(CompilerError::Internal {
                message: format!("check node {node} received two types: {previous} and {ty}"),
            });
        }

        self.node_types.insert(node, ty);

        // wake tasks parked on the node type
        for waiter in self.solver.wake(Dependency::NodeType(node)) {
            self.queue_task(waiter);
        }

        Ok(())
    }

    /// Constrain one source node's runtime value type.
    pub(in crate::check) fn constrain_node_value(
        &mut self,
        site: FlowSite,
        relation: Relation,
        target: dir::GlobalTypeId,
        origin: Origin,
        use_: ValueUse,
    ) -> CompilerResult<Answer<()>> {
        let source = answer!(self.node_type_at(site)?);
        self.push_constraint(Constraint::value(relation, source, target, origin, use_));

        Ok(Answer::Ready(()))
    }

    /// Return one component declaration type, if present.
    pub(in crate::check) fn declaration_type_maybe(
        &self,
        symbol: dir::GlobalSymbolId,
    ) -> Option<dir::GlobalTypeId> {
        self.declaration_types.get(&symbol).copied()
    }

    /// Record one declaration symbol type.
    pub(in crate::check) fn set_declaration_type(
        &mut self,
        symbol: dir::GlobalSymbolId,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<()> {
        if let Some(previous) = self.declaration_type_maybe(symbol)
            && previous != ty
        {
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

    /// Record one binding symbol type.
    pub(in crate::check) fn set_binding_type(
        &mut self,
        symbol: dir::GlobalSymbolId,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<()> {
        let previous = self.binding_type_maybe(symbol);
        if previous.is_some_and(|previous| previous != ty) {
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
            self.push_constraint(Constraint::check(
                Relation::Equal,
                existing,
                ty,
                Origin::Symbol(symbol),
            ));

            return Ok(existing);
        }

        if is_binding {
            self.set_binding_type(symbol, ty)?;
        } else {
            self.set_declaration_type(symbol, ty)?;
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

    /// Return one loaded symbol's checked type.
    pub(in crate::check) fn symbol_type(
        &self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<Answer<dir::GlobalTypeId>> {
        if let Some(ty) = self.symbol_type_maybe(symbol) {
            Ok(Answer::Ready(ty))
        } else {
            Ok(Answer::pending([Dependency::SymbolType(symbol)]))
        }
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
        &self,
        member: &dir::DefinitionMember,
    ) -> CompilerResult<Answer<Option<dir::GlobalTypeId>>> {
        if let Some(symbol) = member.symbol()
            && !matches!(member, dir::DefinitionMember::AssociatedType(_))
        {
            return match self.symbol_type(symbol)? {
                Answer::Ready(ty) => Ok(Answer::Ready(Some(ty))),
                Answer::Pending(blockers) => Ok(Answer::Pending(blockers)),
            };
        }

        let ty = match member {
            dir::DefinitionMember::AssociatedType(associated) => associated.value,
            dir::DefinitionMember::CallSignature(signature)
            | dir::DefinitionMember::ConstructSignature(signature)
            | dir::DefinitionMember::IndexSignature(signature) => Some(signature.ty),
            dir::DefinitionMember::Field(_)
            | dir::DefinitionMember::Method(_)
            | dir::DefinitionMember::AssociatedConst(_)
            | dir::DefinitionMember::Variant(_) => None,
        };

        Ok(Answer::Ready(ty))
    }

    /// Return the checked value type required for one definition member.
    pub(in crate::check) fn require_definition_member_type(
        &self,
        member: &dir::DefinitionMember,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        if let Some(symbol) = member.symbol()
            && !matches!(member, dir::DefinitionMember::AssociatedType(_))
        {
            return Ok(Some(self.require_symbol_type(symbol)?));
        }

        let ty = match member {
            dir::DefinitionMember::AssociatedType(associated) => associated.value,
            dir::DefinitionMember::CallSignature(signature)
            | dir::DefinitionMember::ConstructSignature(signature)
            | dir::DefinitionMember::IndexSignature(signature) => Some(signature.ty),
            dir::DefinitionMember::Field(_)
            | dir::DefinitionMember::Method(_)
            | dir::DefinitionMember::AssociatedConst(_)
            | dir::DefinitionMember::Variant(_) => None,
        };

        Ok(ty)
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

    /// Record the inferred static value of one source symbol as a singleton type.
    pub(in crate::check) fn set_static_value(
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
