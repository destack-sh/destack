use super::*;

#[allow(clippy::too_many_arguments)]
impl Compiler {
    pub(crate) fn infer_scalar_literal(&self, value: &ScalarLiteral) -> TypeLiteral {
        // return the literal type, not the widened primitive type
        // this allows `let x: int = 4` to work via assignability checking
        TypeLiteral::ScalarLiteral(value.clone())
    }

    /// Widen a scalar literal to its primitive type.
    pub(crate) fn widen_scalar_literal_for_module(
        &self,
        module: &Module,
        value: &ScalarLiteral,
    ) -> TypeLiteral {
        let primitive = match value {
            ScalarLiteral::Boolean(_) => PrimitiveType::Boolean,
            ScalarLiteral::Null => return TypeLiteral::Null,
            ScalarLiteral::Integer(value) => {
                if module.language_type.is_destack()
                    && self.is_integer_literal_assignable(*value as i128, &IntType::Int32)
                {
                    PrimitiveType::Int(IntType::Int32)
                } else {
                    PrimitiveType::Number
                }
            }
            ScalarLiteral::Float(_) => {
                if module.language_type.is_destack() {
                    PrimitiveType::Float(FloatType::Float64)
                } else {
                    PrimitiveType::Number
                }
            }
            ScalarLiteral::Bigint(_) => PrimitiveType::Bigint,
            ScalarLiteral::Character(_)
            | ScalarLiteral::String(_)
            | ScalarLiteral::RegexString { .. } => PrimitiveType::String,
        };

        TypeLiteral::Primitive(primitive)
    }

    /// Infer the result type of a binary operation.
    pub(crate) fn infer_binary_operation(
        &self,
        operator: &BinaryOperator,
        left: &Type,
        right: &Type,
        types: &TypeTable,
    ) -> Type {
        match operator {
            // comparison operators: try constant folding, else return boolean
            BinaryOperator::Equal
            | BinaryOperator::NotEqual
            | BinaryOperator::EqualStrict
            | BinaryOperator::NotEqualStrict
            | BinaryOperator::LessThan
            | BinaryOperator::LessThanOrEqual
            | BinaryOperator::GreaterThan
            | BinaryOperator::GreaterThanOrEqual => self
                .try_fold_comparison(operator, left, right)
                .unwrap_or(Type::TypeLiteral {
                    value: TypeLiteral::Primitive(PrimitiveType::Boolean),
                }),

            // `in` always returns boolean
            BinaryOperator::In => Type::TypeLiteral {
                value: TypeLiteral::Primitive(PrimitiveType::Boolean),
            },

            // logical operators: try constant folding, else return boolean
            BinaryOperator::And | BinaryOperator::Or => self
                .try_fold_logical(operator, left, right)
                .unwrap_or(Type::TypeLiteral {
                    value: TypeLiteral::Primitive(PrimitiveType::Boolean),
                }),

            // arithmetic operators: try constant folding, else widen types
            BinaryOperator::Add
            | BinaryOperator::Subtract
            | BinaryOperator::Multiply
            | BinaryOperator::Divide
            | BinaryOperator::Remainder
            | BinaryOperator::Exponent => self
                .try_infer_string_concatenation(operator, left, right, types)
                .or_else(|| self.try_fold_arithmetic(operator, left, right))
                .unwrap_or_else(|| self.widen_numeric_types(left, right)),

            _ => left.clone(),
        }
    }

    /// Try to constant fold a comparison operation on literal types.
    fn try_fold_comparison(
        &self,
        operator: &BinaryOperator,
        left: &Type,
        right: &Type,
    ) -> Option<Type> {
        // extract scalar literals from both sides
        let (left_lit, right_lit) = Self::extract_scalar_literals(left, right)?;

        // compare integers
        if let (ScalarLiteral::Integer(left_value), ScalarLiteral::Integer(right_value)) =
            (left_lit, right_lit)
        {
            let result = match operator {
                BinaryOperator::Equal | BinaryOperator::EqualStrict => left_value == right_value,
                BinaryOperator::NotEqual | BinaryOperator::NotEqualStrict => {
                    left_value != right_value
                }
                BinaryOperator::LessThan => left_value < right_value,
                BinaryOperator::LessThanOrEqual => left_value <= right_value,
                BinaryOperator::GreaterThan => left_value > right_value,
                BinaryOperator::GreaterThanOrEqual => left_value >= right_value,
                _ => return None,
            };
            return Some(Type::TypeLiteral {
                value: TypeLiteral::ScalarLiteral(ScalarLiteral::Boolean(result)),
            });
        }

        // compare floats (or mixed int/float)
        let (left_value, right_value) = Self::to_f64_pair(left_lit, right_lit)?;
        let result = match operator {
            BinaryOperator::Equal | BinaryOperator::EqualStrict => left_value == right_value,
            BinaryOperator::NotEqual | BinaryOperator::NotEqualStrict => left_value != right_value,
            BinaryOperator::LessThan => left_value < right_value,
            BinaryOperator::LessThanOrEqual => left_value <= right_value,
            BinaryOperator::GreaterThan => left_value > right_value,
            BinaryOperator::GreaterThanOrEqual => left_value >= right_value,
            _ => return None,
        };
        Some(Type::TypeLiteral {
            value: TypeLiteral::ScalarLiteral(ScalarLiteral::Boolean(result)),
        })
    }

    /// Try to constant fold a logical operation on literal types.
    fn try_fold_logical(
        &self,
        operator: &BinaryOperator,
        left: &Type,
        right: &Type,
    ) -> Option<Type> {
        // extract boolean literals from both sides
        let (left_lit, right_lit) = Self::extract_scalar_literals(left, right)?;

        if let (ScalarLiteral::Boolean(left_value), ScalarLiteral::Boolean(right_value)) =
            (left_lit, right_lit)
        {
            let result = match operator {
                BinaryOperator::And => *left_value && *right_value,
                BinaryOperator::Or => *left_value || *right_value,
                _ => return None,
            };
            return Some(Type::TypeLiteral {
                value: TypeLiteral::ScalarLiteral(ScalarLiteral::Boolean(result)),
            });
        }

        None
    }

    /// Try to constant fold an arithmetic operation on literal types.
    fn try_fold_arithmetic(
        &self,
        operator: &BinaryOperator,
        left: &Type,
        right: &Type,
    ) -> Option<Type> {
        // extract scalar literals from both sides
        let (left_lit, right_lit) = Self::extract_scalar_literals(left, right)?;

        // try to fold integer operations (preserves integer type)
        if let (ScalarLiteral::Integer(left_value), ScalarLiteral::Integer(right_value)) =
            (left_lit, right_lit)
        {
            let result = match operator {
                BinaryOperator::Add => left_value.checked_add(*right_value),
                BinaryOperator::Subtract => left_value.checked_sub(*right_value),
                BinaryOperator::Multiply => left_value.checked_mul(*right_value),
                BinaryOperator::Divide => {
                    if *right_value != 0 {
                        left_value.checked_div(*right_value)
                    } else {
                        None
                    }
                }
                BinaryOperator::Remainder => {
                    if *right_value != 0 {
                        left_value.checked_rem(*right_value)
                    } else {
                        None
                    }
                }
                BinaryOperator::Exponent => {
                    if *right_value >= 0 && *right_value <= u32::MAX as i64 {
                        left_value.checked_pow(*right_value as u32)
                    } else {
                        None
                    }
                }
                _ => None,
            };
            if let Some(result) = result {
                return Some(Type::TypeLiteral {
                    value: TypeLiteral::ScalarLiteral(ScalarLiteral::Integer(result)),
                });
            }
        }

        // try to fold float operations (if at least one operand is float)
        if matches!(left_lit, ScalarLiteral::Float(_))
            || matches!(right_lit, ScalarLiteral::Float(_))
        {
            let (left_value, right_value) = Self::to_f64_pair(left_lit, right_lit)?;
            let result = match operator {
                BinaryOperator::Add => left_value + right_value,
                BinaryOperator::Subtract => left_value - right_value,
                BinaryOperator::Multiply => left_value * right_value,
                BinaryOperator::Divide => left_value / right_value,
                BinaryOperator::Remainder => left_value % right_value,
                BinaryOperator::Exponent => left_value.powf(right_value),
                _ => return None,
            };
            return Some(Type::TypeLiteral {
                value: TypeLiteral::ScalarLiteral(ScalarLiteral::Float(result)),
            });
        }

        None
    }

    /// Try to infer string concatenation for add.
    fn try_infer_string_concatenation(
        &self,
        operator: &BinaryOperator,
        left: &Type,
        right: &Type,
        types: &TypeTable,
    ) -> Option<Type> {
        if !matches!(operator, BinaryOperator::Add) {
            return None;
        }

        if self.is_string_like_type(left, types) || self.is_string_like_type(right, types) {
            return Some(Type::TypeLiteral {
                value: TypeLiteral::Primitive(PrimitiveType::String),
            });
        }

        None
    }

    /// Check whether a type behaves like a string type.
    pub(crate) fn is_string_like_type(&self, ty: &Type, types: &TypeTable) -> bool {
        match ty {
            Type::TypeLiteral {
                value: TypeLiteral::Primitive(PrimitiveType::String),
            } => true,
            Type::TypeLiteral {
                value: TypeLiteral::ScalarLiteral(ScalarLiteral::String(_)),
            } => true,
            Type::Union { elements } => elements
                .iter()
                .all(|element_id| self.is_string_like_type(types.get_type(*element_id), types)),
            _ => false,
        }
    }

    /// Extract scalar literals from two types.
    fn extract_scalar_literals<'a>(
        left: &'a Type,
        right: &'a Type,
    ) -> Option<(&'a ScalarLiteral, &'a ScalarLiteral)> {
        match (left, right) {
            (
                Type::TypeLiteral {
                    value: TypeLiteral::ScalarLiteral(left_literal),
                },
                Type::TypeLiteral {
                    value: TypeLiteral::ScalarLiteral(right_literal),
                },
            ) => Some((left_literal, right_literal)),
            _ => None,
        }
    }

    /// Convert two scalar literals to f64 values (for numeric operations).
    fn to_f64_pair(left: &ScalarLiteral, right: &ScalarLiteral) -> Option<(f64, f64)> {
        let left_value = match left {
            ScalarLiteral::Integer(i) => *i as f64,
            ScalarLiteral::Float(f) => *f,
            _ => return None,
        };
        let right_value = match right {
            ScalarLiteral::Integer(i) => *i as f64,
            ScalarLiteral::Float(f) => *f,
            _ => return None,
        };
        Some((left_value, right_value))
    }

    /// Widen two numeric types to a common type.
    /// Used when constant folding fails (e.g., `x + 1` where x is a variable).
    pub(crate) fn widen_numeric_types(&self, left: &Type, right: &Type) -> Type {
        let left_prim = Self::to_numeric_primitive(left);
        let right_prim = Self::to_numeric_primitive(right);
        match (left_prim, right_prim) {
            // if both are known primitives, return the wider one
            (Some(left_primitive), Some(right_primitive)) => Type::TypeLiteral {
                value: TypeLiteral::Primitive(Self::wider_numeric_primitive(
                    &left_primitive,
                    &right_primitive,
                )),
            },
            // if one side is a primitive, use it
            (Some(p), None) | (None, Some(p)) => Type::TypeLiteral {
                value: TypeLiteral::Primitive(p),
            },
            // default to number when neither side has numeric primitive metadata
            (None, None) => Type::TypeLiteral {
                value: TypeLiteral::Primitive(PrimitiveType::Number),
            },
        }
    }

    /// Extract the numeric primitive type from a type.
    fn to_numeric_primitive(ty: &Type) -> Option<PrimitiveType> {
        match ty {
            Type::TypeLiteral {
                value: TypeLiteral::Primitive(p),
            } if Self::is_numeric_primitive(p) => Some(*p),
            Type::TypeLiteral {
                value: TypeLiteral::ScalarLiteral(ScalarLiteral::Integer(_)),
            } => None,
            Type::TypeLiteral {
                value: TypeLiteral::ScalarLiteral(ScalarLiteral::Float(_)),
            } => Some(PrimitiveType::Number),
            _ => None,
        }
    }

    /// Check if a primitive type is numeric.
    fn is_numeric_primitive(p: &PrimitiveType) -> bool {
        matches!(
            p,
            PrimitiveType::Number
                | PrimitiveType::Int(_)
                | PrimitiveType::Float(_)
                | PrimitiveType::Bigint
        )
    }

    /// Return the wider of two numeric primitive types.
    fn wider_numeric_primitive(left: &PrimitiveType, right: &PrimitiveType) -> PrimitiveType {
        // number is the widest
        if matches!(left, PrimitiveType::Number) || matches!(right, PrimitiveType::Number) {
            return PrimitiveType::Number;
        }
        // float is wider than int; pick the wider float
        match (left, right) {
            (PrimitiveType::Float(left_float), PrimitiveType::Float(right_float)) => {
                let wider = if left_float.width() >= right_float.width() {
                    *left_float
                } else {
                    *right_float
                };
                return PrimitiveType::Float(wider);
            }
            (PrimitiveType::Float(f), _) | (_, PrimitiveType::Float(f)) => {
                return PrimitiveType::Float(*f);
            }
            _ => {}
        }
        // bigint stays bigint
        if matches!(left, PrimitiveType::Bigint) || matches!(right, PrimitiveType::Bigint) {
            return PrimitiveType::Bigint;
        }
        // compare int widths and return the wider one
        match (left, right) {
            (PrimitiveType::Int(left_int), PrimitiveType::Int(right_int)) => {
                // if either is signed, result should be signed
                let is_signed = left_int.is_signed() || right_int.is_signed();
                match (left_int.width(), right_int.width()) {
                    (Some(left_width), Some(right_width)) => {
                        let width = left_width.max(right_width);
                        PrimitiveType::Int(IntType::Arbitrary { width, is_signed })
                    }
                    // pointer sized ints: cannot determine width at compile time
                    _ => PrimitiveType::Number,
                }
            }
            (PrimitiveType::Int(i), _) | (_, PrimitiveType::Int(i)) => PrimitiveType::Int(*i),
            _ => PrimitiveType::Number,
        }
    }

    /// Infer the result type of a unary operation.
    pub(crate) fn infer_unary_operation(&self, operator: &UnaryOperator, right: &Type) -> Type {
        match operator {
            // constant folding for logical not
            UnaryOperator::Not => self.try_fold_not(right).unwrap_or(Type::TypeLiteral {
                value: TypeLiteral::Primitive(PrimitiveType::Boolean),
            }),
            // constant folding for numeric negation
            UnaryOperator::Negate => self.try_fold_negate(right),
            UnaryOperator::Typeof => Type::TypeLiteral {
                value: TypeLiteral::Primitive(PrimitiveType::String),
            },
            UnaryOperator::Void => Type::TypeLiteral {
                value: TypeLiteral::Undefined,
            },
            _ => right.clone(),
        }
    }

    /// Try to constant fold logical not on a boolean literal.
    fn try_fold_not(&self, right: &Type) -> Option<Type> {
        if let Type::TypeLiteral {
            value: TypeLiteral::ScalarLiteral(ScalarLiteral::Boolean(b)),
        } = right
        {
            return Some(Type::TypeLiteral {
                value: TypeLiteral::ScalarLiteral(ScalarLiteral::Boolean(!b)),
            });
        }
        None
    }

    /// Try to constant fold unary negation on a literal type.
    /// Returns the negated literal type, or the original type if folding is not possible.
    fn try_fold_negate(&self, right: &Type) -> Type {
        if let Type::TypeLiteral {
            value: TypeLiteral::ScalarLiteral(scalar),
        } = right
        {
            match scalar {
                ScalarLiteral::Integer(i) => {
                    if let Some(negated) = i.checked_neg() {
                        return Type::TypeLiteral {
                            value: TypeLiteral::ScalarLiteral(ScalarLiteral::Integer(negated)),
                        };
                    }
                }
                ScalarLiteral::Bigint(i) => {
                    if let Some(negated) = i.checked_neg() {
                        return Type::TypeLiteral {
                            value: TypeLiteral::ScalarLiteral(ScalarLiteral::Bigint(negated)),
                        };
                    }
                }
                ScalarLiteral::Float(f) => {
                    return Type::TypeLiteral {
                        value: TypeLiteral::ScalarLiteral(ScalarLiteral::Float(-f)),
                    };
                }
                _ => {}
            }
        }
        right.clone()
    }

    /// Materialize readonly modifiers for object-like type expressions.
    pub(crate) fn materialize_readonly_type(
        &self,
        source_id: LocalNodeIdAny,
        ty_id: LocalTypeId,
        types: &mut TypeTable,
        deep_readonly: bool,
    ) -> LocalTypeId {
        let mut rewriter = ReadonlyMaterializer::new(source_id);
        if deep_readonly {
            return rewriter.rewrite_type_id(types, ty_id);
        }

        rewriter.apply_shallow(types, ty_id)
    }

    /// Infer the result type of a value of operation.
    pub(crate) fn infer_value_of_operation(
        &self,
        mutability: Option<Mutability>,
        variance: Option<VarianceBound>,
        right_ty_id: LocalTypeId,
    ) -> Type {
        // wrap the owned value with explicit ownership
        Type::ValueOf {
            mutability,
            variance,
            right: right_ty_id,
        }
    }

    /// Infer the result type of a reference of operation.
    pub(crate) fn infer_reference_of_operation(
        &self,
        mutability: Option<Mutability>,
        variance: Option<VarianceBound>,
        right_ty_id: LocalTypeId,
    ) -> Type {
        // wrap the reference with explicit ownership
        Type::ReferenceOf {
            mutability,
            variance,
            right: right_ty_id,
        }
    }
}
