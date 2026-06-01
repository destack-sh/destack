use destack_dir as dir;

use crate::common::dir::r#static::{
    StaticContext, StaticFailure, StaticGuard, StaticGuardError, static_guard,
};
use crate::import::state::ImportState;
use crate::{CompilerResult, ImportError};

impl ImportState<'_> {
    /// Return whether static import decorators attached to one node allow it.
    pub(in crate::import) fn static_allows(
        &mut self,
        owner: dir::LocalNodeIdAny,
    ) -> CompilerResult<bool> {
        let decorators = self.view.get_decorators_any(owner);

        for decorator in decorators {
            match static_guard(self.view, self.strings(), decorator) {
                StaticGuard::Ordinary => {}
                StaticGuard::Rejected(error) => {
                    self.report_static_guard_error(error)?;

                    return Ok(false);
                }
                StaticGuard::Condition(condition) => {
                    self.stats.guards += 1;

                    let Some(value) = self.evaluate_static_guard(condition)? else {
                        return Ok(false);
                    };

                    if !value {
                        return Ok(false);
                    }
                }
            }
        }

        Ok(true)
    }

    /// Return whether any dependency item remains after static gates.
    pub(in crate::import) fn static_allows_any_item(
        &mut self,
        items: &[dir::LocalNodeId<dir::DependencyItem>],
    ) -> CompilerResult<bool> {
        for item in items {
            if self.static_allows(item.into_any())? {
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Evaluate one static guard condition.
    fn evaluate_static_guard(
        &mut self,
        condition: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<Option<bool>> {
        let context = StaticContext::new(
            self.view,
            self.module,
            self.profile,
            self.conditions,
            self.strings(),
        );

        match context.evaluate_boolean(condition) {
            Ok(value) => Ok(Some(value)),
            Err(StaticFailure::NotBoolean(expression)) => {
                let anchor = self.anchor_node(expression.id)?;
                self.report_diagnostic(ImportError::StaticIfRequiresBoolean { anchor });

                Ok(None)
            }
            Err(StaticFailure::NotStatic(expression)) => {
                let anchor = self.anchor_node(expression.id)?;
                self.report_diagnostic(ImportError::StaticIfNotStatic { anchor });

                Ok(None)
            }
        }
    }

    /// Report one malformed static guard.
    fn report_static_guard_error(&mut self, error: StaticGuardError) -> CompilerResult<()> {
        match error {
            StaticGuardError::MissingCondition { node } => {
                let anchor = self.anchor_node(node.id)?;
                self.report_diagnostic(ImportError::StaticIfRequiresCondition { anchor });
            }
            StaticGuardError::MultipleConditions { node } => {
                let anchor = self.anchor_node(node.id)?;
                self.report_diagnostic(ImportError::StaticIfRequiresOneArgument { anchor });
            }
            StaticGuardError::InvalidCondition { node } => {
                let anchor = self.anchor_node(node.id)?;
                self.report_diagnostic(ImportError::StaticIfRequiresCondition { anchor });
            }
        }

        Ok(())
    }
}
