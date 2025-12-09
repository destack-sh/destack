use crate::{AnalyzeResult, Compiler};
use destack_dir::{
    BinaryOperator, Expression, GlobalSymbolId, IntType, LocalNodeId, LocalTypeId, Mutability,
    PrimitiveType, ScalarLiteral, Type, TypeField, TypeLiteral, TypeTable, UnaryOperator,
    VarianceBound,
};
use destack_workspace::Module;

impl Compiler {
    /// Infer the result type of a scalar literal.
    /// Returns the literal type (e.g., `4` has type `4`), allowing assignability to check
    /// whether the literal fits the target type (int32, number, etc.).
    pub(super) fn infer_scalar_literal(&self, value: &ScalarLiteral) -> TypeLiteral {
        // return the literal type, not the widened primitive type
        // this allows `let x: int = 4` to work via assignability checking
        TypeLiteral::ScalarLiteral(value.clone())
    }

    /// Infer the result type of a binary operation.
    pub(super) fn infer_binary_operation(
        &self,
        operator: &BinaryOperator,
        left: &Type,
        right: &Type,
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
                .try_fold_arithmetic(operator, left, right)
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
        if let (ScalarLiteral::Integer(l), ScalarLiteral::Integer(r)) = (left_lit, right_lit) {
            let result = match operator {
                BinaryOperator::Equal | BinaryOperator::EqualStrict => l == r,
                BinaryOperator::NotEqual | BinaryOperator::NotEqualStrict => l != r,
                BinaryOperator::LessThan => l < r,
                BinaryOperator::LessThanOrEqual => l <= r,
                BinaryOperator::GreaterThan => l > r,
                BinaryOperator::GreaterThanOrEqual => l >= r,
                _ => return None,
            };
            return Some(Type::TypeLiteral {
                value: TypeLiteral::ScalarLiteral(ScalarLiteral::Boolean(result)),
            });
        }

        // compare floats (or mixed int/float)
        let (l, r) = Self::to_f64_pair(left_lit, right_lit)?;
        let result = match operator {
            BinaryOperator::Equal | BinaryOperator::EqualStrict => l == r,
            BinaryOperator::NotEqual | BinaryOperator::NotEqualStrict => l != r,
            BinaryOperator::LessThan => l < r,
            BinaryOperator::LessThanOrEqual => l <= r,
            BinaryOperator::GreaterThan => l > r,
            BinaryOperator::GreaterThanOrEqual => l >= r,
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

        if let (ScalarLiteral::Boolean(l), ScalarLiteral::Boolean(r)) = (left_lit, right_lit) {
            let result = match operator {
                BinaryOperator::And => *l && *r,
                BinaryOperator::Or => *l || *r,
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
        if let (ScalarLiteral::Integer(l), ScalarLiteral::Integer(r)) = (left_lit, right_lit) {
            let result = match operator {
                BinaryOperator::Add => l.checked_add(*r),
                BinaryOperator::Subtract => l.checked_sub(*r),
                BinaryOperator::Multiply => l.checked_mul(*r),
                BinaryOperator::Divide => {
                    if *r != 0 {
                        l.checked_div(*r)
                    } else {
                        None
                    }
                }
                BinaryOperator::Remainder => {
                    if *r != 0 {
                        l.checked_rem(*r)
                    } else {
                        None
                    }
                }
                BinaryOperator::Exponent => {
                    if *r >= 0 && *r <= u32::MAX as i64 {
                        l.checked_pow(*r as u32)
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
            let (l, r) = Self::to_f64_pair(left_lit, right_lit)?;
            let result = match operator {
                BinaryOperator::Add => l + r,
                BinaryOperator::Subtract => l - r,
                BinaryOperator::Multiply => l * r,
                BinaryOperator::Divide => l / r,
                BinaryOperator::Remainder => l % r,
                BinaryOperator::Exponent => l.powf(r),
                _ => return None,
            };
            return Some(Type::TypeLiteral {
                value: TypeLiteral::ScalarLiteral(ScalarLiteral::Float(result)),
            });
        }

        None
    }

    /// Extract scalar literals from two types.
    fn extract_scalar_literals<'a>(
        left: &'a Type,
        right: &'a Type,
    ) -> Option<(&'a ScalarLiteral, &'a ScalarLiteral)> {
        match (left, right) {
            (
                Type::TypeLiteral {
                    value: TypeLiteral::ScalarLiteral(l),
                },
                Type::TypeLiteral {
                    value: TypeLiteral::ScalarLiteral(r),
                },
            ) => Some((l, r)),
            _ => None,
        }
    }

    /// Convert two scalar literals to f64 values (for numeric operations).
    fn to_f64_pair(left: &ScalarLiteral, right: &ScalarLiteral) -> Option<(f64, f64)> {
        let l = match left {
            ScalarLiteral::Integer(i) => *i as f64,
            ScalarLiteral::Float(f) => *f,
            _ => return None,
        };
        let r = match right {
            ScalarLiteral::Integer(i) => *i as f64,
            ScalarLiteral::Float(f) => *f,
            _ => return None,
        };
        Some((l, r))
    }

    /// Widen two numeric types to a common type.
    /// Used when constant folding fails (e.g., `x + 1` where x is a variable).
    fn widen_numeric_types(&self, left: &Type, right: &Type) -> Type {
        let left_prim = Self::to_numeric_primitive(left);
        let right_prim = Self::to_numeric_primitive(right);
        match (left_prim, right_prim) {
            // if both are known primitives, return the wider one
            (Some(l), Some(r)) => Type::TypeLiteral {
                value: TypeLiteral::Primitive(Self::wider_numeric_primitive(&l, &r)),
            },
            // if one side is a primitive, use it
            (Some(p), None) | (None, Some(p)) => Type::TypeLiteral {
                value: TypeLiteral::Primitive(p),
            },
            // fallback to number
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
            } => Some(PrimitiveType::Number),
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
            (PrimitiveType::Float(l), PrimitiveType::Float(r)) => {
                let wider = if l.width() >= r.width() { *l } else { *r };
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
            (PrimitiveType::Int(l), PrimitiveType::Int(r)) => {
                // if either is signed, result should be signed
                let is_signed = l.is_signed() || r.is_signed();
                match (l.width(), r.width()) {
                    (Some(lw), Some(rw)) => {
                        let width = lw.max(rw);
                        PrimitiveType::Int(IntType::Arbitrary { width, is_signed })
                    }
                    // pointer-sized ints: can't determine width at compile time
                    _ => PrimitiveType::Number,
                }
            }
            (PrimitiveType::Int(i), _) | (_, PrimitiveType::Int(i)) => PrimitiveType::Int(*i),
            _ => PrimitiveType::Number,
        }
    }

    /// Infer the result type of a unary operation.
    pub(super) fn infer_unary_operation(&self, operator: &UnaryOperator, right: &Type) -> Type {
        match operator {
            // constant folding for logical not
            UnaryOperator::Not => self.try_fold_not(right).unwrap_or(Type::TypeLiteral {
                value: TypeLiteral::Primitive(PrimitiveType::Boolean),
            }),
            // constant folding for numeric negation
            UnaryOperator::Negate => self.try_fold_negate(right),
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

    /// Infer the result type of a value of operation.
    pub(super) fn infer_value_of_operation(
        &self,
        _mutability: Option<Mutability>,
        _variance: Option<VarianceBound>,
        right: &Type,
    ) -> Type {
        // NOTE #Incomplete: resolve value of operation type
        right.clone()
    }

    /// Infer the result type of a reference of operation.
    pub(super) fn infer_reference_of_operation(
        &self,
        _mutability: Option<Mutability>,
        _variance: Option<VarianceBound>,
        right: &Type,
    ) -> Type {
        // NOTE #Incomplete: resolve reference of operation type
        right.clone()
    }

    /// Resolve a remote symbol's value type by ensuring its module is analyzed
    /// and copying the type into the current module's TypeTable.
    pub(super) fn resolve_remote_symbol_value_type(
        &self,
        _module: &Module,
        expression_id: LocalNodeId<Expression>,
        target_symbol: GlobalSymbolId,
        types: &mut TypeTable,
    ) -> AnalyzeResult<LocalTypeId> {
        let remote_module_id = target_symbol.module_id;

        // ensure the remote module is analyzed (may yield)
        self.require_analyze(remote_module_id)?;

        // look up the type in the remote module's TypeTable
        let remote_module = self.program.modules.get(remote_module_id);
        let remote_module = remote_module.read();
        let remote_types = remote_module.dir.types.read();

        // copy the type into our local TypeTable
        if let Some(remote_ty_id) = remote_types.get_value_type_id(target_symbol) {
            let remote_ty = remote_types.get_type(remote_ty_id);
            let local_ty = self.import_type_from_remote(
                expression_id,
                remote_ty,
                &remote_types,
                target_symbol,
                types,
            );
            Ok(local_ty)
        }
        // remote symbol doesn't have a value type, return unknown
        else {
            let ty = Type::TypeLiteral {
                value: TypeLiteral::Unknown,
            };
            Ok(types.insert_type_from(ty, expression_id))
        }
    }

    /// Import a type from a remote module into the current module's TypeTable.
    ///  - For structural types (arrays, objects, ..): recursively copy the type structure.
    ///  - For nominal types (Type::Reference): keep them as references to the original symbol.
    pub(super) fn import_type_from_remote(
        &self,
        expression_id: LocalNodeId<Expression>,
        remote_ty: &Type,
        remote_types: &TypeTable,
        target_symbol: GlobalSymbolId,
        types: &mut TypeTable,
    ) -> LocalTypeId {
        match remote_ty {
            // leaf types: copy directly
            Type::TypeLiteral { value } => types.insert_type_from(
                Type::TypeLiteral {
                    value: value.clone(),
                },
                expression_id,
            ),
            Type::Error => types.insert_type_from(Type::Error, expression_id),

            // array types
            Type::Array { element: None } => {
                types.insert_type_from(Type::Array { element: None }, expression_id)
            }
            Type::Array {
                element: Some(elem_id),
            } => {
                let elem_ty = remote_types.get_type(*elem_id);
                let local_elem = self.import_type_from_remote(
                    expression_id,
                    elem_ty,
                    remote_types,
                    target_symbol,
                    types,
                );
                types.insert_type_from(
                    Type::Array {
                        element: Some(local_elem),
                    },
                    expression_id,
                )
            }

            // tuple types
            Type::Tuple { elements } => {
                let local_elems: Vec<_> = elements
                    .iter()
                    .map(|id| {
                        let ty = remote_types.get_type(*id);
                        self.import_type_from_remote(
                            expression_id,
                            ty,
                            remote_types,
                            target_symbol,
                            types,
                        )
                    })
                    .collect();
                types.insert_type_from(
                    Type::Tuple {
                        elements: local_elems,
                    },
                    expression_id,
                )
            }

            // object types
            Type::Object { fields } => {
                let local_fields: Vec<_> = fields
                    .iter()
                    .map(|field| {
                        let ty = remote_types.get_type(field.ty);
                        let local_ty = self.import_type_from_remote(
                            expression_id,
                            ty,
                            remote_types,
                            target_symbol,
                            types,
                        );
                        TypeField {
                            key: field.key,
                            ty: local_ty,
                            is_optional: field.is_optional,
                            is_readonly: field.is_readonly,
                        }
                    })
                    .collect();
                types.insert_type_from(
                    Type::Object {
                        fields: local_fields,
                    },
                    expression_id,
                )
            }

            // function types
            Type::Function {
                asynchrony,
                cardinality,
                static_parameters,
                dynamic_parameters,
                return_type,
            } => {
                let local_static_params: Vec<_> = static_parameters
                    .iter()
                    .map(|id| {
                        let ty = remote_types.get_type(*id);
                        self.import_type_from_remote(
                            expression_id,
                            ty,
                            remote_types,
                            target_symbol,
                            types,
                        )
                    })
                    .collect();
                let local_dynamic_params: Vec<_> = dynamic_parameters
                    .iter()
                    .map(|id| {
                        let ty = remote_types.get_type(*id);
                        self.import_type_from_remote(
                            expression_id,
                            ty,
                            remote_types,
                            target_symbol,
                            types,
                        )
                    })
                    .collect();
                let local_return = return_type.map(|id| {
                    let ty = remote_types.get_type(id);
                    self.import_type_from_remote(
                        expression_id,
                        ty,
                        remote_types,
                        target_symbol,
                        types,
                    )
                });
                types.insert_type_from(
                    Type::Function {
                        asynchrony: *asynchrony,
                        cardinality: *cardinality,
                        static_parameters: local_static_params,
                        dynamic_parameters: local_dynamic_params,
                        return_type: local_return,
                    },
                    expression_id,
                )
            }

            // union and intersection types
            Type::Union { elements } => {
                let local_elems: Vec<_> = elements
                    .iter()
                    .map(|id| {
                        let ty = remote_types.get_type(*id);
                        self.import_type_from_remote(
                            expression_id,
                            ty,
                            remote_types,
                            target_symbol,
                            types,
                        )
                    })
                    .collect();
                types.insert_type_from(
                    Type::Union {
                        elements: local_elems,
                    },
                    expression_id,
                )
            }
            Type::Intersection { elements } => {
                let local_elems: Vec<_> = elements
                    .iter()
                    .map(|id| {
                        let ty = remote_types.get_type(*id);
                        self.import_type_from_remote(
                            expression_id,
                            ty,
                            remote_types,
                            target_symbol,
                            types,
                        )
                    })
                    .collect();
                types.insert_type_from(
                    Type::Intersection {
                        elements: local_elems,
                    },
                    expression_id,
                )
            }

            // type modifiers: recurse into inner type
            Type::Value { value } => {
                let inner_ty = remote_types.get_type(*value);
                let local_inner = self.import_type_from_remote(
                    expression_id,
                    inner_ty,
                    remote_types,
                    target_symbol,
                    types,
                );
                types.insert_type_from(Type::Value { value: local_inner }, expression_id)
            }
            Type::Mutable { mutability, right } => {
                let inner_ty = remote_types.get_type(*right);
                let local_inner = self.import_type_from_remote(
                    expression_id,
                    inner_ty,
                    remote_types,
                    target_symbol,
                    types,
                );
                types.insert_type_from(
                    Type::Mutable {
                        mutability: *mutability,
                        right: local_inner,
                    },
                    expression_id,
                )
            }
            Type::ValueOf {
                mutability,
                variance,
                right,
            } => {
                let inner_ty = remote_types.get_type(*right);
                let local_inner = self.import_type_from_remote(
                    expression_id,
                    inner_ty,
                    remote_types,
                    target_symbol,
                    types,
                );
                types.insert_type_from(
                    Type::ValueOf {
                        mutability: *mutability,
                        variance: *variance,
                        right: local_inner,
                    },
                    expression_id,
                )
            }
            Type::ReferenceOf {
                mutability,
                variance,
                right,
            } => {
                let inner_ty = remote_types.get_type(*right);
                let local_inner = self.import_type_from_remote(
                    expression_id,
                    inner_ty,
                    remote_types,
                    target_symbol,
                    types,
                );
                types.insert_type_from(
                    Type::ReferenceOf {
                        mutability: *mutability,
                        variance: *variance,
                        right: local_inner,
                    },
                    expression_id,
                )
            }
            Type::Unary { operator, right } => {
                let inner_ty = remote_types.get_type(*right);
                let local_inner = self.import_type_from_remote(
                    expression_id,
                    inner_ty,
                    remote_types,
                    target_symbol,
                    types,
                );
                types.insert_type_from(
                    Type::Unary {
                        operator: *operator,
                        right: local_inner,
                    },
                    expression_id,
                )
            }
            Type::Binary {
                left,
                operator,
                right,
            } => {
                let left_ty = remote_types.get_type(*left);
                let right_ty = remote_types.get_type(*right);
                let local_left = self.import_type_from_remote(
                    expression_id,
                    left_ty,
                    remote_types,
                    target_symbol,
                    types,
                );
                let local_right = self.import_type_from_remote(
                    expression_id,
                    right_ty,
                    remote_types,
                    target_symbol,
                    types,
                );
                types.insert_type_from(
                    Type::Binary {
                        left: local_left,
                        operator: *operator,
                        right: local_right,
                    },
                    expression_id,
                )
            }

            // nominal/reference types: keep as Type::Reference to the original symbol
            Type::Reference {
                symbol,
                static_arguments,
            } => types.insert_type_from(
                Type::Reference {
                    symbol: *symbol,
                    static_arguments: static_arguments.clone(),
                },
                expression_id,
            ),

            // types that can't be meaningfully copied: fall back to reference
            Type::Unevaluated(_) | Type::ArraySized { .. } => types.insert_type_from(
                Type::Reference {
                    symbol: target_symbol,
                    static_arguments: None,
                },
                expression_id,
            ),
        }
    }
}
