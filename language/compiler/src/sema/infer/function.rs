use crate::sema::{
    BodyState, Cause, CauseKind, CheckFailure, CheckOutcome, Expectation, FlowSite, InferMode,
    Relation, ValueCheck,
};
use crate::{CompilerError, CompilerResult};

impl BodyState<'_, '_> {
    /// Check one function value's body in its receiving context.
    ///
    /// The declared callable is constrained against the expected type, which fills in
    /// the parameter and return types the body then checks against.
    pub(in crate::sema) fn check_function_value(
        &mut self,
        site: FlowSite,
        expectation: Option<Expectation>,
        mode: InferMode,
    ) -> CompilerResult<ValueCheck> {
        let node = site.node;
        let Some(body) = self.check.lambdas.get(&node).cloned() else {
            return Err(CompilerError::Internal {
                message: format!("function value {node:?} has no body"),
            });
        };
        // read the declared callable and the context the body checks under
        let callable = self.check.symbol_type(body.symbol)?;
        let output_mode = expectation
            .map_or(mode, |expectation| expectation.mode)
            .descend(false);
        let target = expectation.map_or(callable, |expectation| expectation.target);

        // constrain the declared callable against its expected type
        let carrier = if let Some(expectation) = expectation {
            let origin = site.origin();
            let Some(target) = self.construction_value(origin, expectation.target)? else {
                self.check.commit_node_type(node, callable)?;

                return Ok(ValueCheck {
                    source: callable,
                    outcome: CheckOutcome::Fails(CheckFailure::Relation),
                    target,
                });
            };
            let cause = self
                .check
                .intern_cause(Cause::root(origin, CauseKind::Expression));
            if !self
                .check
                .constrain_type(origin, cause, Relation::Assignable, callable, target)?
            {
                self.check.commit_node_type(node, callable)?;

                return Ok(ValueCheck {
                    source: callable,
                    outcome: CheckOutcome::Fails(CheckFailure::Relation),
                    target,
                });
            }

            // keep the source under satisfies, take the storage form otherwise
            match expectation.relation {
                Relation::Satisfies => callable,
                _ => self.replace_form_value(origin, expectation.target, callable)?,
            }
        } else {
            callable
        };

        // check the body against the filled-in signature
        let parent = expectation.map(|expectation| expectation.cause);
        let checked = body.check(self.check, output_mode, parent)?;
        let outcome = checked.map_or(CheckOutcome::Holds, |check| check.outcome);
        self.check.commit_node_type(node, carrier)?;

        Ok(ValueCheck {
            source: carrier,
            outcome,
            target,
        })
    }
}
