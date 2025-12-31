use crate::{AnalyzeResult, Compiler};
use destack_dir::{
    LocalNodeIdAny, LocalTypeId, PrimitiveType, ScalarLiteral, StaticKey, Type, TypeLiteral,
    TypeTable,
};
use destack_workspace::{Module, ProfileId};

/// Contextual function signature derived from an expected type.
#[derive(Debug, Clone)]
pub(super) struct ExpectedFunctionSignature {
    /// Expected dynamic parameter types.
    pub(super) dynamic_parameters: Vec<LocalTypeId>,
    /// Expected return type.
    pub(super) return_type: Option<LocalTypeId>,
}

impl Compiler {
    /// Derive a contextual function signature from an expected type.
    pub(super) fn expected_function_signature(
        &self,
        expected_ty_id: Option<LocalTypeId>,
        types: &TypeTable,
    ) -> Option<ExpectedFunctionSignature> {
        let expected_ty_id = self.expected_value_type_id(expected_ty_id, types)?;
        match types.get_type(expected_ty_id) {
            Type::Function {
                dynamic_parameters,
                return_type,
                ..
            } => Some(ExpectedFunctionSignature {
                dynamic_parameters: dynamic_parameters.clone(),
                return_type: *return_type,
            }),
            _ => None,
        }
    }

    /// Derive an expected object type id from a contextual type.
    pub(super) fn expected_object_type_id(
        &self,
        module: &Module,
        profile: ProfileId,
        node_id: LocalNodeIdAny,
        expected_ty_id: Option<LocalTypeId>,
        types: &mut TypeTable,
    ) -> AnalyzeResult<Option<LocalTypeId>> {
        // skip when there is no contextual type
        let expected_ty_id = self.expected_value_type_id(expected_ty_id, types);
        let Some(expected_ty_id) = expected_ty_id else {
            return Ok(None);
        };

        // resolve object types or instance types for references
        let expected_type = types.get_type(expected_ty_id);
        if matches!(expected_type, Type::Object { .. }) {
            return Ok(Some(expected_ty_id));
        }

        let Some(symbol) = expected_type.symbol() else {
            return Ok(None);
        };

        self.resolve_instance_type_id_for_symbol(module, profile, node_id, symbol, types)
    }

    /// Resolve an expected field type from a contextual object type and key.
    pub(super) fn expected_field_type_id(
        &self,
        expected_object_ty_id: Option<LocalTypeId>,
        key: &StaticKey,
        types: &TypeTable,
    ) -> Option<LocalTypeId> {
        let expected_object_ty_id = expected_object_ty_id?;
        match types.get_type(expected_object_ty_id) {
            Type::Object { fields, .. } => fields
                .iter()
                .find(|field| field.key.matches(key))
                .map(|field| field.ty),
            _ => None,
        }
    }

    /// Resolve contextual element types for array and tuple expressions.
    pub(super) fn expected_element_types(
        &self,
        expected_ty_id: Option<LocalTypeId>,
        element_count: usize,
        types: &TypeTable,
    ) -> Vec<Option<LocalTypeId>> {
        let mut expected = vec![None; element_count];
        let expected_ty_id = match self.expected_value_type_id(expected_ty_id, types) {
            Some(expected_ty_id) => expected_ty_id,
            None => return expected,
        };

        match types.get_type(expected_ty_id) {
            Type::Tuple { elements } => {
                for (index, element) in elements.iter().enumerate().take(element_count) {
                    expected[index] = Some(element.ty);
                }
            }
            Type::Array {
                element: Some(element_ty_id),
            } => {
                expected.fill(Some(*element_ty_id));
            }
            _ => {}
        }

        expected
    }

    /// Match a scalar literal against a contextual type when possible.
    pub(super) fn expected_type_for_scalar_literal(
        &self,
        value: &ScalarLiteral,
        expected_ty_id: Option<LocalTypeId>,
        types: &TypeTable,
    ) -> Option<LocalTypeId> {
        let expected_ty_id = self.expected_value_type_id(expected_ty_id, types)?;
        self.match_scalar_literal_expected(value, expected_ty_id, types)
    }

    /// Strip a Type::Value wrapper from a type id.
    pub(super) fn expected_value_type_id(
        &self,
        expected_ty_id: Option<LocalTypeId>,
        types: &TypeTable,
    ) -> Option<LocalTypeId> {
        let expected_ty_id = expected_ty_id?;
        let expected_ty_id = match types.get_type(expected_ty_id) {
            Type::Value { value } => *value,
            _ => expected_ty_id,
        };
        if self.is_infer_var_type(expected_ty_id, types) {
            None
        } else {
            Some(expected_ty_id)
        }
    }

    /// Match a scalar literal against an expected type.
    fn match_scalar_literal_expected(
        &self,
        value: &ScalarLiteral,
        expected_ty_id: LocalTypeId,
        types: &TypeTable,
    ) -> Option<LocalTypeId> {
        match types.get_type(expected_ty_id) {
            Type::TypeLiteral {
                value: TypeLiteral::Primitive(PrimitiveType::Number),
            } => match value {
                ScalarLiteral::Integer(_) | ScalarLiteral::Float(_) => Some(expected_ty_id),
                _ => None,
            },
            Type::TypeLiteral {
                value: TypeLiteral::Primitive(PrimitiveType::String),
            } => match value {
                ScalarLiteral::String(_) => Some(expected_ty_id),
                _ => None,
            },
            Type::TypeLiteral {
                value: TypeLiteral::Primitive(PrimitiveType::Boolean),
            } => match value {
                ScalarLiteral::Boolean(_) => Some(expected_ty_id),
                _ => None,
            },
            Type::TypeLiteral {
                value: TypeLiteral::ScalarLiteral(expected_literal),
            } => {
                if expected_literal == value {
                    Some(expected_ty_id)
                } else {
                    None
                }
            }
            Type::Union { elements } => {
                for element in elements {
                    if let Some(matched) =
                        self.match_scalar_literal_expected(value, *element, types)
                    {
                        return Some(matched);
                    }
                }
                None
            }
            _ => None,
        }
    }
}
