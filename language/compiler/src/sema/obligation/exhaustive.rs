use destack_dir as dir;

use crate::sema::{CheckState, ObligationCheck, ObligationFailure, Origin, PatternArm};
use crate::{CompilerError, CompilerResult};

impl CheckState<'_> {
    /// Check whether one match covers every known selector value.
    pub(in crate::sema) fn check_match_exhaustive(
        &mut self,
        origin: Origin,
        source: dir::GlobalNodeIdAny,
        value: dir::GlobalTypeId,
        arms: &[PatternArm],
    ) -> CompilerResult<ObligationCheck> {
        // collect unguarded patterns with valid pattern decisions
        let mut patterns = Vec::new();
        for arm in arms {
            // guarded cases cannot guarantee coverage
            if arm.is_guarded {
                continue;
            }

            match self.decision(arm.pattern.into_any()) {
                Some(dir::Decision::Pattern(_)) => patterns.push(arm.pattern),
                Some(dir::Decision::Rejected | dir::Decision::Poisoned) => {
                    return Ok(ObligationCheck::holds());
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
        let check = if self.decide_patterns_cover(origin, &patterns, value)? {
            ObligationCheck::holds()
        } else {
            let missing = self.uncovered_value(origin, &patterns, value)?;

            ObligationCheck::fail(ObligationFailure::NonExhaustivePattern { source, missing })
        };

        Ok(check)
    }
}
