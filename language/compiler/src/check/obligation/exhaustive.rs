use destack_dir as dir;

use crate::check::{
    Answer, CheckState, DecisionKind, MatchCase, ObligationCheck, ObligationFailure, Origin,
    UncoveredValue, answer,
};
use crate::{CompilerError, CompilerResult};

impl CheckState<'_> {
    /// Check whether one match covers every known selector value.
    pub(in crate::check) fn check_match_exhaustive(
        &mut self,
        origin: Origin,
        source: dir::GlobalNodeIdAny,
        value: dir::GlobalTypeId,
        cases: &[MatchCase],
    ) -> CompilerResult<Answer<ObligationCheck>> {
        // collect unguarded patterns with valid pattern decisions
        let mut patterns = Vec::new();
        for case in cases {
            match case {
                MatchCase::Default => return Ok(Answer::Ready(ObligationCheck::holds())),
                MatchCase::Pattern {
                    pattern,
                    is_guarded: false,
                } => match self.decision_kind(pattern.into_any()) {
                    Some(DecisionKind::Pattern) => patterns.push(*pattern),
                    // rejected patterns already reported and hold vacuously
                    Some(DecisionKind::Rejected) => {
                        return Ok(Answer::Ready(ObligationCheck::holds()));
                    }
                    decision => {
                        let label = self.node_label(pattern.into_any());
                        return Err(CompilerError::Internal {
                            message: format!(
                                "match coverage pattern {label} decided as {decision:?}"
                            ),
                        });
                    }
                },
                // guarded cases cannot guarantee coverage
                MatchCase::Pattern {
                    is_guarded: true, ..
                } => {}
            }
        }

        // reject empty active case sets
        if patterns.is_empty() {
            let failure = ObligationFailure::NonExhaustivePattern {
                source,
                missing: UncoveredValue::Type(value),
            };

            return Ok(Answer::Ready(ObligationCheck::fail(failure)));
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
