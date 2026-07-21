use destack_dir as dir;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::check::{Answer, CheckState, Origin, Relation, answer};

impl CheckState<'_> {
    /// Return the members of one union target with its enclosing forms.
    pub(in crate::check) fn union_arms(
        &mut self,
        origin: Origin,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<Option<SmallVec<[dir::GlobalTypeId; 4]>>>> {
        let chain = self.form_chain(origin, target)?;
        let base = chain.base();
        let dir::Type::Union(union) = self.ty(base)? else {
            return Ok(Answer::Ready(None));
        };

        // retain enclosing forms while exposing each stored union member
        let members =
            SmallVec::<[_; 4]>::from_slice(self.type_ids(base.module_id, union.elements)?);
        let mut arms = SmallVec::with_capacity(members.len());
        for member in members {
            let target = answer!(self.replace_form_value(origin, target, member)?);
            arms.push(target);
        }

        Ok(Answer::Ready(Some(arms)))
    }

    /// Decide a union target by membership when direct proof fell short.
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
        // membership tags the value into the union carrier and never widens
        if relation == Relation::Widens {
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

    /// Decide whether every source relates to one target.
    pub(in crate::check) fn decide_all_sources(
        &mut self,
        origin: Origin,
        relation: Relation,
        sources: &[dir::GlobalTypeId],
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<bool>> {
        let mut decision = Answer::Ready(true);
        for source in sources {
            decision = decision.and(self.decide_relation(origin, relation, *source, target)?);
            if decision.is_ready_false() {
                return Ok(decision);
            }
        }

        Ok(decision)
    }

    /// Decide whether any source relates to one target.
    pub(in crate::check) fn decide_any_source(
        &mut self,
        origin: Origin,
        relation: Relation,
        sources: &[dir::GlobalTypeId],
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<bool>> {
        let mut decision = Answer::Ready(false);
        for source in sources {
            decision = decision.or(self.decide_relation(origin, relation, *source, target)?);
            if decision.is_ready_true() {
                return Ok(decision);
            }
        }

        Ok(decision)
    }

    /// Decide whether one source relates to every target.
    pub(in crate::check) fn decide_all_targets(
        &mut self,
        origin: Origin,
        relation: Relation,
        source: dir::GlobalTypeId,
        targets: &[dir::GlobalTypeId],
    ) -> CompilerResult<Answer<bool>> {
        let mut decision = Answer::Ready(true);
        for target in targets {
            decision = decision.and(self.decide_relation(origin, relation, source, *target)?);
            if decision.is_ready_false() {
                return Ok(decision);
            }
        }

        Ok(decision)
    }

    /// Decide whether one source relates to any target.
    pub(in crate::check) fn decide_any_target(
        &mut self,
        origin: Origin,
        relation: Relation,
        source: dir::GlobalTypeId,
        targets: &[dir::GlobalTypeId],
    ) -> CompilerResult<Answer<bool>> {
        // exact singleton keys prove membership without candidate relations
        if let Some(source_key) = self.static_key_from_type(source)? {
            for target in targets {
                if self.static_key_from_type(*target)? == Some(source_key) {
                    return Ok(Answer::Ready(true));
                }
            }
        }

        let mut decision = Answer::Ready(false);
        for target in targets {
            decision = decision.or(self.decide_relation(origin, relation, source, *target)?);
            if decision.is_ready_true() {
                return Ok(decision);
            }
        }

        Ok(decision)
    }
}
