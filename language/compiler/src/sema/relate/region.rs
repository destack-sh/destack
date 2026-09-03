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
        // relate the extent and the space of two region pairs
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

            // fail on a space conflict between two concrete spaces, constant storage fitting any
            let source_space = self.place_space(source_region.space)?;
            let is_constant_source =
                relation != Relation::Equal && source_space == Some(dir::Space::Constant);
            if spaces == Verdict::Fails
                && source_space.is_some()
                && self.place_space(target_region.space)?.is_some()
                && !is_constant_source
            {
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

        // hold a predicate over a bare region term for every space it may name
        if relation == Relation::Subtype
            && self.memory_kind(source)? == Some(dir::MemoryParameter::Region)
            && self.memory_kind(target)? == Some(dir::MemoryParameter::Place)
        {
            return Ok(Some(Verdict::Holds));
        }

        // relate a region pair to a bare space term through its space
        if let dir::Type::Region(source_region) = self.ty(source)?
            && self.memory_kind(target)? == Some(dir::MemoryParameter::Place)
        {
            return Ok(Some(self.constrain_type(
                origin,
                cause,
                relation,
                source_region.space,
                target,
            )?));
        }
        if let dir::Type::Region(target_region) = self.ty(target)?
            && self.memory_kind(source)? == Some(dir::MemoryParameter::Place)
        {
            return Ok(Some(self.constrain_type(
                origin,
                cause,
                relation,
                source,
                target_region.space,
            )?));
        }

        // decide bare space terms where both spaces are concrete
        if self.memory_kind(source)? == Some(dir::MemoryParameter::Place)
            && self.memory_kind(target)? == Some(dir::MemoryParameter::Place)
        {
            if let (Some(source_space), Some(target_space)) =
                (self.place_space(source)?, self.place_space(target)?)
            {
                return Ok(Some(Verdict::decided(source_space == target_space)));
            }

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
                // bind one free pattern term to the actual
                let pattern = self.shallow_resolve(pattern)?;
                if let dir::Type::Parameter(parameter) = self.ty(pattern)?
                    && parameters.contains(&parameter)
                    && substitution.argument(parameter).is_none()
                {
                    let actual = self.shallow_resolve(actual)?;
                    self.bind_generic_argument(origin, substitution, parameter, actual)?;
                }
                // reject two conflicting concrete spaces
                else if decides
                    && self.place_space(pattern)?.is_some()
                    && self.place_space(actual)?.is_some()
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
