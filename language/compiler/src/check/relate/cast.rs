use destack_dir as dir;

use crate::CompilerResult;
use crate::check::{CauseId, CheckState, Origin, Relation};

impl CheckState<'_> {
    /// Relate two types under explicit castability.
    pub(in crate::check) fn relate_castable(
        &mut self,
        origin: Origin,
        cause: CauseId,
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

        // convert machine scalars explicitly to any numeric width
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
            if self.constrain_type(origin, cause, Relation::Castable, backing, target)? {
                return Ok(true);
            }
        }

        // an assignable relation in either direction admits an explicit cast
        if self.relate_assignable(origin, cause, Relation::Assignable, source, target)? {
            return Ok(true);
        }

        self.relate_assignable(origin, cause, Relation::Assignable, target, source)
    }
}
