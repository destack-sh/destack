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
    ) -> CompilerResult<(mir::Value, mir::TypeId)> {
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
        ty: mir::TypeId,
        dispatch: &Dispatch,
    ) -> CompilerResult<mir::Constant> {
        let value = u128::from(matches!(dispatch, Dispatch::One));
        let pointer_bits = self.state.layout.pointer_bits();
        let constant = match self.state.tree.type_definition(ty) {
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

    /// Return the ordering one parameter binds, a closed ordering as itself.
    pub(super) fn ordering_argument(
        &self,
        ordering: mir::MemoryOrdering,
    ) -> CompilerResult<mir::MemoryOrdering> {
        let mir::MemoryOrdering::Parameter(index) = ordering else {
            return Ok(ordering);
        };
        let Some(mir::GenericArgument::Value(value)) = self.arguments.get(index as usize) else {
            return Err(CompilerError::Internal {
                message: format!("an ordering parameter {index} bound outside a value argument"),
            });
        };
        let mir::Static::Integer(ordinal) = *self.state.tree.static_value(*value) else {
            return Err(CompilerError::Internal {
                message: format!("an ordering parameter {index} bound outside an enum case"),
            });
        };
        u32::try_from(ordinal)
            .ok()
            .and_then(mir::MemoryOrdering::from_ordinal)
            .ok_or_else(|| CompilerError::Internal {
                message: format!("an atomic ordering ordinal {ordinal} without a declared case"),
            })
    }

    /// Return the closed constant one value parameter binds, at the constant's own type.
    pub(super) fn constant_argument(
        &self,
        index: u32,
        ty: mir::TypeId,
    ) -> CompilerResult<mir::Constant> {
        let Some(mir::GenericArgument::Value(value)) = self.arguments.get(index as usize) else {
            return Err(CompilerError::Internal {
                message: format!("a value parameter {index} bound outside a value argument"),
            });
        };
        let value = self.state.tree.static_value(*value).clone();
        let constant = match (value, self.state.tree.type_definition(ty)) {
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

/// The representation one nullish constant specializes to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum NullishCase {
    /// The payloadless case of a variant representation.
    Case(u32),
    /// The unit value of a singleton representation.
    Unit,
    /// The null pointer of a pointer representation.
    Pointer,
}

impl Specialization<'_, '_> {
    /// Return the representation one nullish constant takes at an instance type.
    pub(super) fn nullish_case(
        &self,
        ty: mir::TypeId,
        is_undefined: bool,
    ) -> CompilerResult<NullishCase> {
        let stored = self.state.tree.storage_type(ty);
        match self.state.tree.type_definition(stored) {
            // select the payloadless case holding the singleton
            mir::Type::Variant { cases, .. } => cases
                .iter()
                .position(|case| self.is_nullish(case.ty, is_undefined))
                .map(|case| NullishCase::Case(case as u32))
                .ok_or_else(|| CompilerError::Internal {
                    message: "a nullish constant at a variant without its case".to_string(),
                }),
            // the singleton itself holds no bytes
            _ if self.is_nullish(stored, is_undefined) => Ok(NullishCase::Unit),
            // a null pointer stays a null constant
            mir::Type::Pointer { .. } if !is_undefined => Ok(NullishCase::Pointer),
            other => Err(CompilerError::Internal {
                message: format!("a nullish constant at the representation {other:?}"),
            }),
        }
    }

    /// Return whether one type stores the singleton a nullish constant names.
    fn is_nullish(&self, ty: mir::TypeId, is_undefined: bool) -> bool {
        let stored = self.state.tree.storage_type(ty);

        matches!(
            (is_undefined, self.state.tree.type_definition(stored)),
            (true, mir::Type::Void) | (false, mir::Type::Null)
        )
    }
}
