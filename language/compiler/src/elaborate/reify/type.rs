use destack_dir::{
    Argument, CastOperator, Declaration, Expression, GlobalSymbolId, LocalNodeId, LocalTypeId,
    Member, NodeTree, NodeType, PrimitiveType, Resolution, ScalarLiteral, Type, TypeLiteral,
    TypeTable,
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
