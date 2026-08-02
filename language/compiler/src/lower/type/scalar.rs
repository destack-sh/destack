use destack_dir as dir;
use destack_mir as mir;

use crate::lower::ModuleLowerer;
use crate::{CompilerResult, LowerError};

impl ModuleLowerer<'_> {
    /// Lower one type to its concrete scalar form.
    pub(in crate::lower) fn scalar_type(&self, ty: &dir::Type) -> CompilerResult<mir::Type> {
        match ty {
            dir::Type::Void => Ok(mir::Type::Void),
            // store no runtime value for singleton types
            dir::Type::Literal(_) => Ok(mir::Type::Void),
            dir::Type::Null | dir::Type::Undefined => Ok(mir::Type::Void),
            dir::Type::Primitive(primitive) => self.lower_primitive_type(primitive),
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
            dir::PrimitiveType::Float(float) => Ok(mir::Type::Float(match float {
                dir::FloatType::Float16 => mir::FloatType::Float16,
                dir::FloatType::Bfloat16 => mir::FloatType::Bfloat16,
                dir::FloatType::Float32 => mir::FloatType::Float32,
                dir::FloatType::Float64 => mir::FloatType::Float64,
            })),
            other => Err(LowerError::Unsupported {
                anchor: self.module.into(),
                construct: format!("the '{other:?}' type"),
            }
            .into()),
        }
    }
}
