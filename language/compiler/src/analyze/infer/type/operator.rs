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

            // in/instanceof always return boolean (no constant folding)
            BinaryOperator::In | BinaryOperator::InstanceOf => Type::TypeLiteral {
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

    /// Infer the result type of a type unary operation.
    pub(crate) fn infer_type_unary_operation(
        &self,
        ctx: &mut TypeContext<'_>,
        expression_id: LocalNodeId<Expression>,
        operator: &TypeUnaryOperator,
        right_ty_id: LocalTypeId,
    ) -> Type {
        match operator {
            TypeUnaryOperator::Not => {
                // invert boolean literals and otherwise return boolean
                let right_ty_id = self.unwrap_type_value(right_ty_id, ctx.types);
                let right_ty = ctx.types.get_type(right_ty_id).clone();
                self.try_fold_not(&right_ty).unwrap_or(Type::TypeLiteral {
                    value: TypeLiteral::Primitive(PrimitiveType::Boolean),
                })
            }
            TypeUnaryOperator::Must => {
                // strip nullish types for must
                let right_ty_id = self.unwrap_type_value(right_ty_id, ctx.types);
                let (non_nullish, _) = self.strip_nullish_from_union(right_ty_id, ctx.types);
                let Some(non_nullish) = non_nullish else {
                    return Type::TypeLiteral {
                        value: TypeLiteral::Never,
                    };
                };
                ctx.types.get_type(non_nullish).clone()
            }
            TypeUnaryOperator::Type => {
                // normalize to a type descriptor for `type`
                let right_ty = ctx.types.get_type(right_ty_id).clone();
                if let Type::Value { value } = right_ty {
                    return Type::Value { value };
                }
                Type::Value { value: right_ty_id }
            }
            TypeUnaryOperator::Readonly => {
                // normalize readonly modifiers
                let right_ty_id = self.unwrap_type_value(right_ty_id, ctx.types);
                let deep_readonly = ctx.options.deep_readonly;
                let readonly_id = self.materialize_readonly_type(
                    expression_id.into_any(),
                    right_ty_id,
                    ctx.types,
                    deep_readonly,
                );
                ctx.types.get_type(readonly_id).clone()
            }
            TypeUnaryOperator::AsConst => {
                // normalize const modifiers with deep readonly
                let right_ty_id = self.unwrap_type_value(right_ty_id, ctx.types);
                let readonly_id = self.materialize_readonly_type(
                    expression_id.into_any(),
                    right_ty_id,
                    ctx.types,
                    true,
                );
                ctx.types.get_type(readonly_id).clone()
            }
            TypeUnaryOperator::AsComptime => {
                // as comptime only changes type-index interpretation
                let right_ty_id = self.unwrap_type_value(right_ty_id, ctx.types);
                ctx.types.get_type(right_ty_id).clone()
            }
            TypeUnaryOperator::Keyof => {
                // resolve keys for keyof expressions
                let right_ty_id = self.unwrap_type_value(right_ty_id, ctx.types);
                let mut visited = Vec::new();
                let key_type_id = self.normalize_keyof_type(
                    &mut ctx.reborrow(),
                    expression_id.into_any(),
                    None,
                    right_ty_id,
                    NormalizationMode::Assign,
                    RelationMode::TYPE_OPERATOR,
                    &mut visited,
                );
                ctx.types.get_type(key_type_id).clone()
            }
            TypeUnaryOperator::Typeof => {
                // typeof expressions evaluate to strings in value contexts
                Type::TypeLiteral {
                    value: TypeLiteral::Primitive(PrimitiveType::String),
                }
            }
            TypeUnaryOperator::Newtype => {
                // newtype is a no-op in expression contexts
                let right_ty_id = self.unwrap_type_value(right_ty_id, ctx.types);
                ctx.types.get_type(right_ty_id).clone()
            }
        }
    }

    /// Infer the result type of a type binary operation.
    pub(crate) fn infer_type_binary_operation(
        &self,
        ctx: &mut TypeContext<'_>,
        expression_id: LocalNodeId<Expression>,
        operator: &TypeBinaryOperator,
        left_ty_id: LocalTypeId,
        right_ty_id: LocalTypeId,
    ) -> Type {
        match operator {
            TypeBinaryOperator::Cast => {
                // type assertion: `x as T`
                // allow explicit raw pointer casts
                let allow_pointer_cast = {
                    let left_ty = ctx.types.get_type(left_ty_id);
                    let right_ty = ctx.types.get_type(right_ty_id);

                    matches!(
                        (left_ty, right_ty),
                        (Type::PointerOf { .. }, Type::PointerOf { .. })
                    ) || matches!(
                        (left_ty, right_ty),
                        (
                            Type::TypeLiteral {
                                value: TypeLiteral::Null | TypeLiteral::Undefined,
                            },
                            Type::PointerOf { .. }
                        )
                    )
                };

                // allow explicit enum backing casts
                let allow_enum_cast = {
                    let left_ty = ctx.types.get_type(left_ty_id).clone();
                    let right_ty = ctx.types.get_type(right_ty_id).clone();

                    self.is_enum_backing_cast(&mut ctx.reborrow(), &left_ty, &right_ty)
                };
                let allow_record_cast =
                    self.allow_record_like_cast(ctx.profile, left_ty_id, right_ty_id, ctx.types);

                // check if cast is valid (types overlap: at least one direction is assignable)
                let left_to_right =
                    self.is_type_assignable(&mut ctx.reborrow(), right_ty_id, left_ty_id);
                let right_to_left =
                    self.is_type_assignable(&mut ctx.reborrow(), left_ty_id, right_ty_id);

                // reject unsafe type assertions when configured
                if ctx.options.no_unsafe_type_assertions
                    && matches!(ctx.module.source, ModuleSource::User)
                {
                    // read the source type
                    let left_ty = ctx.types.get_type(left_ty_id);

                    // check for any or unknown assertions
                    let is_any_or_unknown = matches!(
                        left_ty,
                        Type::TypeLiteral {
                            value: TypeLiteral::Any | TypeLiteral::Unknown,
                        }
                    );
                    let is_unsafe_cast = is_any_or_unknown
                        || (left_to_right == Assignability::NotAssignable
                            && !allow_pointer_cast
                            && !allow_enum_cast
                            && !allow_record_cast);

                    if is_unsafe_cast {
                        self.error(AnalyzeError::UnsafeTypeAssertionDisabled {
                            node: expression_id
                                .into_global_any(ctx.module.id)
                                .into_anchored(Some(ctx.profile)),
                        });
                    }
                }

                // report invalid casts when types do not overlap
                if !allow_pointer_cast
                    && !allow_enum_cast
                    && !allow_record_cast
                    && left_to_right == Assignability::NotAssignable
                    && right_to_left == Assignability::NotAssignable
                {
                    // neither direction works: illegal cast
                    self.error(AnalyzeError::InvalidCast {
                        node: expression_id
                            .into_global_any(ctx.module.id)
                            .into_anchored(Some(ctx.profile)),
                        from_ty: left_ty_id.into_global(ctx.module.id),
                        to_ty: right_ty_id.into_global(ctx.module.id),
                    });
                }
                // cast returns the target (right) type
                ctx.types.get_type(right_ty_id).clone()
            }
            TypeBinaryOperator::Satisfies => {
                // satisfies returns the original (left) type, not the asserted type
                ctx.types.get_type(left_ty_id).clone()
            }
            TypeBinaryOperator::Is | TypeBinaryOperator::InstanceOf => {
                // unwrap type descriptor values to the underlying type
                let target_ty_id = match ctx.types.get_type(right_ty_id) {
                    Type::Value { value } => *value,
                    _ => right_ty_id,
                };

                // enforce class-only instanceof targets
                if matches!(operator, TypeBinaryOperator::InstanceOf) {
                    let is_class_target = ctx
                        .types
                        .get_type(target_ty_id)
                        .symbol()
                        .is_some_and(|symbol| symbol.local_id.ty == SymbolType::Class);
                    if !is_class_target {
                        self.error(AnalyzeError::InvalidInstanceOfTarget {
                            node: expression_id
                                .into_global_any(ctx.module.id)
                                .into_anchored(Some(ctx.profile)),
                        });
                    }
                }

                // record runtime check kind for guard expressions
                let value_type_id = self.unwrap_type_value(left_ty_id, ctx.types);
                let runtime_check_kind = self.runtime_check_kind_for_relation(
                    &mut ctx.reborrow(),
                    value_type_id,
                    target_ty_id,
                );
                if let Some(kind) = runtime_check_kind {
                    ctx.types
                        .set_runtime_check_kind(expression_id.into_global_any(ctx.module.id), kind);
                }

                Type::TypeLiteral {
                    value: TypeLiteral::Primitive(PrimitiveType::Boolean),
                }
            }
            TypeBinaryOperator::In => {
                // check if the left type is a member of the right type keys
                let left_ty_id = self.unwrap_type_value(left_ty_id, ctx.types);
                let right_ty_id = self.unwrap_type_value(right_ty_id, ctx.types);

                // compute the key space for the right type
                let mut visited = Vec::new();
                let key_type_id = self.normalize_keyof_type(
                    &mut ctx.reborrow(),
                    expression_id.into_any(),
                    None,
                    right_ty_id,
                    NormalizationMode::Assign,
                    RelationMode::TYPE_OPERATOR,
                    &mut visited,
                );

                // compare the left type against the key space
                let assignability =
                    self.is_type_assignable(&mut ctx.reborrow(), key_type_id, left_ty_id);

                // only emit boolean literals when the relation is static
                let is_decidable =
                    self.type_operator_is_decidable(&mut ctx.reborrow(), left_ty_id, right_ty_id);
                self.boolean_type_for_assignability(assignability, is_decidable)
            }
            TypeBinaryOperator::Extends | TypeBinaryOperator::Implements => {
                // check assignability for extends/implements
                let left_ty_id = self.unwrap_type_value(left_ty_id, ctx.types);
                let right_ty_id = self.unwrap_type_value(right_ty_id, ctx.types);

                // compare the left type against the right type
                let assignability =
                    self.is_type_assignable(&mut ctx.reborrow(), right_ty_id, left_ty_id);

                // only emit boolean literals when the relation is static
                let is_decidable =
                    self.type_operator_is_decidable(&mut ctx.reborrow(), left_ty_id, right_ty_id);
                self.boolean_type_for_assignability(assignability, is_decidable)
            }
        }
    }

    /// Decide whether a type relation can be reduced to a boolean literal.
    fn type_operator_is_decidable(
        &self,
        ctx: &mut TypeContext<'_>,
        left_ty_id: LocalTypeId,
        right_ty_id: LocalTypeId,
    ) -> bool {
        // static parameters make assignability depend on runtime values
        let mut static_visited = HashSet::new();
        let left_contains_static =
            self.type_contains_static_parameters(ctx.type_view(), left_ty_id, &mut static_visited);
        let mut static_visited = HashSet::new();
        let right_contains_static =
            self.type_contains_static_parameters(ctx.type_view(), right_ty_id, &mut static_visited);
        !(left_contains_static || right_contains_static)
    }

    /// Build a boolean type literal from assignability results.
    fn boolean_type_for_assignability(
        &self,
        assignability: Assignability,
        is_decidable: bool,
    ) -> Type {
        if !is_decidable {
            return Type::TypeLiteral {
                value: TypeLiteral::Primitive(PrimitiveType::Boolean),
            };
        }

        let value = matches!(assignability, Assignability::Assignable);
        Type::TypeLiteral {
            value: TypeLiteral::ScalarLiteral(ScalarLiteral::Boolean(value)),
        }
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
