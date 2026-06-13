use destack_artifact::DiagnosticBuilder;
use destack_dir as dir;

use crate::CompilerResult;
use crate::check::{Answer, CheckError, CheckState, MatchCase, Origin};

impl CheckState<'_> {
    /// Check whether one match covers every known selector value.
    pub(in crate::check) fn check_match_exhaustive(
        &mut self,
        source: dir::GlobalNodeIdAny,
        value: dir::GlobalTypeId,
        cases: &[MatchCase],
    ) -> CompilerResult<Answer<Option<DiagnosticBuilder<CheckError>>>> {
        let origin = Origin::Node(source);

        // collect unguarded covering patterns
        let mut patterns = Vec::new();
        for case in cases {
            match case {
                MatchCase::Default => return Ok(Answer::Ready(None)),
                MatchCase::Pattern {
                    pattern,
                    guard: None,
                } => patterns.push(*pattern),
                // guarded cases cannot guarantee coverage
                MatchCase::Pattern { guard: Some(_), .. } => {}
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
        let decision = self.decide_patterns_cover(origin, &patterns, value)?;
        let diagnostic = match decision {
            Answer::Ready(true) => None,
            Answer::Ready(false) => {
                let missing = self.uncovered_witness(origin, &patterns, value)?;
                let (module, anchor) = self.source_anchor(source);

                let error = CheckError::NonExhaustivePattern {
                    anchor,
                    module,
                    missing,
                };

                Some(error.help("cover the remaining values or add a wildcard '_' arm"))
            }
            Answer::Pending(blockers) => return Ok(Answer::Pending(blockers)),
        };

        Ok(Answer::Ready(diagnostic))
    }
}
