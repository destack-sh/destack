use destack_dir as dir;

use crate::CompilerResult;
use crate::check::{Answer, CheckState, Origin, Relation};

impl CheckState<'_> {
    /// Decide a union target by membership when direct proof fell short.
    ///
    /// Opaque sources prove through their bounds first, but any source
    /// sits inside a union that spells it as a member. The membership
    /// distributes the caller's relation over the union elements.
    pub(in crate::check) fn decide_union_membership(
        &mut self,
        origin: Origin,
        relation: Relation,
        decision: Answer<bool>,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<bool>> {
        if decision.is_ready_true() {
            return Ok(decision);
        }
        let dir::Type::Union(union) = self.ty(target)? else {
            return Ok(decision);
        };

        // relate the source against any one element
        let elements = self.type_ids(target.module_id, union.elements)?.to_vec();
        let mut membership = decision;
        for element in elements {
            membership = membership.or(self.decide_relation(origin, relation, source, element)?);
            if membership.is_ready_true() {
                break;
            }
        }

        Ok(membership)
    }

    /// Decide whether every union element assigns to one target.
    pub(in crate::check) fn decide_all_assignable(
        &mut self,
        origin: Origin,
        sources: &[dir::GlobalTypeId],
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<bool>> {
        let mut decision = Answer::Ready(true);
        for source in sources {
            decision = decision.and(self.decide_relation(
                origin,
                Relation::Assignable,
                *source,
                target,
            )?);
            if decision.is_ready_false() {
                return Ok(decision);
            }
        }

        Ok(decision)
    }

    /// Decide whether one source assigns to any union element.
    pub(in crate::check) fn decide_any_assignable(
        &mut self,
        origin: Origin,
        source: dir::GlobalTypeId,
        targets: &[dir::GlobalTypeId],
    ) -> CompilerResult<Answer<bool>> {
        let mut decision = Answer::Ready(false);
        for target in targets {
            decision =
                decision.or(self.decide_relation(origin, Relation::Assignable, source, *target)?);
            if decision.is_ready_true() {
                return Ok(decision);
            }
        }

        Ok(decision)
    }
}
