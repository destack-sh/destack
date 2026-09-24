use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::smallvec;

use crate::sema::{
    AssignmentSelection, CandidateSource, Cause, CauseKind, Check, CheckState, FlowSite,
    MemberCandidate, MemberLookup, MemberRole, Origin, PlaceCheck, PlaceUse, Relation, StoreTarget,
    Value, Verdict, WriteMode,
};
use crate::{CompilerError, CompilerResult};

/// One member place selected from a member lookup.
pub(in crate::sema) struct MemberAssignmentSelection {
    /// The member read before an update, when required.
    pub(in crate::sema) read: Option<dir::MemberDecision>,
    /// The selected member write.
    write: dir::MemberDecision,
    /// Where the value is stored.
    store: StoreTarget,
}

impl MemberAssignmentSelection {
    /// Return the selected read and write member resolutions.
    pub(in crate::sema) fn into_resolutions(
        self,
    ) -> (Option<dir::MemberDecision>, dir::MemberDecision) {
        (self.read, self.write)
    }
}

impl CheckState<'_> {
    /// Return one checked expression value with its selected storage.
    pub(in crate::sema) fn expression_value(
        &mut self,
        site: FlowSite,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<Value> {
        self.commit_expression_place(site, ty)?;
        let place = self
            .decisions(site.node.module_id)
            .place_resolution(site.node)
            .copied();

        Ok(Value {
            ty,
            node: Some(site.node),
            place,
            is_fresh: self.is_fresh_node(site.node)?,
        })
    }

    /// Commit the addressable storage designated by one checked expression.
    pub(in crate::sema) fn commit_expression_place(
        &mut self,
        site: FlowSite,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<()> {
        if let Some(place) = self.select_expression_place(site, ty)? {
            self.commit_place(site.node, place)?;
        }

        Ok(())
    }

    /// Return the addressable storage one checked expression designates, `None` for a value.
    pub(in crate::sema) fn select_expression_place(
        &mut self,
        site: FlowSite,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<Option<dir::PlaceResolution>> {
        // designate storage from expressions only
        if site.node.local_id.ty != dir::NodeType::Expression {
            return Ok(None);
        }

        // select the place each addressable expression form designates
        let expression = site.node.into_typed::<dir::Expression>();
        let expression_kind = self
            .module(expression.module_id)
            .view()
            .get(expression.local_id)
            .clone();
        // place by the expression's own syntax
        let place = match expression_kind {
            dir::Expression::Identifier { .. } => self.binding_place(site, ty)?,
            dir::Expression::This | dir::Expression::Super => Some(self.this_place(site, ty)?),
            dir::Expression::Member { left, .. } => {
                // read a member without a decision as a namespace binding
                match self.is_stored_access(site.node) {
                    Some(true) => Some(self.project_expression_place(site, left, ty)?),
                    Some(false) => None,
                    None if self.reference_symbol(site.node)?.is_some() => {
                        self.binding_place(site, ty)?
                    }
                    None => None,
                }
            }
            dir::Expression::Index {
                left,
                index: Some(_),
                ..
            } => (self.is_stored_access(site.node) == Some(true))
                .then(|| self.project_expression_place(site, left, ty))
                .transpose()?,
            dir::Expression::Unary {
                operator: dir::UnaryOperator::Dereference,
                right,
            } => Some(self.project_expression_place(site, right, ty)?),
            _ => None,
        };

        Ok(place)
    }

    /// Return the space one binding's direct value lives in, local unless declared.
    pub(in crate::sema) fn binding_space(
        &self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<dir::Space> {
        let bindings = self.binding_table(symbol.module_id)?;
        let binding = bindings.get_symbol(symbol.local_id);

        Ok(match binding.is_shared {
            true => dir::Space::Shared,
            false => dir::Space::Local,
        })
    }

    /// Return whether the member or subscript decision at one node stores its target.
    fn is_stored_access(&self, node: dir::GlobalNodeIdAny) -> Option<bool> {
        // read the stored flag off a member, subscript, or assignment decision
        match self.decisions(node.module_id).decision(node)? {
            dir::Decision::Member(decision) => Some(decision.is_stored()),
            dir::Decision::Subscript(decision) => Some(decision.is_stored()),
            dir::Decision::Assignment(assignment) => match &assignment.write {
                dir::WriteResolution::Member(decision) => Some(decision.is_stored()),
                dir::WriteResolution::Subscript(decision) => Some(decision.is_stored()),
                _ => Some(false),
            },
            _ => None,
        }
    }

    /// Return one lexical binding place.
    fn binding_place(
        &mut self,
        site: FlowSite,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<Option<dir::PlaceResolution>> {
        let Some(symbol) = self.reference_symbol(site.node)? else {
            return Ok(None);
        };

        // body places come from binding symbols
        if self.symbol_kind(symbol).map(|kind| !kind.is_binding())? {
            return Ok(None);
        }

        // module bindings live for the program, body bindings for their frame
        let bindings = self.binding_table(symbol.module_id)?;
        let binding = bindings.get_symbol(symbol.local_id);
        let is_static = binding.scope.id == bindings.module_scope().id;
        let is_immutable = binding.binding_mutability == Some(dir::Mutability::Immutable);
        let lifetime = if is_static {
            dir::Lifetime::Static
        } else {
            dir::Lifetime::Frame
        };
        let direct_space = self.binding_space(symbol)?;

        // defer the access term while the slot stays open
        if let Some(root) = self.root_variable(ty)? {
            let origin = site.origin();
            let direct =
                self.direct_binding_place(site, ty, direct_space, lifetime, is_immutable)?;
            let access = self.open_memory_type(origin, dir::MemoryParameter::Access)?;
            let check = self.queue_check_stalled(Check::Place(PlaceCheck { site, ty }), &[root])?;
            let variable = self
                .root_variable(access)?
                .ok_or_else(|| CompilerError::Internal {
                    message: "a deferred place selection left an open term".to_string(),
                })?;
            self.set_variable_default(variable, direct.access)?;
            self.fulfill.binders.insert(variable, check);

            return Ok(Some(dir::PlaceResolution { access, ..direct }));
        }

        Ok(Some(self.direct_binding_place(
            site,
            ty,
            direct_space,
            lifetime,
            is_immutable,
        )?))
    }

    /// Bind one deferred binding place's terms to the place its closed slot selects.
    pub(in crate::sema) fn select_deferred_binding_place(
        &mut self,
        site: FlowSite,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<()> {
        // leave a poisoned slot's terms to their defaults
        let slot = self.shallow_resolve(ty)?;
        if matches!(self.ty(slot)?, dir::Type::Error) {
            return Ok(());
        }

        // select the place the closed slot reads
        let selected = self
            .binding_place(site, ty)?
            .ok_or_else(|| CompilerError::Internal {
                message: format!(
                    "deferred place {} reads no binding",
                    self.node_label(site.node)
                ),
            })?;

        // leave the terms of a discarded selection to their defaults
        let Some(deferred) = self
            .decisions(site.node.module_id)
            .place_resolution(site.node)
            .copied()
        else {
            return Ok(());
        };

        // equate the deferred access with the selected access
        let origin = site.origin();
        let cause = self.intern_cause(Cause::root(origin, CauseKind::Expression));
        self.constrain_type(
            origin,
            cause,
            Relation::Equal,
            deferred.access,
            selected.access,
        )?;

        Ok(())
    }

    /// Return the place one binding holds a direct value at.
    fn direct_binding_place(
        &mut self,
        site: FlowSite,
        ty: dir::GlobalTypeId,
        space: dir::Space,
        lifetime: dir::Lifetime,
        is_immutable: bool,
    ) -> CompilerResult<dir::PlaceResolution> {
        let place = self.root_place(site.origin(), ty, space, lifetime)?;
        let access = if is_immutable {
            self.access_literal(dir::Access::Immutable)?
        } else {
            place.access
        };

        Ok(dir::PlaceResolution { access, ..place })
    }

    /// Return the place one method receiver roots, parametric for ambient classes.
    fn this_place(
        &mut self,
        site: FlowSite,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<dir::PlaceResolution> {
        let origin = site.origin();
        let chain = self.form_chain(origin, ty)?;

        // read the place a borrowed receiver's region names
        let placement = match chain.region() {
            Some(region) => self.region_space(region)?,
            None => None,
        };
        let placement = match placement {
            Some(place) => place,
            // root another receiver at its declared space, local for every frame value
            None => {
                let symbol = match self.ty(chain.base())? {
                    dir::Type::Application(instance) => Some(instance.symbol),
                    _ => None,
                };
                let declared_space = match symbol {
                    Some(symbol) => self.nominal_space(symbol)?,
                    None => None,
                };
                let space = declared_space.unwrap_or(dir::Space::Local);

                return self.root_place(origin, ty, space, dir::Lifetime::Frame);
            }
        };
        let place = match self.literal_space(placement)? {
            Some(space) => self.root_place(origin, ty, space, dir::Lifetime::Frame)?,
            None => {
                // retain exclusive access to the direct receiver storage
                let lifetime = self.lifetime_literal(dir::Lifetime::Frame)?;
                let access = self.access_literal(dir::Access::Exclusive)?;
                let place = dir::PlaceResolution {
                    placement,
                    lifetime,
                    access,
                };

                self.project_place(origin, ty, ty, place)?
            }
        };

        Ok(place)
    }

    /// Return one root storage place.
    pub(in crate::sema) fn root_place(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
        space: dir::Space,
        lifetime: dir::Lifetime,
    ) -> CompilerResult<dir::PlaceResolution> {
        let placement = self.space_literal(space)?;
        let lifetime = self.lifetime_literal(lifetime)?;

        let access = self.access_literal(dir::Access::Exclusive)?;
        let place = dir::PlaceResolution {
            placement,
            lifetime,
            access,
        };

        self.project_place(origin, ty, ty, place)
    }

    /// Return the selected place, or create one temporary value place.
    pub(in crate::sema) fn value_place(
        &mut self,
        origin: Origin,
        value: Value,
    ) -> CompilerResult<dir::PlaceResolution> {
        if let Some(place) = value.place {
            return Ok(place);
        }

        // root the place at the value's own storage
        self.root_place(origin, value.ty, dir::Space::Local, dir::Lifetime::Frame)
    }

    /// Project one child expression place from its receiver.
    fn project_expression_place(
        &mut self,
        site: FlowSite,
        receiver: dir::LocalNodeId<dir::Expression>,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<dir::PlaceResolution> {
        let module = site.node.module_id;
        let receiver_site = self.visit_site(receiver.into_global_any(module))?;
        let (_, receiver_type) = self.infer_receiver(receiver_site)?;
        let receiver_place = match self
            .decisions(module)
            .place_resolution(receiver_site.node)
            .copied()
        {
            Some(place) => place,
            None => {
                let is_static = self.member_receiver_space(receiver_site.node, receiver_type)?
                    == dir::MemberSpace::Static;
                let lifetime = if is_static {
                    dir::Lifetime::Static
                } else {
                    dir::Lifetime::Frame
                };

                self.root_place(
                    receiver_site.origin(),
                    receiver_type,
                    dir::Space::Local,
                    lifetime,
                )?
            }
        };

        // place a member of a managed receiver in the object the handle names
        let receiver_place = if self.is_managed_value(receiver_site.origin(), receiver_type)? {
            self.project_managed_object(receiver_site.origin(), receiver_type, receiver_place)?
        } else {
            receiver_place
        };
        let place = self.project_place(site.origin(), receiver_type, ty, receiver_place)?;

        Ok(place)
    }

    /// Project one place through a checked type.
    fn project_place(
        &mut self,
        origin: Origin,
        qualifier: dir::GlobalTypeId,
        ty: dir::GlobalTypeId,
        mut place: dir::PlaceResolution,
    ) -> CompilerResult<dir::PlaceResolution> {
        let chain = self.form_chain(origin, qualifier)?;

        // read the space the handle's object declares
        let is_handle = chain.ownership_form().is_none();
        let declared = match is_handle {
            true => self.type_space(chain.base())?,
            false => None,
        };
        if let Some(space) = declared {
            place.placement = self.space_literal(space)?;
        }

        // read shared storage through a handle readonly
        if declared == Some(dir::Space::Shared) {
            place.access = self.access_literal(dir::Access::Readonly)?;
        }

        // project borrow lifetime, access, and referent place across the indirection
        if let Some(form) = chain.ownership_form()
            && let dir::Form::Borrowed(borrow) = form.form
        {
            let borrow = self.type_borrow(qualifier.module_id, borrow)?;
            place.access = self.intersect_access(origin, place.access, borrow.access)?;
            let region = self.shallow_resolve(borrow.region)?;
            match self.ty(region)? {
                dir::Type::Region(pair) => {
                    place.lifetime = pair.extent;
                    place.placement = self.normalize(origin, pair.space)?;
                }
                // a rigid or literal region is its own extent and space
                _ => {
                    place.lifetime = borrow.region;
                    place.placement = borrow.region;
                }
            }
        }

        // preserve an explicit readonly qualifier over every ownership form
        if chain.is_readonly() {
            place.access = self.access_literal(dir::Access::Readonly)?;
        }

        // let the projected value qualify its storage view further
        if qualifier != ty {
            return self.project_place(origin, ty, ty, place);
        }

        Ok(place)
    }

    /// Resolve the place one borrow lends, a strong rung granted where the referent admits it.
    pub(in crate::sema) fn borrowed_place(
        &mut self,
        origin: Origin,
        value: Value,
        requested: Option<dir::Access>,
        borrows_value: bool,
    ) -> CompilerResult<dir::PlaceResolution> {
        // borrow the value itself, else the object behind its managed handle
        let mut place = match borrows_value {
            true => self.value_place(origin, value)?,
            false => self.object_place(origin, value)?,
        };

        // grant a strong rung over aliasable storage where the stored type admits it
        let granted = self.access_of(place.access)?;
        if let Some(rung @ (dir::Access::Immutable | dir::Access::Exclusive)) = requested
            && matches!(
                (granted, rung),
                (Some(dir::Access::Mutable), _)
                    | (Some(dir::Access::Readonly), dir::Access::Immutable)
            )
        {
            let stored = match value.node {
                Some(node) => self
                    .decisions(node.module_id)
                    .narrowing(node)
                    .map(|narrowing| narrowing.union),
                None => None,
            };
            let target = self.form_chain(origin, stored.unwrap_or(value.ty))?.base();
            if self.decide_aliased_access(origin, target, rung)? == Verdict::Holds {
                place.access = self.access_literal(rung)?;
            }
        }

        Ok(place)
    }

    /// Project the place of the object one managed handle names.
    pub(in crate::sema) fn project_managed_object(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
        mut place: dir::PlaceResolution,
    ) -> CompilerResult<dir::PlaceResolution> {
        let chain = self.form_chain(origin, ty)?;
        place.placement = match self.type_space(chain.base())? {
            Some(space) => self.space_literal(space)?,
            None => place.placement,
        };
        place.lifetime = self.lifetime_literal(dir::Lifetime::Managed)?;
        place.access = self.access_literal(match chain.is_readonly() {
            true => dir::Access::Readonly,
            false => dir::Access::Mutable,
        })?;

        Ok(place)
    }

    /// Return the place one value's object lives at: behind a managed handle, else its storage.
    pub(in crate::sema) fn object_place(
        &mut self,
        origin: Origin,
        value: Value,
    ) -> CompilerResult<dir::PlaceResolution> {
        let slot = self.value_place(origin, value)?;
        match self.is_managed_value(origin, value.ty)? {
            true => self.project_managed_object(origin, value.ty, slot),
            false => Ok(slot),
        }
    }

    /// Return whether one value is read through a managed handle.
    pub(in crate::sema) fn is_managed_value(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<bool> {
        let chain = self.form_chain(origin, ty)?;

        Ok(self.form_ownership(origin, &chain)? == Some(dir::Ownership::Managed))
    }

    /// Return the strongest access permitted by both access terms.
    fn intersect_access(
        &mut self,
        origin: Origin,
        left: dir::GlobalTypeId,
        right: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        // answer one term meeting itself or exclusive directly
        let exclusive = self.access_literal(dir::Access::Exclusive)?;
        if left == right || right == exclusive {
            return Ok(left);
        }
        if left == exclusive {
            return Ok(right);
        }

        // meet two closed rungs on the access lattice
        if let (Some(left), Some(right)) = (self.access_of(left)?, self.access_of(right)?) {
            return self.access_literal(left.meet(right));
        }

        // meet an open term symbolically: equal rungs, the other side of exclusive, else readonly
        let readonly = self.access_literal(dir::Access::Readonly)?;
        let mut access = readonly;
        for (left, right, then_type) in [
            (left, right, left),
            (right, exclusive, left),
            (left, exclusive, right),
        ] {
            access =
                self.intern_operation(dir::TypeOperation::Conditional(dir::ConditionalType {
                    left,
                    right,
                    then_type,
                    else_type: access,
                    is_distributive: true,
                }))?;
        }

        self.normalize(origin, access)
    }

    /// Commit one expression's selected place.
    fn commit_place(
        &mut self,
        node: dir::GlobalNodeIdAny,
        place: dir::PlaceResolution,
    ) -> CompilerResult<()> {
        // keep the first place a node committed
        if self
            .decisions(node.module_id)
            .place_resolution(node)
            .is_some()
        {
            return Ok(());
        }

        // commit the selected place
        self.module_mut(node.module_id)
            .decisions_tail
            .set_place_resolution(node, place);

        Ok(())
    }

    /// Select one assignment target expression.
    pub(in crate::sema) fn select_assignment(
        &mut self,
        site: FlowSite,
        expression: dir::LocalNodeId<dir::Expression>,
        use_: PlaceUse,
    ) -> CompilerResult<Option<AssignmentSelection>> {
        let module = site.node.module_id;
        let source = site.node;
        let origin = site.origin();
        let expression_id = expression;
        let expression = self.module(module).view().get(expression).clone();

        // build the path by the expression's own syntax
        match expression {
            // value
            dir::Expression::Identifier { name } => {
                let path = dir::Path {
                    segments: smallvec![name],
                };

                self.select_name_assignment(source, &path, use_)
            }
            // value.member
            dir::Expression::Member {
                left,
                name: Some(name),
                ..
            } => {
                let reference = self.module(module).resolved.references.get(source);
                if reference.is_some() {
                    let Some(path) = self.module(module).view().reference_path(expression_id)
                    else {
                        return Err(CompilerError::Internal {
                            message: format!(
                                "resolved assignment name {source:?} has no reference path"
                            ),
                        });
                    };

                    return self.select_name_assignment(source, &path, use_);
                }

                let initializes = self.initializing_owner(module, left);
                let receiver_node = left.into_global_any(module);
                let receiver_site = self.visit_site(receiver_node)?;
                let (_, mut receiver) = self.infer_receiver(receiver_site)?;
                let receiver_value = self.expression_value(receiver_site, receiver)?;
                let receiver_place = self.value_place(receiver_site.origin(), receiver_value)?;
                if let Some(split) = self.split_nullish_type(origin, receiver)? {
                    self.report_possibly_nullish(origin, split.rejected.label().to_string())?;
                    receiver = split.value;
                }

                // keep a place projected through a readonly view readonly for writes
                let receiver =
                    self.readonly_write_receiver(origin, use_, receiver, receiver_place)?;
                let receiver_value = Value {
                    ty: receiver,
                    ..receiver_value
                };

                // the receiver split its nullish arms above, so the subject rejects nothing here
                let key = dir::StaticKey::Name(name);
                let (subject, _, [receiver, _]) =
                    self.resolve_member_subject(origin, receiver_node, receiver, receiver)?;
                let receiver_value = Value {
                    ty: receiver,
                    ..receiver_value
                };
                let mut lookup = self.match_member(
                    origin,
                    module,
                    receiver_value,
                    subject,
                    key,
                    dir::Access::Mutable,
                    None,
                )?;

                // project the answer onto its physical receiver arms
                self.adjust_narrowed_lookup(origin, receiver, subject.target, &mut lookup)?;

                let Some(selection) =
                    self.select_member_assignment(origin, receiver_value, key, use_, lookup)?
                else {
                    return Ok(None);
                };
                let stored_key = selection.write.stored_key();
                let store = selection.store;
                let read = selection.read.map(dir::ReadResolution::Member);
                let write = dir::WriteResolution::Member(selection.write);

                let target = Self::assignment_target(read, write, store, source, initializes);

                // commit the stored member path
                if let Some(key) = stored_key {
                    self.commit_projected_access(source, receiver_node, key)?;
                }

                Ok(Some(target))
            }
            // value[index]
            dir::Expression::Index {
                left,
                index: Some(index),
                ..
            } => {
                let initializes = self.initializing_owner(module, left);
                let receiver_node = left.into_global_any(module);
                let receiver_site = self.visit_site(receiver_node)?;
                let (_, receiver) = self.infer_receiver(receiver_site)?;
                let receiver_value = self.expression_value(receiver_site, receiver)?;
                let receiver_place = self.value_place(receiver_site.origin(), receiver_value)?;

                // keep a place projected through a readonly view readonly for writes
                let receiver =
                    self.readonly_write_receiver(origin, use_, receiver, receiver_place)?;
                let receiver_value = Value {
                    ty: receiver,
                    ..receiver_value
                };
                let index_node = index.into_global_any(module);
                let index_key = self.module(module).view().get(index).static_key();
                let index_site = self.visit_site(index_node)?;
                let index = self.infer_node_type(index_site, PlaceUse::Read)?;
                let receiver_type = self.readable_value(receiver)?;
                let space = self.member_receiver_space(receiver_node, receiver)?;

                // select the subscript operator for the receiver and index
                let selection = self.select_subscript(
                    origin,
                    module,
                    use_,
                    receiver_value,
                    receiver_type,
                    space,
                    index_node,
                    index,
                )?;

                // require a selection with a key converting across every selected runtime arm
                let selection = match selection {
                    Some(selection)
                        if self.check_subscript_key(
                            index_site,
                            index,
                            selection.key_types(),
                        )? =>
                    {
                        selection
                    }
                    _ => {
                        self.report_rejected_operator(
                            source,
                            origin,
                            "[]".to_string(),
                            &[receiver_type, index],
                            None,
                        )?;

                        return Ok(None);
                    }
                };
                let is_stored_write = selection.is_stored_write();
                let Some((read, write)) = selection.into_place() else {
                    return Ok(None);
                };

                let target =
                    Self::assignment_target(read, write, StoreTarget::Exact, source, initializes);

                // commit the stored subscript path
                if is_stored_write && let Some(key) = index_key {
                    self.commit_projected_access(source, receiver_node, key)?;
                }

                Ok(Some(target))
            }
            // *value
            dir::Expression::Unary {
                operator: dir::UnaryOperator::Dereference,
                right,
            } => {
                let receiver_node = right.into_global_any(module);
                let receiver_site = self.visit_site(receiver_node)?;
                let receiver = self.infer_node_type(receiver_site, PlaceUse::Read)?;
                let receiver_value = self.expression_value(receiver_site, receiver)?;
                let Some(write) =
                    self.select_dereference(origin, receiver_value, dir::Access::Mutable)?
                else {
                    // report the missing mutable access on a readable place
                    if self
                        .select_dereference(origin, receiver_value, dir::Access::Readonly)?
                        .is_some()
                    {
                        self.report_borrow_access_not_granted(
                            origin,
                            dir::Access::Mutable,
                            Some(dir::Access::Readonly),
                            receiver,
                        )?;
                    }

                    return Ok(None);
                };
                let read = match use_ {
                    PlaceUse::Update => {
                        let Some(read) =
                            self.select_dereference(origin, receiver_value, dir::Access::Readonly)?
                        else {
                            return Ok(None);
                        };

                        Some(dir::ReadResolution::Dereference(read))
                    }
                    PlaceUse::Write | PlaceUse::Read => None,
                };

                // record the mutable access a write through the pointer requires
                let is_aliased = self.type_is_aliased(origin, receiver)?;
                self.commit_required_access(receiver_node, dir::Access::Mutable, is_aliased);
                let write = dir::WriteResolution::Dereference(write);

                Ok(Some(AssignmentSelection {
                    read,
                    write,
                    store: StoreTarget::Exact,
                    mode: WriteMode::Direct,
                    source,
                }))
            }
            _ => Err(CompilerError::Internal {
                message: format!("assignment pattern place {source:?} is not writable"),
            }),
        }
    }

    /// Select the member place represented by one lookup.
    pub(in crate::sema) fn select_member_assignment(
        &mut self,
        origin: Origin,
        receiver: Value,
        key: dir::StaticKey,
        use_: PlaceUse,
        lookup: MemberLookup,
    ) -> CompilerResult<Option<MemberAssignmentSelection>> {
        // select one exact place for every runtime arm
        let arms = lookup.arms();
        let is_union = arms.iter().any(|group| group.arm.is_some());
        let mut reads = Vec::with_capacity(arms.len());
        let mut writes = Vec::with_capacity(arms.len());
        let mut is_optional = false;
        for group in arms {
            is_optional |= group.is_optional();
            let arm_receiver = match group.arm {
                Some(arm) => Value {
                    ty: arm.receiver,
                    ..receiver
                },
                None => receiver,
            };
            let Some((read, write)) =
                self.select_arm_write(origin, arm_receiver, key, use_, &group.candidates)?
            else {
                return Ok(None);
            };
            reads.extend(read);
            writes.push(write);
        }

        // place one arm directly; accept what every write accepts and read their union for several
        if !is_union && writes.len() == 1 {
            let write = writes.remove(0);
            let store = match is_optional {
                true => StoreTarget::Optional,
                false => StoreTarget::Exact,
            };

            return Ok(Some(MemberAssignmentSelection {
                read: reads.pop().map(dir::OperationResolution::One),
                write: dir::OperationResolution::One(write),
                store,
            }));
        }
        if is_optional {
            return Err(CompilerError::Internal {
                message: "an optional member written through several union arms".to_string(),
            });
        }
        let write_types = writes.iter().map(|write| write.ty).collect::<Vec<_>>();
        let write = dir::OperationResolution::Union {
            arms: writes,
            ty: self.normalized_intersection_type(write_types)?,
        };
        let read = if reads.is_empty() {
            None
        } else {
            let types = reads.iter().map(|read| read.ty).collect::<Vec<_>>();
            let ty = self.normalized_union_type(types)?;
            Some(dir::OperationResolution::Union { arms: reads, ty })
        };

        Ok(Some(MemberAssignmentSelection {
            read,
            write,
            store: StoreTarget::Exact,
        }))
    }

    /// Select the read and write accesses one runtime arm's candidates expose for a write.
    fn select_arm_write(
        &mut self,
        origin: Origin,
        receiver: Value,
        key: dir::StaticKey,
        use_: PlaceUse,
        candidates: &[&MemberCandidate],
    ) -> CompilerResult<Option<(Option<dir::MemberAccess>, dir::MemberAccess)>> {
        let written_key = self.format_static_key(&key);

        // split the candidates by the role each declares
        let fields = candidates
            .iter()
            .copied()
            .filter(|candidate| candidate.role == MemberRole::Field)
            .collect::<Vec<_>>();
        let getters = candidates
            .iter()
            .copied()
            .filter(|candidate| candidate.role == MemberRole::Getter)
            .collect::<Vec<_>>();
        let setters = candidates
            .iter()
            .copied()
            .filter(|candidate| candidate.role == MemberRole::Setter)
            .collect::<Vec<_>>();

        // reject a key that several candidates would write
        if fields.len() > 1 || setters.len() > 1 || (fields.len() == 1 && setters.len() == 1) {
            self.report_ambiguous_member(origin, written_key)?;

            return Ok(None);
        }

        // write a field into its own storage directly
        if let Some(field) = fields.into_iter().next() {
            // deny a write the field's declared visibility rejects
            if let Some(symbol) = field.symbol() {
                self.check_symbol_access(origin, symbol, &written_key)?;
            }

            // write a field once from its declaring constructor, else through its write access
            let read_type = field.read_type(self)?;
            let initializes = match &field.source {
                CandidateSource::Declared(declared) => {
                    self.current_initializes() == Some(declared.owner)
                }
                _ => false,
            };
            let write = match (field.access.write(), initializes, read_type) {
                (Some(write), _, _) => write,
                (None, true, Some(read)) => read,
                (None, ..) => {
                    self.report_readonly_member(origin, written_key)?;

                    return Ok(None);
                }
            };
            let read = match (use_, read_type) {
                (PlaceUse::Write, _) | (_, None) => None,
                (_, Some(ty)) => Some(field.access(receiver.ty, key, ty)),
            };
            let write = field.access(receiver.ty, key, write);

            return Ok(Some((read, write)));
        }

        // otherwise the write goes through a setter call
        let Some(setter) = setters.into_iter().next() else {
            if !getters.is_empty() {
                self.report_readonly_member(origin, written_key)?;
            }

            return Ok(None);
        };

        // deny a write the setter's declared visibility rejects
        if let Some(symbol) = setter.symbol() {
            self.check_symbol_access(origin, symbol, &written_key)?;
        }

        // reopen the winning setter at this use site
        let setter = &setter.instantiate(origin, self)?;
        let Some(call) = self.select_setter_call(origin, receiver, setter)? else {
            // report the access the refusing setter's receiver requires
            let mut requested = dir::Access::Mutable;
            if let Some(callable) = setter.callable
                && let Some(this) = self
                    .signature_head(callable)?
                    .and_then(|signature| signature.this_parameter)
                && let Some(dir::ReceiverMode::Borrowed { access, .. }) =
                    self.this_parameter_mode(this)?
            {
                requested = access;
            }
            self.report_borrow_access_not_granted(origin, requested, None, receiver.ty)?;

            return Ok(None);
        };
        let write = dir::MemberAccess::new(
            receiver.ty,
            dir::MemberTarget::Call(Box::new(call)),
            setter.access.store(),
        );
        let read = match use_ {
            PlaceUse::Update => match getters.as_slice() {
                [getter] => {
                    // deny the update's read when the getter's visibility rejects it
                    if let Some(symbol) = getter.symbol() {
                        self.check_symbol_access(origin, symbol, &written_key)?;
                    }

                    // reopen the winning getter at this use site
                    let getter = &getter.instantiate(origin, self)?;
                    let Some(call) = self.select_getter_call(origin, receiver, getter)? else {
                        return Ok(None);
                    };

                    Some(dir::MemberAccess::new(
                        receiver.ty,
                        dir::MemberTarget::Call(Box::new(call)),
                        getter.access.store(),
                    ))
                }
                [] => {
                    self.report_write_only_member(origin, written_key)?;

                    return Ok(None);
                }
                _ => {
                    self.report_ambiguous_member(origin, written_key)?;

                    return Ok(None);
                }
            },
            PlaceUse::Write => None,
            PlaceUse::Read => return Ok(None),
        };

        Ok(Some((read, write)))
    }

    /// Build one write target, a constructor initializing its own field or a direct write.
    fn assignment_target(
        read: Option<dir::ReadResolution>,
        write: dir::WriteResolution,
        store: StoreTarget,
        source: dir::GlobalNodeIdAny,
        initializes: Option<dir::GlobalSymbolId>,
    ) -> AssignmentSelection {
        let mode = match initializes {
            Some(owner) => WriteMode::Initialize { owner },
            None => WriteMode::Direct,
        };

        AssignmentSelection {
            read,
            write,
            store,
            mode,
            source,
        }
    }

    /// Return the declaration initialized through one direct constructor receiver.
    fn initializing_owner(
        &self,
        module: ModuleId,
        receiver: dir::LocalNodeId<dir::Expression>,
    ) -> Option<dir::GlobalSymbolId> {
        match self.module(module).view().get(receiver) {
            dir::Expression::This | dir::Expression::Super => self.current_initializes(),
            _ => None,
        }
    }

    /// Return one write receiver, an owned value viewed readonly where its place grants no write.
    fn readonly_write_receiver(
        &mut self,
        origin: Origin,
        use_: PlaceUse,
        receiver: dir::GlobalTypeId,
        place: dir::PlaceResolution,
    ) -> CompilerResult<dir::GlobalTypeId> {
        // read through a readonly place, an immutable one holding an owned value in its own storage
        let is_viewed = match (use_, self.access_of(place.access)?) {
            (PlaceUse::Read, _) => false,
            (_, Some(dir::Access::Readonly)) => true,
            (_, Some(dir::Access::Immutable)) => {
                self.default_ownership(origin, receiver)? == Some(dir::Ownership::Owned)
            }
            _ => false,
        };
        if !is_viewed {
            return Ok(receiver);
        }

        // view the receiver through a readonly form
        let readonly = self.intern_type(dir::Type::Form(dir::FormType {
            form: dir::Form::Readonly,
            value: receiver,
        }))?;

        Ok(readonly)
    }

    /// Select one assignment place resolved as a lexical name.
    fn select_name_assignment(
        &mut self,
        source: dir::GlobalNodeIdAny,
        path: &dir::Path,
        use_: PlaceUse,
    ) -> CompilerResult<Option<AssignmentSelection>> {
        let reference = self
            .module(source.module_id)
            .resolved
            .references
            .get(source)
            .cloned();

        // read the binding the reference names
        match reference {
            Some(dir::Reference::Bound(symbols)) => {
                let symbols = self.present_symbols(&symbols);
                let [symbol] = symbols.as_slice() else {
                    self.report_ambiguous_reference(source.module_id, source.local_id, path)?;
                    self.commit_error_node(source)?;

                    return Ok(None);
                };

                // decide the place name and commit its capture
                if self
                    .resolutions(source.module_id)
                    .name_resolution(source)
                    .is_none()
                {
                    self.capture_symbol_reference(source, *symbol)?;
                    self.commit_name(source, dir::NameResolution::new(*symbol))?;
                }

                let ty = self.symbol_type(*symbol)?;
                let read = (use_ != PlaceUse::Write).then_some(dir::ReadResolution::Binding {
                    symbol: *symbol,
                    ty,
                });
                let write = dir::WriteResolution::Binding {
                    symbol: *symbol,
                    ty,
                };

                // commit the binding path
                self.commit_access(source, dir::AccessPath::symbol(*symbol))?;

                Ok(Some(AssignmentSelection {
                    read,
                    write,
                    store: StoreTarget::Exact,
                    mode: WriteMode::Direct,
                    source,
                }))
            }
            Some(dir::Reference::Namespace { .. } | dir::Reference::TypeLiteral(_)) => {
                self.report_invalid_assignment_target(source.module_id, source.local_id);
                self.commit_error_node(source)?;

                Ok(None)
            }
            Some(dir::Reference::Ambiguous(_)) => {
                self.report_ambiguous_reference(source.module_id, source.local_id, path)?;
                self.commit_error_node(source)?;

                Ok(None)
            }
            Some(dir::Reference::Missing) | None => {
                self.report_unresolved_reference(source.module_id, source.local_id, path)?;
                self.commit_error_node(source)?;

                Ok(None)
            }
            Some(dir::Reference::Projected { .. }) => Err(CompilerError::Internal {
                message: format!("assignment name {source:?} has a projected reference"),
            }),
        }
    }
}
