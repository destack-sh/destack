use tspp_dir as dir;
use tspp_repository::ProviderError;

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
                "assignment target {global:?} has no assignment decision"
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
                            "stored projection {global:?} has no member decision"
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
                            "stored projection {global:?} has no subscript decision"
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

        // reject a mutable use, a move, or a capture of the replacement borrow
        let has_incompatible_use = self.flows.binding_occurrences().any(|occurrence| {
            occurrence.symbol == symbol
                && view.is_inside(occurrence.node, within)
                && !excluded.is_some_and(|excluded| view.is_inside(occurrence.node, excluded))
                && (occurrence.uses.may_mutate()
                    || occurrence.uses.contains(dir::BindingUse::MUTABLE)
                    || occurrence.uses.contains(dir::BindingUse::MOVE)
                    || occurrence.uses.contains(dir::BindingUse::EXCLUSIVE)
                    || occurrence.uses.contains(dir::BindingUse::CAPTURE))
        });

        Ok(!has_incompatible_use)
    }

    /// Return the strongest access granted through one place.
    pub(crate) fn place_access(
        &self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> Result<Option<dir::Access>, ProviderError> {
        let place = self
            .decisions
            .place_resolution(expression.into_global_any(self.id));

        match place {
            Some(place) => self.dir.memory_access(place.access),
            None => Ok(None),
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

    /// Return the stable storage selected by one expression.
    pub fn access_resolution(
        &self,
        node: dir::LocalNodeId<dir::Expression>,
    ) -> Option<&dir::AccessResolution> {
        let global = node.into_global_any(self.id);

        self.decisions.access_resolution(global)
    }

    /// Return whether two nodes select the same storage.
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

    /// Return whether one access path takes a mutable use beneath a node.
    pub(crate) fn takes_mutable_within(
        &self,
        root: &dir::AccessPath,
        within: dir::LocalNodeIdAny,
        occurrences: &[dir::AccessOccurrence],
        excluded: &[dir::LocalNodeIdAny],
    ) -> bool {
        let view = self.view();
        occurrences.iter().any(|occurrence| {
            view.is_inside(occurrence.node, within)
                && occurrence.path.starts_with(root)
                && !excluded.contains(&occurrence.node)
                && (occurrence.uses.may_mutate()
                    || occurrence.uses.contains(dir::BindingUse::MUTABLE))
        })
    }

    /// Return the weakest access the uses of one access path beneath a node require.
    pub(crate) fn required_access_within(
        &self,
        root: &dir::AccessPath,
        within: dir::LocalNodeIdAny,
        occurrences: &[dir::AccessOccurrence],
    ) -> dir::Access {
        let uses = self.access_uses_within(root, within, occurrences);
        let writes = uses.may_mutate() || uses.contains(dir::BindingUse::MUTABLE);
        let excludes = uses.contains(dir::BindingUse::EXCLUSIVE);

        dir::Access::of(writes, excludes)
    }
}
