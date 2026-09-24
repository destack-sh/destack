use destack_dir as dir;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::sema::{CauseId, CheckState, Origin, Relation, Verdict};

impl CheckState<'_> {
    /// Relate two types under an explicit cast: the cast table, then inclusion.
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

            // require every exact format the operands name
            if parameters.is_empty()
                && !formats.is_empty()
                && formats.iter().all(|source| source.widens_to(format))
            {
                return Ok(Verdict::Holds);
            }
        }

        // accept the first explicit cast rule that holds
        let mut verdict = Verdict::Fails;

        // enum variants project explicitly to their declared literal value
        if let dir::Type::Variant(variant) = self.ty(source)? {
            let owner = self.normalize(origin, variant.owner)?;
            let value = match self.ty(owner)? {
                dir::Type::Application(instance) => {
                    match self.definition(instance.symbol)?.as_deref() {
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
                    }
                }
                _ => None,
            };

            // relate the projected literal against the cast target
            if let Some(value) = value {
                let literal = dir::Literal::from(value);
                let literal = self.intern_type(dir::Type::Literal(literal))?;
                verdict = verdict.or(self.relate_castable(origin, cause, literal, target)?);
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
            // require every named enum's backing to convert to the cast target
            let mut backed = Verdict::Holds;
            for family in families.iter().copied() {
                let dir::ScalarFamily::Enum(symbol) = family else {
                    continue;
                };
                let declared = self.definition(symbol)?;
                let Some(dir::Definition::Enum(definition)) = declared.as_deref() else {
                    backed = Verdict::Fails;
                    break;
                };
                let primitive = match definition.backing {
                    dir::EnumBackingType::Integer(integer) => dir::PrimitiveType::Integer(integer),
                    dir::EnumBackingType::String => dir::PrimitiveType::String,
                };
                let backing = self.intern_type(dir::Type::Primitive(primitive))?;
                let projected = self.relate_castable(origin, cause, backing, target)?;
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

        // step one newtype layer of the value or of a raw pointee to exactly its backing
        for (stepped_source, stepped_target) in self.newtype_steps(origin, source, target)? {
            let stepped =
                self.decide_relation(origin, Relation::Equal, stepped_source, stepped_target)?;
            if stepped == Verdict::Holds {
                return self.constrain_type(
                    origin,
                    cause,
                    Relation::Equal,
                    stepped_source,
                    stepped_target,
                );
            }
            verdict = verdict.or(stepped);
        }

        // cast an included source explicitly to its target
        Ok(verdict.or(self.constrain_type(origin, cause, Relation::Subtype, source, target)?))
    }

    /// Return the pairs one newtype step equates.
    pub(in crate::sema) fn newtype_steps(
        &mut self,
        origin: Origin,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<SmallVec<[(dir::GlobalTypeId, dir::GlobalTypeId); 4]>> {
        let mut layers = SmallVec::<[_; 2]>::from_slice(&[(source, target)]);
        if let (Some(source), Some(target)) = (
            self.raw_pointee(origin, source)?,
            self.raw_pointee(origin, target)?,
        ) {
            layers.push((source, target));
        }
        let mut steps = SmallVec::new();
        for (source, target) in layers {
            if let Some(instance) = self.decompose_newtype(origin, source)? {
                steps.push((instance.backing, target));
            }
            if let Some(instance) = self.decompose_newtype(origin, target)? {
                steps.push((source, instance.backing));
            }
        }

        Ok(steps)
    }

    /// Return the pointee of one raw pointer type.
    pub(in crate::sema) fn raw_pointee(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        let chain = self.form_chain(origin, ty)?;
        let is_raw = chain
            .ownership_form()
            .is_some_and(|form| matches!(form.form, dir::Form::Raw));

        Ok(is_raw.then(|| chain.base()))
    }
}
