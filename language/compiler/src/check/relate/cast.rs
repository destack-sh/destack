use destack_dir as dir;

use crate::CompilerResult;
use crate::check::{CheckState, Origin, Relation};

impl CheckState<'_> {
    /// Decide explicit castability.
    pub(in crate::check) fn decide_castable(
        &mut self,
        origin: Origin,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<bool> {
        // lossless numeric widening requires explicit cast
        if let (dir::Type::Primitive(source), dir::Type::Primitive(target)) =
            (self.ty(source)?, self.ty(target)?)
            && source.widens_to(target)
        {
            return Ok(true);
        }

        // machine scalars convert explicitly to any numeric width;
        //  parameters classify through their scalar bounds
        if matches!(
            self.ty(target)?,
            dir::Type::Primitive(dir::PrimitiveType::Integer(_) | dir::PrimitiveType::Float(_))
        ) && let Some(families) = self.scalar_families(origin, source)?
            && !families.is_empty()
            && families.iter().all(|family| {
                matches!(
                    family,
                    dir::ScalarFamily::Domain(
                        dir::ScalarDomain::Integer
                            | dir::ScalarDomain::Float
                            | dir::ScalarDomain::Character
                    )
                )
            })
        {
            return Ok(true);
        }

        // concrete newtypes project explicitly to their backing type
        if let Some(instance) = self.decompose_newtype(origin, source)? {
            let backing = instance.backing;
            if self.decide_relation(origin, Relation::Castable, backing, target)? {
                return Ok(true);
            }
        }

        // an assignable relation in either direction admits an explicit cast
        if self.decide_assignable(origin, Relation::Assignable, source, target)? {
            return Ok(true);
        }

        self.decide_assignable(origin, Relation::Assignable, target, source)
    }
}
