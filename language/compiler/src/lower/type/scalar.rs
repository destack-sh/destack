use destack_dir as dir;
use destack_mir as mir;

use crate::lower::LowerState;
use crate::{CompilerError, CompilerResult, LowerError};

impl LowerState<'_> {
    /// Return one literal of a union whose arms share one scalar domain.
    ///
    /// The union stores such arms untagged.
    pub(in crate::lower) fn scalar_literal_union(
        &self,
        id: dir::GlobalTypeId,
    ) -> CompilerResult<Option<dir::Literal>> {
        // require a union head
        let dir::Type::Union(union) = self.ty(id)? else {
            return Ok(None);
        };

        // require every flattened arm to share the first arm's scalar domain
        let mut first = None;
        for element in self.flatten_union_members(id.module_id, &union)? {
            let dir::Type::Literal(literal) = self.ty(element)? else {
                return Ok(None);
            };
            let Some(domain) = literal.scalar_domain() else {
                return Ok(None);
            };
            match first {
                None => first = Some(literal),
                Some(kept) if kept.scalar_domain() == Some(domain) => {}
                Some(_) => return Ok(None),
            }
        }

        Ok(first)
    }

    /// Lower one type to its concrete scalar form.
    pub(in crate::lower) fn scalar_type(&self, ty: &dir::Type) -> CompilerResult<mir::Type> {
        match ty {
            dir::Type::Void => Ok(mir::Type::Void),
            dir::Type::Literal(_) => Ok(mir::Type::Void),
            dir::Type::Null | dir::Type::Undefined => Ok(mir::Type::Void),
            dir::Type::Primitive(primitive) => self.lower_primitive_type(primitive),
            // reject open heads, which materialize resolves before lowering
            dir::Type::Member(_)
            | dir::Type::Operation(_)
            | dir::Type::Variable(_)
            | dir::Type::This => Err(CompilerError::Internal {
                message: format!("an unreduced '{}' type", ty.variant_name()),
            }),
            // reject every other head
            other => Err(LowerError::Unsupported {
                anchor: self.module.into(),
                construct: format!("the '{}' type", other.variant_name()),
            }
            .into()),
        }
    }

    /// Lower one primitive type to its concrete scalar form.
    fn lower_primitive_type(&self, primitive: &dir::PrimitiveType) -> CompilerResult<mir::Type> {
        match primitive {
            dir::PrimitiveType::Boolean => Ok(mir::Type::Boolean),
            dir::PrimitiveType::Integer(integer) => Ok(match integer.width() {
                Some(width) => mir::Type::Int {
                    width,
                    is_signed: integer.is_signed(),
                },
                None if integer.is_signed() => mir::Type::Isize,
                None => mir::Type::Usize,
            }),
            dir::PrimitiveType::Character => Ok(mir::Type::Int {
                width: 32,
                is_signed: false,
            }),
            dir::PrimitiveType::Float(float) => Ok(mir::Type::Float(match float {
                dir::FloatType::Float32 => mir::FloatType::Float32,
                dir::FloatType::Float64 => mir::FloatType::Float64,
            })),
            // reject every other primitive
            other => Err(LowerError::Unsupported {
                anchor: self.module.into(),
                construct: format!("the '{}' type", other.as_str()),
            }
            .into()),
        }
    }
}
