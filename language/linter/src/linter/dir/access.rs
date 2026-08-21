use destack_dir as dir;
use destack_repository::ProviderError;

use super::DirModule;

impl DirModule<'_> {
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

    /// Return whether one read observes its value in place instead of taking it.
    pub(crate) fn reads_in_place(&self, node: dir::LocalNodeIdAny) -> Result<bool, ProviderError> {
        // a readonly borrow observes the value through a reference
        let adjusted = self.adjusted_type_id(node)?;
        if self.dir.borrow_access(adjusted)? == Some(dir::Access::Readonly) {
            return Ok(true);
        }

        // projecting a member or an element observes the value where it lives
        let view = self.view();
        let Some(parent) = view.get_parent_any(node) else {
            return Ok(false);
        };
        if parent.ty != dir::NodeType::Expression {
            return Ok(false);
        }
        let parent = dir::LocalNodeId::<dir::Expression>::new(parent.id);

        Ok(match view.get(parent) {
            dir::Expression::Member { left, .. } | dir::Expression::Index { left, .. } => {
                left.into_any() == node
            }
            _ => false,
        })
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
