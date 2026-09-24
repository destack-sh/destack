use destack_dir as dir;

use crate::CompilerResult;
use crate::sema::{
    CauseId, CheckState, GenericParameterId, Origin, Relation, TypeSubstitution, Verdict,
};

impl CheckState<'_> {
    /// Relate two region terms, or return None for pairs outside the region kinds.
    pub(in crate::sema) fn relate_region_terms(
        &mut self,
        origin: Origin,
        cause: CauseId,
        relation: Relation,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Option<Verdict>> {
        // read both terms through their reductions
        let source = self.normalize(origin, source)?;
        let target = self.normalize(origin, target)?;

        // relate the space of two region pairs here, the extent bounds pushed for verify to prove
        if let (dir::Type::Region(source_region), dir::Type::Region(target_region)) =
            (self.ty(source)?, self.ty(target)?)
        {
            self.constrain_type(
                origin,
                cause,
                relation,
                source_region.extent,
                target_region.extent,
            )?;
            let spaces = self.constrain_type(
                origin,
                cause,
                relation,
                source_region.space,
                target_region.space,
            )?;
            if spaces == Verdict::Fails {
                return Ok(Some(Verdict::Fails));
            }

            return Ok(Some(Verdict::Holds));
        }

        // let a bare region term abstract every region, pairs included
        if self.memory_kind(source)? == Some(dir::MemoryParameter::Region)
            && self.memory_kind(target)? == Some(dir::MemoryParameter::Region)
        {
            return Ok(Some(Verdict::Holds));
        }

        Ok(None)
    }

    /// Match two region terms binding free pattern terms, or return None for other kinds.
    pub(in crate::sema) fn match_region_terms(
        &mut self,
        origin: Origin,
        parameters: &[GenericParameterId],
        substitution: &mut TypeSubstitution,
        pattern: dir::GlobalTypeId,
        actual: dir::GlobalTypeId,
    ) -> CompilerResult<Option<bool>> {
        // match the extent and the space of two region pairs
        if let (dir::Type::Region(pattern_region), dir::Type::Region(actual_region)) =
            (self.ty(pattern)?, self.ty(actual)?)
        {
            let pairs = [
                (pattern_region.extent, actual_region.extent, false),
                (pattern_region.space, actual_region.space, true),
            ];
            for (pattern, actual, decides) in pairs {
                // bind one free pattern term to the actual, a bound one comparing its binding
                let mut pattern = self.shallow_resolve(pattern)?;
                if let dir::Type::Parameter(parameter) = self.ty(pattern)?
                    && parameters.contains(&parameter)
                {
                    match substitution.argument(parameter) {
                        Some(bound) => pattern = self.shallow_resolve(bound)?,
                        None => {
                            let actual = self.shallow_resolve(actual)?;
                            self.bind_generic_argument(origin, substitution, parameter, actual)?;

                            continue;
                        }
                    }
                }
                // reject two conflicting spaces
                if decides
                    && self.decide_relation(origin, Relation::Equal, pattern, actual)?
                        == Verdict::Fails
                {
                    return Ok(Some(false));
                }
            }

            return Ok(Some(true));
        }

        // bind a free bare region pattern to the whole actual region
        if let dir::Type::Parameter(parameter) = self.ty(pattern)?
            && parameters.contains(&parameter)
            && substitution.argument(parameter).is_none()
            && self.memory_kind(actual)? == Some(dir::MemoryParameter::Region)
        {
            self.bind_generic_argument(origin, substitution, parameter, actual)?;

            return Ok(Some(true));
        }

        // match bare region terms freely
        if self.memory_kind(pattern)? == Some(dir::MemoryParameter::Region)
            && self.memory_kind(actual)? == Some(dir::MemoryParameter::Region)
        {
            return Ok(Some(true));
        }

        Ok(None)
    }
}
