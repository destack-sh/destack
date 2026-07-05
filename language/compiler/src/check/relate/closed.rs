use destack_dir as dir;

use crate::CompilerResult;
use crate::check::{Answer, CheckState, Origin, Relation, answer};

impl CheckState<'_> {
    /// Decide one relation between closed type roots, growing the stack.
    pub(in crate::check) fn decide_relation(
        &mut self,
        origin: Origin,
        relation: Relation,
        left: dir::GlobalTypeId,
        right: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<bool>> {
        destack_core::ensure_sufficient_stack(|| {
            self.decide_relation_inner(origin, relation, left, right)
        })
    }

    /// Decide one relation on the grown stack.
    fn decide_relation_inner(
        &mut self,
        origin: Origin,
        relation: Relation,
        left: dir::GlobalTypeId,
        right: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<bool>> {
        // reduce both operands before structural comparison
        let left = answer!(self.reduce_type(origin, left)?);
        let right = answer!(self.reduce_type(origin, right)?);
        if left == right {
            return Ok(Answer::Ready(true));
        }

        // exact property keys compare by key identity
        if let (Some(left_key), Some(right_key)) = (
            self.static_key_from_type(left)?,
            self.static_key_from_type(right)?,
        ) {
            return Ok(Answer::Ready(left_key == right_key));
        }

        // key parameter and this queries by their assuming scope
        let flags = self.type_flags(left)? | self.type_flags(right)?;
        let scope = match flags.has_parameter() || flags.has_this() {
            true => self.origin_scope(origin),
            false => None,
        };

        // reuse memoized answers, treating in-flight pairs as recursive cycles
        if let Some(holds) = self.relations().lookup(relation, left, right, scope) {
            return Ok(Answer::Ready(holds));
        }
        let frame = self.relations().enter(relation, left, right, scope);

        // dispatch to the relation's decider
        let decision = match relation {
            Relation::Equal => self.decide_equal(origin, left, right),
            Relation::Assignable => self.decide_assignable(origin, left, right),
            Relation::MethodAssignable => self.decide_method_assignable(origin, left, right),
            Relation::Writable => self.decide_writable(origin, left, right),
            Relation::Castable => self.decide_castable(origin, left, right),
            Relation::Satisfies | Relation::Extends | Relation::Implements => {
                self.decide_satisfies(origin, relation, left, right)
            }
        };

        // memoize settled decisions, forget pending or failed attempts
        match &decision {
            Ok(Answer::Ready(holds)) => {
                self.relations().finish(frame, *holds);
            }
            _ => self.relations().cancel(frame),
        }

        decision
    }

    /// Decide every relation in one pair list.
    pub(in crate::check) fn decide_each(
        &mut self,
        origin: Origin,
        relation: Relation,
        pairs: &[(dir::GlobalTypeId, dir::GlobalTypeId)],
    ) -> CompilerResult<Answer<bool>> {
        let mut decision = Answer::Ready(true);
        for (left, right) in pairs.iter().copied() {
            decision = decision.and(self.decide_relation(origin, relation, left, right)?);
            if decision.is_ready_false() {
                return Ok(decision);
            }
        }

        Ok(decision)
    }
}
