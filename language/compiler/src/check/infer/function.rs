use crate::check::{
    Answer, BodyState, Cause, CauseKind, CheckFailure, CheckOutcome, Expectation, FlowSite,
    InferMode, Relation, ValueCheck, answer,
};
use crate::{CompilerError, CompilerResult};

impl BodyState<'_, '_> {
    /// Check one function value's body in its receiving context.
    ///
    /// Contextual flow is the ordinary relation: the structural decomposition
    /// assigns parameters, equates the contextual return slot to its contract,
    /// and evidence transmission routes body candidates into open inference.
    pub(in crate::check) fn check_function_value(
        &mut self,
        site: FlowSite,
        expectation: Option<Expectation>,
    ) -> CompilerResult<Answer<ValueCheck>> {
        let node = site.node;
        let Some(body) = self.check.lambdas.get(&node).copied() else {
            return Err(CompilerError::Internal {
                message: format!("function value {node:?} has no body"),
            });
        };
        let callable = answer!(self.check.symbol_type(body.symbol)?);
        let output_mode = expectation.map_or(InferMode::Exact, |expectation| {
            expectation.mode.descend(false)
        });
        let target = expectation.map_or(callable, |expectation| expectation.target);

        // constrain the declaration callable against its contextual payload
        let carrier = if let Some(expectation) = expectation {
            let origin = site.origin();
            let Some(target) = answer!(self.construction_value(origin, expectation.target)?) else {
                self.check.commit_node_type(node, callable)?;

                return Ok(Answer::Ready(ValueCheck {
                    source: callable,
                    outcome: CheckOutcome::Fails(CheckFailure::Relation),
                    target,
                }));
            };
            let cause = self
                .check
                .intern_cause(Cause::root(origin, CauseKind::Expression));
            if !answer!(self.check.constrain_type(
                origin,
                cause,
                Relation::Assignable,
                callable,
                target,
            )?) {
                self.check.commit_node_type(node, callable)?;

                return Ok(Answer::Ready(ValueCheck {
                    source: callable,
                    outcome: CheckOutcome::Fails(CheckFailure::Relation),
                    target,
                }));
            }

            // construction takes storage forms while satisfies preserves the source
            match expectation.relation {
                Relation::Satisfies => callable,
                _ => answer!(self.replace_form_value(origin, expectation.target, callable)?),
            }
        } else {
            callable
        };

        let parent = expectation.map(|expectation| expectation.cause);

        let checked = answer!(body.check(self.check, output_mode, parent)?);
        let outcome = checked.map_or(CheckOutcome::Holds, |check| check.outcome);
        self.check.commit_node_type(node, carrier)?;

        Ok(Answer::Ready(ValueCheck {
            source: carrier,
            outcome,
            target,
        }))
    }
}
