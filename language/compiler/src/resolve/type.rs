use crate::{Compiler, ResolveResult};
use dyst_ast::StringId;
use dyst_dir::{
    Expression, FloatType, IntType, NodeId, PrimitiveType, Type, TypeLiteral, TypeUnaryOperator,
    UnaryOperator,
};

impl<'a> Compiler<'a> {
    /// Resolve a Type (in-place).
    pub fn resolve_type(&mut self, ty_id: NodeId<Type>) -> ResolveResult<()> {
        let ty = self.session.tree.get(ty_id);
        let Type::UnresolvedExpression(expression_id) = *ty else {
            return Ok(());
        };

        // resolve and update in-place
        let resolved_ty = self.try_resolve_expression_to_type_value(expression_id)?;
        let mut ty = self.session.tree.get_mut(ty_id);
        *ty = resolved_ty;

        Ok(())
    }

    /// Try to Resolve an Expression as a Type id.
    fn try_resolve_expression_to_type(
        &mut self,
        expression_id: NodeId<Expression>,
    ) -> ResolveResult<NodeId<Type>> {
        let ty = self.try_resolve_expression_to_type_value(expression_id)?;
        Ok(self.session.tree.insert_from_dir(ty, expression_id))
    }

    /// Try to Resolve an Expression as a Type.
    /// Returns the resolved Type value, or a Type::UnresolvedExpression if it fails.
    fn try_resolve_expression_to_type_value(
        &mut self,
        expression_id: NodeId<Expression>,
    ) -> ResolveResult<Type> {
        let ty = self
            .resolve_expression_to_type(expression_id)?
            .unwrap_or(Type::UnresolvedExpression(expression_id));
        Ok(ty)
    }

    /// Resolve an Expression into a Type (in-place).
    fn resolve_expression_to_type(
        &mut self,
        expression_id: NodeId<Expression>,
    ) -> ResolveResult<Option<Type>> {
        let expression = self.session.tree.get(expression_id);

        let ty = match expression.as_ref() {
            Expression::ScalarLiteral { value } => {
                Type::Scalar(TypeLiteral::ScalarLiteral(value.clone()))
            }
            Expression::TypeLiteral { value } => Type::Scalar(value.clone()),

            // not
            Expression::Unary {
                operator: UnaryOperator::Not,
                right,
            } => {
                let type_id = self.try_resolve_expression_to_type(*right)?;
                Type::Unary {
                    operator: TypeUnaryOperator::Not,
                    right: type_id,
                }
            }
            // maybe
            Expression::Maybe { left } => {
                let type_id = self.try_resolve_expression_to_type(*left)?;
                Type::Unary {
                    operator: TypeUnaryOperator::Maybe,
                    right: type_id,
                }
            }
            // must
            Expression::Must { left } => {
                let type_id = self.try_resolve_expression_to_type(*left)?;
                Type::Unary {
                    operator: TypeUnaryOperator::Must,
                    right: type_id,
                }
            }
            // value
            Expression::ValueOf {
                mutability,
                variance,
                right,
            } => {
                let mutability = *mutability;
                let variance = *variance;
                let type_id = self.try_resolve_expression_to_type(*right)?;
                Type::ValueOf {
                    mutability,
                    variance,
                    right: type_id,
                }
            }
            // reference
            Expression::ReferenceOf {
                mutability,
                variance,
                right,
            } => {
                let mutability = *mutability;
                let variance = *variance;
                let type_id = self.try_resolve_expression_to_type(*right)?;
                Type::ReferenceOf {
                    mutability,
                    variance,
                    right: type_id,
                }
            }
            // unary
            Expression::TypeUnary { operator, right } => {
                let right_id = self.try_resolve_expression_to_type(*right)?;
                Type::Unary {
                    operator: *operator,
                    right: right_id,
                }
            }
            // binary
            Expression::TypeBinary {
                left,
                operator,
                right,
            } => {
                let left_id = self.try_resolve_expression_to_type(*left)?;
                let right_id = self.try_resolve_expression_to_type(*right)?;
                Type::Binary {
                    left: left_id,
                    operator: *operator,
                    right: right_id,
                }
            }

            // range
            Expression::RangeLiteral {
                start,
                end,
                is_inclusive,
            } => {
                let start_id = self.try_resolve_expression_to_type(*start)?;
                let end_id = self.try_resolve_expression_to_type(*end)?;
                Type::Range {
                    start: start_id,
                    end: end_id,
                    is_inclusive: *is_inclusive,
                }
            }
            // tuple
            Expression::TupleLiteral { ty, .. } if ty.is_none() => {
                todo!()
            }
            // struct
            Expression::StructLiteral { ty, .. } if ty.is_none() => {
                todo!()
            }

            // array or slice
            Expression::Index { left, right } => {
                // array with static length
                if let Some(right) = right {
                    let left_id = self.try_resolve_expression_to_type(*left)?;
                    Type::ArraySized {
                        element: left_id,
                        count: *right,
                    }
                }
                // slice
                else {
                    let left_id = self.try_resolve_expression_to_type(*left)?;
                    Type::Array {
                        element: Some(left_id),
                    }
                }
            }

            _ => return Ok(None),
        };

        Ok(Some(ty))
    }

    /// Whether the token string encodes a type literal with an explicit width.
    fn is_type_with_width(&self, prefix: &'static str, target: &str) -> Option<u16> {
        if let Some(target) = target.strip_prefix(prefix) {
            target.parse::<u16>().ok()
        } else {
            None
        }
    }

    /// Resolve an expression string into a DIR type literal.
    pub fn resolve_string_to_type(&mut self, string_id: StringId) -> Option<TypeLiteral> {
        let string = self.session.strings.get(string_id);
        match string.as_ref() {
            // undefined
            "undefined" => Some(TypeLiteral::Undefined),
            // unknown
            "unknown" => Some(TypeLiteral::Unknown),
            // void
            "void" => Some(TypeLiteral::Void),
            // null
            "null" => Some(TypeLiteral::Null),
            // any
            "any" => Some(TypeLiteral::Any),
            // never
            "never" => Some(TypeLiteral::Never),
            // boolean
            "boolean" => Some(TypeLiteral::Primitive(PrimitiveType::Boolean)),
            // character
            "character" => Some(TypeLiteral::Primitive(PrimitiveType::Character)),
            // string
            "string" => Some(TypeLiteral::Primitive(PrimitiveType::String)),
            // bigint
            "bigint" => Some(TypeLiteral::Primitive(PrimitiveType::Bigint)),
            // number
            "number" => Some(TypeLiteral::Primitive(PrimitiveType::Number)),
            // Self
            "Self" => panic!("self type can't be resolved"),
            // int (followed by number or nothing)
            "int" => Some(TypeLiteral::Primitive(PrimitiveType::Int(
                IntType::Arbitrary {
                    width: self.options.resolve.default_int_width,
                    is_signed: true,
                }
                .simplify(),
            ))),
            "intp" => Some(TypeLiteral::Primitive(PrimitiveType::Int(IntType::IntP))),
            int_str if let Some(width) = self.is_type_with_width("int", int_str) => {
                Some(TypeLiteral::Primitive(PrimitiveType::Int(
                    IntType::Arbitrary {
                        width,
                        is_signed: true,
                    }
                    .simplify(),
                )))
            }
            // uint (followed by number or nothing)
            "uint" => Some(TypeLiteral::Primitive(PrimitiveType::Int(
                IntType::Arbitrary {
                    width: self.options.resolve.default_int_width,
                    is_signed: false,
                }
                .simplify(),
            ))),
            "uintp" => Some(TypeLiteral::Primitive(PrimitiveType::Int(IntType::UintP))),
            uint_str if let Some(width) = self.is_type_with_width("uint", uint_str) => {
                Some(TypeLiteral::Primitive(PrimitiveType::Int(
                    IntType::Arbitrary {
                        width,
                        is_signed: false,
                    }
                    .simplify(),
                )))
            }
            uint_str if let Some(width) = self.is_type_with_width("u", uint_str) => {
                Some(TypeLiteral::Primitive(PrimitiveType::Int(
                    IntType::Arbitrary {
                        width,
                        is_signed: false,
                    }
                    .simplify(),
                )))
            }
            // float (followed by number or nothing)
            "float" => Some(TypeLiteral::Primitive(PrimitiveType::Float(
                FloatType::Arbitrary {
                    width: self.options.resolve.default_float_width,
                }
                .simplify(),
            ))),
            float_str if let Some(width) = self.is_type_with_width("float", float_str) => {
                Some(TypeLiteral::Primitive(PrimitiveType::Float(
                    FloatType::Arbitrary { width }.simplify(),
                )))
            }
            // composite type
            _ => None,
        }
    }
}
