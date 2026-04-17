use crate::Compiler;
use destack_builtin::LanguageSymbol;
use destack_dir::{
    PrimitiveType, ScalarLiteral, StaticArgument, StaticExpression, Type, TypeLiteral, TypeTable,
    WellKnownSymbol,
};
use destack_workspace::ProfileId;

impl Compiler {
    /// Return the well known symbol for a primitive or literal type.
    pub(crate) fn well_known_symbol_for_type_literal(
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
    pub(crate) fn well_known_symbol_for_type(
        &self,
        receiver_ty: &Type,
        types: &TypeTable,
    ) -> Option<WellKnownSymbol> {
        // map structural and literal receiver types
        match receiver_ty {
            Type::Reference { symbol, .. } => {
                types
                    .get_alias_target_type_id(*symbol)
                    .and_then(|alias_id| match types.get_type(alias_id) {
                        Type::TypeLiteral { value } => {
                            self.well_known_symbol_for_type_literal(value)
                        }
                        _ => None,
                    })
            }
            Type::Array { is_readonly, .. } | Type::ArraySized { is_readonly, .. } => {
                if *is_readonly {
                    Some(WellKnownSymbol::ReadonlyArray)
                } else {
                    Some(WellKnownSymbol::Array)
                }
            }
            Type::Tuple { is_readonly, .. } => {
                if *is_readonly {
                    Some(WellKnownSymbol::ReadonlyArray)
                } else {
                    Some(WellKnownSymbol::Array)
                }
            }
            Type::Object {
                call_signatures,
                construct_signatures,
                ..
            } => {
                if !call_signatures.is_empty() || !construct_signatures.is_empty() {
                    Some(WellKnownSymbol::Function)
                } else {
                    Some(WellKnownSymbol::Object)
                }
            }
            Type::Function { .. } => Some(WellKnownSymbol::Function),
            Type::TypeLiteral { value } => self.well_known_symbol_for_type_literal(value),
            _ => None,
        }
    }

    /// Build a well known reference type for implicit member lookup.
    pub(crate) fn well_known_type(
        &self,
        profile: ProfileId,
        receiver_ty: &Type,
        types: &mut TypeTable,
    ) -> Option<Type> {
        // type-as-value uses the Type<T> descriptor
        if let Type::Value { value } = receiver_ty {
            let symbol = self.get_language_symbol(profile, LanguageSymbol::Type)?;
            let argument = StaticArgument::value(StaticExpression::Type { ty: *value });
            return Some(Type::Reference {
                symbol,
                generic_arguments: Some(vec![argument]),
            });
        }

        // resolve the well known symbol for this receiver
        let well_known_symbol = self.well_known_symbol_for_type(receiver_ty, types)?;
        let symbol =
            if let Some(symbol) = self.get_well_known_type_symbol(profile, well_known_symbol) {
                symbol
            } else if matches!(well_known_symbol, WellKnownSymbol::ReadonlyArray) {
                // fall back to Array when ReadonlyArray is unavailable
                self.get_well_known_type_symbol(profile, WellKnownSymbol::Array)?
            } else {
                return None;
            };

        // build static arguments for array like receivers
        let generic_arguments = match receiver_ty {
            Type::Array { element, .. } => element.map(|element| {
                vec![StaticArgument::value(StaticExpression::Type {
                    ty: element,
                })]
            }),
            Type::ArraySized { element, .. } => {
                Some(vec![StaticArgument::value(StaticExpression::Type {
                    ty: *element,
                })])
            }
            Type::Tuple { elements, .. } => {
                if elements.is_empty() {
                    None
                } else {
                    let element_types = elements.iter().map(|element| element.ty).collect();
                    let source_type_id = elements[0].ty;
                    let element_ty_id =
                        self.union_type_from_list(element_types, source_type_id, types);
                    Some(vec![StaticArgument::value(StaticExpression::Type {
                        ty: element_ty_id,
                    })])
                }
            }
            _ => None,
        };

        Some(Type::Reference {
            symbol,
            generic_arguments,
        })
    }
}
