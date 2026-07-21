use destack_dir as dir;

use crate::CompilerResult;
use crate::check::{Answer, CheckState, Origin, Relation, ScalarFamily, answer};

impl CheckState<'_> {
    /// Decide explicit castability.
    pub(in crate::check) fn decide_castable(
        &mut self,
        origin: Origin,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<bool>> {
        // lossless numeric widening requires explicit cast
        if let (dir::Type::Primitive(source), dir::Type::Primitive(target)) =
            (self.ty(source)?, self.ty(target)?)
            && source.widens_to(target)
        {
            return Ok(Answer::Ready(true));
        }

        // machine scalars convert explicitly to any numeric width;
        //  parameters classify through their scalar bounds
        if matches!(
            self.ty(target)?,
            dir::Type::Primitive(dir::PrimitiveType::Integer(_) | dir::PrimitiveType::Float(_))
        ) && let Some(families) = answer!(self.scalar_families(origin, source)?)
            && !families.is_empty()
            && families.iter().all(|family| {
                matches!(
                    family,
                    ScalarFamily::Domain(
                        dir::ScalarDomain::Integer
                            | dir::ScalarDomain::Float
                            | dir::ScalarDomain::Character
                    )
                )
            })
        {
            return Ok(Answer::Ready(true));
        }

        // concrete newtypes project explicitly to their backing type
        if let Some(instance) = self.decompose_newtype(origin, source)? {
            let backing = instance.backing;
            let projected = self.decide_relation(origin, Relation::Castable, backing, target)?;
            if !matches!(projected, Answer::Ready(false)) {
                return Ok(projected);
            }
        }

        let forward = self.decide_assignable(origin, Relation::Assignable, source, target)?;
        if forward.is_ready_true() {
            return Ok(Answer::Ready(true));
        }

        let backward = self.decide_assignable(origin, Relation::Assignable, target, source)?;
        if backward.is_ready_true() {
            return Ok(Answer::Ready(true));
        }

        Ok(forward.or(backward))
    }
}
