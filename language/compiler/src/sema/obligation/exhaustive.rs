use tspp_dir as dir;

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

        // accept any covering pattern alternative, an undecided cover reporting as non-exhaustive
        let covered = self.decide_patterns_cover(origin, &patterns, value)?;
        let is_exhaustive = covered.holds();
        let check = if is_exhaustive {
            ObligationCheck::holds()
        } else {
            let missing = self.uncovered_value(origin, &patterns, value)?;

            ObligationCheck::fail(ObligationFailure::NonExhaustivePattern { source, missing })
        };

        // commit the coverage proof downstream consumers reuse
        let proof = self.decide_pattern_coverage(origin, arms, &patterns, is_exhaustive)?;
        self.commit_decision(source, dir::Decision::Coverage(proof))?;

        Ok(check)
    }

    /// Decide the disjointness and redundancy proof over one match's arms.
    fn decide_pattern_coverage(
        &mut self,
        origin: Origin,
        arms: &[PatternArm],
        patterns: &[dir::GlobalNodeId<dir::Pattern>],
        is_exhaustive: bool,
    ) -> CompilerResult<dir::CoverageDecision> {
        // read the value subset each unguarded arm accepts
        let mut accepted = Vec::with_capacity(patterns.len());
        for pattern in patterns {
            accepted.push(self.pattern_predicate_target(origin, *pattern)?);
        }

        // arms reorder safely when every accepted pair stays disjoint
        let mut is_disjoint = arms.iter().all(|arm| !arm.is_guarded);
        for (index, left) in accepted.iter().enumerate() {
            for right in accepted.iter().skip(index + 1) {
                let provably_apart = match (left, right) {
                    (Some(left), Some(right)) => !self.types_may_overlap(origin, *left, *right)?,
                    _ => false,
                };
                is_disjoint &= provably_apart;
            }
        }

        // collect the arms their preceding arms already cover
        let mut redundant = Vec::new();
        for (index, pattern) in patterns.iter().enumerate() {
            let Some(value) = accepted[index] else {
                continue;
            };

            // cover with the preceding unguarded arms, which select unconditionally
            let covering = patterns[..index]
                .iter()
                .zip(&arms[..index])
                .filter(|(_, arm)| !arm.is_guarded)
                .map(|(pattern, _)| *pattern)
                .collect::<Vec<_>>();
            if covering.is_empty() {
                continue;
            }

            // an undecided cover keeps the arm live, matching today's collapse
            let covered = self.decide_patterns_cover(origin, &covering, value)?;
            if covered.holds() {
                redundant.push(*pattern);
            }
        }

        Ok(dir::CoverageDecision {
            is_exhaustive,
            is_disjoint,
            redundant,
        })
    }
}
