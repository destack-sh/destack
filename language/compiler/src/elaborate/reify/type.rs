#![allow(dead_code)]

use destack_dir as dir;
use dir::{
    Argument, CastOperator, Declaration, DispatchResolution, Expression, GlobalSymbolId,
    LocalNodeId, LocalTypeId, Member, NodeType, PrimitiveType, Resolution, ScalarLiteral, Type,
    TypeLiteral, TypeTable,
};

use crate::elaborate::ElaborateState;
use crate::{Compiler, ElaborateResult};

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Resolve expected argument types for a call when possible.
    pub(super) fn expected_argument_types_for_call(
        &self,
        state: &mut ElaborateState<'_>,
        expression_id: LocalNodeId<Expression>,
        arguments: &[LocalNodeId<Argument>],
    ) -> ElaborateResult<Option<Vec<Option<LocalTypeId>>>> {
        // resolve the call resolution
        let Some(resolution) = state
            .types
            .resolution(expression_id.into_global_any(state.module_id))
            .cloned()
        else {
            return Ok(None);
        };
        let candidates = match resolution {
            Resolution::Dispatch(DispatchResolution::Static { target, .. }) => vec![target],
            Resolution::Dispatch(DispatchResolution::Dynamic { targets, .. }) => targets,
            _ => return Ok(None),
        };

        // collect resolved signatures for all candidates
        let mut signatures = Vec::new();
        for candidate in candidates {
            let Some(resolved_signature) = candidate.signature else {
                return Ok(None);
            };
            signatures.push(resolved_signature);
        }

        // map positional arguments to parameter types when uniform across candidates
        let mut expected_types = Vec::with_capacity(arguments.len());
        for (index, argument_id) in arguments.iter().enumerate() {
            let argument = state.tree.get(*argument_id);
            let expected_type_id = match argument {
                Argument::Positional { .. } => {
                    let mut expected = None;
                    let mut is_uniform = true;
                    for signature in &signatures {
                        let Some(param_ty_id) = signature.parameters.get(index).copied() else {
                            is_uniform = false;
                            break;
                        };
                        if let Some(current) = expected {
                            if current != param_ty_id {
                                is_uniform = false;
                                break;
                            }
                        } else {
                            expected = Some(param_ty_id);
                        }
                    }
                    if is_uniform { expected } else { None }
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
        state: &ElaborateState<'_>,
        expression_id: LocalNodeId<Expression>,
    ) -> Option<LocalTypeId> {
        // start from the parent node
        let mut current = state.tree.get_parent(expression_id.id);

        // walk up the tree looking for a function or method
        while let Some(node_id) = current {
            // check function declarations
            if node_id.ty == NodeType::Declaration {
                let declaration_id = node_id.into_typed::<Declaration>();
                let declaration = state.tree.get(declaration_id);

                // return the declared function return type
                if let Declaration::Function(declaration) = declaration
                    && declaration.signature.return_type.is_some()
                {
                    let symbol = declaration.symbol.into_global(state.module_id);
                    return self.return_type_for_symbol(state, symbol);
                }
            }

            // check method declarations
            if node_id.ty == NodeType::Member {
                let member_id = node_id.into_typed::<Member>();
                let member = state.tree.get(member_id);

                // return the declared method return type
                if let Member::Method {
                    symbol, signature, ..
                } = member
                    && signature.return_type.is_some()
                {
                    let symbol = symbol.into_global(state.module_id);
                    return self.return_type_for_symbol(state, symbol);
                }
            }

            current = state.tree.get_parent(node_id.id);
        }

        None
    }

    /// Return a function return type for a symbol when available.
    pub(super) fn return_type_for_symbol(
        &self,
        state: &ElaborateState<'_>,
        symbol: GlobalSymbolId,
    ) -> Option<LocalTypeId> {
        // read the function value type
        let value_type_id = state.types.get_value_type_id(symbol)?;

        // unwrap to a function return type
        self.return_type_from_type_id(state, value_type_id)
    }

    /// Return a function return type from a type id.
    pub(super) fn return_type_from_type_id(
        &self,
        state: &ElaborateState<'_>,
        type_id: LocalTypeId,
    ) -> Option<LocalTypeId> {
        // unwrap value types when needed
        match state.types.get_type(type_id) {
            Type::Function(function) => function.return_type,
            Type::Value(value) => self.return_type_from_type_id(state, value.value),
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
    let Type::Literal(dir::LiteralType { value }) = ty else {
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
        let ty = Type::Literal(dir::LiteralType {
            value: TypeLiteral::Primitive(PrimitiveType::Number),
        });
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
        Type::Literal(dir::LiteralType {
            value: TypeLiteral::Primitive(PrimitiveType::Int(_))
        }) | Type::Literal(dir::LiteralType {
            value: TypeLiteral::ScalarLiteral(ScalarLiteral::Integer(_))
        })
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
        Type::Literal(dir::LiteralType {
            value: TypeLiteral::Any
        })
    )
}

/// Check whether a type is the `unknown` type.
pub(super) fn is_unknown_type(ty: &Type) -> bool {
    matches!(
        ty,
        Type::Literal(dir::LiteralType {
            value: TypeLiteral::Unknown
        })
    )
}

/// Check whether a type is a string type.
pub(super) fn is_string_type(ty: &Type) -> bool {
    matches!(
        ty,
        Type::Literal(dir::LiteralType {
            value: TypeLiteral::Primitive(PrimitiveType::String)
        }) | Type::Literal(dir::LiteralType {
            value: TypeLiteral::ScalarLiteral(ScalarLiteral::String(_))
        })
    )
}

/// Check whether a type is a scalar literal type.
pub(super) fn is_scalar_literal_type(ty: &Type) -> bool {
    matches!(
        ty,
        Type::Literal(dir::LiteralType {
            value: TypeLiteral::ScalarLiteral(_),
        })
    )
}

/// Return whether a scalar literal and primitive share one runtime family.
pub(super) fn has_matching_scalar_runtime_family(source: &Type, target: &Type) -> bool {
    matches!(
        (source, target),
        (
            Type::Literal(dir::LiteralType {
                value: TypeLiteral::ScalarLiteral(ScalarLiteral::Boolean(_)),
            }),
            Type::Literal(dir::LiteralType {
                value: TypeLiteral::Primitive(PrimitiveType::Boolean),
            }),
        ) | (
            Type::Literal(dir::LiteralType {
                value: TypeLiteral::ScalarLiteral(ScalarLiteral::Character(_)),
            }),
            Type::Literal(dir::LiteralType {
                value: TypeLiteral::Primitive(PrimitiveType::Character),
            }),
        ) | (
            Type::Literal(dir::LiteralType {
                value: TypeLiteral::ScalarLiteral(ScalarLiteral::String(_)),
            }),
            Type::Literal(dir::LiteralType {
                value: TypeLiteral::Primitive(PrimitiveType::String),
            }),
        ) | (
            Type::Literal(dir::LiteralType {
                value: TypeLiteral::ScalarLiteral(ScalarLiteral::RegexString { .. }),
            }),
            Type::Literal(dir::LiteralType {
                value: TypeLiteral::Primitive(PrimitiveType::String),
            }),
        ) | (
            Type::Literal(dir::LiteralType {
                value: TypeLiteral::ScalarLiteral(ScalarLiteral::Bigint(_)),
            }),
            Type::Literal(dir::LiteralType {
                value: TypeLiteral::Primitive(PrimitiveType::Bigint),
            }),
        )
    )
}

/// Return whether one implicit value cast preserves the runtime family.
pub(super) fn has_matching_implicit_value_runtime_family(source: &Type, target: &Type) -> bool {
    if has_matching_scalar_runtime_family(source, target) {
        return true;
    }

    matches!(
        (source, target),
        (
            Type::Literal(dir::LiteralType {
                value: TypeLiteral::Primitive(PrimitiveType::Int(_) | PrimitiveType::Float(_)),
            }),
            Type::Literal(dir::LiteralType {
                value: TypeLiteral::Primitive(PrimitiveType::Number),
            }),
        ) | (
            Type::Literal(dir::LiteralType {
                value: TypeLiteral::ScalarLiteral(
                    ScalarLiteral::Integer(_) | ScalarLiteral::Float(_),
                ),
            }),
            Type::Literal(dir::LiteralType {
                value: TypeLiteral::Primitive(PrimitiveType::Number),
            }),
        )
    )
}

/// Check whether a type is a pointer type.
pub(super) fn is_pointer_type(ty: &Type) -> bool {
    let _ = ty;
    false
}

/// Check whether a type is a union type.
pub(super) fn is_union_type(ty: &Type) -> bool {
    matches!(ty, Type::Union(_))
}

/// Check whether a type is a nullable union type.
pub(super) fn is_nullable_union(ty: &Type, types: &TypeTable) -> bool {
    let Type::Union(union) = ty else {
        return false;
    };

    union.elements.iter().any(|element_id| {
        matches!(
            types.get_type(*element_id),
            Type::Literal(dir::LiteralType {
                value: TypeLiteral::Null | TypeLiteral::Undefined,
            })
        )
    })
}

/// Check whether a type is the `object` type literal.
pub(super) fn is_object_type(ty: &Type) -> bool {
    matches!(
        ty,
        Type::Literal(dir::LiteralType {
            value: TypeLiteral::Object
        })
    )
}
