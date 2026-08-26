use destack_dir as dir;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::sema::{CauseId, CheckState, Origin, Relation, Verdict};

impl CheckState<'_> {
    /// Relate two types under explicit castability.
    pub(in crate::sema) fn relate_castable(
        &mut self,
        origin: Origin,
        cause: CauseId,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Verdict> {
        // lossless numeric widening requires explicit cast
        if let (dir::Type::Primitive(source), dir::Type::Primitive(target)) =
            (self.ty(source)?, self.ty(target)?)
            && source.widens_to(target)
        {
            return Ok(Verdict::Holds);
        }

        // scalar values convert explicitly to formats holding every inhabitant
        if let dir::Type::Primitive(format) = self.ty(target)?
            && matches!(
                format,
                dir::PrimitiveType::Integer(_) | dir::PrimitiveType::Float(_)
            )
            && let Some(families) = self.scalar_families(origin, source)?
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
            let mut parameters = SmallVec::new();
            let mut formats = SmallVec::new();
            self.collect_builtin_scalar_formats(
                origin,
                source,
                &families,
                &mut parameters,
                &mut formats,
            )?;

            // require every exact format, with open parameters leaving the set incomplete
            if parameters.is_empty()
                && !formats.is_empty()
                && formats.iter().all(|source| source.widens_to(format))
            {
                return Ok(Verdict::Holds);
            }
        }

        let mut verdict = Verdict::Fails;

        // enum variants project explicitly to their declared literal value
        if let dir::Type::Variant(variant) = self.ty(source)? {
            let owner = self.normalize(origin, variant.owner)?;
            let value = match self.ty(owner)? {
                dir::Type::Application(instance) => match self.definition(instance.symbol)? {
                    Some(dir::Definition::Enum(definition)) => {
                        definition.members.iter().find_map(|member| match member {
                            dir::DefinitionMember::EnumVariant(declared)
                                if declared.symbol == variant.variant =>
                            {
                                Some(declared.value)
                            }
                            _ => None,
                        })
                    }
                    _ => None,
                },
                _ => None,
            };

            // relate the projected literal against the cast target
            if let Some(value) = value {
                let literal = dir::Literal::from(value);
                let literal = self.intern_type(dir::Type::Literal(literal))?;
                verdict = verdict.or(self.constrain_type(
                    origin,
                    cause,
                    Relation::Castable,
                    literal,
                    target,
                )?);
                if verdict == Verdict::Holds {
                    return Ok(Verdict::Holds);
                }
            }
        }

        // enums project explicitly to their backing scalar
        if let Some(families) = self.scalar_families(origin, source)?
            && !families.is_empty()
            && families
                .iter()
                .all(|family| matches!(family, dir::ScalarFamily::Enum(_)))
        {
            // require every named enum's backing to reach the cast target
            let mut backed = Verdict::Holds;
            for family in families.iter().copied() {
                let dir::ScalarFamily::Enum(symbol) = family else {
                    continue;
                };
                let Some(dir::Definition::Enum(definition)) = self.definition(symbol)? else {
                    backed = Verdict::Fails;
                    break;
                };
                let primitive = match definition.backing {
                    dir::EnumBackingType::Integer(integer) => dir::PrimitiveType::Integer(integer),
                    dir::EnumBackingType::String => dir::PrimitiveType::String,
                };
                let backing = self.intern_type(dir::Type::Primitive(primitive))?;
                let projected =
                    self.constrain_type(origin, cause, Relation::Castable, backing, target)?;
                if projected == Verdict::Fails {
                    backed = Verdict::Fails;
                    break;
                }
                if projected == Verdict::Ambiguous {
                    backed = Verdict::Ambiguous;
                }
            }
            verdict = verdict.or(backed);
            if verdict == Verdict::Holds {
                return Ok(Verdict::Holds);
            }
        }

        // concrete newtypes project explicitly to their backing type
        if let Some(instance) = self.decompose_newtype(origin, source)? {
            let backing = instance.backing;
            verdict = verdict.or(self.constrain_type(
                origin,
                cause,
                Relation::Castable,
                backing,
                target,
            )?);
            if verdict == Verdict::Holds {
                return Ok(Verdict::Holds);
            }
        }

        // an assignable source casts explicitly to its target
        Ok(verdict.or(self.relate_assignable(
            origin,
            cause,
            Relation::Assignable,
            source,
            target,
        )?))
    }
}
