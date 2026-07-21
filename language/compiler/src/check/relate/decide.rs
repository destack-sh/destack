use destack_core::ensure_sufficient_stack;
use destack_dir as dir;

use crate::CompilerResult;
use crate::check::{Answer, CheckState, Origin, Relation, answer};

impl CheckState<'_> {
    /// Decide one relation between closed type roots, growing the stack.
    pub(in crate::check) fn decide_relation(
        &mut self,
        origin: Origin,
        relation: Relation,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<bool>> {
        ensure_sufficient_stack(|| self.decide_relation_recursive(origin, relation, source, target))
    }

    /// Decide one relation recursively on the grown stack.
    fn decide_relation_recursive(
        &mut self,
        origin: Origin,
        relation: Relation,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<bool>> {
        let source = answer!(self.reduce_type(origin, source)?);
        let target = answer!(self.reduce_type(origin, target)?);
        if source == target {
            return Ok(Answer::Ready(true));
        }

        // refined targets require the base and the refined member equality
        if let Some(refined) = self.refined_head(target)? {
            let base = answer!(self.decide_relation(origin, relation, source, refined.base)?);
            if !base {
                return Ok(Answer::Ready(false));
            }
            let arguments = self.intern_type_ids(origin.module(), &[])?;
            let projected = self.intern_member(
                origin.module(),
                dir::MemberType {
                    owner: source,
                    key: refined.key,
                    arguments,
                    qualifier: None,
                },
            )?;

            return self.decide_relation(origin, Relation::Equal, projected, refined.value);
        }

        // refined sources imply their base application
        if let Some(refined) = self.refined_head(source)? {
            return self.decide_relation(origin, relation, refined.base, target);
        }

        // irreducible conditionals relate through both branches
        if matches!(relation, Relation::Assignable | Relation::Widens)
            && let Some(dir::TypeOperation::Conditional(conditional)) =
                self.operation_head(source)?
        {
            let then_branch =
                answer!(self.decide_relation(origin, relation, conditional.then_type, target)?);
            if !then_branch {
                return Ok(Answer::Ready(false));
            }

            return self.decide_relation(origin, relation, conditional.else_type, target);
        }

        // exact property keys compare by key identity
        if let (Some(source_key), Some(target_key)) = (
            self.static_key_from_type(source)?,
            self.static_key_from_type(target)?,
        ) {
            return Ok(Answer::Ready(source_key == target_key));
        }

        // key parameter and this queries by their assuming scope
        let flags = self.type_flags(source)? | self.type_flags(target)?;
        let scope = match flags.has_parameter() || flags.has_this() {
            true => self.origin_scope(origin)?,
            false => None,
        };

        // reuse memoized answers, treating active pairs as recursive cycles
        if let Some(holds) = self
            .solver
            .relations
            .lookup(relation, source, target, scope)
        {
            return Ok(Answer::Ready(holds));
        }
        let attempt = self.solver.relations.enter(relation, source, target, scope);

        // dispatch to the relation's decider
        let decision = match relation {
            Relation::Equal => self.decide_equal(origin, source, target),
            Relation::Assignable | Relation::Widens => {
                self.decide_assignable(origin, relation, source, target)
            }
            Relation::MethodAssignable => self.decide_method_assignable(origin, source, target),
            Relation::Writable => self.decide_writable(origin, source, target),
            Relation::Castable => self.decide_castable(origin, source, target),
            Relation::Satisfies | Relation::Extends | Relation::Implements => {
                self.decide_satisfies(origin, relation, source, target)
            }
        };

        // memoize settled decisions, forget pending or failed attempts
        match &decision {
            Ok(Answer::Ready(holds)) => {
                self.solver.relations.finish(attempt, *holds);
            }
            _ => self.solver.relations.cancel(attempt),
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
        for (source, target) in pairs.iter().copied() {
            decision = decision.and(self.decide_relation(origin, relation, source, target)?);
            if decision.is_ready_false() {
                return Ok(decision);
            }
        }

        Ok(decision)
    }
}
