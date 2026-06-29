use destack_dir as dir;
use destack_source::ModuleId;

use crate::check::{
    CheckState, Constraint, ExpectedType, FlowSite, FlowState, GenericInductionParameter, Origin,
    PlaceUse, Relation, Task, ValueUse, Widening, WriteTarget,
};
use crate::{CompilerError, CompilerResult};

/// State used only while walking one module.
pub(in crate::check) struct WalkState<'check, 'state> {
    /// The component check state being populated.
    pub(in crate::check) check: &'check mut CheckState<'state>,
    /// The visible DIR tree being walked.
    pub(in crate::check) tree: dir::View<'check>,
    /// The module being walked.
    pub(in crate::check) module: ModuleId,
    /// How elided borrow lifetimes are handled in the active type position.
    borrow_lifetime_elision: BorrowLifetimeElision,
    /// Elided borrow lifetime holes tracked by the active return type.
    return_borrow_lifetimes: Vec<dir::TypeVariableId>,
    /// Flow state for the current module walk.
    flow: FlowState,
}

/// How elided borrow lifetimes are handled while walking types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) enum BorrowLifetimeElision {
    /// Elided borrow lifetimes can induce hidden generic parameters.
    Induce,
    /// Elided borrow lifetimes are tracked for a return type rule.
    TrackReturn,
    /// Elided borrow lifetimes close to the current frame.
    Frame,
}

/// Checked-position type expected for one expression occurrence.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) struct Expectation {
    /// The type expected by this check.
    pub(in crate::check) expected: ExpectedType,
    /// The relation the expression value must satisfy.
    pub(in crate::check) relation: Relation,
    /// The source that produced this expectation.
    pub(in crate::check) origin: Origin,
    /// The expected value use.
    pub(in crate::check) use_: ValueUse,
}

impl Expectation {
    /// Create an assignable value expectation.
    pub(in crate::check) fn assignable(
        target: dir::GlobalTypeId,
        origin: Origin,
        use_: ValueUse,
    ) -> Self {
        Self {
            expected: ExpectedType::Type(target),
            relation: Relation::Assignable,
            origin,
            use_,
        }
    }

    /// Create an assignable value expectation from another node's type.
    pub(in crate::check) fn assignable_node(
        target: dir::GlobalNodeIdAny,
        origin: Origin,
        use_: ValueUse,
    ) -> Self {
        Self {
            expected: ExpectedType::Node(target),
            relation: Relation::Assignable,
            origin,
            use_,
        }
    }

    /// Create an assignable value expectation from a writable place.
    pub(in crate::check) fn assignable_place(
        target: WriteTarget,
        origin: Origin,
        use_: ValueUse,
    ) -> Self {
        Self {
            expected: ExpectedType::Place(target),
            relation: Relation::Assignable,
            origin,
            use_,
        }
    }
}

impl<'check, 'state> WalkState<'check, 'state> {
    /// Create walk state for one module.
    pub(in crate::check) fn new(
        module: ModuleId,
        tree: dir::View<'check>,
        check: &'check mut CheckState<'state>,
    ) -> Self {
        Self {
            check,
            tree,
            module,
            borrow_lifetime_elision: BorrowLifetimeElision::Induce,
            return_borrow_lifetimes: Vec::new(),
            flow: FlowState::default(),
        }
    }

    /// Return flow state for the active module.
    pub(in crate::check) fn flow(&self) -> &FlowState {
        &self.flow
    }

    /// Return mutable flow state for the active module.
    pub(in crate::check) fn flow_mut(&mut self) -> &mut FlowState {
        &mut self.flow
    }

    /// Return whether elided borrow lifetimes close to frame.
    pub(in crate::check) fn borrow_lifetimes_close_to_frame(&self) -> bool {
        self.borrow_lifetime_elision == BorrowLifetimeElision::Frame
    }

    /// Move completed walk state back into check state.
    pub(in crate::check) fn finish(self) {
        let module = self.module;
        let flows = self.flow.into_points();

        self.check.module_mut(module).flows = flows;
    }

    /// Walk one return type while tracking elided borrow lifetimes.
    pub(in crate::check) fn walk_return_type_expression(
        &mut self,
        source: dir::LocalNodeIdAny,
        id: dir::LocalNodeId<dir::TypeExpression>,
        has_body: bool,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let previous = self.borrow_lifetime_elision;
        let first_tracked = self.return_borrow_lifetimes.len();
        self.borrow_lifetime_elision = BorrowLifetimeElision::TrackReturn;
        let result = self.walk_type_expression(id);
        self.borrow_lifetime_elision = previous;
        let ty = result?;

        // reject bodyless borrowed returns whose lifetime has no source
        let tracked = self.return_borrow_lifetimes.split_off(first_tracked);
        if !has_body && !tracked.is_empty() {
            self.check
                .report_ambient_lifetime_elided(self.module, source);
            for variable in tracked {
                let error = self.push_type(dir::Type::Error, source)?;
                self.check.set_solution(variable, error)?;
            }
        }

        Ok(ty)
    }

    /// Walk one type expression with elided borrow lifetimes closed to frame.
    pub(in crate::check) fn walk_frame_type_expression(
        &mut self,
        id: dir::LocalNodeId<dir::TypeExpression>,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let previous = self.borrow_lifetime_elision;
        self.borrow_lifetime_elision = BorrowLifetimeElision::Frame;
        let result = self.walk_type_expression(id);
        self.borrow_lifetime_elision = previous;

        result
    }

    /// Record one elided borrow lifetime opened while walking a type.
    pub(in crate::check) fn record_borrow_lifetime_elision(
        &mut self,
        variable: dir::TypeVariableId,
        induction: GenericInductionParameter,
    ) -> CompilerResult<()> {
        match self.borrow_lifetime_elision {
            BorrowLifetimeElision::Induce => {
                self.check.generics.insert_induction(variable, induction)?;
            }
            BorrowLifetimeElision::TrackReturn => {
                self.return_borrow_lifetimes.push(variable);
            }
            BorrowLifetimeElision::Frame => unreachable!("frame elision opens no variable"),
        }

        Ok(())
    }

    /// Return one node's checked type when it is already available.
    pub(in crate::check) fn node_type_maybe<T: dir::Node>(
        &self,
        id: dir::LocalNodeId<T>,
    ) -> Option<dir::GlobalTypeId> {
        let node = id.into_global_any(self.module);
        self.check.node_type_maybe(node)
    }

    /// Open one explicit type variable at a source node.
    pub(in crate::check) fn open_variable_type(
        &mut self,
        source: dir::LocalNodeIdAny,
        widening: Widening,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let origin = Origin::Node(source.into_global(self.module));
        let variable = self.check.allocate_variable(self.module, origin, widening);

        self.check.push_variable_type(variable, source)
    }

    /// Write one node's own type.
    pub(in crate::check) fn write_node_type<T: dir::Node>(
        &mut self,
        id: dir::LocalNodeId<T>,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let node = id.into_global_any(self.module);
        self.check.set_node_type(node, ty)?;

        Ok(ty)
    }

    /// Collect one relation constraint.
    pub(in crate::check) fn relate_type(
        &mut self,
        origin: Origin,
        relation: Relation,
        left: dir::GlobalTypeId,
        right: dir::GlobalTypeId,
    ) {
        self.check
            .push_constraint(Constraint::check(relation, left, right, origin));
    }

    /// Collect one value relation constraint.
    pub(in crate::check) fn relate_value(
        &mut self,
        origin: Origin,
        use_: ValueUse,
        relation: Relation,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) {
        self.check
            .push_constraint(Constraint::value(relation, source, target, origin, use_));
    }

    /// Queue one source-node task.
    pub(in crate::check) fn queue_node_task<T: dir::Node>(
        &mut self,
        id: dir::LocalNodeId<T>,
    ) -> CompilerResult<()> {
        self.queue_node_task_with_use(id, PlaceUse::Read)
    }

    /// Queue one source-node task with an explicit place use.
    pub(in crate::check) fn queue_node_task_with_use<T: dir::Node>(
        &mut self,
        id: dir::LocalNodeId<T>,
        use_: PlaceUse,
    ) -> CompilerResult<()> {
        let site = FlowSite {
            node: id.into_global_any(self.module),
            flow: self.flow().point(),
        };
        self.check.queue_task(Task::Infer { site, use_ });

        Ok(())
    }

    /// Queue one source-node check task.
    pub(in crate::check) fn queue_node_check<T: dir::Node>(
        &mut self,
        id: dir::LocalNodeId<T>,
        expectation: Expectation,
    ) {
        let site = FlowSite {
            node: id.into_global_any(self.module),
            flow: self.flow().point(),
        };
        self.check.queue_task(Task::Check {
            site,
            expected: expectation.expected,
            relation: expectation.relation,
            origin: expectation.origin,
            use_: expectation.use_,
        });
    }

    /// Queue one symbol binding from an initializer expression.
    pub(in crate::check) fn queue_bind(
        &mut self,
        symbol: dir::GlobalSymbolId,
        initializer: dir::LocalNodeId<dir::Expression>,
        widening: Widening,
    ) {
        self.check.queue_task(Task::Bind {
            symbol,
            initializer: initializer.into_global(self.module),
            widening,
        });
    }

    /// Mark one source-node inference task complete.
    pub(in crate::check) fn complete_node_infer<T: dir::Node>(&mut self, id: dir::LocalNodeId<T>) {
        let site = FlowSite {
            node: id.into_global_any(self.module),
            flow: self.flow().point(),
        };

        self.check.solver.complete_task(Task::Infer {
            site,
            use_: PlaceUse::Read,
        });
    }

    /// Return one symbol's checked type.
    pub(in crate::check) fn symbol_type(
        &mut self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        if let Some(ty) = self.check.symbol_type_maybe(symbol) {
            return Ok(ty);
        }

        // read committed symbols from imported components
        let ty = if !self.check.is_component_module(symbol.module_id) {
            self.external_symbol_type(symbol)?
        }
        // local variables use body-owned binding types
        else if self.check.symbol_kind(symbol) == dir::SymbolKind::Variable {
            self.binding_type(symbol, Widening::Preserve)?
        }
        // local declarations use stable declaration types
        else {
            self.declaration_type(symbol, Widening::Preserve)?
        };

        Ok(ty)
    }

    /// Return one imported symbol's committed type.
    fn external_symbol_type(
        &mut self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let symbol = self.check.resolve_symbol_alias(symbol)?;
        self.check.import_external_module(symbol.module_id)?;
        let committed = self
            .check
            .external_modules
            .get(&symbol.module_id)
            .and_then(|external| external.types.get_symbol_type_id(symbol));
        let Some(ty) = committed else {
            return Err(CompilerError::Internal {
                message: format!("external symbol {symbol:?} has no imported type"),
            });
        };

        Ok(ty)
    }

    /// Return one declaration type, inferring recursive and forward references.
    pub(in crate::check) fn declaration_type(
        &mut self,
        symbol: dir::GlobalSymbolId,
        widening: Widening,
    ) -> CompilerResult<dir::GlobalTypeId> {
        if let Some(ty) = self.check.declaration_type_maybe(symbol) {
            return Ok(ty);
        }

        // infer declaration types on demand for recursive and forward references
        let origin = Origin::Symbol(symbol);
        let source = self
            .check
            .module(symbol.module_id)
            .symbol_declaration_node(symbol.local_id)?;
        let variable = self
            .check
            .allocate_variable(symbol.module_id, origin, widening);
        let ty = self.check.push_variable_type(variable, source)?;
        self.check.set_declaration_type(symbol, ty)?;

        Ok(ty)
    }

    /// Return one binding's working type, opening a variable with the
    /// requested widening policy when missing.
    pub(in crate::check) fn binding_type(
        &mut self,
        symbol: dir::GlobalSymbolId,
        widening: Widening,
    ) -> CompilerResult<dir::GlobalTypeId> {
        if let Some(ty) = self.check.binding_type_maybe(symbol) {
            return Ok(ty);
        }

        let origin = Origin::Symbol(symbol);
        let source = self
            .check
            .module(symbol.module_id)
            .symbol_declaration_node(symbol.local_id)?;
        let variable = self
            .check
            .allocate_variable(symbol.module_id, origin, widening);
        let ty = self.check.push_variable_type(variable, source)?;
        self.check.set_binding_type(symbol, ty)?;

        Ok(ty)
    }

    /// Bind one symbol's type to an exact type.
    ///
    /// When no symbol type exists yet, the exact type becomes the stable symbol entry.
    /// When a symbol type already exists, the existing entry is equated with the exact type.
    pub(in crate::check) fn bind_symbol_type(
        &mut self,
        symbol: dir::GlobalSymbolId,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        self.check.bind_symbol_type(symbol, ty)
    }

    /// Set one symbol's static value singleton type.
    pub(in crate::check) fn set_static_value(
        &mut self,
        symbol: dir::GlobalSymbolId,
        value: dir::GlobalTypeId,
    ) -> CompilerResult<()> {
        self.check.set_static_value(symbol, value)
    }

    /// Push one working type at a source node.
    pub(in crate::check) fn push_type(
        &mut self,
        ty: dir::Type,
        source: dir::LocalNodeIdAny,
    ) -> CompilerResult<dir::GlobalTypeId> {
        self.check.push_type(self.module, ty, source)
    }

    /// Return a normalized union type.
    pub(in crate::check) fn normalized_union_type(
        &mut self,
        elements: impl IntoIterator<Item = dir::GlobalTypeId>,
        source: dir::LocalNodeIdAny,
    ) -> CompilerResult<dir::GlobalTypeId> {
        self.check
            .normalized_union_type(self.module, elements, source)
    }

    /// Return a reference type for one well-known library declaration.
    pub(in crate::check) fn language_type_reference(
        &mut self,
        source: dir::LocalNodeIdAny,
        item: dir::LanguageItem,
        arguments: Vec<dir::GlobalTypeId>,
    ) -> CompilerResult<dir::GlobalTypeId> {
        self.check
            .push_language_type(self.module, source, item, arguments)
    }

    /// Return a value type with `undefined` included.
    pub(in crate::check) fn optional_value_type(
        &mut self,
        ty: dir::GlobalTypeId,
        source: dir::LocalNodeIdAny,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let undefined = self.push_type(dir::Type::Undefined, source)?;

        self.normalized_union_type([ty, undefined], source)
    }
}
