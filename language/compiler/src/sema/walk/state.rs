use destack_dir as dir;
use destack_source::ModuleId;

use crate::sema::{
    Cause, CauseKind, CheckState, Expectation, FlowSite, FlowState, InducedParameterOwner, Origin,
    Relation, RelationCheck, ValueUse, VariableKind,
};
use crate::{CheckError, CompilerResult};

/// State used only while walking one module.
pub(in crate::sema) struct WalkState<'check, 'state> {
    /// The module check state being populated.
    pub(in crate::sema) check: &'check mut CheckState<'state>,
    /// The visible DIR tree being walked.
    pub(in crate::sema) tree: dir::View<'check>,
    /// The module being walked.
    pub(in crate::sema) module: ModuleId,
    /// The elision rule for borrow regions in the active type position.
    pub(in crate::sema) elision: ElisionSite,
    /// The declaration receiving the parameters induced by the active signature walk.
    pub(in crate::sema) induced_owner: Option<InducedParameterOwner>,
    /// The function type whose own binder receives the borrow regions of the active walk.
    pub(in crate::sema) binder_owner: Option<InducedParameterOwner>,
    /// Whether this walk runs inside a body, where value reads check fixed requirements.
    pub(in crate::sema) is_body: bool,
    /// Elided borrow regions tracked by the active return type.
    elided_return_regions: Vec<dir::GlobalTypeId>,
    /// Conditional extends clauses enclosing the active type position.
    pub(in crate::sema) extends_clauses: u32,
}

/// The rule for elided borrow regions, one per walked type position.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::sema) enum ElisionSite {
    /// Signature positions induce one hidden parameter per elided memory component.
    Signature,
    /// Return positions track one placeholder for the return election rule.
    Return,
    /// Body positions leave regions to inference.
    Body,
    /// Module binding positions borrow local storage with a static extent.
    Module,
    /// Member positions induce parameters on the enclosing declaration.
    Member,
}

impl<'state> std::ops::Deref for WalkState<'_, 'state> {
    type Target = CheckState<'state>;

    fn deref(&self) -> &Self::Target {
        self.check
    }
}

impl std::ops::DerefMut for WalkState<'_, '_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.check
    }
}

impl<'check, 'state> WalkState<'check, 'state> {
    /// Create walk state for one module.
    pub(in crate::sema) fn new(
        module: ModuleId,
        tree: dir::View<'check>,
        check: &'check mut CheckState<'state>,
    ) -> Self {
        Self {
            check,
            tree,
            module,
            elision: ElisionSite::Signature,
            induced_owner: None,
            binder_owner: None,
            is_body: false,
            elided_return_regions: Vec::new(),
            extends_clauses: 0,
        }
    }

    /// Walk body positions, where declared value reads check requirements.
    pub(in crate::sema) fn for_body(mut self) -> Self {
        self.is_body = true;

        self
    }

    /// Return the symbol one declaration node of the walked module declares.
    pub(in crate::sema) fn declared_symbol(
        &self,
        node: dir::LocalNodeIdAny,
    ) -> Option<dir::GlobalSymbolId> {
        self.check.module(self.module).declaration_symbol(node)
    }

    /// Return flow state for the active module.
    pub(in crate::sema) fn flow(&self) -> &FlowState {
        &self.check.flow
    }

    /// Return mutable flow state for the active module.
    pub(in crate::sema) fn flow_mut(&mut self) -> &mut FlowState {
        &mut self.check.flow
    }

    /// Flush the cursor's durable points into the module flow table.
    pub(in crate::sema) fn flush_flows(&mut self) -> CompilerResult<()> {
        let module = self.module;
        let flow = std::mem::take(&mut self.check.flow);
        let state = self.check.module_mut(module);
        flow.sync_to(&mut state.flow_points);
        self.check.flow = flow;

        Ok(())
    }

    /// Commit one completed walk and reset the flow cursor.
    pub(in crate::sema) fn commit(mut self) -> CompilerResult<()> {
        self.flush_flows()?;
        self.check.flow.reset_cursor();

        Ok(())
    }

    /// Enter one source node occurrence at the current flow point.
    pub(in crate::sema) fn enter_node<T: dir::Node>(
        &mut self,
        id: dir::LocalNodeId<T>,
    ) -> CompilerResult<FlowSite> {
        // commit each site once and reuse it at every later visit
        self.check.visit_site(id.into_global_any(self.module))
    }

    /// Return one source node occurrence site the walk already visited.
    pub(in crate::sema) fn node_site<T: dir::Node>(
        &self,
        id: dir::LocalNodeId<T>,
    ) -> CompilerResult<FlowSite> {
        self.check.node_site(id.into_global_any(self.module))
    }

    /// Check one source node against an assignable target.
    pub(in crate::sema) fn check_assignable<T: dir::Node>(
        &mut self,
        id: dir::LocalNodeId<T>,
        target: dir::GlobalTypeId,
        kind: CauseKind,
        use_: ValueUse,
    ) -> CompilerResult<()> {
        let site = self.node_site(id)?;
        let cause = self.check.intern_cause(Cause::root(site.origin(), kind));
        let expectation = Expectation::assignable(target, cause, use_);
        self.check.check_node(site, expectation)?;

        Ok(())
    }

    /// Walk one type expression under an explicit region elision rule.
    pub(in crate::sema) fn walk_type_expression_in(
        &mut self,
        id: dir::LocalNodeId<dir::TypeExpression>,
        elision: ElisionSite,
    ) -> CompilerResult<dir::GlobalTypeId> {
        self.with_elision(elision, |walk| walk.walk_type_expression(id))
    }

    /// Run one walk step under one elision site, restoring the enclosing site after.
    pub(in crate::sema) fn with_elision<R>(
        &mut self,
        elision: ElisionSite,
        step: impl FnOnce(&mut Self) -> CompilerResult<R>,
    ) -> CompilerResult<R> {
        let previous = std::mem::replace(&mut self.elision, elision);
        let result = step(self);
        self.elision = previous;

        result
    }

    /// Walk one type annotation in a value position.
    pub(in crate::sema) fn walk_value_type(
        &mut self,
        node: dir::LocalNodeId<dir::TypeExpression>,
    ) -> CompilerResult<dir::GlobalTypeId> {
        // read the row the declare pass wrote, else walk the type here
        let ty = match self
            .check
            .declared_node_type(node.into_global_any(self.module))
        {
            Some(declared) => declared,
            None => self.walk_type_expression_type(node)?,
        };
        self.commit_node_type(node, ty)?;

        Ok(ty)
    }

    /// Walk one binding's annotation at the site its regions elide by.
    pub(in crate::sema) fn walk_binding_type(
        &mut self,
        node: dir::LocalNodeId<dir::TypeExpression>,
        is_ambient: bool,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let site = match self.flow().current_function().is_some() && !is_ambient {
            true => ElisionSite::Body,
            false => ElisionSite::Module,
        };

        self.walk_value_type_in(node, site)
    }

    /// Walk one type annotation in a value position under one elision site.
    pub(in crate::sema) fn walk_value_type_in(
        &mut self,
        node: dir::LocalNodeId<dir::TypeExpression>,
        elision: ElisionSite,
    ) -> CompilerResult<dir::GlobalTypeId> {
        self.with_elision(elision, |walk| walk.walk_value_type(node))
    }

    /// Walk one return type while tracking its elided borrow components.
    pub(in crate::sema) fn walk_return_type_expression(
        &mut self,
        id: dir::LocalNodeId<dir::TypeExpression>,
    ) -> CompilerResult<(dir::GlobalTypeId, Vec<dir::GlobalTypeId>)> {
        let first_region = self.elided_return_regions.len();
        let result = self.with_elision(ElisionSite::Return, |walk| walk.walk_value_type(id));
        let tracked = self.elided_return_regions.split_off(first_region);

        Ok((result?, tracked))
    }

    /// Induce the region of one signature-level elision.
    pub(in crate::sema) fn induce_signature_region(
        &mut self,
        source: dir::LocalNodeIdAny,
    ) -> CompilerResult<dir::GlobalTypeId> {
        self.with_elision(ElisionSite::Signature, |walk| {
            walk.induce_region(source, None)
        })
    }

    /// Return the region of one borrow with an elided region.
    pub(in crate::sema) fn elided_borrow_region(
        &mut self,
        source: dir::LocalNodeIdAny,
        payload: Option<dir::GlobalTypeId>,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let region = self.induce_region(source, payload)?;
        if self.elision == ElisionSite::Return {
            self.elided_return_regions.push(region);
        }

        Ok(region)
    }

    /// Pair one region term with the space its payload stores in.
    pub(in crate::sema) fn borrow_region(
        &mut self,
        source: dir::LocalNodeIdAny,
        region: dir::GlobalTypeId,
        payload: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        // keep a region pair and a region parameter with its storage
        if matches!(
            self.check.ty(region)?,
            dir::Type::Region(_) | dir::Type::Parameter(_)
        ) {
            return Ok(region);
        }

        // pair an extent with the space the payload stores in
        let space = self.payload_space(source, payload)?;

        self.check.intern_region(region, space)
    }

    /// Return the space term one borrowed payload stores in.
    fn payload_space(
        &mut self,
        source: dir::LocalNodeIdAny,
        payload: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let origin = Origin::Node(
            source.into_global(self.module),
            self.flow().template_scope(),
        );

        self.check.space_term(origin, payload)
    }

    /// Return the region one elided borrow takes at the active site.
    pub(in crate::sema) fn induce_region(
        &mut self,
        source: dir::LocalNodeIdAny,
        payload: Option<dir::GlobalTypeId>,
    ) -> CompilerResult<dir::GlobalTypeId> {
        // close an elided region by the site it stands in
        match self.elision {
            ElisionSite::Signature | ElisionSite::Member => self.induce_parameter_on(
                self.binder_owner.or(self.induced_owner),
                source,
                dir::MemoryParameter::Region,
            ),
            ElisionSite::Return | ElisionSite::Body => {
                self.open_memory_hole(source, dir::MemoryParameter::Region)
            }
            ElisionSite::Module => {
                let extent = self.lifetime_literal(dir::Lifetime::Static)?;
                let place = match payload {
                    Some(payload) => self.payload_space(source, payload)?,
                    None => self.check.local_space()?,
                };

                self.check.intern_region(extent, place)
            }
        }
    }

    /// Return one elided memory component in a written application slot.
    pub(in crate::sema) fn elided_memory_component(
        &mut self,
        source: dir::LocalNodeIdAny,
        kind: dir::MemoryParameter,
    ) -> CompilerResult<dir::GlobalTypeId> {
        if kind == dir::MemoryParameter::Region {
            return self.elided_borrow_region(source, None);
        }

        // signature positions induce hidden parameters
        if self.elision == ElisionSite::Signature {
            return self.induce_memory_parameter(source, kind);
        }

        // leave the component to inference elsewhere
        self.open_memory_hole(source, kind)
    }

    /// Induce one memory parameter on the active owner's template.
    pub(in crate::sema) fn induce_memory_parameter(
        &mut self,
        source: dir::LocalNodeIdAny,
        kind: dir::MemoryParameter,
    ) -> CompilerResult<dir::GlobalTypeId> {
        self.induce_parameter_on(self.induced_owner, source, kind)
    }

    /// Induce one memory parameter on one owner's template.
    fn induce_parameter_on(
        &mut self,
        owner: Option<InducedParameterOwner>,
        source: dir::LocalNodeIdAny,
        memory: dir::MemoryParameter,
    ) -> CompilerResult<dir::GlobalTypeId> {
        // leave the component to inference for a contextual signature without a declaration
        let Some(owner) = owner else {
            return self.open_memory_hole(source, memory);
        };

        // type declarations write their lifetimes
        let site = source.into_global(self.module);
        let declaration_symbol = self
            .check
            .module(owner.declaration.module_id)
            .declaration_symbol(owner.declaration.local_id);
        let is_type_declaration = match declaration_symbol {
            Some(symbol) => self.check.symbol_kind(symbol)?.is_type_definition(),
            None => false,
        };
        if is_type_declaration && memory == dir::MemoryParameter::Region {
            self.check
                .report_elided_lifetime_in_named_declaration(owner.declaration, site)?;

            return self.intern_type(dir::Type::Error);
        }

        // induce the parameter on the owner's template
        let template = self
            .check
            .open_generic_template(owner.declaration, self.flow().template_scope())?;
        let parameter = self
            .check
            .push_induced_memory_parameter(template, site, memory)?;

        self.intern_type(dir::Type::Parameter(parameter))
    }

    /// Open one memory inference hole for a body position.
    pub(in crate::sema) fn open_memory_hole(
        &mut self,
        source: dir::LocalNodeIdAny,
        kind: dir::MemoryParameter,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let origin = Origin::Node(
            source.into_global(self.module),
            self.flow().template_scope(),
        );

        self.check.open_memory_type(origin, kind)
    }

    /// Report one written `_` a declaration position cannot infer, returning the error type.
    pub(in crate::sema) fn report_declaration_hole(
        &mut self,
        source: dir::LocalNodeIdAny,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        if !self.check.is_declaring() {
            return Ok(None);
        }
        let anchor = self.check.diagnostic_anchor(self.module, source);
        let error = CheckError::CannotInferType {
            anchor,
            module: self.module,
        };
        self.check.report(self.module, error);

        Ok(Some(self.intern_type(dir::Type::Error)?))
    }

    /// Open one inference hole at a source node.
    pub(in crate::sema) fn open_type_hole(
        &mut self,
        source: dir::LocalNodeIdAny,
        kind: VariableKind,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let origin = Origin::Node(
            source.into_global(self.module),
            self.flow().template_scope(),
        );
        let variable = self.check.open_variable_of(origin, kind);

        self.check.variable_type(variable)
    }

    /// Commit one source node type.
    pub(in crate::sema) fn commit_node_type<T: dir::Node>(
        &mut self,
        id: dir::LocalNodeId<T>,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let node = id.into_global_any(self.module);
        self.check.visit_site(node)?;
        self.check.commit_node_type(node, ty)?;

        Ok(ty)
    }

    /// Collect one relation constraint.
    pub(in crate::sema) fn relate_type(
        &mut self,
        origin: Origin,
        kind: CauseKind,
        relation: Relation,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<()> {
        let cause = self.check.intern_cause(Cause::root(origin, kind));
        self.check
            .push_relation(RelationCheck::new(origin, relation, source, target, cause))?;

        Ok(())
    }

    /// Return one binding type slot.
    pub(in crate::sema) fn binding_type_slot(
        &mut self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        if let Some(ty) = self.check.binding_type_maybe(symbol) {
            return Ok(ty);
        }

        // open the symbol's variable and commit it as the binding's type
        let variable = self.check.symbol_variable(symbol);
        let ty = self.check.variable_type(variable)?;
        self.check.commit_binding_type(symbol, ty)?;

        Ok(ty)
    }

    /// Return a value type with `undefined` included.
    pub(in crate::sema) fn optional_value_type(
        &mut self,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let undefined = self.intern_type(dir::Type::Undefined)?;

        self.normalized_union_type([ty, undefined])
    }

    /// Return a defaulted value type with `undefined` removed.
    pub(in crate::sema) fn defaulted_value_type(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        self.check
            .without_union_members(origin, ty, |member| matches!(member, dir::Type::Undefined))
    }
}
