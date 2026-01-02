use crate::Compiler;
use destack_dir::{
    PrimitiveType, ScalarLiteral, StaticArgument, StaticExpression, Type, TypeLiteral, TypeTable,
    WellKnownSymbol,
};
use destack_workspace::ProfileId;

impl Compiler {
    /// Return the well known symbol for a primitive or literal type.
    pub(super) fn well_known_symbol_for_type_literal(
        &self,
        value: &TypeLiteral,
    ) -> Option<WellKnownSymbol> {
        // map primitives and scalar literals to wrapper symbols
        match value {
            TypeLiteral::Primitive(PrimitiveType::Boolean) => Some(WellKnownSymbol::Boolean),
            TypeLiteral::Primitive(PrimitiveType::String)
            | TypeLiteral::Primitive(PrimitiveType::Character) => Some(WellKnownSymbol::String),
            TypeLiteral::Primitive(PrimitiveType::Bigint) => Some(WellKnownSymbol::BigInt),
            TypeLiteral::Primitive(PrimitiveType::Number)
            | TypeLiteral::Primitive(PrimitiveType::Int(_))
            | TypeLiteral::Primitive(PrimitiveType::Float(_)) => Some(WellKnownSymbol::Number),
            TypeLiteral::Primitive(PrimitiveType::Symbol)
            | TypeLiteral::Primitive(PrimitiveType::UniqueSymbol) => Some(WellKnownSymbol::Symbol),
            TypeLiteral::ScalarLiteral(ScalarLiteral::Boolean(_)) => Some(WellKnownSymbol::Boolean),
            TypeLiteral::ScalarLiteral(ScalarLiteral::String(_))
            | TypeLiteral::ScalarLiteral(ScalarLiteral::Character(_))
            | TypeLiteral::ScalarLiteral(ScalarLiteral::RegexString { .. }) => {
                Some(WellKnownSymbol::String)
            }
            TypeLiteral::ScalarLiteral(ScalarLiteral::Bigint(_)) => Some(WellKnownSymbol::BigInt),
            TypeLiteral::ScalarLiteral(ScalarLiteral::Integer(_))
            | TypeLiteral::ScalarLiteral(ScalarLiteral::Float(_)) => Some(WellKnownSymbol::Number),
            _ => None,
        }
    }

    /// Return the well known symbol for implicit member lookup on a type.
    pub(super) fn well_known_symbol_for_type(
        &self,
        receiver_ty: &Type,
        types: &TypeTable,
    ) -> Option<WellKnownSymbol> {
        // follow type as value wrappers
        if let Type::Value { value } = receiver_ty {
            let value_ty = types.get_type(*value);
            return self.well_known_symbol_for_type(value_ty, types);
        }

        // map structural and literal receiver types
        match receiver_ty {
            Type::Array { .. } | Type::ArraySized { .. } | Type::Tuple { .. } => {
                Some(WellKnownSymbol::Array)
            }
            Type::Object { .. } => Some(WellKnownSymbol::Object),
            Type::Function { .. } => Some(WellKnownSymbol::Function),
            Type::TypeLiteral { value } => self.well_known_symbol_for_type_literal(value),
            _ => None,
        }
    }

    /// Build a well known reference type for implicit member lookup.
    pub(super) fn well_known_type(
        &self,
        profile: ProfileId,
        receiver_ty: &Type,
        types: &mut TypeTable,
    ) -> Option<Type> {
        // resolve the well known symbol for this receiver
        let well_known_symbol = self.well_known_symbol_for_type(receiver_ty, types)?;
        let symbol = self.get_well_known_type_symbol(profile, well_known_symbol)?;

        // unwrap type as value wrappers for array element handling
        let receiver_ty = if let Type::Value { value } = receiver_ty {
            types.get_type(*value)
        } else {
            receiver_ty
        };

        // build static arguments for array like receivers
        let static_arguments = match receiver_ty {
            Type::Array { element } => element.map(|element| {
                vec![StaticArgument::value(StaticExpression::Type {
                    ty: element,
                })]
            }),
            Type::ArraySized { element, .. } => {
                Some(vec![StaticArgument::value(StaticExpression::Type {
                    ty: *element,
                })])
            }
            Type::Tuple { elements } => {
                if elements.is_empty() {
                    None
                } else {
                    let element_types = elements.iter().map(|element| element.ty).collect();
                    let element_ty_id = self.union_type_from_list(element_types, types);
                    Some(vec![StaticArgument::value(StaticExpression::Type {
                        ty: element_ty_id,
                    })])
                }
            }
            _ => None,
        };

        Some(Type::Reference {
            symbol,
            static_arguments,
        })
    }
}
