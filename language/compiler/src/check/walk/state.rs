use destack_core::FxIndexMap;
use destack_dir as dir;
use destack_source::ModuleId;

use crate::check::{
    Cause, CauseKind, CheckState, Constraint, Expectation, FlowPointId, FlowSite, FlowState,
    Origin, Relation, ValueUse, VariableRole, Widening,
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
    node_flows: FxIndexMap<dir::GlobalNodeIdAny, FlowPointId>,
    /// Generic template assumed by each checked source node.
    node_scopes: FxIndexMap<dir::GlobalNodeIdAny, Option<dir::GlobalGenericTemplateId>>,
}

/// How elided borrow lifetimes are handled while walking types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BorrowLifetimeElision {
    /// Elided borrow lifetimes generate hidden generic parameters.
    Generate,
    /// Elided borrow lifetimes are tracked for a return type rule.
    TrackReturn,
    /// Elided borrow lifetimes close to the current frame.
    Frame,
    /// Elided borrow lifetimes close to the static lifetime.
    Static,
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
            borrow_lifetime_elision: BorrowLifetimeElision::Generate,
            return_borrow_lifetimes: Vec::new(),
            flow: FlowState::default(),
            node_flows: FxIndexMap::default(),
            node_scopes: FxIndexMap::default(),
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

    /// Commit completed walk state back into check state.
    pub(in crate::check) fn commit(self) -> CompilerResult<()> {
        let module = self.module;
        let state = self.check.module_mut(module);
        let offset = self.flow.append_to(&mut state.flows);

        // commit every node at its rebased durable flow point
        for (node, flow) in self.node_flows {
            let flow = flow.appended(offset);
            if let Some(previous) = state.node_flows.insert(node, flow) {
                return Err(CompilerError::Internal {
                    message: format!(
                        "check node {node:?} was committed at both {previous:?} and {flow:?}"
                    ),
                });
            }
        }

        // commit every node's lexical generic scope
        for (node, scope) in self.node_scopes {
            if state.node_scopes.insert(node, scope).is_some() {
                return Err(CompilerError::Internal {
                    message: format!("check node {node:?} received two generic scopes"),
                });
            }
        }

        Ok(())
    }

    /// Enter one source node occurrence at the current flow point.
    pub(in crate::check) fn enter_node<T: dir::Node>(
        &mut self,
        id: dir::LocalNodeId<T>,
    ) -> CompilerResult<FlowSite> {
        let node = id.into_global_any(self.module);
        let flow = self.flow().point();
        let scope = self.flow().template_scope();

        // reject repeated node walks
        if let Some(previous) = self.node_flows.get(&node) {
            let label = self.check.node_label(node);

            return Err(CompilerError::Internal {
                message: format!(
                    "check node {label} was walked more than once at flow {previous:?} and {flow:?}"
                ),
            });
        }

        // record the node's flow site
        self.node_flows.insert(node, flow);
        self.node_scopes.insert(node, scope);

        Ok(FlowSite { node, flow, scope })
    }

    /// Return one source node occurrence site already reached by the walk.
    pub(in crate::check) fn node_site<T: dir::Node>(
        &self,
        id: dir::LocalNodeId<T>,
    ) -> CompilerResult<FlowSite> {
        let node = id.into_global_any(self.module);
        let Some(flow) = self.node_flows.get(&node).copied() else {
            let node = self.check.node_label(node);

            return Err(CompilerError::Internal {
                message: format!("walk node {node} has no recorded runtime flow"),
            });
        };
        let Some(scope) = self.node_scopes.get(&node).copied() else {
            let node = self.check.node_label(node);

            return Err(CompilerError::Internal {
                message: format!("check node {node} has no recorded origin"),
            });
        };

        Ok(FlowSite { node, flow, scope })
    }

    /// Commit the generic template active at one source node.
    pub(in crate::check) fn commit_node_scope<T: dir::Node>(
        &mut self,
        id: dir::LocalNodeId<T>,
    ) -> CompilerResult<()> {
        let node = id.into_global_any(self.module);
        let scope = self.flow().template_scope();
        if let Some(previous) = self.node_scopes.insert(node, scope)
            && previous != scope
        {
            return Err(CompilerError::Internal {
                message: format!(
                    "check node {} received two generic scopes",
                    self.check.node_label(node)
                ),
            });
        }

        Ok(())
    }

    /// Queue one source node to satisfy an assignable target.
    pub(in crate::check) fn queue_assignable<T: dir::Node>(
        &mut self,
        id: dir::LocalNodeId<T>,
        target: dir::GlobalTypeId,
        kind: CauseKind,
        use_: ValueUse,
    ) -> CompilerResult<()> {
        let site = self.node_site(id)?;
        let cause = self.check.intern_cause(Cause::root(site.origin(), kind));
        let expectation = Expectation::assignable(target, cause, use_);
        self.check.queue_check(site, expectation);

        Ok(())
    }

    /// Walk one return type while tracking elided borrow lifetimes.
    pub(in crate::check) fn walk_return_type_expression(
        &mut self,
        id: dir::LocalNodeId<dir::TypeExpression>,
    ) -> CompilerResult<(dir::GlobalTypeId, Vec<dir::TypeVariableId>)> {
        let previous = self.borrow_lifetime_elision;
        let first_tracked = self.return_borrow_lifetimes.len();
        self.borrow_lifetime_elision = BorrowLifetimeElision::TrackReturn;
        let result = self.walk_type_expression(id);
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
        let result = self.walk_type_expression(id);
        self.borrow_lifetime_elision = previous;

        result
    }

    /// Walk one type expression with elided borrow lifetimes closed to static.
    pub(in crate::check) fn walk_static_type_expression(
        &mut self,
        id: dir::LocalNodeId<dir::TypeExpression>,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let previous = self.borrow_lifetime_elision;
        self.borrow_lifetime_elision = BorrowLifetimeElision::Static;
        let result = self.walk_type_expression(id);
        self.borrow_lifetime_elision = previous;

        result
    }

    /// Return one induced lifetime for a synthesized receiver borrow.
    pub(in crate::check) fn generated_receiver_borrow_lifetime(
        &mut self,
        source: dir::LocalNodeIdAny,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let previous = self.borrow_lifetime_elision;
        self.borrow_lifetime_elision = BorrowLifetimeElision::Generate;
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
        // ambient module bindings outlive every frame
        if self.borrow_lifetime_elision == BorrowLifetimeElision::Static {
            return self.intern_type(dir::Type::Memory(dir::MemoryLiteral::Lifetime(
                dir::Lifetime::Static,
            )));
        }

        // open one hidden lifetime parameter
        let constraint = self.memory_parameter_constraint(dir::MemoryParameter::Lifetime)?;
        let role = VariableRole::Memory {
            kind: dir::MemoryParameter::Lifetime,
            constraint,
        };
        let lifetime = self.open_type_hole(source, Widening::Never, role)?;
        let Some(variable) = self.check.root_variable(lifetime)? else {
            return Ok(lifetime);
        };

        self.commit_borrow_lifetime_elision(variable)?;

        Ok(lifetime)
    }

    /// Open one hidden memory parameter hole.
    pub(in crate::check) fn open_memory_hole(
        &mut self,
        source: dir::LocalNodeIdAny,
        kind: dir::MemoryParameter,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let constraint = self.memory_parameter_constraint(kind)?;
        let role = VariableRole::Memory { kind, constraint };

        self.open_type_hole(source, Widening::Never, role)
    }

    /// Return one memory-domain constraint type.
    fn memory_parameter_constraint(
        &mut self,
        kind: dir::MemoryParameter,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        let Some(symbol) = self.check.global.language.symbol(kind.language_item()) else {
            return Ok(None);
        };
        let arguments = self.intern_type_ids(&[])?;
        let constraint = self.intern_type(dir::Type::Application(dir::GenericApplication {
            symbol,
            arguments,
        }))?;

        Ok(Some(constraint))
    }

    /// Commit one elided borrow lifetime opened while walking a type.
    fn commit_borrow_lifetime_elision(
        &mut self,
        variable: dir::TypeVariableId,
    ) -> CompilerResult<()> {
        match self.borrow_lifetime_elision {
            BorrowLifetimeElision::Generate => {}
            BorrowLifetimeElision::TrackReturn => {
                self.return_borrow_lifetimes.push(variable);
            }
            BorrowLifetimeElision::Frame | BorrowLifetimeElision::Static => {
                return Err(CompilerError::Internal {
                    message: "closed lifetime elision cannot record a generated variable".into(),
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
        role: VariableRole,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let origin = Origin::Node(
            source.into_global(self.module),
            self.flow().template_scope(),
        );
        let variable = self.check.allocate_variable(origin, widening, role);

        self.check.variable_type(variable)
    }

    /// Commit one source node type.
    pub(in crate::check) fn commit_node_type<T: dir::Node>(
        &mut self,
        id: dir::LocalNodeId<T>,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        self.commit_node_scope(id)?;

        let node = id.into_global_any(self.module);
        self.check.commit_node_type(node, ty)?;

        Ok(ty)
    }

    /// Collect one relation constraint.
    pub(in crate::check) fn relate_type(
        &mut self,
        origin: Origin,
        kind: CauseKind,
        relation: Relation,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) {
        let cause = self.check.intern_cause(Cause::root(origin, kind));
        self.check
            .push_constraint(Constraint::r#type(origin, relation, source, target, cause));
    }

    /// Collect one generic argument bound constraint.
    pub(in crate::check) fn relate_generic_bound(
        &mut self,
        source: dir::GlobalNodeIdAny,
        application: dir::GlobalTypeId,
        parameter: dir::GlobalGenericParameterId,
        argument: dir::GlobalTypeId,
        bound: dir::GlobalTypeId,
    ) {
        let origin = Origin::Node(source, self.flow().template_scope());
        let cause = self
            .check
            .intern_cause(Cause::root(origin, CauseKind::Bound { parameter }));
        self.check.push_constraint(Constraint::generic_bound(
            origin,
            argument,
            bound,
            application,
            cause,
        ));
    }

    /// Return one symbol's type slot.
    pub(in crate::check) fn symbol_type_slot(
        &mut self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        if let Some(ty) = self.check.symbol_type_maybe(symbol) {
            return Ok(ty);
        }

        // report foreign value reads while declaring
        let ty = if !self.check.is_own_module(symbol.module_id) && self.check.is_declaration() {
            let source = self
                .check
                .walking_declarations
                .last()
                .map(|node| node.local_id)
                .unwrap_or(self.check.module.parsed.anchor_expression.into_any());
            self.check
                .report_export_type_not_derivable(self.module, source);

            self.intern_type(dir::Type::Error)?
        }
        // read committed symbols from imported modules
        else if !self.check.is_own_module(symbol.module_id) {
            self.external_symbol_type(symbol)?
        }
        // local variables use body-owned binding types
        else if self.check.symbol_kind(symbol)? == dir::SymbolKind::Variable {
            self.binding_type_slot(symbol, Widening::Never)?
        }
        // local declarations use stable declaration types
        else {
            self.declaration_type_slot(symbol, Widening::Never)?
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

        // infer declaration types when recursive and forward references need a slot
        let origin = Origin::Symbol(symbol);
        let variable = self
            .check
            .allocate_variable(origin, widening, VariableRole::Regular);
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
            .allocate_variable(origin, widening, VariableRole::Regular);
        let ty = self.check.variable_type(variable)?;
        let space = {
            let bindings = self.check.module(symbol.module_id).binding_table();

            bindings.get_symbol(symbol.local_id).binding_space
        };
        let place = space
            .map(|space| {
                self.intern_type(dir::Type::Memory(dir::MemoryLiteral::Place(
                    dir::Place::Space(space),
                )))
            })
            .transpose()?;

        // keep declared storage outside the inferred payload
        let ty = place
            .map(|place| {
                self.intern_type(dir::Type::Form(dir::FormType {
                    form: dir::Form::Placed { place },
                    value: ty,
                }))
            })
            .transpose()?
            .unwrap_or(ty);
        self.check.commit_binding_type(symbol, ty)?;

        Ok(ty)
    }

    /// Bind one symbol's type to an exact type.
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
        self.check.intern_type(ty)
    }

    /// Intern one borrow form into this module's working segment.
    pub(in crate::check) fn intern_borrow(
        &mut self,
        lifetime: dir::GlobalTypeId,
        access: dir::GlobalTypeId,
    ) -> CompilerResult<dir::Form> {
        self.check.intern_borrow(lifetime, access)
    }

    /// Intern one member projection into this module's working segment.
    pub(in crate::check) fn intern_member(
        &mut self,
        member: dir::MemberType,
    ) -> CompilerResult<dir::GlobalTypeId> {
        self.check.intern_member(member)
    }

    /// Intern one refined application into this module's working segment.
    pub(in crate::check) fn intern_refined(
        &mut self,
        refined: dir::RefinedType,
    ) -> CompilerResult<dir::GlobalTypeId> {
        self.check.intern_refined(self.module, refined)
    }

    /// Intern one function signature into this module's working segment.
    pub(in crate::check) fn intern_signature(
        &mut self,
        signature: dir::FunctionSignatureType,
    ) -> CompilerResult<dir::GlobalTypeId> {
        self.check.intern_signature(signature)
    }

    /// Intern one type operation into this module's working segment.
    pub(in crate::check) fn intern_operation(
        &mut self,
        operation: dir::TypeOperation,
    ) -> CompilerResult<dir::GlobalTypeId> {
        self.check.intern_operation(operation)
    }

    /// Intern one type id list into this module's working segment.
    pub(in crate::check) fn intern_type_ids(
        &mut self,
        values: &[dir::GlobalTypeId],
    ) -> CompilerResult<dir::TypeListId> {
        self.check.intern_type_ids(values)
    }

    /// Intern one tuple element list into this module's working segment.
    pub(in crate::check) fn intern_elements(
        &mut self,
        values: &[dir::TypeElement],
    ) -> CompilerResult<dir::TypeListId> {
        self.check.intern_elements(self.module, values)
    }

    /// Intern one shape property list into this module's working segment.
    pub(in crate::check) fn intern_properties(
        &mut self,
        values: &[dir::TypeProperty],
    ) -> CompilerResult<dir::TypeListId> {
        self.check.intern_properties(self.module, values)
    }

    /// Intern one function parameter list into this module's working segment.
    pub(in crate::check) fn intern_parameters(
        &mut self,
        values: &[dir::FunctionParameterType],
    ) -> CompilerResult<dir::TypeListId> {
        self.check.intern_parameters(values)
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
        self.check.normalized_union_type(elements)
    }

    /// Return a reference type for one well-known library declaration.
    pub(in crate::check) fn language_type_reference(
        &mut self,
        item: dir::LanguageItem,
        arguments: &[dir::GlobalTypeId],
    ) -> CompilerResult<dir::GlobalTypeId> {
        self.check.language_type(item, arguments)
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
