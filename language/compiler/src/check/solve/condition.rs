use destack_dir as dir;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::check::{Answer, CheckState, Condition, Dependency, Origin};

impl CheckState<'_> {
    /// Decide whether one symbol's @if availability conditions hold.
    pub(in crate::check) fn decide_availability(
        &mut self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<Answer<bool>> {
        let Some(module) = self.modules.get(&symbol.module_id) else {
            return Ok(Answer::Ready(true));
        };
        let predicates = match module.availability.get(&symbol) {
            Some(Condition::When(predicates)) => predicates.clone(),
            _ => return Ok(Answer::Ready(true)),
        };

        self.decide_condition(&predicates)
    }

    /// Decide whether every condition predicate holds.
    ///
    /// Predicates that stay symbolic belong to a still-open generic
    /// declaration and count as held: guarded code is checked under its
    /// guard until instantiation closes the predicates.
    pub(in crate::check) fn decide_condition(
        &mut self,
        predicates: &[dir::GlobalTypeId],
    ) -> CompilerResult<Answer<bool>> {
        let mut blockers = SmallVec::<[Dependency; 2]>::new();

        for predicate in predicates {
            let origin = Origin::Type(*predicate);
            let reduced = match self.evaluate_root(origin, *predicate)? {
                Answer::Ready(reduced) => reduced,
                Answer::Pending(dependencies) => {
                    blockers.extend(dependencies);

                    continue;
                }
            };

            match self.ty(reduced)? {
                // false predicates fail the whole condition
                dir::Type::Literal(dir::ScalarLiteral::Boolean(false)) => {
                    return Ok(Answer::Ready(false));
                }
                dir::Type::Literal(dir::ScalarLiteral::Boolean(true)) => {}
                // non-boolean conditions fail loudly and read as absent
                dir::Type::Literal(_) => {
                    let (module, anchor) = self.origin_diagnostic_anchor(origin)?;
                    let error = crate::CheckError::InvalidCondition { anchor, module };
                    self.module_mut(module).diagnostics.push(error.into());

                    return Ok(Answer::Ready(false));
                }
                // open-generic guards stay assumed while the declaration is open
                _ => {}
            }
        }

        if blockers.is_empty() {
            Ok(Answer::Ready(true))
        } else {
            Ok(Answer::pending(blockers))
        }
    }
}
