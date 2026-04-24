use super::*;

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Check readonly assignability for array types.
    pub(super) fn array_readonly_assignable(
        &self,
        target_readonly: bool,
        source_readonly: bool,
    ) -> bool {
        if source_readonly && !target_readonly {
            return false;
        }
        true
    }

    /// Report unsound array variance for mutable arrays when configured.
    pub(super) fn check_unsound_array_variance(
        &self,
        ctx: &mut AssignContext<'_>,
        anchor: LocalNodeIdAny,
        target_readonly: bool,
        source_readonly: bool,
        target_element: LocalTypeId,
        source_element: LocalTypeId,
    ) {
        if !ctx.options.no_unsound_variance {
            return;
        }

        if target_readonly || source_readonly {
            return;
        }

        let target_assignable = self
            .is_type_assignable(
                &mut ctx.type_context_reborrow(),
                target_element,
                source_element,
            )
            .is_assignable();
        let source_assignable = self
            .is_type_assignable(
                &mut ctx.type_context_reborrow(),
                source_element,
                target_element,
            )
            .is_assignable();

        if target_assignable && !source_assignable {
            self.report_unsound_variance(&*ctx, anchor);
        }
    }

    /// Check readonly assignability for tuple elements.
    pub(super) fn tuple_element_readonly_assignable(
        &self,
        target_readonly: bool,
        source_readonly: bool,
    ) -> bool {
        if source_readonly && !target_readonly {
            return false;
        }
        true
    }

    /// Check if a tuple is readonly.
    pub(super) fn tuple_is_readonly(&self, tuple_is_readonly: bool) -> bool {
        tuple_is_readonly
    }

    /// Check type literal assignability.
    pub(super) fn is_type_literal_assignable(
        &self,
        target: &TypeLiteral,
        source: &TypeLiteral,
        options: &AnalyzeOptions,
    ) -> Assignability {
        // exact match
        if target == source {
            return Assignability::Assignable;
        }

        match (target, source) {
            // any/unknown already handled above, but handle for completeness
            (TypeLiteral::Any, _) | (TypeLiteral::Unknown, _) => Assignability::Assignable,
            (_, TypeLiteral::Never) | (_, TypeLiteral::Any) => Assignability::Assignable,

            // null is only assignable to null (or any/unknown)
            (TypeLiteral::Null, TypeLiteral::Null) => Assignability::Assignable,

            // object is assignable to object
            (TypeLiteral::Object, TypeLiteral::Object) => Assignability::Assignable,

            // undefined is only assignable to undefined or void
            (TypeLiteral::Void, TypeLiteral::Undefined) => Assignability::Assignable,
            (TypeLiteral::Undefined, TypeLiteral::Undefined) => Assignability::Assignable,

            // primitives: check for exact match or numeric widening
            (
                TypeLiteral::Primitive(target_primitive),
                TypeLiteral::Primitive(source_primitive),
            ) => {
                if target_primitive == source_primitive
                    || self.is_primitive_exactly_equivalent(target_primitive, source_primitive)
                    || (!options.no_implicit_conversions
                        && self.is_primitive_numeric_assignable(target_primitive, source_primitive))
                {
                    Assignability::Assignable
                } else {
                    Assignability::NotAssignable
                }
            }

            // scalar literal to primitive: check if literal is of that primitive type
            (TypeLiteral::Primitive(primitive_type), TypeLiteral::ScalarLiteral(literal)) => {
                if self.is_scalar_literal_assignable(literal, primitive_type, options) {
                    Assignability::Assignable
                } else {
                    Assignability::NotAssignable
                }
            }

            // everything else: not assignable
            _ => Assignability::NotAssignable,
        }
    }

    /// Check whether primitive type variants describe the same concrete type.
    pub(super) fn is_primitive_exactly_equivalent(
        &self,
        target: &PrimitiveType,
        source: &PrimitiveType,
    ) -> bool {
        match (target, source) {
            (PrimitiveType::Int(target_int), PrimitiveType::Int(source_int)) => {
                target_int.simplify() == source_int.simplify()
            }
            (PrimitiveType::Float(target_float), PrimitiveType::Float(source_float)) => {
                target_float.simplify() == source_float.simplify()
            }
            _ => false,
        }
    }

    /// Check if a scalar literal value matches a primitive type.
    pub(crate) fn is_scalar_literal_assignable(
        &self,
        literal: &ScalarLiteral,
        ty: &PrimitiveType,
        options: &AnalyzeOptions,
    ) -> bool {
        match (literal, ty) {
            // boolean
            (ScalarLiteral::Boolean(_), PrimitiveType::Boolean) => true,
            // string (including regex strings)
            (
                ScalarLiteral::String(_) | ScalarLiteral::RegexString { .. },
                PrimitiveType::String,
            ) => true,
            // integer and float to number (JavaScript style)
            (ScalarLiteral::Integer(_) | ScalarLiteral::Float(_), PrimitiveType::Number) => true,
            // integer literal to specific int type: check range
            (ScalarLiteral::Integer(value), PrimitiveType::Int(int_type)) => {
                self.is_integer_literal_assignable(*value as i128, int_type)
            }
            // float literal to specific float type: always allowed (may lose precision)
            (ScalarLiteral::Float(_), PrimitiveType::Float(_)) => true,
            // integer literal to float type: always allowed (implicit conversion)
            (ScalarLiteral::Integer(_), PrimitiveType::Float(_)) => {
                !options.no_implicit_conversions
            }
            // bigint
            (ScalarLiteral::Bigint(_), PrimitiveType::Bigint) => true,
            // character
            (ScalarLiteral::Character(_), PrimitiveType::Character) => true,
            _ => false,
        }
    }

    /// Check if an integer literal value fits within the range of a specific int type.
    pub(crate) fn is_integer_literal_assignable(&self, value: i128, int_type: &IntType) -> bool {
        let (min, max) = match int_type {
            IntType::Int8 => (i8::MIN as i128, i8::MAX as i128),
            IntType::Int16 => (i16::MIN as i128, i16::MAX as i128),
            IntType::Int32 => (i32::MIN as i128, i32::MAX as i128),
            IntType::Int64 => (i64::MIN as i128, i64::MAX as i128),
            IntType::Int128 | IntType::Int256 => (i128::MIN, i128::MAX),
            IntType::Uint8 => (0, u8::MAX as i128),
            IntType::Uint16 => (0, u16::MAX as i128),
            IntType::Uint32 => (0, u32::MAX as i128),
            IntType::Uint64 => (0, u64::MAX as i128),
            IntType::Uint128 | IntType::Uint256 => (0, i128::MAX), // (can't represent u128::MAX in i128)
            IntType::Isize | IntType::Usize => {
                // pointer sized integers: use target platform pointer size
                // for now, assume 64 bit
                if int_type.is_signed() {
                    (i64::MIN as i128, i64::MAX as i128)
                } else {
                    (0, u64::MAX as i128)
                }
            }
            IntType::Arbitrary { width, is_signed } => {
                if *is_signed {
                    let half_range = 1i128 << (width - 1);
                    (-half_range, half_range - 1)
                } else {
                    let max_val = if *width >= 128 {
                        i128::MAX
                    } else {
                        (1i128 << width) - 1
                    };
                    (0, max_val)
                }
            }
        };
        value >= min && value <= max
    }

    /// Check if numeric widening from source to target is allowed.
    /// Widening is allowed when assigning a smaller numeric type to a larger one.
    pub(super) fn is_primitive_numeric_assignable(
        &self,
        target: &PrimitiveType,
        source: &PrimitiveType,
    ) -> bool {
        match (target, source) {
            // number accepts any numeric type (JS compatibility)
            (PrimitiveType::Number, PrimitiveType::Int(_) | PrimitiveType::Float(_)) => true,

            // float widening: float32 to float64
            (PrimitiveType::Float(target_float), PrimitiveType::Float(source_float)) => {
                target_float.width() >= source_float.width()
            }

            // int to float: always allowed (may lose precision for large ints)
            (PrimitiveType::Float(_), PrimitiveType::Int(_)) => true,

            // signed int widening: int8 to int16 to int32 to int64 to int128
            (PrimitiveType::Int(target_int), PrimitiveType::Int(source_int)) => {
                match (target_int.width(), source_int.width()) {
                    (Some(tw), Some(sw)) => {
                        if target_int.is_signed() == source_int.is_signed() {
                            // same signedness: target must be at least as wide
                            tw >= sw
                        } else if target_int.is_signed() && !source_int.is_signed() {
                            // unsigned to signed: target must be strictly wider
                            // (uint8 max 255 fits in int16, but not int8)
                            tw > sw
                        } else {
                            // signed to unsigned: not safe (negative values)
                            false
                        }
                    }
                    // pointer sized ints: only allow same signedness
                    _ => false,
                }
            }

            _ => false,
        }
    }
}
