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
    /// Return the scalar families one type can hold, or `None` when it is not scalar.
    pub(in crate::check) fn scalar_families(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<Option<SmallVec<[ScalarFamily; 4]>>>> {
        let root = answer!(self.reduce_type_head(origin, ty)?);
        let families = match self.ty(root)? {
            dir::Type::EnumMember(member) => Some(smallvec![ScalarFamily::Enum(member.owner)]),
            dir::Type::Application(instance)
                if matches!(
                    self.definition(instance.symbol)?,
                    Some(dir::Definition::Enum(_))
                ) =>
            {
                Some(smallvec![ScalarFamily::Enum(root)])
            }
            // scalar markers classify as their whole domain
            dir::Type::Application(instance)
                if self.language_item(instance.symbol)? == Some(dir::LanguageItem::Integer) =>
            {
                Some(smallvec![ScalarFamily::Domain(dir::ScalarDomain::Integer)])
            }
            dir::Type::Application(instance)
                if self.language_item(instance.symbol)? == Some(dir::LanguageItem::Float) =>
            {
                Some(smallvec![ScalarFamily::Domain(dir::ScalarDomain::Float)])
            }
            // template literal patterns live in the string domain
            dir::Type::Operation(operation)
                if matches!(
                    self.type_operation(root.module_id, operation)?,
                    dir::TypeOperation::TemplateLiteral(_)
                ) =>
            {
                Some(smallvec![ScalarFamily::Domain(dir::ScalarDomain::String)])
            }
            // classify parameters through their scalar bound
            dir::Type::Parameter(parameter) => {
                let Some(bound) = answer!(self.scalar_parameter_bound(origin, parameter)?) else {
                    return Ok(Answer::Ready(None));
                };

                return self.scalar_families(origin, bound);
            }
            // arithmetic operations stay within their operand families
            dir::Type::Operation(operation)
                if let dir::TypeOperation::StaticBinary(binary) =
                    self.type_operation(root.module_id, operation)? =>
            {
                if binary.operator.yields_boolean() {
                    return Ok(Answer::Ready(Some(smallvec![ScalarFamily::Domain(
                        dir::ScalarDomain::Boolean
                    )])));
                }
                let Some(mut families) = answer!(self.scalar_families(origin, binary.left)?) else {
                    return Ok(Answer::Ready(None));
                };
                let Some(right) = answer!(self.scalar_families(origin, binary.right)?) else {
                    return Ok(Answer::Ready(None));
                };
                for family in right {
                    if !families.contains(&family) {
                        families.push(family);
                    }
                }

                Some(families)
            }
            dir::Type::Operation(operation)
                if let dir::TypeOperation::StaticUnary(unary) =
                    self.type_operation(root.module_id, operation)? =>
            {
                if unary.operator.yields_boolean() {
                    return Ok(Answer::Ready(Some(smallvec![ScalarFamily::Domain(
                        dir::ScalarDomain::Boolean
                    )])));
                }

                return self.scalar_families(origin, unary.target);
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

    /// Return the scalar result type of one static operation.
    pub(in crate::check) fn static_operation_type(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<Option<dir::GlobalTypeId>>> {
        let root = answer!(self.reduce_type_head(origin, ty)?);
        match self.ty(root)? {
            // binary operators join their operand types
            dir::Type::Operation(operation)
                if let dir::TypeOperation::StaticBinary(binary) =
                    self.type_operation(root.module_id, operation)? =>
            {
                if binary.operator.yields_boolean() {
                    let boolean = self.intern_type(
                        origin.module(),
                        dir::Type::Primitive(dir::PrimitiveType::Boolean),
                    )?;

                    return Ok(Answer::Ready(Some(boolean)));
                }
                let left = answer!(self.scalar_operand_type(origin, binary.left)?);
                let right = answer!(self.scalar_operand_type(origin, binary.right)?);

                // literals adopt their partner operand's type
                let joined = match (left, right) {
                    (Some(left), Some(right)) if left == right => Some(left),
                    (Some(left), None) => Some(left),
                    (None, Some(right)) => Some(right),
                    _ => None,
                };

                Ok(Answer::Ready(joined))
            }

            // unary operators keep their operand type
            dir::Type::Operation(operation)
                if let dir::TypeOperation::StaticUnary(unary) =
                    self.type_operation(root.module_id, operation)? =>
            {
                if unary.operator.yields_boolean() {
                    let boolean = self.intern_type(
                        origin.module(),
                        dir::Type::Primitive(dir::PrimitiveType::Boolean),
                    )?;

                    return Ok(Answer::Ready(Some(boolean)));
                }

                self.scalar_operand_type(origin, unary.target)
            }

            _ => Ok(Answer::Ready(None)),
        }
    }

    /// Return one operand's concrete scalar type.
    fn scalar_operand_type(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<Option<dir::GlobalTypeId>>> {
        let root = answer!(self.reduce_type_head(origin, ty)?);
        match self.ty(root)? {
            // concrete scalars type themselves
            dir::Type::Primitive(_) => Ok(Answer::Ready(Some(root))),

            // literals and ranges adopt their partner operand
            dir::Type::Literal(_) | dir::Type::Range(_) => Ok(Answer::Ready(None)),

            // parameters type through their scalar bound
            dir::Type::Parameter(parameter) => {
                let Some(bound) = answer!(self.scalar_parameter_bound(origin, parameter)?) else {
                    return Ok(Answer::Ready(None));
                };

                self.scalar_operand_type(origin, bound)
            }

            // nested operations type through their own result
            dir::Type::Operation(operation)
                if matches!(
                    self.type_operation(root.module_id, operation)?,
                    dir::TypeOperation::StaticBinary(_) | dir::TypeOperation::StaticUnary(_)
                ) =>
            {
                self.static_operation_type(origin, root)
            }

            _ => Ok(Answer::Ready(None)),
        }
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
