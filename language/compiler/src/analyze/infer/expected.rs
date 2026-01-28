use std::collections::HashMap;

use crate::analyze::common::RelationMode;
use crate::{AnalyzeOptions, AnalyzeResult, Compiler};
use destack_dir::{
    Declaration, Expression, GlobalSymbolId, LocalNodeId, LocalTypeId, Member, NodeTree,
    NormalizationMode, PrimitiveType, Property, ScalarLiteral, StaticArgument, StaticKey,
    SymbolTable, SymbolType, Type, TypeKind, TypeLiteral, TypeTable,
};
use destack_workspace::{Module, ProfileId};

/// Contextual function signature derived from an expected type.
#[derive(Debug, Clone)]
pub(super) struct ExpectedFunctionSignature {
    /// Expected `this` parameter type.
    pub(super) this_parameter: Option<LocalTypeId>,
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
                this_parameter,
                dynamic_parameters,
                return_type,
                ..
            } => Some(ExpectedFunctionSignature {
                this_parameter: *this_parameter,
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

        // normalize mapped, alias, and object shapes into concrete object types
        let normalized_ty_id = self.normalize_type_with_relation(
            module,
            profile,
            expected_ty_id,
            symbols,
            types,
            NormalizationMode::Assign,
            RelationMode::TYPE_OPS,
        );
        if matches!(types.get_type(normalized_ty_id), Type::Object { .. }) {
            return Ok(Some(normalized_ty_id));
        }

        // reuse concrete object types when normalization does not change them
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

    /// Derive an expected object type for an object literal with a union context.
    pub(super) fn expected_object_type_for_literal_union(
        &self,
        module: &Module,
        profile: ProfileId,
        expected_ty_id: Option<LocalTypeId>,
        properties: &[LocalNodeId<Property>],
        options: &AnalyzeOptions,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> AnalyzeResult<Option<LocalTypeId>> {
        let expected_ty_id = self.expected_value_type(expected_ty_id, types);
        let Some(expected_ty_id) = expected_ty_id else {
            return Ok(None);
        };

        let normalized_ty_id = self.normalize_type_with_relation(
            module,
            profile,
            expected_ty_id,
            symbols,
            types,
            NormalizationMode::Assign,
            RelationMode::TYPE_OPS,
        );
        let Type::Union { elements } = types.get_type(normalized_ty_id).clone() else {
            return Ok(None);
        };

        let mut literal_filters = Vec::new();
        for property_id in properties {
            let property = tree.get(*property_id);
            let Property::Field { key, value, .. } = property else {
                continue;
            };
            let Some(key) = key else {
                continue;
            };
            let Some(value_id) = value else {
                continue;
            };
            let Some(static_key) =
                self.static_key_from_dynamic_key(profile, *key, tree, symbols, types)
            else {
                continue;
            };
            let Expression::ScalarLiteral { value } = tree.get(*value_id) else {
                continue;
            };

            let literal_type = Type::TypeLiteral {
                value: self.infer_scalar_literal(value),
            };
            let literal_type_id = types.insert_type_from_any(literal_type, value_id.into_any());
            literal_filters.push((static_key, literal_type_id));
        }
        if literal_filters.is_empty() {
            return Ok(None);
        }

        let mut matching_elements = Vec::new();
        'elements: for element_id in elements {
            for (key, literal_type_id) in &literal_filters {
                let field_info = self.type_field_type_for_key(
                    module, profile, element_id, key, tree, symbols, types,
                )?;
                let Some((field_type_id, _)) = field_info else {
                    continue 'elements;
                };

                let is_assignable = self
                    .is_type_assignable(
                        module,
                        profile,
                        symbols,
                        field_type_id,
                        *literal_type_id,
                        types,
                        options,
                    )
                    .is_assignable();
                if !is_assignable {
                    continue 'elements;
                }
            }
            matching_elements.push(element_id);
        }

        if matching_elements.len() != 1 {
            return Ok(None);
        }

        let matched_id = matching_elements[0];
        let normalized_id = self.normalize_type_with_relation(
            module,
            profile,
            matched_id,
            symbols,
            types,
            NormalizationMode::Assign,
            RelationMode::TYPE_OPS,
        );
        if matches!(types.get_type(normalized_id), Type::Object { .. }) {
            return Ok(Some(normalized_id));
        }

        Ok(Some(matched_id))
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
            Type::Object {
                fields,
                index_signatures,
                ..
            } => {
                if let Some(field) = fields.iter().find(|field| field.key.matches(key)) {
                    return Some(field.ty);
                }

                for signature in index_signatures {
                    if self.index_signature_allows_key(key, signature.key_type, types) {
                        return Some(signature.value_type);
                    }
                }

                None
            }
            _ => None,
        }
    }

    // check whether an index signature key type can accept a static key
    fn index_signature_allows_key(
        &self,
        key: &StaticKey,
        key_type_id: LocalTypeId,
        types: &TypeTable,
    ) -> bool {
        match types.get_type(key_type_id) {
            Type::Union { elements } => elements
                .iter()
                .any(|element_id| self.index_signature_allows_key(key, *element_id, types)),
            Type::TypeLiteral {
                value: TypeLiteral::Primitive(PrimitiveType::String),
            } => key.is_string_like(),
            Type::TypeLiteral {
                value: TypeLiteral::Primitive(PrimitiveType::Number | PrimitiveType::Float(_)),
            }
            | Type::TypeLiteral {
                value: TypeLiteral::Primitive(PrimitiveType::Int(_)),
            } => key.is_string_like(),
            Type::TypeLiteral {
                value:
                    TypeLiteral::Primitive(PrimitiveType::Symbol)
                    | TypeLiteral::Primitive(PrimitiveType::UniqueSymbol),
            } => key.is_symbol_like(),
            _ => false,
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
            Type::ArraySized { element, .. } => {
                expected.fill(Some(*element));
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
            Type::ArraySized { element, .. } => Some(*element),
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
        options: &AnalyzeOptions,
    ) -> Option<LocalTypeId> {
        let expected_ty_id = self.expected_value_type(expected_ty_id, types)?;
        self.match_scalar_literal_expected(value, expected_ty_id, types, options)
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
        options: &AnalyzeOptions,
    ) -> Option<LocalTypeId> {
        match types.get_type(expected_ty_id) {
            Type::TypeLiteral {
                value: TypeLiteral::Primitive(primitive),
            } => {
                if self.is_scalar_literal_assignable(value, primitive, options) {
                    Some(expected_ty_id)
                } else {
                    None
                }
            }
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
                    if self
                        .match_scalar_literal_expected(value, *element, types, options)
                        .is_some()
                    {
                        return Some(expected_ty_id);
                    }
                }
                None
            }
            _ => None,
        }
    }
}
