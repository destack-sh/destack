use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::check::{
    CheckState, Condition, Constraint, ConstraintCause, FlowState, GenericInductionParameter,
    Origin, Relation, Task, Widening,
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
        }

        Ok(())
    }

    /// Return one node's checked type, opening a slot when selection will fill it later.
    pub(in crate::check) fn node_type<T: dir::Node>(
        &mut self,
        id: dir::LocalNodeId<T>,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let node = id.into_global_any(self.module);
        if let Some(ty) = self.check.node_type_maybe(node) {
            return Ok(ty);
        }

        self.open_inferred_node_type(id, Widening::Preserve)
    }

    /// Open one inferred type as one node's checked type.
    pub(in crate::check) fn open_inferred_node_type<T: dir::Node>(
        &mut self,
        id: dir::LocalNodeId<T>,
        widening: Widening,
    ) -> CompilerResult<dir::GlobalTypeId> {
        self.open_inferred_node_type_any(id.into_any(), widening)
    }

    /// Open one inferred type as one node's checked type.
    pub(in crate::check) fn open_inferred_node_type_any(
        &mut self,
        id: dir::LocalNodeIdAny,
        widening: Widening,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let node = id.into_global(self.module);
        if let Some(ty) = self.check.node_type_maybe(node) {
            return Ok(ty);
        }

        // create the explicitly requested node hole
        let origin = Origin::Node(node);
        let variable = self.check.allocate_variable(self.module, origin, widening);
        let ty = self.check.push_variable_type(variable, id)?;
        self.check.set_node_type(node, ty)?;

        Ok(ty)
    }

    /// Open one inference type at a source node.
    pub(in crate::check) fn open_type(
        &mut self,
        source: dir::LocalNodeIdAny,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let origin = Origin::Node(source.into_global(self.module));
        let variable = self
            .check
            .allocate_variable(self.module, origin, Widening::Preserve);

        self.check.push_variable_type(variable, source)
    }

    /// Return the effective type for one value expression, preferring
    /// the active flow narrowing over the node's own type.
    pub(in crate::check) fn expression_type(
        &mut self,
        id: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<dir::GlobalTypeId> {
        if let Some(narrowed) = self.flow_path_narrowing(id) {
            return Ok(narrowed);
        }

        self.node_type(id)
    }

    /// Bind one node's own type to an exact type.
    ///
    /// When no node type exists yet, the exact type becomes the stable node entry.
    /// When a node type already exists, the existing entry is equated with the exact type.
    pub(in crate::check) fn bind_node_type<T: dir::Node>(
        &mut self,
        id: dir::LocalNodeId<T>,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let node = id.into_global_any(self.module);
        let Some(target) = self.check.node_type_maybe(node) else {
            self.check.set_node_type(node, ty)?;

            return Ok(ty);
        };

        self.relate_type(Origin::Node(node), Relation::Equal, target, ty);

        Ok(target)
    }

    /// Copy one checked node type into another node.
    pub(in crate::check) fn copy_node_type<T: dir::Node, U: dir::Node>(
        &mut self,
        target: dir::LocalNodeId<T>,
        source: dir::LocalNodeId<U>,
    ) -> CompilerResult<()> {
        let ty = self.node_type(source)?;
        self.bind_node_type(target, ty)?;

        Ok(())
    }

    /// Expect one node's type to flow into a contextual type.
    pub(in crate::check) fn expect_assignable<T: dir::Node>(
        &mut self,
        id: dir::LocalNodeId<T>,
        expected: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let node = id.into_global_any(self.module);
        let target = self.node_type(id)?;
        self.push_relation(
            Origin::Node(node),
            ConstraintCause::Annotation,
            Relation::Assignable,
            target,
            expected,
        );

        Ok(target)
    }

    /// Collect one relation constraint under the active static guard.
    pub(in crate::check) fn relate_type(
        &mut self,
        origin: Origin,
        relation: Relation,
        left: dir::GlobalTypeId,
        right: dir::GlobalTypeId,
    ) {
        self.push_relation(origin, ConstraintCause::Inference, relation, left, right);
    }

    /// Collect one relation constraint with its failure context.
    pub(in crate::check) fn push_relation(
        &mut self,
        origin: Origin,
        cause: ConstraintCause,
        relation: Relation,
        left: dir::GlobalTypeId,
        right: dir::GlobalTypeId,
    ) {
        let condition = self.flow.active_static_guard();
        self.check.push_constraint(Constraint {
            relation,
            left,
            right,
            origin,
            condition,
            cause,
        });
    }

    /// Queue one node decision under the active static guard.
    pub(in crate::check) fn queue_decision(
        &mut self,
        node: dir::GlobalNodeIdAny,
    ) -> CompilerResult<()> {
        // record the guard context the decision must run under
        if let Condition::When(predicates) = self.flow.active_static_guard() {
            self.check.set_node_condition(node, predicates);
        }

        self.check.queue_task(Task::Decide(node));

        Ok(())
    }

    /// Return one symbol's checked type, opening a slot for local forward references.
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
            self.declaration_type(symbol)?
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

    /// Return one declaration type, opening a slot for recursive and forward references.
    fn declaration_type(
        &mut self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        // open declaration types on demand for recursive and forward references
        let origin = Origin::Symbol(symbol);
        let source = self
            .check
            .module(symbol.module_id)
            .symbol_declaration_node(symbol.local_id)?;
        let variable = self
            .check
            .allocate_variable(symbol.module_id, origin, Widening::Preserve);
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
        let is_binding = self.check.symbol_kind(symbol) == dir::SymbolKind::Variable;
        let existing = if is_binding {
            self.check.binding_type_maybe(symbol)
        } else {
            self.check.declaration_type_maybe(symbol)
        };

        if let Some(existing) = existing {
            self.relate_type(Origin::Symbol(symbol), Relation::Equal, existing, ty);

            return Ok(existing);
        }

        if is_binding {
            self.check.set_binding_type(symbol, ty)?;
        } else {
            self.check.set_declaration_type(symbol, ty)?;
        }

        Ok(ty)
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
    pub(in crate::check) fn union_type(
        &mut self,
        elements: impl IntoIterator<Item = dir::GlobalTypeId>,
        source: dir::LocalNodeIdAny,
    ) -> CompilerResult<dir::GlobalTypeId> {
        self.check.union_type(self.module, elements, source)
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

        self.union_type([ty, undefined], source)
    }

    /// Return the predicates of the active static guard.
    pub(in crate::check) fn guard_predicates(&self) -> SmallVec<[dir::GlobalTypeId; 2]> {
        match self.flow.active_static_guard() {
            Condition::Always => SmallVec::new(),
            Condition::When(predicates) => predicates,
        }
    }
}
