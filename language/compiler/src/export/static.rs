use tspp_dir as dir;

use crate::ExportResult;
use crate::export::state::ExportState;
use crate::r#static::{StaticError, StaticEvaluator, StaticGuard};

impl ExportState<'_> {
    /// Return whether static export decorators attached to one node allow it.
    pub(in crate::export) fn static_allows(
        &mut self,
        owner: dir::LocalNodeIdAny,
    ) -> ExportResult<bool> {
        self.stats.static_checks += 1;

        if let Some(value) = self.static_visibility_by_node.get(&owner).copied() {
            self.stats.static_cache_hits += 1;

            return Ok(value);
        }

        let decorators = self.view.get_decorators_any(owner);

        for decorator in decorators {
            match StaticGuard::classify(self.view, self.strings(), decorator) {
                StaticGuard::Ordinary => {}
                StaticGuard::Rejected(_) => {
                    // checking owns the guard diagnostics; export only decides presence
                    self.static_visibility_by_node.insert(owner, false);

                    return Ok(false);
                }
                StaticGuard::Condition(condition) => {
                    self.stats.guards += 1;

                    let Some(value) = self.evaluate_static_guard(condition)? else {
                        self.static_visibility_by_node.insert(owner, false);

                        return Ok(false);
                    };

                    if !value {
                        self.static_visibility_by_node.insert(owner, false);

                        return Ok(false);
                    }
                }
            }
        }

        self.static_visibility_by_node.insert(owner, true);

        Ok(true)
    }

    /// Return dependency items that remain after static gates.
    pub(in crate::export) fn static_allowed_items(
        &mut self,
        items: &[dir::LocalNodeId<dir::DependencyItem>],
    ) -> ExportResult<Vec<dir::LocalNodeId<dir::DependencyItem>>> {
        let mut allowed = Vec::new();

        for item in items {
            if self.static_allows(item.into_any())? {
                allowed.push(*item);
            }
        }

        Ok(allowed)
    }

    /// Return whether static decorators on one node and its owners allow it.
    pub(in crate::export) fn static_allows_owners(
        &mut self,
        node: dir::LocalNodeIdAny,
    ) -> ExportResult<bool> {
        let mut current = Some(node);

        while let Some(node) = current {
            if !self.static_allows(node)? {
                return Ok(false);
            }

            current = self.view.get_parent_any(node);
        }

        Ok(true)
    }

    /// Evaluate one static guard condition.
    fn evaluate_static_guard(
        &mut self,
        condition: dir::LocalNodeId<dir::Expression>,
    ) -> ExportResult<Option<bool>> {
        let evaluator = StaticEvaluator::new(
            self.view,
            self.module,
            self.package,
            self.environment,
            self.profile,
            self.strings(),
        );

        // checking owns the guard diagnostics; export only decides presence
        match evaluator.evaluate_boolean(condition) {
            Ok(value) => Ok(Some(value)),
            Err(StaticError::NotBoolean(_) | StaticError::NotStatic(_)) => Ok(None),
        }
    }
}
