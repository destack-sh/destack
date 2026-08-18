use destack_dir as dir;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::sema::{CauseId, CheckState, Origin, Relation, Verdict};

impl CheckState<'_> {
    /// Return the members of one union target with its enclosing forms.
    pub(in crate::sema) fn union_arms(
        &mut self,
        origin: Origin,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Option<SmallVec<[dir::GlobalTypeId; 4]>>> {
        // normalize the base so aliases expose their stored union
        let chain = self.form_chain(origin, target)?;
        let base = self.normalize(origin, chain.base())?;
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
    pub(in crate::sema) fn union_leaves(
        &mut self,
        origin: Origin,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Option<SmallVec<[dir::GlobalTypeId; 4]>>> {
        // reduce a stuck named head, which may still hide a union
        let mut arms = self.union_arms(origin, target)?;
        if arms.is_none() {
            let head = self.structurally_normalize(origin, target)?;
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
                    let head = self.structurally_normalize(origin, member)?;
                    match self.union_arms(origin, head)? {
                        Some(nested) => members.extend(nested),
                        None => leaves.push(member),
                    }
                }
            }
        }

        Ok(Some(leaves))
    }

    /// Relate a union target by membership when direct proof fell short.
    pub(in crate::sema) fn relate_union_membership(
        &mut self,
        origin: Origin,
        cause: CauseId,
        relation: Relation,
        decision: Verdict,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Verdict> {
        // a direct proof needs no membership fallback
        if decision == Verdict::Holds {
            return Ok(Verdict::Holds);
        }

        // membership tags the value into the union carrier and never widens
        if relation == Relation::Widens {
            return Ok(decision);
        }
        let dir::Type::Union(union) = self.ty(target)? else {
            return Ok(decision);
        };

        // relate the source against any one element, closed arms before open arms
        let elements: SmallVec<[_; 8]> = self.type_ids(target.module_id, union.elements)?.into();
        let mut open: SmallVec<[_; 8]> = SmallVec::new();
        let mut verdict = decision;
        for element in elements {
            if matches!(self.ty(element)?, dir::Type::Variable(_)) {
                open.push(element);
                continue;
            }
            verdict = verdict.or(self.constrain_type(origin, cause, relation, source, element)?);
            if verdict == Verdict::Holds {
                return Ok(Verdict::Holds);
            }
        }
        for element in open {
            verdict = verdict.or(self.constrain_type(origin, cause, relation, source, element)?);
            if verdict == Verdict::Holds {
                return Ok(Verdict::Holds);
            }
        }

        Ok(verdict)
    }

    /// Relate every source to one target.
    pub(in crate::sema) fn relate_all_sources(
        &mut self,
        origin: Origin,
        cause: CauseId,
        relation: Relation,
        sources: &[dir::GlobalTypeId],
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Verdict> {
        let mut verdict = Verdict::Holds;
        for source in sources {
            verdict = verdict.and(self.constrain_type(origin, cause, relation, *source, target)?);
            if verdict == Verdict::Fails {
                return Ok(Verdict::Fails);
            }
        }

        Ok(verdict)
    }

    /// Relate any one source to one target.
    pub(in crate::sema) fn relate_any_source(
        &mut self,
        origin: Origin,
        cause: CauseId,
        relation: Relation,
        sources: &[dir::GlobalTypeId],
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Verdict> {
        // choose one open arm by constraint over the whole candidate set
        let mut is_open = self.type_flags(target)?.has_variable();
        for source in sources {
            is_open = is_open || self.type_flags(*source)?.has_variable();
        }
        if is_open {
            let candidates = sources
                .iter()
                .map(|source| (*source, target))
                .collect::<SmallVec<[_; 4]>>();

            return self.constrain_any_relation(origin, cause, relation, &candidates);
        }

        // relate any one source against the target
        let mut verdict = Verdict::Fails;
        for source in sources {
            verdict = verdict.or(self.constrain_type(origin, cause, relation, *source, target)?);
            if verdict == Verdict::Holds {
                return Ok(Verdict::Holds);
            }
        }

        Ok(verdict)
    }

    /// Relate one source to every target.
    pub(in crate::sema) fn relate_all_targets(
        &mut self,
        origin: Origin,
        cause: CauseId,
        relation: Relation,
        source: dir::GlobalTypeId,
        targets: &[dir::GlobalTypeId],
    ) -> CompilerResult<Verdict> {
        let mut verdict = Verdict::Holds;
        for target in targets {
            verdict = verdict.and(self.constrain_type(origin, cause, relation, source, *target)?);
            if verdict == Verdict::Fails {
                return Ok(Verdict::Fails);
            }
        }

        Ok(verdict)
    }

    /// Relate one source to any one target.
    pub(in crate::sema) fn relate_any_target(
        &mut self,
        origin: Origin,
        cause: CauseId,
        relation: Relation,
        source: dir::GlobalTypeId,
        targets: &[dir::GlobalTypeId],
    ) -> CompilerResult<Verdict> {
        // exact singleton keys prove membership without candidate relations
        if let Some(source_key) = self.static_key_from_type(source)? {
            for target in targets {
                if self.static_key_from_type(*target)? == Some(source_key) {
                    return Ok(Verdict::Holds);
                }
            }
        }

        // choose one open arm by constraint over the whole candidate set
        let mut is_open = self.type_flags(source)?.has_variable();
        for target in targets {
            is_open = is_open || self.type_flags(*target)?.has_variable();
        }
        if is_open {
            let candidates = targets
                .iter()
                .map(|target| (source, *target))
                .collect::<SmallVec<[_; 4]>>();

            return self.constrain_any_relation(origin, cause, relation, &candidates);
        }

        // relate the source against any one target
        let mut verdict = Verdict::Fails;
        for target in targets {
            verdict = verdict.or(self.constrain_type(origin, cause, relation, source, *target)?);
            if verdict == Verdict::Holds {
                return Ok(Verdict::Holds);
            }
        }

        Ok(verdict)
    }
}
