use destack_dir as dir;

use super::DirModule;

/// One authored for-of expression.
#[derive(Debug, Clone, Copy)]
pub(crate) struct ForOf<'a> {
    /// Whether iteration awaits each value.
    pub(crate) asynchrony: dir::Asynchrony,
    /// The element binding.
    pub(crate) binding: &'a dir::ForEachBinding,
    /// The iterated expression.
    pub(crate) iterator: dir::LocalNodeId<dir::Expression>,
    /// The loop body.
    pub(crate) body: dir::LocalNodeId<dir::Block>,
}

impl DirModule<'_> {
    /// Return one authored for-of expression.
    pub(crate) fn for_of(
        &self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> Option<ForOf<'_>> {
        // select the shared for-each variant by its operator
        let view = self.view();
        let dir::Expression::ForEach {
            asynchrony,
            operator: dir::ForEachOperator::Of,
            binding,
            iterator,
            body,
            ..
        } = view.get(expression)
        else {
            return None;
        };

        Some(ForOf {
            asynchrony: *asynchrony,
            binding,
            iterator: *iterator,
            body: *body,
        })
    }

    /// Return the body of one authored iteration expression.
    pub(crate) fn iteration_body(
        &self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> Option<dir::LocalNodeId<dir::Block>> {
        match self.view().get(expression) {
            dir::Expression::While { body, .. }
            | dir::Expression::ForEach { body, .. }
            | dir::Expression::For { body, .. }
            | dir::Expression::Loop { body, .. } => Some(*body),
            _ => None,
        }
    }

    /// Return the target selected by one unlabeled break or continue expression.
    pub(crate) fn unlabeled_transfer_target(
        &self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> Option<dir::LocalNodeId<dir::Expression>> {
        let view = self.view();
        let transfer = view.get(expression);
        if !matches!(
            transfer,
            dir::Expression::Break { .. } | dir::Expression::Continue { .. }
        ) {
            return None;
        }
        let mut parent = view.get_parent_for(expression);

        // climb to the nearest compatible target or callable boundary
        while let Some(node) = parent {
            // stop at nested callable ownership
            if self.callable_body(node).is_some() {
                return None;
            }

            // select the nearest iteration or compatible switch
            if let Ok(expression) = node.try_into_typed::<dir::Expression>() {
                let is_target = self.iteration_body(expression).is_some()
                    || matches!(transfer, dir::Expression::Break { .. })
                        && matches!(view.get(expression), dir::Expression::Switch { .. });
                if is_target {
                    return Some(expression);
                }
            }

            parent = view.get_parent_any(node);
        }

        None
    }
}
