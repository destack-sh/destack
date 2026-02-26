use super::*;
use crate::analyze::common::SymbolTypeView;

#[allow(clippy::too_many_arguments)]
impl Compiler {
    pub(crate) fn is_interface_implemented(
        &self,
        ctx: SymbolTypeView<'_>,
        ty: &Type,
        interface_item: LanguageSymbol,
    ) -> bool {
        let interface_symbol = self.language_symbol(ctx.profile, interface_item);
        match ty {
            Type::Value { value } => {
                let inner_ty = ctx.types.get_type(*value);
                self.is_interface_implemented(ctx, inner_ty, interface_item)
            }
            Type::Reference { symbol, .. } => {
                let canonical_symbol = self.canonical_symbol_id(
                    ctx.module_symbol_view(),
                    *symbol,
                    CanonicalSymbolMode::FollowAliases,
                );
                self.is_type_lineage_assignable(ctx, canonical_symbol, interface_symbol)
            }
            Type::Union { elements } => elements.iter().all(|element_id| {
                let element_ty = ctx.types.get_type(*element_id);
                self.is_interface_implemented(ctx, element_ty, interface_item)
            }),
            _ => false,
        }
    }

    /// Check whether a type is definitely a struct type.
    pub(crate) fn is_definitely_struct_type(&self, ty: &Type) -> bool {
        match ty {
            Type::Reference { symbol, .. } => symbol.ty() == SymbolType::Struct,
            _ => false,
        }
    }

    /// Check whether one operator operand type is still indeterminate.
    pub(crate) fn operator_operand_is_indeterminate(&self, ty: &Type, types: &TypeTable) -> bool {
        match ty {
            _ if ty.is_infer() => true,
            Type::TypeLiteral {
                value: TypeLiteral::Unknown,
            } => true,
            Type::Union { elements } => elements.iter().any(|element_id| {
                self.operator_operand_is_indeterminate(types.get_type(*element_id), types)
            }),
            _ => false,
        }
    }

    /// Check whether a type behaves like a numeric type.
    pub(crate) fn is_numeric_like_type(&self, ty: &Type, types: &TypeTable) -> bool {
        match ty {
            Type::TypeLiteral {
                value: TypeLiteral::Primitive(primitive),
            } => matches!(
                primitive,
                PrimitiveType::Number
                    | PrimitiveType::Int(_)
                    | PrimitiveType::Float(_)
                    | PrimitiveType::Bigint
            ),
            Type::TypeLiteral {
                value:
                    TypeLiteral::ScalarLiteral(
                        ScalarLiteral::Integer(_)
                        | ScalarLiteral::Float(_)
                        | ScalarLiteral::Bigint(_),
                    ),
            } => true,
            Type::Union { elements } => elements
                .iter()
                .all(|element_id| self.is_numeric_like_type(types.get_type(*element_id), types)),
            _ => false,
        }
    }

    /// Check whether a type is a primitive or scalar literal for builtin operators.
    pub(crate) fn is_primitive_literal_type(&self, ty: &Type, types: &TypeTable) -> bool {
        match ty {
            Type::TypeLiteral {
                value:
                    TypeLiteral::Primitive(_)
                    | TypeLiteral::ScalarLiteral(_)
                    | TypeLiteral::Null
                    | TypeLiteral::Undefined,
            } => true,
            Type::Union { elements } => elements.iter().all(|element_id| {
                self.is_primitive_literal_type(types.get_type(*element_id), types)
            }),
            _ => false,
        }
    }

    /// Check whether a type is a literal value.
    pub(crate) fn is_literal_value_type(&self, ty: &Type) -> bool {
        matches!(
            ty,
            Type::TypeLiteral {
                value: TypeLiteral::ScalarLiteral(_) | TypeLiteral::Null | TypeLiteral::Undefined,
            }
        )
    }

    /// Extract the return type from a function type.
    pub(crate) fn function_return_type(
        &self,
        fn_ty_id: LocalTypeId,
        types: &TypeTable,
    ) -> Option<LocalTypeId> {
        match types.get_type(fn_ty_id) {
            Type::Function { return_type, .. } => *return_type,
            _ => None,
        }
    }

    /// Decide whether a return type allows implicit fallthrough.
    pub(crate) fn return_type_allows_fallthrough_infer(
        &self,
        ty_id: LocalTypeId,
        types: &TypeTable,
    ) -> bool {
        match types.get_type(ty_id) {
            Type::TypeLiteral {
                value:
                    TypeLiteral::Void
                    | TypeLiteral::Undefined
                    | TypeLiteral::Any
                    | TypeLiteral::Unknown
                    | TypeLiteral::Infer,
            } => true,
            Type::Predicate { asserts: true, .. } => true,
            Type::Union { elements } => elements
                .iter()
                .any(|element| self.return_type_allows_fallthrough_infer(*element, types)),
            Type::InferVar { .. } => true,
            Type::Error => true,
            _ => false,
        }
    }

    /// Check whether one type is semantic top-like (`any` or `unknown`).
    pub(crate) fn type_is_semantic_top_like(
        &self,
        type_id: LocalTypeId,
        types: &TypeTable,
    ) -> bool {
        let ty = types.get_type(type_id);
        matches!(
            ty,
            Type::TypeLiteral {
                value: TypeLiteral::Any | TypeLiteral::Unknown,
            }
        )
    }

    pub(crate) fn is_nullish_type(&self, ty: &Type) -> bool {
        matches!(
            ty,
            Type::TypeLiteral {
                value: TypeLiteral::Null | TypeLiteral::Undefined,
            }
        )
    }
}
