use destack_dir as dir;
use destack_repository::ProviderError;

use super::DirModule;

impl DirModule<'_> {
    /// Return the root expression beneath one stored assignment target.
    pub(crate) fn stored_write_root(
        &self,
        target: dir::LocalNodeId<dir::Expression>,
    ) -> Result<Option<dir::LocalNodeId<dir::Expression>>, ProviderError> {
        let global = target.into_global_any(self.id);
        let resolution = self.decisions.assignment_decision(global).ok_or_else(|| {
            ProviderError::internal(format!(
                "checked assignment target {global:?} has no assignment decision"
            ))
        })?;

        // require a member or element backed by stored state
        if !resolution.write.is_stored() {
            return Ok(None);
        }

        // select the receiver that carries the written state
        let view = self.view();
        let mut receiver = match view.get(target) {
            dir::Expression::Member { left, .. } | dir::Expression::Index { left, .. } => *left,
            _ => {
                return Err(ProviderError::internal(format!(
                    "stored assignment target {global:?} is not a member or index expression"
                )));
            }
        };

        // follow stored projections to their root expression
        loop {
            let global = receiver.into_global_any(self.id);
            let left = match view.get(receiver) {
                dir::Expression::Member { left, .. } => {
                    let decision = self.decisions.member_decision(global).ok_or_else(|| {
                        ProviderError::internal(format!(
                            "checked stored projection {global:?} has no member decision"
                        ))
                    })?;
                    if !decision.is_stored() {
                        return Ok(Some(receiver));
                    }

                    *left
                }
                dir::Expression::Index { left, .. } => {
                    let decision = self.decisions.subscript_decision(global).ok_or_else(|| {
                        ProviderError::internal(format!(
                            "checked stored projection {global:?} has no subscript decision"
                        ))
                    })?;
                    if !decision.is_stored() {
                        return Ok(Some(receiver));
                    }

                    *left
                }
                _ => return Ok(Some(receiver)),
            };

            // stop where the selected state can remain visible through another reference
            let type_id = self.node_type_id(receiver.into_any())?;
            if self.dir.default_ownership(type_id)? != Some(dir::Ownership::Owned) {
                return Ok(Some(receiver));
            }

            receiver = left;
        }
    }

    /// Return whether retained uses of one binding accept a readonly borrowed value.
    pub(crate) fn binding_accepts_readonly_borrow(
        &self,
        symbol: dir::GlobalSymbolId,
        within: dir::LocalNodeIdAny,
        excluded: Option<dir::LocalNodeIdAny>,
    ) -> Result<bool, ProviderError> {
        let view = self.view();

        // reject mutation and capture of the replacement borrow
        let has_incompatible_use = self.flows.binding_occurrences().any(|occurrence| {
            occurrence.symbol == symbol
                && view.is_inside(occurrence.node, within)
                && (occurrence.uses.may_mutate()
                    || occurrence.uses.contains(dir::BindingUse::EXCLUSIVE)
                    || occurrence.uses.contains(dir::BindingUse::CAPTURE))
        });
        if has_incompatible_use {
            return Ok(false);
        }

        // require every retained direct read to use readonly borrowing
        let root = dir::AccessPath::symbol(symbol);
        for occurrence in self.flows.access_occurrences() {
            if occurrence.path != root
                || !occurrence.uses.contains(dir::BindingUse::READ)
                || !view.is_inside(occurrence.node, within)
                || excluded.is_some_and(|excluded| view.is_inside(occurrence.node, excluded))
            {
                continue;
            }
            let is_explicit_readonly = occurrence
                .node
                .try_into_typed::<dir::Expression>()
                .ok()
                .and_then(|node| view.get_parent_for(node))
                .and_then(|parent| parent.try_into_typed::<dir::Expression>().ok())
                .is_some_and(|parent| {
                    matches!(
                        view.get(parent),
                        dir::Expression::BorrowOf { mutability, .. }
                            if mutability.map(dir::Mutability::access)
                                == Some(dir::Access::Readonly)
                    )
                });
            let adjusted = self.adjusted_type_id(occurrence.node)?;
            if !is_explicit_readonly
                && self.dir.borrow_access(adjusted)? != Some(dir::Access::Readonly)
                && self.consumer_borrow_access(occurrence.node)? != Some(dir::Access::Readonly)
            {
                return Ok(false);
            }
        }

        Ok(true)
    }

    /// Return the borrow access the consuming operation requires from one place expression.
    fn consumer_borrow_access(
        &self,
        node: dir::LocalNodeIdAny,
    ) -> Result<Option<dir::Access>, ProviderError> {
        let view = self.view();
        let Ok(mut current) = node.try_into_typed::<dir::Expression>() else {
            return Ok(None);
        };

        // walk stored projections out to the operation that takes the place
        loop {
            let Some(parent) = view
                .get_parent_for(current)
                .and_then(|parent| parent.try_into_typed::<dir::Expression>().ok())
            else {
                return Ok(None);
            };
            match view.get(parent) {
                // member projections continue to their own consumer
                dir::Expression::Member { left, .. } if *left == current => {
                    let Some(dir::OperationResolution::One(access)) =
                        self.member_decision(parent)?
                    else {
                        return Ok(None);
                    };
                    match &access.target {
                        // stored projections read in place
                        dir::MemberTarget::Field(_)
                        | dir::MemberTarget::Projection { .. }
                        | dir::MemberTarget::Index(_) => current = parent,
                        // accessors take the receiver through their selected call
                        dir::MemberTarget::Call(call) => {
                            return self.call_receiver_borrow_access(call);
                        }
                        // methods take the receiver at the enclosing call
                        dir::MemberTarget::Symbol(_) | dir::MemberTarget::OverloadSet(_) => {
                            current = parent;
                        }
                        dir::MemberTarget::Intersection(_) => return Ok(None),
                    }
                }
                // calls take a member callee's receiver through the selected call
                dir::Expression::Call { left, .. } if *left == current => {
                    let Some(dir::OperationResolution::One(call)) = self.call_decision(parent)?
                    else {
                        return Ok(None);
                    };

                    return self.call_receiver_borrow_access(call);
                }
                _ => return Ok(None),
            }
        }
    }

    /// Return the receiver borrow access one selected call records.
    fn call_receiver_borrow_access(
        &self,
        call: &dir::Call,
    ) -> Result<Option<dir::Access>, ProviderError> {
        let adjustments = match &call.target {
            dir::CallableTarget::Symbol { function, .. } => match &function.receiver {
                Some(receiver) => &receiver.adjustments,
                None => return Ok(None),
            },
            dir::CallableTarget::Dynamic { dispatch, .. } => &dispatch.receiver.adjustments,
            dir::CallableTarget::Expression { .. } => return Ok(None),
        };
        for adjustment in adjustments {
            if let dir::ReceiverAdjustment::Borrow { ty } = adjustment {
                return self.dir.borrow_access(*ty);
            }
        }

        Ok(None)
    }

    /// Return the strongest access granted through one checked place.
    pub(crate) fn place_access(
        &self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> Result<Option<dir::Access>, ProviderError> {
        let view = self.view();
        let mut expression = expression;

        // follow projections to the nearest place the checker resolved
        loop {
            if let Some(place) = self
                .decisions
                .place_resolution(expression.into_global_any(self.id))
            {
                return self.dir.memory_access(place.access).map(Some);
            }

            expression = match view.get(expression) {
                dir::Expression::Member { left, .. } | dir::Expression::Index { left, .. } => *left,
                dir::Expression::Unary {
                    operator: dir::UnaryOperator::Dereference,
                    right,
                } => *right,
                _ => return Ok(None),
            };
        }
    }

    /// Return the place beneath one explicit dereference.
    pub(crate) fn dereferenced_place(
        &self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> dir::LocalNodeId<dir::Expression> {
        match self.view().get(expression) {
            dir::Expression::Unary {
                operator: dir::UnaryOperator::Dereference,
                right,
            } => *right,
            _ => expression,
        }
    }

    /// Return the stable storage selected by one checked expression.
    pub fn access_resolution(
        &self,
        node: dir::LocalNodeId<dir::Expression>,
    ) -> Option<&dir::AccessResolution> {
        let global = node.into_global_any(self.id);

        self.decisions.access_resolution(global)
    }

    /// Return whether two nodes select the same checked storage.
    pub(crate) fn is_same_access(
        &self,
        left: dir::LocalNodeIdAny,
        right: dir::LocalNodeIdAny,
    ) -> bool {
        let left = self.decisions.access_resolution(left.into_global(self.id));
        let right = self.decisions.access_resolution(right.into_global(self.id));

        left.is_some() && left == right
    }

    /// Return the recorded uses of one stable access and its projections within a node.
    pub(crate) fn access_uses_within(
        &self,
        path: &dir::AccessPath,
        node: dir::LocalNodeIdAny,
        occurrences: &[dir::AccessOccurrence],
    ) -> dir::BindingUse {
        let view = self.view();
        let mut uses = dir::BindingUse::default();

        // merge uses of the selected storage beneath the selected node
        for occurrence in occurrences {
            if occurrence.path.starts_with(path) && view.is_inside(occurrence.node, node) {
                uses |= occurrence.uses;
            }
        }

        uses
    }

    /// Return whether one expression selects the same storage throughout a node subtree.
    pub(crate) fn is_stable_access(
        &self,
        expression: dir::LocalNodeId<dir::Expression>,
        within: dir::LocalNodeIdAny,
        occurrences: &[dir::AccessOccurrence],
    ) -> bool {
        let Some(selected) = self.access_resolution(expression) else {
            return false;
        };
        let view = self.view();
        let expression = expression.into_any();

        // reject other writes that replace the selected value or one of its prefixes
        !occurrences.iter().any(|occurrence| {
            occurrence.node != expression
                && occurrence.uses.may_mutate()
                && selected.path().starts_with(&occurrence.path)
                && view.is_inside(occurrence.node, within)
        })
    }

    /// Return the weakest access sufficient for one binding's checked uses.
    pub(crate) fn weakest_binding_access(
        &self,
        symbol: dir::GlobalSymbolId,
        within: dir::LocalNodeIdAny,
        occurrences: &[dir::AccessOccurrence],
    ) -> Result<dir::Access, ProviderError> {
        let view = self.view();
        let root = dir::AccessPath::symbol(symbol);
        let mut required = dir::Access::Readonly;

        // combine access requirements beneath the binding storage
        for occurrence in occurrences {
            if !view.is_inside(occurrence.node, within) || !occurrence.path.starts_with(&root) {
                continue;
            }

            // preserve exclusive access demanded by a checked borrow
            let access = if occurrence.uses.contains(dir::BindingUse::EXCLUSIVE) {
                dir::Access::Exclusive
            }
            // replacing the binding root requires exclusive access
            else if occurrence.uses.contains(dir::BindingUse::WRITE) {
                match occurrence.path == root {
                    true => dir::Access::Exclusive,
                    false => dir::Access::Mutable,
                }
            }
            // preserve access selected by checked reborrows
            else if occurrence.uses.contains(dir::BindingUse::MUTATE) {
                let type_id = self.adjusted_type_id(occurrence.node)?;
                match self.dir.borrow_access(type_id)? {
                    Some(access) => access,
                    None => dir::Access::Mutable,
                }
            }
            // reads require only readonly access
            else {
                dir::Access::Readonly
            };
            if access.grants(required) {
                required = access;
            }
        }

        Ok(required)
    }
}
