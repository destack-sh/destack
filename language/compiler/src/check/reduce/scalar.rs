use destack_dir as dir;
use smallvec::{SmallVec, smallvec};

use crate::CompilerResult;
use crate::check::{Answer, CheckState, Origin, answer};

/// One scalar operand family: a plain value domain or one enum's members.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) enum ScalarFamily {
    /// One scalar value domain.
    Domain(dir::ScalarDomain),
    /// Members of one enum declaration.
    Enum(dir::GlobalTypeId),
}

impl ScalarFamily {
    /// Return whether this family holds builtin numerics.
    pub(in crate::check) fn is_numeric(self) -> bool {
        matches!(self, Self::Domain(domain) if domain.is_numeric())
    }

    /// Return whether this family holds only integers.
    pub(in crate::check) fn is_integral(self) -> bool {
        matches!(self, Self::Domain(domain) if domain.is_integral())
    }
}

impl CheckState<'_> {
    /// Return the scalar families one type can hold, or `None` when
    /// it is not fully scalar.
    ///
    /// Leaves classify through their `dir` scalar domain.
    /// Parameters classify through their bounds in scope and unions
    /// through every distinct element family.
    pub(in crate::check) fn scalar_families(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<Option<SmallVec<[ScalarFamily; 4]>>>> {
        let root = answer!(self.reduce_type_head(origin, ty)?);
        let families = match self.ty(root)? {
            dir::Type::EnumMember(member) => Some(smallvec![ScalarFamily::Enum(member.owner)]),
            dir::Type::Instance(instance)
                if matches!(
                    self.definition(instance.symbol),
                    Some(dir::Definition::Enum(_))
                ) =>
            {
                Some(smallvec![ScalarFamily::Enum(root)])
            }
            // classify parameters through their scalar bound
            dir::Type::Parameter(parameter) => {
                let Some(bound) = answer!(self.scalar_parameter_bound(origin, parameter)?) else {
                    return Ok(Answer::Ready(None));
                };

                return self.scalar_families(origin, bound);
            }
            // collect every distinct element family
            dir::Type::Union(union) => {
                let elements: SmallVec<[_; 4]> =
                    SmallVec::from_slice(self.type_ids(root.module_id, union.elements)?);
                let mut families = SmallVec::<[ScalarFamily; 4]>::new();
                for element in elements {
                    let Some(element_families) = answer!(self.scalar_families(origin, element)?)
                    else {
                        return Ok(Answer::Ready(None));
                    };
                    for family in element_families {
                        if !families.contains(&family) {
                            families.push(family);
                        }
                    }
                }

                Some(families)
            }
            // classify leaves through their scalar domain
            other => other
                .scalar_domain()
                .map(|domain| smallvec![ScalarFamily::Domain(domain)]),
        };

        Ok(Answer::Ready(families))
    }

    /// Return one parameter's first bound holding only scalars.
    pub(in crate::check) fn scalar_parameter_bound(
        &mut self,
        origin: Origin,
        parameter: dir::GlobalGenericParameterId,
    ) -> CompilerResult<Answer<Option<dir::GlobalTypeId>>> {
        for bound in self.parameter_bounds(origin, parameter)? {
            // self-referential bounds cannot classify themselves
            let root = answer!(self.reduce_type_head(origin, bound)?);
            if matches!(self.ty(root)?, dir::Type::Parameter(_)) {
                continue;
            }

            if answer!(self.scalar_families(origin, bound)?).is_some() {
                return Ok(Answer::Ready(Some(bound)));
            }
        }

        Ok(Answer::Ready(None))
    }
}
