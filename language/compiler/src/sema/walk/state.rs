use destack_dir as dir;
use destack_source::ModuleId;

use crate::sema::{
    Cause, CauseKind, CheckState, Expectation, FlowSite, FlowState, InducedParameterOwner, Origin,
    Relation, RelationCheck, ValueUse, VariableKind,
};
use crate::{CheckError, CompilerError, CompilerResult};

/// State used only while walking one module.
pub(in crate::sema) struct WalkState<'check, 'state> {
    /// The module check state being populated.
    pub(in crate::sema) check: &'check mut CheckState<'state>,
    /// The visible DIR tree being walked.
    pub(in crate::sema) tree: dir::View<'check>,
    /// The module being walked.
    pub(in crate::sema) module: ModuleId,
    /// The elision rule for borrow regions in the active type position.
    region_elision: ElisionSite,
    /// The declaration receiving the parameters induced by the active signature walk.
    pub(in crate::sema) induced_owner: Option<InducedParameterOwner>,
    /// Whether this walk runs inside a body, where value reads check fixed requirements.
    pub(in crate::sema) is_body: bool,
    /// Elided borrow regions tracked by the active return type.
    elided_return_regions: Vec<dir::TypeVariableId>,
    /// Conditional extends clauses enclosing the active type position.
    pub(in crate::sema) extends_clauses: u32,
}

/// The rule for elided borrow regions, one per walked type position.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::sema) enum ElisionSite {
    /// Signature positions induce one hidden parameter per elided coordinate.
    Signature,
    /// Return positions track one placeholder for the return election rule.
    Return,
    /// Body positions close extents at the frame and leave spaces to inference.
    Body,
    /// Module binding positions read constant storage forever.
    Module,
    /// Member positions place elided components at the enclosing instance's own place.
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
            region_elision: ElisionSite::Signature,
            induced_owner: None,
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
    ///
    /// Sites commit into module state at their first visit, so
    /// syncing the point log is all a flush adds.
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

    /// Return one source node occurrence site already reached by the walk.
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
        let previous = self.region_elision;
        self.region_elision = elision;
        let result = self.walk_type_expression(id);
        self.region_elision = previous;

        result
    }

    /// Walk one return type while tracking its elided borrow components.
    pub(in crate::sema) fn walk_return_type_expression(
        &mut self,
        id: dir::LocalNodeId<dir::TypeExpression>,
    ) -> CompilerResult<(dir::GlobalTypeId, Vec<dir::TypeVariableId>)> {
        let previous = self.region_elision;
        let first_region = self.elided_return_regions.len();
        self.region_elision = ElisionSite::Return;
        let result = self.walk_type_expression(id);
        self.region_elision = previous;
        let tracked = self.elided_return_regions.split_off(first_region);

        Ok((result?, tracked))
    }

    /// Induce one region for a synthesized receiver borrow.
    pub(in crate::sema) fn induce_receiver_borrow_region(
        &mut self,
        source: dir::LocalNodeIdAny,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let previous = self.region_elision;
        self.region_elision = ElisionSite::Signature;
        let result = self.elided_borrow_region(source);
        self.region_elision = previous;

        result
    }

    /// Return the region one borrow with an elided region carries.
    pub(in crate::sema) fn elided_borrow_region(
        &mut self,
        source: dir::LocalNodeIdAny,
    ) -> CompilerResult<dir::GlobalTypeId> {
        // signatures and returns elide the whole region as one term
        match self.region_elision {
            ElisionSite::Signature | ElisionSite::Member => {
                return self.induce_memory_parameter(source, dir::MemoryParameter::Region);
            }
            ElisionSite::Return => return self.elided_borrow_extent(source),
            ElisionSite::Body | ElisionSite::Module => {}
        }
        let extent = self.elided_borrow_extent(source)?;
        let spaces = self.elided_memory_component(source, dir::MemoryParameter::Place)?;

        self.check.intern_region(extent, spaces)
    }

    /// Return the elided extent for one borrow region.
    pub(in crate::sema) fn elided_borrow_extent(
        &mut self,
        source: dir::LocalNodeIdAny,
    ) -> CompilerResult<dir::GlobalTypeId> {
        // close an elided region by the site it stands in
        match self.region_elision {
            ElisionSite::Signature | ElisionSite::Member => {
                self.induce_memory_parameter(source, dir::MemoryParameter::Region)
            }
            ElisionSite::Return => {
                let extent = self.open_memory_hole(source, dir::MemoryParameter::Region)?;
                if let Some(variable) = self.check.root_variable(extent)? {
                    self.elided_return_regions.push(variable);
                }

                Ok(extent)
            }
            ElisionSite::Body => self.lifetime_literal(dir::Lifetime::Frame),
            ElisionSite::Module => self.lifetime_literal(dir::Lifetime::Static),
        }
    }

    /// Lift one bare space term into the region holding that coordinate.
    pub(in crate::sema) fn lift_place_to_region(
        &mut self,
        source: dir::LocalNodeIdAny,
        place: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let extent = self.elided_borrow_extent(source)?;

        self.check.intern_region(extent, place)
    }

    /// Return one elided memory component in a written application slot.
    pub(in crate::sema) fn elided_memory_component(
        &mut self,
        source: dir::LocalNodeIdAny,
        kind: dir::MemoryParameter,
    ) -> CompilerResult<dir::GlobalTypeId> {
        // signature positions induce hidden parameters
        if self.region_elision == ElisionSite::Signature {
            return self.induce_memory_parameter(source, kind);
        }

        // module bindings reference constant storage
        if kind == dir::MemoryParameter::Place && self.region_elision == ElisionSite::Module {
            return self.check.place_literal(dir::Space::Constant);
        }

        // member positions project the enclosing instance's own place
        if kind == dir::MemoryParameter::Place && self.region_elision == ElisionSite::Member {
            let this = self.intern_type(dir::Type::This)?;

            return self.language_type_reference(dir::LanguageItem::PlaceOf, &[this]);
        }

        // body and return positions leave the component to inference
        self.open_memory_hole(source, kind)
    }

    /// Induce one memory parameter on the active owner's template.
    ///
    /// Signature elision writes the parameter at the elided position, so a
    /// committed declaration type never carries an inference variable.
    pub(in crate::sema) fn induce_memory_parameter(
        &mut self,
        source: dir::LocalNodeIdAny,
        kind: dir::MemoryParameter,
    ) -> CompilerResult<dir::GlobalTypeId> {
        // contextual signatures without a declaration leave the component to inference
        let Some(owner) = self.induced_owner else {
            return self.open_memory_hole(source, kind);
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
        if is_type_declaration && kind == dir::MemoryParameter::Region {
            self.check
                .report_elided_lifetime_in_named_declaration(owner.declaration, site)?;

            return self.intern_type(dir::Type::Error);
        }

        // induce the parameter on the owner's template
        let template = self.check.open_generic_template(owner.declaration)?;
        let parameter = self
            .check
            .push_induced_memory_parameter(template, site, kind)?;

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

    /// Return one symbol's type slot.
    pub(in crate::sema) fn symbol_type_slot(
        &mut self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        if let Some(ty) = self.check.adopt_symbol_type_maybe(symbol)? {
            return Ok(ty);
        }

        // report foreign value reads while declaring
        let ty = if !self.check.is_own_module(symbol.module_id) && self.check.is_declaring() {
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
        else if self.check.symbol_kind(symbol)?.is_binding() {
            self.binding_type_slot(symbol)?
        }
        // local declarations use stable declaration types
        else {
            self.declaration_type_slot(symbol)?
        };

        Ok(ty)
    }

    /// Return one imported symbol's committed type.
    fn external_symbol_type(
        &mut self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let committed = self
            .check
            .external(symbol.module_id)?
            .and_then(|external| external.types().get_symbol_type_id(symbol));
        let Some(ty) = committed else {
            return Err(CompilerError::Internal {
                message: format!("external symbol {symbol:?} has no imported type"),
            });
        };

        Ok(ty)
    }

    /// Return one declaration type slot.
    pub(in crate::sema) fn declaration_type_slot(
        &mut self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        if let Some(ty) = self.check.declaration_type_maybe(symbol) {
            return Ok(ty);
        }

        // infer declaration types when recursion and forward references need one
        let origin = Origin::Symbol(symbol);
        let variable = self.check.open_variable(origin);
        let ty = self.check.variable_type(variable)?;
        self.check.commit_declaration_type(symbol, ty)?;

        Ok(ty)
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

    /// Intern one type into this module's working segment.
    pub(in crate::sema) fn intern_type(
        &mut self,
        ty: dir::Type,
    ) -> CompilerResult<dir::GlobalTypeId> {
        self.check.intern_type(ty)
    }

    /// Intern one borrow form into this module's working segment.
    pub(in crate::sema) fn intern_borrow(
        &mut self,
        region: dir::GlobalTypeId,
        access: dir::GlobalTypeId,
    ) -> CompilerResult<dir::Form> {
        self.check.intern_borrow(region, access)
    }

    /// Intern one member projection into this module's working segment.
    pub(in crate::sema) fn intern_member(
        &mut self,
        member: dir::MemberType,
    ) -> CompilerResult<dir::GlobalTypeId> {
        self.check.intern_member(member)
    }

    /// Intern one refined application into this module's working segment.
    pub(in crate::sema) fn intern_refined(
        &mut self,
        refined: dir::RefinedType,
    ) -> CompilerResult<dir::GlobalTypeId> {
        self.check.intern_refined(refined)
    }

    /// Intern one function signature into this module's working segment.
    pub(in crate::sema) fn intern_signature(
        &mut self,
        signature: dir::FunctionSignatureType,
    ) -> CompilerResult<dir::GlobalTypeId> {
        self.check.intern_signature(signature)
    }

    /// Intern one type operation into this module's working segment.
    pub(in crate::sema) fn intern_operation(
        &mut self,
        operation: dir::TypeOperation,
    ) -> CompilerResult<dir::GlobalTypeId> {
        self.check.intern_operation(operation)
    }

    /// Intern one type id list into this module's working segment.
    pub(in crate::sema) fn intern_type_ids(
        &mut self,
        values: &[dir::GlobalTypeId],
    ) -> CompilerResult<dir::TypeListId> {
        self.check.intern_type_ids(values)
    }

    /// Intern one tuple element list into this module's working segment.
    pub(in crate::sema) fn intern_elements(
        &mut self,
        values: &[dir::TypeElement],
    ) -> CompilerResult<dir::TypeListId> {
        self.check.intern_elements(values)
    }

    /// Intern one shape property list into this module's working segment.
    pub(in crate::sema) fn intern_properties(
        &mut self,
        values: &[dir::TypeProperty],
    ) -> CompilerResult<dir::TypeListId> {
        self.check.intern_properties(values)
    }

    /// Intern one function parameter list into this module's working segment.
    pub(in crate::sema) fn intern_parameters(
        &mut self,
        values: &[dir::FunctionParameterType],
    ) -> CompilerResult<dir::TypeListId> {
        self.check.intern_parameters(values)
    }

    /// Intern one index signature list into this module's working segment.
    pub(in crate::sema) fn intern_index_signatures(
        &mut self,
        values: &[dir::TypeIndexSignature],
    ) -> CompilerResult<dir::TypeListId> {
        self.check.intern_index_signatures(values)
    }

    /// Intern one string list into this module's working segment.
    pub(in crate::sema) fn intern_strings(
        &mut self,
        values: &[destack_source::StringId],
    ) -> CompilerResult<dir::TypeListId> {
        self.check.intern_strings(values)
    }

    /// Return a normalized union type.
    pub(in crate::sema) fn normalized_union_type(
        &mut self,
        elements: impl IntoIterator<Item = dir::GlobalTypeId>,
    ) -> CompilerResult<dir::GlobalTypeId> {
        self.check.normalized_union_type(elements)
    }

    /// Return a reference type for one well-known library declaration.
    pub(in crate::sema) fn language_type_reference(
        &mut self,
        item: dir::LanguageItem,
        arguments: &[dir::GlobalTypeId],
    ) -> CompilerResult<dir::GlobalTypeId> {
        self.check.language_type(item, arguments)
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
