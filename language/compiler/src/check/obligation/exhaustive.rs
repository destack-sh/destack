use destack_dir as dir;

use crate::check::{
    Answer, CheckState, DecisionKind, ObligationCheck, ObligationFailure, Origin, PatternArm,
    answer,
};
use crate::{CompilerError, CompilerResult};

impl CheckState<'_> {
    /// Check whether one match covers every known selector value.
    pub(in crate::check) fn check_match_exhaustive(
        &mut self,
        origin: Origin,
        source: dir::GlobalNodeIdAny,
        value: dir::GlobalTypeId,
        arms: &[PatternArm],
    ) -> CompilerResult<Answer<ObligationCheck>> {
        // collect unguarded patterns with valid pattern decisions
        let mut patterns = Vec::new();
        for arm in arms {
            // guarded cases cannot guarantee coverage
            if arm.is_guarded {
                continue;
            }

            match self.decision_kind(arm.pattern.into_any()) {
                Some(DecisionKind::Pattern) => patterns.push(arm.pattern),
                // rejected patterns already reported and hold vacuously
                Some(DecisionKind::Rejected) => {
                    return Ok(Answer::Ready(ObligationCheck::holds()));
                }
                decision => {
                    let label = self.node_label(arm.pattern.into_any());
                    return Err(CompilerError::Internal {
                        message: format!("match coverage pattern {label} decided as {decision:?}"),
                    });
                }
            }
        }

        // accept any covering pattern alternative
        let check = if answer!(self.decide_patterns_cover(origin, &patterns, value)?) {
            ObligationCheck::holds()
        } else {
            let missing = self.uncovered_value(origin, &patterns, value)?;

            ObligationCheck::fail(ObligationFailure::NonExhaustivePattern { source, missing })
        };

        Ok(Answer::Ready(check))
    }
}
