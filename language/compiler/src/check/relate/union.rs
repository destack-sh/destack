use destack_dir as dir;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::check::{CheckState, Origin, Relation};

impl CheckState<'_> {
    /// Return the members of one union target with its enclosing forms.
    pub(in crate::check) fn union_arms(
        &mut self,
        origin: Origin,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Option<SmallVec<[dir::GlobalTypeId; 4]>>> {
        let chain = self.form_chain(origin, target)?;
        let base = chain.base();
        let dir::Type::Union(union) = self.ty(base)? else {
            return Ok(None);
        };

        // retain enclosing forms while exposing each stored union member
        let members =
            SmallVec::<[_; 4]>::from_slice(self.type_ids(base.module_id, union.elements)?);
        let mut arms = SmallVec::with_capacity(members.len());
        for member in members {
            let target = self.replace_form_value(origin, target, member)?;
            arms.push(target);
        }

        Ok(Some(arms))
    }

    /// Return the leaf members of one union target, expanding stuck named heads.
    pub(in crate::check) fn union_leaves(
        &mut self,
        origin: Origin,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Option<SmallVec<[dir::GlobalTypeId; 4]>>> {
        // reduce a stuck named head, which may still hide a union
        let mut arms = self.union_arms(origin, target)?;
        if arms.is_none() {
            let head = self.normalize_stuck(origin, target)?;
            if head != target {
                arms = self.union_arms(origin, head)?;
            }
        }

        let Some(mut members) = arms else {
            return Ok(None);
        };

        // expand nested and aliased unions into their leaf members
        let mut leaves = SmallVec::<[dir::GlobalTypeId; 4]>::new();
        let mut index = 0;
        while index < members.len() {
            let member = members[index];
            index += 1;
            match self.union_arms(origin, member)? {
                Some(nested) => members.extend(nested),
                None => {
                    let head = self.normalize_stuck(origin, member)?;
                    match self.union_arms(origin, head)? {
                        Some(nested) => members.extend(nested),
                        None => leaves.push(member),
                    }
                }
            }
        }

        Ok(Some(leaves))
    }

    /// Decide a union target by membership when direct proof fell short.
    pub(in crate::check) fn decide_union_membership(
        &mut self,
        origin: Origin,
        relation: Relation,
        decision: bool,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<bool> {
        // a direct proof needs no membership fallback
        if decision {
            return Ok(true);
        }

        // membership tags the value into the union carrier and never widens
        if relation == Relation::Widens {
            return Ok(false);
        }
        let dir::Type::Union(union) = self.ty(target)? else {
            return Ok(false);
        };

        // relate the source against any one element
        let elements = self.type_ids(target.module_id, union.elements)?.to_vec();
        for element in elements {
            if self.decide_relation(origin, relation, source, element)? {
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Decide whether every source relates to one target.
    pub(in crate::check) fn decide_all_sources(
        &mut self,
        origin: Origin,
        relation: Relation,
        sources: &[dir::GlobalTypeId],
        target: dir::GlobalTypeId,
    ) -> CompilerResult<bool> {
        for source in sources {
            if !self.decide_relation(origin, relation, *source, target)? {
                return Ok(false);
            }
        }

        Ok(true)
    }

    /// Decide whether any source relates to one target.
    pub(in crate::check) fn decide_any_source(
        &mut self,
        origin: Origin,
        relation: Relation,
        sources: &[dir::GlobalTypeId],
        target: dir::GlobalTypeId,
    ) -> CompilerResult<bool> {
        for source in sources {
            if self.decide_relation(origin, relation, *source, target)? {
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Decide whether one source relates to every target.
    pub(in crate::check) fn decide_all_targets(
        &mut self,
        origin: Origin,
        relation: Relation,
        source: dir::GlobalTypeId,
        targets: &[dir::GlobalTypeId],
    ) -> CompilerResult<bool> {
        for target in targets {
            if !self.decide_relation(origin, relation, source, *target)? {
                return Ok(false);
            }
        }

        Ok(true)
    }

    /// Decide whether one source relates to any target.
    pub(in crate::check) fn decide_any_target(
        &mut self,
        origin: Origin,
        relation: Relation,
        source: dir::GlobalTypeId,
        targets: &[dir::GlobalTypeId],
    ) -> CompilerResult<bool> {
        // exact singleton keys prove membership without candidate relations
        if let Some(source_key) = self.static_key_from_type(source)? {
            for target in targets {
                if self.static_key_from_type(*target)? == Some(source_key) {
                    return Ok(true);
                }
            }
        }

        // relate the source against any one target
        for target in targets {
            if self.decide_relation(origin, relation, source, *target)? {
                return Ok(true);
            }
        }

        Ok(false)
    }
}
