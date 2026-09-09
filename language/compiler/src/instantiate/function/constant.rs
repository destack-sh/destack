use destack_core::float_to_bits;
use destack_mir as mir;

use crate::instantiate::function::Specialization;
use crate::instantiate::module::Dispatch;
use crate::{CompilerError, CompilerResult};

impl Specialization<'_, '_> {
    /// Return the destination of one structural requirement call with its type in this tree.
    pub(super) fn structural_result(
        &mut self,
        destination: Option<mir::Value>,
    ) -> CompilerResult<(mir::Value, mir::LocalNodeId<mir::Type>)> {
        let Some(destination) = destination else {
            return Err(CompilerError::Internal {
                message: "a structural requirement call without a destination".to_string(),
            });
        };
        let Some(result) = self.source.get(self.template).value_type(destination) else {
            return Err(CompilerError::Internal {
                message: "a structural requirement call without a typed destination".to_string(),
            });
        };

        Ok((destination, self.ty(result)))
    }

    /// Return the identity constant one structural dispatch names at a scalar representation.
    pub(super) fn identity_constant(
        &self,
        ty: mir::LocalNodeId<mir::Type>,
        dispatch: &Dispatch,
    ) -> CompilerResult<mir::Constant> {
        let value = u128::from(matches!(dispatch, Dispatch::One));
        let pointer_bits = self.state.layout.pointer_bits();
        let constant = match self.state.tree.get(ty) {
            mir::Type::Int { width, is_signed } => match is_signed {
                true => mir::Constant::Int {
                    value: value as i128,
                    width: *width,
                    is_signed: true,
                },
                false => mir::Constant::UInt {
                    value,
                    width: *width,
                },
            },
            mir::Type::Isize => mir::Constant::Int {
                value: value as i128,
                width: pointer_bits,
                is_signed: true,
            },
            mir::Type::Usize => mir::Constant::UInt {
                value,
                width: pointer_bits,
            },
            mir::Type::Float(format) => mir::Constant::Float {
                bits: float_to_bits(format.format(), value as f64),
                format: *format,
            },
            other => {
                return Err(CompilerError::Internal {
                    message: format!("an identity constant at the non-scalar type {other:?}"),
                });
            }
        };

        Ok(constant)
    }

    /// Return the closed constant one value parameter binds, at the constant's own type.
    pub(super) fn constant_argument(
        &self,
        index: u32,
        ty: mir::LocalNodeId<mir::Type>,
    ) -> CompilerResult<mir::Constant> {
        let Some(mir::GenericArgument::Value(value)) = self.arguments.get(index as usize) else {
            return Err(CompilerError::Internal {
                message: format!("a value parameter {index} bound outside a value argument"),
            });
        };
        let value = self.state.tree.static_value(*value).clone();
        let constant = match (value, self.state.tree.get(ty)) {
            (mir::Static::Integer(value), mir::Type::Int { width, is_signed }) => match is_signed {
                true => mir::Constant::Int {
                    value: i128::from(value),
                    width: *width,
                    is_signed: true,
                },
                false => mir::Constant::UInt {
                    value: value as u128,
                    width: *width,
                },
            },
            (mir::Static::Boolean(value), mir::Type::Boolean) => mir::Constant::Boolean { value },
            (mir::Static::Float(bits), mir::Type::Float(format)) => mir::Constant::Float {
                bits: float_to_bits(format.format(), f64::from_bits(bits)),
                format: *format,
            },
            (value, ty) => {
                return Err(CompilerError::Internal {
                    message: format!("a value parameter {value:?} bound at the type {ty:?}"),
                });
            }
        };

        Ok(constant)
    }
}
