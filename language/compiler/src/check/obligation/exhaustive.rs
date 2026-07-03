use destack_artifact::DiagnosticBuilder;
use destack_dir as dir;

use crate::check::{
    Answer, CheckError, CheckState, Decision, Dependency, MatchCase, Origin, answer,
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
    ) -> CompilerResult<Answer<Option<DiagnosticBuilder<CheckError>>>> {
        // collect unguarded patterns with valid pattern decisions
        let mut patterns = Vec::new();
        for case in cases {
            match case {
                MatchCase::Default => return Ok(Answer::Ready(None)),
                MatchCase::Pattern {
                    pattern,
                    is_guarded: false,
                } => match self.decision(pattern.into_any()) {
                    Some(Decision::Pattern(_)) => patterns.push(*pattern),
                    Some(Decision::Rejected) => {
                        return Ok(Answer::Ready(None));
                    }
                    Some(decision) => {
                        return Err(CompilerError::Internal {
                            message: format!(
                                "match coverage pattern {pattern:?} has non-pattern decision {decision:?}"
                            ),
                        });
                    }
                    None => return Ok(Answer::pending([Dependency::Decision(pattern.into_any())])),
                },
                // guarded cases cannot guarantee coverage
                MatchCase::Pattern {
                    is_guarded: true, ..
                } => {}
            }
        }

        // reject empty active case sets
        if patterns.is_empty() {
            let missing = self.format_type(value);
            let (module, anchor) = self.source_anchor(source);

            let error = CheckError::NonExhaustivePattern {
                anchor,
                module,
                missing,
            };

            return Ok(Answer::Ready(Some(
                error.help("cover the remaining values or add a wildcard '_' arm"),
            )));
        }

        // accept any covering pattern alternative
        let diagnostic = if answer!(self.decide_patterns_cover(origin, &patterns, value)?) {
            None
        } else {
            let missing = self.uncovered_witness(origin, &patterns, value)?;
            let (module, anchor) = self.source_anchor(source);
            let error = CheckError::NonExhaustivePattern {
                anchor,
                module,
                missing,
            };

            Some(error.help("cover the remaining values or add a wildcard '_' arm"))
        };

        Ok(Answer::Ready(diagnostic))
    }
}
