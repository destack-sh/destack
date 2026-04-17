use crate::{LocalTypeId, StaticArgument, StaticExpression, Type, TypeTable};

/// Check whether two types are semantically equal for casting.
pub fn are_types_semantically_equal(source: &Type, target: &Type, types: &TypeTable) -> bool {
    match (source, target) {
        // type literals: compare directly (any==any, unknown==unknown, object==object, etc.)
        (Type::TypeLiteral { value: v1 }, Type::TypeLiteral { value: v2 }) => v1 == v2,

        // this type
        (Type::This, Type::This) => true,

        // references: same symbol with equivalent static arguments
        (
            Type::Reference {
                symbol: s1,
                generic_arguments: a1,
            },
            Type::Reference {
                symbol: s2,
                generic_arguments: a2,
            },
        ) => s1 == s2 && are_static_arguments_equal(a1, a2, types),

        // arrays: same element type
        (
            Type::Array {
                element: e1,
                is_readonly: r1,
            },
            Type::Array {
                element: e2,
                is_readonly: r2,
            },
        ) => {
            if r1 != r2 {
                return false;
            }
            match (e1, e2) {
                (Some(e1), Some(e2)) => are_types_equal(*e1, *e2, types),
                (None, None) => true,
                _ => false,
            }
        }

        // sized arrays: same element type and count expression
        (
            Type::ArraySized {
                element: e1,
                count: c1,
                is_readonly: r1,
            },
            Type::ArraySized {
                element: e2,
                count: c2,
                is_readonly: r2,
            },
        ) => r1 == r2 && are_types_equal(*e1, *e2, types) && are_types_equal(*c1, *c2, types),

        // tuples: same elements
        (
            Type::Tuple {
                elements: e1,
                is_readonly: r1,
            },
            Type::Tuple {
                elements: e2,
                is_readonly: r2,
            },
        ) => {
            if r1 != r2 {
                return false;
            }
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
                generic_parameters: sp1,
                this_parameter: tp1,
                parameters: dp1,
                return_type: rt1,
            },
            Type::Function {
                asynchrony: a2,
                cardinality: c2,
                generic_parameters: sp2,
                this_parameter: tp2,
                parameters: dp2,
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

        // unions: same elements (order sensitive for now)
        (Type::Union { elements: e1 }, Type::Union { elements: e2 }) => {
            are_type_lists_equal(e1, e2, types)
        }

        // intersections: same elements (order sensitive for now)
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
pub fn are_types_equal(a: LocalTypeId, b: LocalTypeId, types: &TypeTable) -> bool {
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
