use destack_dir as dir;
use destack_source::ModuleId;
use indexmap::IndexMap;

use crate::check::{
    BindSource, CheckState, Constraint, ConstraintSubject, ExpectedType, FlowPointId, FlowSite,
    FlowState, GenericInductionParameter, GenericPosition, Origin, PlaceUse, Relation, Task,
    TypeConstraint, ValueUse, Widening,
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
    /// Elided borrow lifetimes tracked by the active return type.
    return_borrow_lifetimes: Vec<dir::TypeVariableId>,
    /// Flow state for the current module walk.
    flow: FlowState,
    /// Entry flow point for each source node occurrence walked in this module.
    node_flows: IndexMap<dir::GlobalNodeIdAny, (FlowPointId, Option<dir::GlobalGenericTemplateId>)>,
    /// Capture directive waiting for an immediate function value initializer.
    capture_directive: Option<dir::CaptureDirective>,
}

/// How elided borrow lifetimes are handled while walking types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BorrowLifetimeElision {
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
        target: FlowSite,
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
}

impl<'check, 'state> WalkState<'check, 'state> {
    /// Create walk state for one module.
    pub(in crate::check) fn new(
        module: ModuleId,
        tree: dir::View<'check>,
        check: &'check mut CheckState<'state>,
    ) -> Self {
        let state = check.module_mut(module);
        let flow = FlowState::from_points(std::mem::take(&mut state.flows));
        let node_flows = std::mem::take(&mut state.node_flows);

        Self {
            check,
            tree,
            module,
            borrow_lifetime_elision: BorrowLifetimeElision::Induce,
            return_borrow_lifetimes: Vec::new(),
            flow,
            node_flows,
            capture_directive: None,
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

    /// Walk one initializer with a capture directive for its immediate function value.
    pub(in crate::check) fn with_capture_directive<T>(
        &mut self,
        directive: Option<dir::CaptureDirective>,
        f: impl FnOnce(&mut Self) -> CompilerResult<T>,
    ) -> CompilerResult<T> {
        let previous = self.capture_directive.take();
        self.capture_directive = directive;
        let result = f(self);

        self.capture_directive = previous;

        result
    }

    /// Take the pending capture directive for the current function value.
    pub(in crate::check) fn take_capture_directive(&mut self) -> Option<dir::CaptureDirective> {
        self.capture_directive.take()
    }

    /// Commit completed walk state back into check state.
    pub(in crate::check) fn commit(self) {
        let module = self.module;
        let flows = self.flow.into_points();
        let node_flows = self.node_flows;

        let state = self.check.module_mut(module);
        state.flows = flows;
        state.node_flows = node_flows;
    }

    /// Return whether one declaration's implementation is a compiler intrinsic.
    ///
    /// A decorator resolving to the `intrinsic` language item carries
    /// the implementation, so such declarations need no body.
    pub(in crate::check) fn is_intrinsic<T: dir::Node>(
        &mut self,
        id: dir::LocalNodeId<T>,
    ) -> CompilerResult<bool> {
        for decorator_id in self.tree.get_decorators(id) {
            let decorator = self.tree.get(decorator_id);
            // unwrap a decorator call to its callee
            let expression = match self.tree.get(decorator.expression) {
                dir::Expression::Call { left, .. } => *left,
                _ => decorator.expression,
            };
            let source = expression.into_global_any(self.module);
            let Some(dir::Reference::Bound(symbols)) = self
                .check
                .module(self.module)
                .resolved
                .references
                .get(source)
                .cloned()
            else {
                continue;
            };
            for symbol in symbols {
                if self.check.language_item(symbol)? == Some(dir::LanguageItem::Intrinsic) {
                    return Ok(true);
                }
            }
        }

        Ok(false)
    }

    /// Enter one source node occurrence at the current flow point.
    pub(in crate::check) fn enter_node<T: dir::Node>(
        &mut self,
        id: dir::LocalNodeId<T>,
    ) -> CompilerResult<FlowSite> {
        let node = id.into_global_any(self.module);
        let flow = self.flow().point();
        let scope = self.flow().template_scope();
        if let Some((previous, _)) = self.node_flows.insert(node, (flow, scope))
            && previous != flow
        {
            let node = self.check.node_label(node);

            return Err(CompilerError::Internal {
                message: format!("check node {node} was walked under two flow sites"),
            });
        }

        Ok(FlowSite { node, flow, scope })
    }

    /// Return one source node occurrence site already reached by the walk.
    pub(in crate::check) fn node_site<T: dir::Node>(
        &self,
        id: dir::LocalNodeId<T>,
    ) -> CompilerResult<FlowSite> {
        let node = id.into_global_any(self.module);
        let Some((flow, scope)) = self.node_flows.get(&node).copied() else {
            let node = self.check.node_label(node);

            return Err(CompilerError::Internal {
                message: format!("check node {node} has no recorded flow site"),
            });
        };

        Ok(FlowSite { node, flow, scope })
    }

    /// Walk one return type while tracking elided borrow lifetimes.
    pub(in crate::check) fn walk_return_type_expression(
        &mut self,
        id: dir::LocalNodeId<dir::TypeExpression>,
    ) -> CompilerResult<(dir::GlobalTypeId, Vec<dir::TypeVariableId>)> {
        let previous = self.borrow_lifetime_elision;
        let first_tracked = self.return_borrow_lifetimes.len();
        self.borrow_lifetime_elision = BorrowLifetimeElision::TrackReturn;
        let result = self.walk_type_expression(id, GenericPosition::Annotation);
        self.borrow_lifetime_elision = previous;
        let tracked = self.return_borrow_lifetimes.split_off(first_tracked);

        Ok((result?, tracked))
    }

    /// Walk one type expression with elided borrow lifetimes closed to frame.
    pub(in crate::check) fn walk_frame_type_expression(
        &mut self,
        id: dir::LocalNodeId<dir::TypeExpression>,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let previous = self.borrow_lifetime_elision;
        self.borrow_lifetime_elision = BorrowLifetimeElision::Frame;
        let result = self.walk_type_expression(id, GenericPosition::Annotation);
        self.borrow_lifetime_elision = previous;

        result
    }

    /// Return one induced lifetime for a rung-3 receiver borrow.
    ///
    /// The synthesis runs outside type-expression walks, so the
    /// induction mode is forced for its duration.
    pub(in crate::check) fn induced_receiver_borrow_lifetime(
        &mut self,
        source: dir::LocalNodeIdAny,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let previous = self.borrow_lifetime_elision;
        self.borrow_lifetime_elision = BorrowLifetimeElision::Induce;
        let result = self.elided_borrow_lifetime(source);
        self.borrow_lifetime_elision = previous;

        result
    }

    /// Return the lifetime type for one elided borrow lifetime.
    pub(in crate::check) fn elided_borrow_lifetime(
        &mut self,
        source: dir::LocalNodeIdAny,
    ) -> CompilerResult<dir::GlobalTypeId> {
        if self.borrow_lifetime_elision == BorrowLifetimeElision::Frame {
            return self.intern_type(dir::Type::Memory(dir::MemoryLiteral::Lifetime(
                dir::Lifetime::Frame,
            )));
        }

        // open one hidden lifetime parameter
        let lifetime = self.open_type_hole(source, Widening::Preserve)?;
        let Some(variable) = self.check.root_variable(lifetime)? else {
            return Ok(lifetime);
        };

        // constrain the induced parameter to the lifetime kind
        let constraint = match self
            .check
            .environment
            .language
            .symbol(dir::LanguageItem::Lifetime)
        {
            Some(symbol) => {
                let arguments = self.intern_type_ids(&[])?;

                Some(self.intern_type(dir::Type::Instance(dir::GenericInstance {
                    symbol,
                    arguments,
                }))?)
            }
            None => None,
        };
        let induction = GenericInductionParameter {
            name_prefix: "L",
            constraint,
            is_comptime: true,
            induction: dir::GenericParameterInduction::Form,
        };
        self.commit_borrow_lifetime_elision(variable, induction)?;

        Ok(lifetime)
    }

    /// Commit one elided borrow lifetime opened while walking a type.
    fn commit_borrow_lifetime_elision(
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
            BorrowLifetimeElision::Frame => {
                return Err(CompilerError::Internal {
                    message: "frame lifetime elision cannot record an induced variable".into(),
                });
            }
        }

        Ok(())
    }

    /// Open one inference hole at a source node.
    pub(in crate::check) fn open_type_hole(
        &mut self,
        source: dir::LocalNodeIdAny,
        widening: Widening,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let origin = Origin::Node(
            source.into_global(self.module),
            self.flow().template_scope(),
        );
        let variable = self.check.allocate_variable(self.module, origin, widening);

        self.check.variable_type(variable)
    }

    /// Commit one source node type.
    pub(in crate::check) fn commit_node_type<T: dir::Node>(
        &mut self,
        id: dir::LocalNodeId<T>,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let node = id.into_global_any(self.module);
        self.check.commit_node_type(node, ty)?;

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

    /// Collect one generic argument bound constraint.
    pub(in crate::check) fn relate_generic_bound(
        &mut self,
        origin: Origin,
        source: dir::GlobalNodeIdAny,
        argument: dir::GlobalTypeId,
        bound: dir::GlobalTypeId,
    ) {
        self.check
            .push_constraint(Constraint::Check(TypeConstraint {
                relation: Relation::Satisfies,
                left: argument,
                right: bound,
                origin,
                subject: Some(ConstraintSubject::GenericArgument { source }),
            }));
    }

    /// Queue one source node task with an explicit place use.
    pub(in crate::check) fn queue_node_task<T: dir::Node>(
        &mut self,
        id: dir::LocalNodeId<T>,
        use_: PlaceUse,
    ) -> CompilerResult<()> {
        let site = self.node_site(id)?;
        self.check.queue_task(Task::Infer { site, use_ });

        Ok(())
    }

    /// Walk one expression evaluated for its value.
    pub(in crate::check) fn walk_value_expression(
        &mut self,
        id: dir::LocalNodeId<dir::Expression>,
        use_: PlaceUse,
    ) -> CompilerResult<()> {
        self.walk_expression(id, self.tree.get(id))?;
        self.queue_node_task(id, use_)
    }

    /// Queue one source node check task.
    pub(in crate::check) fn queue_node_check<T: dir::Node>(
        &mut self,
        id: dir::LocalNodeId<T>,
        expectation: Expectation,
    ) -> CompilerResult<()> {
        let site = self.node_site(id)?;
        self.check.queue_task(Task::Check {
            site,
            expected: expectation.expected,
            relation: expectation.relation,
            origin: expectation.origin,
            use_: expectation.use_,
        });

        Ok(())
    }

    /// Queue one symbol binding from an initializer expression.
    pub(in crate::check) fn queue_bind_initializer(
        &mut self,
        symbol: dir::GlobalSymbolId,
        initializer: dir::LocalNodeId<dir::Expression>,
        widening: Widening,
    ) -> CompilerResult<()> {
        let site = self.node_site(initializer)?;
        self.check.queue_task(Task::Bind {
            symbol,
            source: BindSource::Initializer { site, widening },
        });

        Ok(())
    }

    /// Queue one symbol binding from a type graph.
    pub(in crate::check) fn queue_bind_type(
        &mut self,
        symbol: dir::GlobalSymbolId,
        ty: dir::GlobalTypeId,
    ) {
        self.check.queue_task(Task::Bind {
            symbol,
            source: BindSource::Type(ty),
        });
    }

    /// Return one symbol's type slot.
    pub(in crate::check) fn symbol_type_slot(
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
            self.binding_type_slot(symbol, Widening::Preserve)?
        }
        // local declarations use stable declaration types
        else {
            self.declaration_type_slot(symbol, Widening::Preserve)?
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

    /// Return one declaration type slot.
    pub(in crate::check) fn declaration_type_slot(
        &mut self,
        symbol: dir::GlobalSymbolId,
        widening: Widening,
    ) -> CompilerResult<dir::GlobalTypeId> {
        if let Some(ty) = self.check.declaration_type_maybe(symbol) {
            return Ok(ty);
        }

        // infer declaration types on demand for recursive and forward references
        let origin = Origin::Symbol(symbol);
        let variable = self
            .check
            .allocate_variable(symbol.module_id, origin, widening);
        let ty = self.check.variable_type(variable)?;
        self.check.commit_declaration_type(symbol, ty)?;

        Ok(ty)
    }

    /// Return one binding type slot.
    pub(in crate::check) fn binding_type_slot(
        &mut self,
        symbol: dir::GlobalSymbolId,
        widening: Widening,
    ) -> CompilerResult<dir::GlobalTypeId> {
        if let Some(ty) = self.check.binding_type_maybe(symbol) {
            return Ok(ty);
        }

        let origin = Origin::Symbol(symbol);
        let variable = self
            .check
            .allocate_variable(symbol.module_id, origin, widening);
        let ty = self.check.variable_type(variable)?;
        self.check.commit_binding_type(symbol, ty)?;

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
    pub(in crate::check) fn commit_static_value(
        &mut self,
        symbol: dir::GlobalSymbolId,
        value: dir::GlobalTypeId,
    ) -> CompilerResult<()> {
        self.check.commit_static_value(symbol, value)
    }

    /// Intern one type into this module's working segment.
    pub(in crate::check) fn intern_type(
        &mut self,
        ty: dir::Type,
    ) -> CompilerResult<dir::GlobalTypeId> {
        self.check.intern_type(self.module, ty)
    }

    /// Intern one type id list into this module's working segment.
    pub(in crate::check) fn intern_type_ids(
        &mut self,
        values: &[dir::GlobalTypeId],
    ) -> CompilerResult<dir::TypeListId> {
        self.check.intern_type_ids(self.module, values)
    }

    /// Intern one tuple element list into this module's working segment.
    pub(in crate::check) fn intern_elements(
        &mut self,
        values: &[dir::TypeElement],
    ) -> CompilerResult<dir::TypeListId> {
        self.check.intern_elements(self.module, values)
    }

    /// Intern one shape field list into this module's working segment.
    pub(in crate::check) fn intern_fields(
        &mut self,
        values: &[dir::TypeField],
    ) -> CompilerResult<dir::TypeListId> {
        self.check.intern_fields(self.module, values)
    }

    /// Intern one function parameter list into this module's working segment.
    pub(in crate::check) fn intern_parameters(
        &mut self,
        values: &[dir::FunctionParameterType],
    ) -> CompilerResult<dir::TypeListId> {
        self.check.intern_parameters(self.module, values)
    }

    /// Intern one index signature list into this module's working segment.
    pub(in crate::check) fn intern_index_signatures(
        &mut self,
        values: &[dir::TypeIndexSignature],
    ) -> CompilerResult<dir::TypeListId> {
        self.check.intern_index_signatures(self.module, values)
    }

    /// Intern one string list into this module's working segment.
    pub(in crate::check) fn intern_strings(
        &mut self,
        values: &[destack_source::StringId],
    ) -> CompilerResult<dir::TypeListId> {
        self.check.intern_strings(self.module, values)
    }

    /// Return a normalized union type.
    pub(in crate::check) fn normalized_union_type(
        &mut self,
        elements: impl IntoIterator<Item = dir::GlobalTypeId>,
    ) -> CompilerResult<dir::GlobalTypeId> {
        self.check.normalized_union_type(self.module, elements)
    }

    /// Return a reference type for one well-known library declaration.
    pub(in crate::check) fn language_type_reference(
        &mut self,
        item: dir::LanguageItem,
        arguments: &[dir::GlobalTypeId],
    ) -> CompilerResult<dir::GlobalTypeId> {
        self.check.language_type(self.module, item, arguments)
    }

    /// Return a value type with `undefined` included.
    pub(in crate::check) fn optional_value_type(
        &mut self,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let undefined = self.intern_type(dir::Type::Undefined)?;

        self.normalized_union_type([ty, undefined])
    }
}
