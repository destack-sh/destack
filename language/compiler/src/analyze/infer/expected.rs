use std::collections::HashMap;

use crate::{AnalyzeOptions, AnalyzeResult, Compiler};
use destack_dir::{
    Declaration, Expression, GlobalSymbolId, LocalNodeId, LocalTypeId, Member, NodeTree,
    PrimitiveType, ScalarLiteral, StaticArgument, StaticKey, SymbolTable, SymbolType, Type,
    TypeKind, TypeLiteral, TypeTable,
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

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Derive a contextual function signature from an expected type.
    pub(super) fn expected_function_signature(
        &self,
        expected_ty_id: Option<LocalTypeId>,
        types: &TypeTable,
    ) -> Option<ExpectedFunctionSignature> {
        let expected_ty_id = self.expected_value_type(expected_ty_id, types)?;
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
    pub(super) fn expected_object_type(
        &self,
        module: &Module,
        profile: ProfileId,
        expected_ty_id: Option<LocalTypeId>,
        options: &AnalyzeOptions,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> AnalyzeResult<Option<LocalTypeId>> {
        // skip when there is no contextual type
        let expected_ty_id = self.expected_value_type(expected_ty_id, types);
        let Some(expected_ty_id) = expected_ty_id else {
            return Ok(None);
        };

        // reuse concrete object types
        let expected_type = types.get_type(expected_ty_id).clone();
        if matches!(expected_type, Type::Object { .. }) {
            return Ok(Some(expected_ty_id));
        }

        // resolve the reference symbol and arguments
        let source_id = types.get_type_source(expected_ty_id);
        let (symbol, static_arguments): (GlobalSymbolId, Option<Vec<StaticArgument>>) =
            match expected_type {
                Type::Reference {
                    symbol,
                    static_arguments,
                } => (symbol, static_arguments),
                _ => {
                    let Some(symbol) = expected_type.symbol() else {
                        return Ok(None);
                    };
                    (symbol, None)
                }
            };

        // unwrap local nominal aliases to their declared types for tagged literals
        let mut declared_type_id = None;
        if symbol.module_id == module.id {
            let symbol_entry = symbols.get_symbol(symbol.local_id);
            if symbol_entry.ty == SymbolType::Newtype
                && let Some(primary_declaration) = symbol_entry.primary_declaration
                && let Ok(declaration_id) = primary_declaration.try_into_typed::<Declaration>()
            {
                let declaration_id: LocalNodeId<Declaration> = declaration_id.into();
                if let Declaration::Type {
                    kind: TypeKind::Nominal,
                    value,
                    ..
                } = tree.get(declaration_id)
                {
                    let value_id = value.into_global_any(module.id);
                    if let Some(value_ty_id) = types.get_declared_type_id(value_id) {
                        if matches!(types.get_type(value_ty_id), Type::Unevaluated(_)) {
                            self.evaluate_type(module, profile, value_ty_id, tree, symbols, types)?;
                        }
                        declared_type_id = Some(value_ty_id);
                    }
                }
            }
        }

        // resolve the instance type for the reference
        let instance_ty_id =
            self.resolve_instance_type_for_symbol(module, profile, source_id, symbol, types)?;
        let Some(instance_ty_id) = instance_ty_id else {
            return Ok(None);
        };
        let instance_ty_id = declared_type_id.unwrap_or(instance_ty_id);

        // return the instance type when no static arguments exist
        let Some(static_arguments) = static_arguments else {
            return Ok(Some(instance_ty_id));
        };

        // resolve static arguments for substitution
        let resolved_arguments = self.resolve_type_reference_static_arguments(
            module,
            profile,
            source_id,
            symbol,
            Some(static_arguments.as_slice()),
            true,
            options,
            tree,
            symbols,
            types,
        )?;
        let Some(resolved_arguments) = resolved_arguments else {
            return Ok(Some(instance_ty_id));
        };

        // reuse instance type when no substitutions are needed
        if resolved_arguments.is_empty() {
            return Ok(Some(instance_ty_id));
        }

        // build substitutions for type parameters
        let substitutions = self.build_type_parameter_substitutions_for_symbol(
            module,
            profile,
            symbol,
            source_id,
            &resolved_arguments,
            tree,
            symbols,
            types,
        );

        // reuse instance type when no substitutions are needed
        if substitutions.is_empty() {
            return Ok(Some(instance_ty_id));
        }

        // substitute parameters inside the instance type
        let mut cache = HashMap::new();
        let substituted =
            self.substitute_static_parameters(instance_ty_id, &substitutions, types, &mut cache);

        Ok(Some(substituted))
    }

    /// Derive an expected object type for tagged object literals.
    pub(super) fn expected_tagged_object_type(
        &self,
        module: &Module,
        profile: ProfileId,
        expression_id: LocalNodeId<Expression>,
        ty_id: LocalTypeId,
        expected_object_ty_id: Option<LocalTypeId>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> AnalyzeResult<Option<LocalTypeId>> {
        // skip when no contextual object type exists
        let Some(expected_object_ty_id) = expected_object_ty_id else {
            return Ok(None);
        };

        // resolve the referenced symbol for the tagged type
        let Type::Reference { symbol, .. } = types.get_type(ty_id) else {
            return Ok(Some(expected_object_ty_id));
        };

        // resolve the primary declaration for the symbol
        let symbol_entry = symbols.get_symbol(symbol.local_id);
        let Some(primary_declaration) = symbol_entry.primary_declaration else {
            return Ok(Some(expected_object_ty_id));
        };
        if primary_declaration.module_id != module.id {
            return Ok(Some(expected_object_ty_id));
        }
        let Ok(primary_declaration) = primary_declaration.try_into_typed::<Declaration>() else {
            return Ok(Some(expected_object_ty_id));
        };
        let declaration_id: LocalNodeId<Declaration> = primary_declaration.into();
        let declaration = tree.get(declaration_id);

        // ensure the declaration is a struct or class
        if !matches!(
            declaration,
            Declaration::Struct { .. } | Declaration::Class { .. }
        ) {
            return Ok(Some(expected_object_ty_id));
        }

        // collect declared field keys from the struct or class
        let Some(member_ids) = declaration.member_ids() else {
            return Ok(Some(expected_object_ty_id));
        };
        let mut declared_field_keys = Vec::new();
        for member_id in member_ids {
            let member = tree.get(*member_id);
            let Member::Field { key: Some(key), .. } = member else {
                continue;
            };
            if let Some(static_key) =
                self.static_key_from_dynamic_key(profile, *key, tree, symbols, types)
            {
                declared_field_keys.push(static_key);
            }
        }
        if declared_field_keys.is_empty() {
            return Ok(Some(expected_object_ty_id));
        }

        // filter expected fields down to declared fields
        let Type::Object {
            fields,
            call_signatures,
            construct_signatures,
            index_signatures,
        } = types.get_type(expected_object_ty_id).clone()
        else {
            return Ok(Some(expected_object_ty_id));
        };
        let mut filtered_fields = Vec::new();
        for field in fields {
            if declared_field_keys
                .iter()
                .any(|key| key.matches(&field.key))
            {
                filtered_fields.push(field);
            }
        }
        let filtered_type = Type::Object {
            fields: filtered_fields,
            call_signatures,
            construct_signatures,
            index_signatures,
        };
        let filtered_type_id = types.insert_type_from(filtered_type, expression_id);

        Ok(Some(filtered_type_id))
    }

    /// Prefer a contextual tagged type when it supplies missing static arguments.
    /// Requires the tag expression to detect explicit static arguments.
    pub(super) fn expected_tag_reference_type_from_context(
        &self,
        tag_expression_id: LocalNodeId<Expression>,
        expected_ty_id: Option<LocalTypeId>,
        tag_ty_id: LocalTypeId,
        tree: &NodeTree,
        types: &TypeTable,
    ) -> LocalTypeId {
        // skip when no contextual type exists
        let Some(expected_ty_id) = self.expected_value_type(expected_ty_id, types) else {
            return tag_ty_id;
        };

        // keep explicit static arguments on the tag
        let has_explicit_arguments = match tree.get(tag_expression_id) {
            Expression::LocalReference {
                static_arguments, ..
            }
            | Expression::ModuleReference {
                static_arguments, ..
            }
            | Expression::GlobalReference {
                static_arguments, ..
            } => static_arguments
                .as_ref()
                .is_some_and(|arguments| !arguments.is_empty()),
            _ => false,
        };
        if has_explicit_arguments {
            return tag_ty_id;
        }

        // resolve reference symbols for the tag and expected types
        let Type::Reference {
            symbol: tag_symbol,
            static_arguments: _,
        } = types.get_type(tag_ty_id)
        else {
            return tag_ty_id;
        };
        let Type::Reference {
            symbol: expected_symbol,
            static_arguments: expected_arguments,
        } = types.get_type(expected_ty_id)
        else {
            return tag_ty_id;
        };

        // accept contextual arguments when the symbols match
        if tag_symbol == expected_symbol && expected_arguments.is_some() {
            return expected_ty_id;
        }

        tag_ty_id
    }

    /// Resolve an expected field type from a contextual object type and key.
    pub(super) fn expected_field_type(
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
        let expected_ty_id = match self.expected_value_type(expected_ty_id, types) {
            Some(expected_ty_id) => expected_ty_id,
            None => return expected,
        };

        match types.get_type(expected_ty_id) {
            Type::Tuple { elements, .. } => {
                for (index, element) in elements.iter().enumerate().take(element_count) {
                    expected[index] = Some(element.ty);
                }
            }
            Type::Array {
                element: Some(element_ty_id),
                ..
            } => {
                expected.fill(Some(*element_ty_id));
            }
            _ => {}
        }

        expected
    }

    /// Resolve a contextual element type for an array literal.
    pub(super) fn expected_array_element_type(
        &self,
        expected_ty_id: Option<LocalTypeId>,
        types: &TypeTable,
    ) -> Option<LocalTypeId> {
        let expected_ty_id = self.expected_value_type(expected_ty_id, types)?;
        match types.get_type(expected_ty_id) {
            Type::Array {
                element: Some(element_ty_id),
                ..
            } => Some(*element_ty_id),
            _ => None,
        }
    }

    /// Match a scalar literal against a contextual type when possible.
    pub(super) fn expected_type_for_scalar_literal(
        &self,
        value: &ScalarLiteral,
        expected_ty_id: Option<LocalTypeId>,
        types: &TypeTable,
    ) -> Option<LocalTypeId> {
        let expected_ty_id = self.expected_value_type(expected_ty_id, types)?;
        self.match_scalar_literal_expected(value, expected_ty_id, types)
    }

    /// Strip a Type::Value wrapper from a type id.
    pub(super) fn expected_value_type(
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
