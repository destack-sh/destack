use destack_dir::{
    Argument, CastOperator, Declaration, Expression, GlobalSymbolId, LocalNodeId, LocalTypeId,
    Member, NodeTree, NodeType, PrimitiveType, Resolution, ScalarLiteral, StaticArgument,
    StaticExpression, Type, TypeLiteral, TypeTable,
};
use destack_source::ModuleId;

use crate::{Compiler, ElaborateResult};

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Resolve expected argument types for a call when possible.
    pub(super) fn expected_argument_types_for_call(
        &self,
        module_id: ModuleId,
        expression_id: LocalNodeId<Expression>,
        dynamic_arguments: &[LocalNodeId<Argument>],
        tree: &NodeTree,
        types: &mut TypeTable,
    ) -> ElaborateResult<Option<Vec<Option<LocalTypeId>>>> {
        // resolve the call resolution
        let Some(resolution_id) =
            types.get_resolution_for_node(expression_id.into_global_any(module_id))
        else {
            return Ok(None);
        };
        let resolution = types.get_resolution(resolution_id).clone();
        let candidate = match resolution {
            Resolution::Static { candidate, .. } => candidate,
            _ => return Ok(None), // #Incomplete: handle dynamic resolution
        };
        let Some(resolved_signature) = candidate.resolved_signature else {
            return Ok(None);
        };

        // #Incomplete: map named and spread arguments to parameters
        // map positional arguments to parameter types
        let mut expected_types = Vec::with_capacity(dynamic_arguments.len());
        for (index, argument_id) in dynamic_arguments.iter().enumerate() {
            let argument = tree.get(*argument_id);
            let expected_type_id = match argument {
                Argument::Positional { .. } => {
                    resolved_signature.dynamic_parameters.get(index).copied()
                }
                _ => None,
            };
            expected_types.push(expected_type_id);
        }

        Ok(Some(expected_types))
    }

    /// Find the declared return type for a return expression.
    pub(super) fn enclosing_return_type(
        &self,
        module_id: ModuleId,
        expression_id: LocalNodeId<Expression>,
        tree: &NodeTree,
        types: &TypeTable,
    ) -> Option<LocalTypeId> {
        // start from the parent node
        let mut current = tree.get_parent(expression_id.id);

        // walk up the tree looking for a function or method
        while let Some(node_id) = current {
            // check function declarations
            if node_id.ty == NodeType::Declaration {
                let declaration_id = node_id.into_typed::<Declaration>();
                let declaration = tree.get(declaration_id);

                // return the declared function return type
                if let Declaration::Function {
                    descriptor,
                    signature,
                    ..
                } = declaration
                    && signature.return_type.is_some()
                {
                    let symbol = descriptor.symbol.into_global(module_id);
                    return self.return_type_for_symbol(symbol, types);
                }
            }

            // check method declarations
            if node_id.ty == NodeType::Member {
                let member_id = node_id.into_typed::<Member>();
                let member = tree.get(member_id);

                // return the declared method return type
                if let Member::Method {
                    symbol, signature, ..
                } = member
                    && signature.return_type.is_some()
                {
                    let symbol = symbol.into_global(module_id);
                    return self.return_type_for_symbol(symbol, types);
                }
            }

            current = tree.get_parent(node_id.id);
        }

        None
    }

    /// Return a function return type for a symbol when available.
    pub(super) fn return_type_for_symbol(
        &self,
        symbol: GlobalSymbolId,
        types: &TypeTable,
    ) -> Option<LocalTypeId> {
        // read the function value type
        let value_type_id = types.get_value_type_id(symbol)?;

        // unwrap to a function return type
        self.return_type_from_type_id(value_type_id, types)
    }

    /// Return a function return type from a type id.
    pub(super) fn return_type_from_type_id(
        &self,
        type_id: LocalTypeId,
        types: &TypeTable,
    ) -> Option<LocalTypeId> {
        // unwrap value types when needed
        match types.get_type(type_id) {
            Type::Function { return_type, .. } => *return_type,
            Type::Value { value } => self.return_type_from_type_id(*value, types),
            _ => None,
        }
    }
}

/// Numeric classification for cast selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NumericKind {
    /// Integer with width and signedness.
    Int { width: u16, is_signed: bool },
    /// Float with width.
    Float { width: u16 },
}

/// Return the numeric kind for a type when possible.
fn numeric_kind_for_type(ty: &Type) -> Option<NumericKind> {
    // only primitive or scalar literal types are numeric
    let Type::TypeLiteral { value } = ty else {
        return None;
    };

    match value {
        TypeLiteral::Primitive(PrimitiveType::Number) => Some(NumericKind::Float { width: 64 }),
        TypeLiteral::Primitive(PrimitiveType::Int(int_type)) => {
            let width = int_type.width()?;
            Some(NumericKind::Int {
                width,
                is_signed: int_type.is_signed(),
            })
        }
        TypeLiteral::Primitive(PrimitiveType::Float(float_type)) => Some(NumericKind::Float {
            width: float_type.width(),
        }),
        TypeLiteral::ScalarLiteral(ScalarLiteral::Integer(_)) => Some(NumericKind::Int {
            width: 64,
            is_signed: true,
        }),
        TypeLiteral::ScalarLiteral(ScalarLiteral::Float(_)) => {
            Some(NumericKind::Float { width: 64 })
        }
        _ => None,
    }
}

/// Classify a numeric cast when both sides are numeric.
pub(super) fn numeric_cast_operator(source: &Type, target: &Type) -> Option<CastOperator> {
    // read numeric kinds from both sides
    let source_kind = numeric_kind_for_type(source)?;
    let target_kind = numeric_kind_for_type(target)?;

    match (source_kind, target_kind) {
        (
            NumericKind::Int {
                width: left_width,
                is_signed: left_signed,
            },
            NumericKind::Int {
                width: right_width,
                is_signed: right_signed,
            },
        ) => {
            if left_width == right_width && left_signed != right_signed {
                return Some(CastOperator::IntSignChange);
            }

            if right_width > left_width {
                return Some(CastOperator::IntWiden);
            }

            if right_width < left_width {
                return Some(CastOperator::IntNarrow);
            }

            Some(CastOperator::Identity)
        }
        (NumericKind::Float { width: left_width }, NumericKind::Float { width: right_width }) => {
            if right_width > left_width {
                return Some(CastOperator::FloatWiden);
            }

            if right_width < left_width {
                return Some(CastOperator::FloatNarrow);
            }

            Some(CastOperator::Identity)
        }
        (NumericKind::Int { .. }, NumericKind::Float { .. }) => Some(CastOperator::IntToFloat),
        (NumericKind::Float { .. }, NumericKind::Int { .. }) => Some(CastOperator::FloatToInt),
    }
}

/// Return the common numeric type id for a binary operation.
pub(super) fn common_numeric_type_id_for_binary(
    left_type_id: LocalTypeId,
    right_type_id: LocalTypeId,
    source_id: LocalNodeId<Expression>,
    types: &mut TypeTable,
) -> Option<LocalTypeId> {
    // read the left and right types
    let left_type = types.get_type(left_type_id);
    let right_type = types.get_type(right_type_id);

    // require numeric kinds for both sides
    let left_kind = numeric_kind_for_type(left_type)?;
    let right_kind = numeric_kind_for_type(right_type)?;

    // prefer the non literal side when paired with a literal
    let left_is_literal = is_scalar_literal_type(left_type);
    let right_is_literal = is_scalar_literal_type(right_type);

    if left_is_literal && !right_is_literal {
        return Some(right_type_id);
    }

    if right_is_literal && !left_is_literal {
        return Some(left_type_id);
    }

    // widen literal only expressions to number
    if left_is_literal && right_is_literal {
        let ty = Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::Number),
        };
        let type_id = types.insert_type_from(ty, source_id);
        return Some(type_id);
    }

    // select the wider numeric type between the operands
    let prefer_left = prefer_left_numeric_kind(left_kind, right_kind);
    let preferred_id = if prefer_left {
        left_type_id
    } else {
        right_type_id
    };

    Some(preferred_id)
}

/// Check whether a type is an integer type.
pub(super) fn is_integer_type(ty: &Type) -> bool {
    matches!(
        ty,
        Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::Int(_))
        } | Type::TypeLiteral {
            value: TypeLiteral::ScalarLiteral(ScalarLiteral::Integer(_))
        }
    )
}

/// Choose whether the left numeric kind should be preferred.
fn prefer_left_numeric_kind(left: NumericKind, right: NumericKind) -> bool {
    match (left, right) {
        (NumericKind::Float { width: left_width }, NumericKind::Float { width: right_width }) => {
            left_width >= right_width
        }
        (
            NumericKind::Int {
                width: left_width,
                is_signed: left_signed,
            },
            NumericKind::Int {
                width: right_width,
                is_signed: right_signed,
            },
        ) => {
            if left_width != right_width {
                return left_width > right_width;
            }

            if left_signed != right_signed {
                return left_signed;
            }

            true
        }
        (NumericKind::Float { .. }, NumericKind::Int { .. }) => true,
        (NumericKind::Int { .. }, NumericKind::Float { .. }) => false,
    }
}

/// Check whether a type is the `any` type.
pub(super) fn is_any_type(ty: &Type) -> bool {
    matches!(
        ty,
        Type::TypeLiteral {
            value: TypeLiteral::Any
        }
    )
}

/// Check whether a type is the `unknown` type.
pub(super) fn is_unknown_type(ty: &Type) -> bool {
    matches!(
        ty,
        Type::TypeLiteral {
            value: TypeLiteral::Unknown
        }
    )
}

/// Check whether a type is a string type.
pub(super) fn is_string_type(ty: &Type) -> bool {
    matches!(
        ty,
        Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::String)
        } | Type::TypeLiteral {
            value: TypeLiteral::ScalarLiteral(ScalarLiteral::String(_))
        }
    )
}

/// Check whether a type is a scalar literal type.
pub(super) fn is_scalar_literal_type(ty: &Type) -> bool {
    matches!(
        ty,
        Type::TypeLiteral {
            value: TypeLiteral::ScalarLiteral(_),
        }
    )
}

/// Check whether a type is a pointer type.
pub(super) fn is_pointer_type(ty: &Type) -> bool {
    matches!(ty, Type::PointerOf { .. })
}

/// Check whether a type is a union type.
pub(super) fn is_union_type(ty: &Type) -> bool {
    matches!(ty, Type::Union { .. })
}

/// Check whether a type is a nullable union type.
pub(super) fn is_nullable_union(ty: &Type, types: &TypeTable) -> bool {
    let Type::Union { elements } = ty else {
        return false;
    };

    elements.iter().any(|element_id| {
        matches!(
            types.get_type(*element_id),
            Type::TypeLiteral {
                value: TypeLiteral::Null | TypeLiteral::Undefined,
            }
        )
    })
}

/// Check whether a type is the `object` type literal.
pub(super) fn is_object_type(ty: &Type) -> bool {
    matches!(
        ty,
        Type::TypeLiteral {
            value: TypeLiteral::Object
        }
    )
}

/// Check whether two types are semantically equal for casting.
pub(super) fn are_types_semantically_equal(
    source: &Type,
    target: &Type,
    types: &TypeTable,
) -> bool {
    match (source, target) {
        // type literals: compare directly (any==any, unknown==unknown, object==object, etc.)
        (Type::TypeLiteral { value: v1 }, Type::TypeLiteral { value: v2 }) => v1 == v2,

        // this type
        (Type::This, Type::This) => true,

        // references: same symbol with equivalent static arguments
        (
            Type::Reference {
                symbol: s1,
                static_arguments: a1,
            },
            Type::Reference {
                symbol: s2,
                static_arguments: a2,
            },
        ) => s1 == s2 && are_static_arguments_equal(a1, a2, types),

        // arrays: same element type
        (Type::Array { element: e1 }, Type::Array { element: e2 }) => match (e1, e2) {
            (Some(e1), Some(e2)) => are_types_equal(*e1, *e2, types),
            (None, None) => true,
            _ => false,
        },

        // sized arrays: same element type and count expression
        (
            Type::ArraySized {
                element: e1,
                count: c1,
            },
            Type::ArraySized {
                element: e2,
                count: c2,
            },
        ) => c1 == c2 && are_types_equal(*e1, *e2, types),

        // tuples: same elements
        (Type::Tuple { elements: e1 }, Type::Tuple { elements: e2 }) => {
            e1.len() == e2.len()
                && e1.iter().zip(e2.iter()).all(|(a, b)| {
                    a.label == b.label
                        && a.is_optional == b.is_optional
                        && a.is_readonly == b.is_readonly
                        && a.is_rest == b.is_rest
                        && are_types_equal(a.ty, b.ty, types)
                })
        }

        // functions: same signature
        (
            Type::Function {
                asynchrony: a1,
                cardinality: c1,
                static_parameters: sp1,
                this_parameter: tp1,
                dynamic_parameters: dp1,
                return_type: rt1,
            },
            Type::Function {
                asynchrony: a2,
                cardinality: c2,
                static_parameters: sp2,
                this_parameter: tp2,
                dynamic_parameters: dp2,
                return_type: rt2,
            },
        ) => {
            a1 == a2
                && c1 == c2
                && are_type_lists_equal(sp1, sp2, types)
                && are_optional_types_equal(*tp1, *tp2, types)
                && are_type_lists_equal(dp1, dp2, types)
                && are_optional_types_equal(*rt1, *rt2, types)
        }

        // unions: same elements (order-sensitive for now)
        (Type::Union { elements: e1 }, Type::Union { elements: e2 }) => {
            are_type_lists_equal(e1, e2, types)
        }

        // intersections: same elements (order-sensitive for now)
        (Type::Intersection { elements: e1 }, Type::Intersection { elements: e2 }) => {
            are_type_lists_equal(e1, e2, types)
        }

        // pointers: same target type
        (
            Type::PointerOf {
                mutability: m1,
                right: r1,
            },
            Type::PointerOf {
                mutability: m2,
                right: r2,
            },
        ) => m1 == m2 && are_types_equal(*r1, *r2, types),

        // value types
        (Type::Value { value: v1 }, Type::Value { value: v2 }) => are_types_equal(*v1, *v2, types),

        // default: not equal
        _ => false,
    }
}

/// Check if two types are semantically equal.
fn are_types_equal(a: LocalTypeId, b: LocalTypeId, types: &TypeTable) -> bool {
    // fast path: same type
    if a == b {
        return true;
    }

    // compare the underlying types
    let t1 = types.get_type(a);
    let t2 = types.get_type(b);
    are_types_semantically_equal(t1, t2, types)
}

/// Check if two optional types are equal.
fn are_optional_types_equal(
    a: Option<LocalTypeId>,
    b: Option<LocalTypeId>,
    types: &TypeTable,
) -> bool {
    match (a, b) {
        (Some(a), Some(b)) => are_types_equal(a, b, types),
        (None, None) => true,
        _ => false,
    }
}

/// Check if two type lists are equal.
fn are_type_lists_equal(a: &[LocalTypeId], b: &[LocalTypeId], types: &TypeTable) -> bool {
    a.len() == b.len()
        && a.iter()
            .zip(b.iter())
            .all(|(a, b)| are_types_equal(*a, *b, types))
}

/// Check if two static argument lists are equal.
fn are_static_arguments_equal(
    a: &Option<Vec<StaticArgument>>,
    b: &Option<Vec<StaticArgument>>,
    types: &TypeTable,
) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(a), Some(b)) => {
            a.len() == b.len()
                && a.iter().zip(b.iter()).all(|(arg1, arg2)| {
                    match (arg1, arg2) {
                        // compare evaluated static arguments
                        (
                            StaticArgument::Evaluated { value: v1, .. },
                            StaticArgument::Evaluated { value: v2, .. },
                        ) => are_static_expressions_equal(v1, v2, types),
                        // unevaluated arguments must be the same node
                        (
                            StaticArgument::Unevaluated { node: n1 },
                            StaticArgument::Unevaluated { node: n2 },
                        ) => n1 == n2,
                        _ => false,
                    }
                })
        }
        _ => false,
    }
}

/// Check if two static expressions are equal.
fn are_static_expressions_equal(
    a: &StaticExpression,
    b: &StaticExpression,
    types: &TypeTable,
) -> bool {
    match (a, b) {
        (StaticExpression::Type { ty: t1 }, StaticExpression::Type { ty: t2 }) => {
            are_types_equal(*t1, *t2, types)
        }
        (
            StaticExpression::ScalarLiteral { value: v1 },
            StaticExpression::ScalarLiteral { value: v2 },
        ) => v1 == v2,
        (
            StaticExpression::TypeLiteral { value: v1 },
            StaticExpression::TypeLiteral { value: v2 },
        ) => v1 == v2,
        // unevaluated and declaration: compare by identity
        _ => a == b,
    }
}
