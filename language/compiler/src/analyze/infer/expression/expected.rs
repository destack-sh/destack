use crate::analyze::common::{CanonicalSymbolMode, InferContext, RelationMode};
use crate::{AnalyzeOptions, AnalyzeResult, Compiler, WellKnownSymbol};
use destack_dir::{
    Declaration, Expression, LocalNodeId, LocalTypeId, Member, NodeTree, NormalizationMode,
    PrimitiveType, Property, ScalarLiteral, StaticKey, SymbolType, Type, TypeExpression,
    TypeLiteral, TypeTable,
};

/// Contextual function signature derived from an expected type.
#[derive(Debug, Clone)]
pub(crate) struct ExpectedFunctionSignature {
    /// Expected `this` parameter type.
    pub(crate) this_parameter: Option<LocalTypeId>,
    /// Expected dynamic parameter types.
    pub(crate) parameters: Vec<LocalTypeId>,
    /// Expected return type.
    pub(crate) return_type: Option<LocalTypeId>,
}

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Derive a contextual function signature from an expected type.
    pub(crate) fn expected_function_signature(
        &self,
        expected_ty_id: Option<LocalTypeId>,
        types: &TypeTable,
    ) -> Option<ExpectedFunctionSignature> {
        let expected_ty_id = self.expected_value_type(expected_ty_id, types)?;
        match types.get_type(expected_ty_id) {
            Type::Function {
                this_parameter,
                parameters,
                return_type,
                ..
            } => Some(ExpectedFunctionSignature {
                this_parameter: *this_parameter,
                parameters: parameters.clone(),
                return_type: *return_type,
            }),
            _ => None,
        }
    }

    /// Derive an expected object type id from a contextual type.
    pub(crate) fn expected_object_type(
        &self,
        ctx: &mut InferContext<'_>,
        expected_ty_id: Option<LocalTypeId>,
    ) -> AnalyzeResult<Option<LocalTypeId>> {
        // skip when there is no contextual type
        let expected_ty_id = self.expected_value_type(expected_ty_id, ctx.types);
        let Some(expected_ty_id) = expected_ty_id else {
            return Ok(None);
        };
        let expected_requires_convergence =
            self.type_requires_infer_convergence(ctx.type_view(), expected_ty_id);

        // keep unsolved object contexts stable until infer convergence
        if matches!(ctx.types.get_type(expected_ty_id), Type::Object { .. })
            && expected_requires_convergence
        {
            return Ok(Some(expected_ty_id));
        }

        // normalize mapped, alias, and object shapes into concrete object types
        let normalized_ty_id = self.normalize_type_with_relation(
            &mut ctx.type_context_reborrow(),
            expected_ty_id,
            NormalizationMode::Assign,
            RelationMode::EXPECTED_TYPE,
        );
        if matches!(ctx.types.get_type(normalized_ty_id), Type::Object { .. }) {
            let normalized_requires_convergence =
                self.type_requires_infer_convergence(ctx.type_view(), normalized_ty_id);
            if expected_requires_convergence && !normalized_requires_convergence {
                return Ok(Some(expected_ty_id));
            }
            return Ok(Some(normalized_ty_id));
        }

        // reuse concrete object types when normalization does not change them
        let expected_type = ctx.types.get_type(expected_ty_id).clone();
        if matches!(expected_type, Type::Object { .. }) {
            return Ok(Some(expected_ty_id));
        }

        let source_id = ctx.types.get_type_source(expected_ty_id);
        let expected_type = ctx.types.get_type(expected_ty_id).clone();

        // resolve the canonical reference target behind the expected type
        let (symbol, generic_arguments) = match expected_type {
            Type::Reference {
                symbol,
                generic_arguments,
            } => (symbol, generic_arguments),
            _ => {
                let Some(symbol) = expected_type.symbol() else {
                    return Ok(None);
                };
                (symbol, None)
            }
        };
        let symbol = self.canonical_symbol_id(
            ctx.module_symbol_view(),
            symbol,
            CanonicalSymbolMode::FollowAliases,
        );

        // keep dedicated record-like contexts on the map or record path
        let is_record_like_symbol = self
            .get_well_known_type_symbol(ctx.profile, WellKnownSymbol::Map)
            .is_some_and(|map_symbol| map_symbol == symbol)
            || self
                .get_well_known_type_symbol(ctx.profile, WellKnownSymbol::Record)
                .is_some_and(|record_symbol| record_symbol == symbol);
        if is_record_like_symbol {
            return Ok(None);
        }

        // nominal targets should not be treated as plain object contexts
        if symbol.ty() == SymbolType::Newtype
            || self.symbol_is_nominal_interface(ctx.tree_symbol_view(), symbol)
        {
            return Ok(None);
        }

        // resolve the specialized instance type for the reference
        let Some(instance_ty_id) = self.specialized_instance_type_for_reference(
            &mut ctx.type_context_reborrow(),
            source_id,
            symbol,
            generic_arguments.as_deref(),
        ) else {
            return Ok(None);
        };
        Ok(Some(instance_ty_id))
    }

    /// Derive an expected object type for an object literal with a union context.
    pub(crate) fn expected_object_type_for_literal_union(
        &self,
        ctx: &mut InferContext<'_>,
        expected_ty_id: Option<LocalTypeId>,
        properties: &[LocalNodeId<Property>],
        _options: &AnalyzeOptions,
    ) -> AnalyzeResult<Option<LocalTypeId>> {
        let expected_ty_id = self.expected_value_type(expected_ty_id, ctx.types);
        let Some(expected_ty_id) = expected_ty_id else {
            return Ok(None);
        };

        let normalized_ty_id = self.normalize_type_with_relation(
            &mut ctx.type_context_reborrow(),
            expected_ty_id,
            NormalizationMode::Assign,
            RelationMode::EXPECTED_TYPE,
        );
        let Type::Union { elements } = ctx.types.get_type(normalized_ty_id).clone() else {
            return Ok(None);
        };

        let mut literal_filters = Vec::new();
        for property_id in properties {
            let property = ctx.tree.get(*property_id);
            let Property::Field { key, value, .. } = property else {
                continue;
            };
            let Some(static_key) = self.static_key_from_key(
                ctx.compiler_context.revision(),
                ctx.profile,
                ctx.tree,
                ctx.symbols,
                ctx.types,
                *key,
            ) else {
                continue;
            };
            let value_id = *value;
            let Expression::ScalarLiteral { value } = ctx.tree.get(value_id) else {
                continue;
            };

            let literal_type = Type::TypeLiteral {
                value: self.infer_scalar_literal(value),
            };
            let literal_type_id = ctx
                .types
                .insert_type_from_any(literal_type, value_id.into_any());
            literal_filters.push((static_key, literal_type_id));
        }
        if literal_filters.is_empty() {
            return Ok(None);
        }

        let mut matching_elements = Vec::new();
        'elements: for element_id in elements {
            for (key, literal_type_id) in &literal_filters {
                let field_info = self.type_field_type_for_key(
                    &mut ctx.type_context_reborrow(),
                    element_id,
                    key,
                )?;
                let Some((field_type_id, _)) = field_info else {
                    continue 'elements;
                };

                let is_assignable = self
                    .is_type_assignable(
                        &mut ctx.type_context_reborrow(),
                        field_type_id,
                        *literal_type_id,
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
            &mut ctx.type_context_reborrow(),
            matched_id,
            NormalizationMode::Assign,
            RelationMode::EXPECTED_TYPE,
        );
        if matches!(ctx.types.get_type(normalized_id), Type::Object { .. }) {
            return Ok(Some(normalized_id));
        }

        Ok(Some(matched_id))
    }

    /// Derive an expected object type for tagged object literals.
    pub(crate) fn expected_tagged_object_type(
        &self,
        ctx: &mut InferContext<'_>,
        expression_id: LocalNodeId<Expression>,
        ty_id: LocalTypeId,
        expected_object_ty_id: Option<LocalTypeId>,
    ) -> AnalyzeResult<Option<LocalTypeId>> {
        // skip when no contextual object type exists
        let Some(expected_object_ty_id) = expected_object_ty_id else {
            return Ok(None);
        };

        // resolve the referenced symbol for the tagged type
        let Type::Reference { symbol, .. } = ctx.types.get_type(ty_id) else {
            return Ok(Some(expected_object_ty_id));
        };

        // only read local symbol ctx for local reference symbols
        if symbol.module_id != ctx.module.id {
            return Ok(Some(expected_object_ty_id));
        }

        // resolve the primary declaration for the symbol
        let symbol_entry = ctx.symbols.get_symbol(symbol.local_id);
        let Some(primary_declaration) = symbol_entry.primary_declaration else {
            return Ok(Some(expected_object_ty_id));
        };
        if primary_declaration.module_id != ctx.module.id {
            return Ok(Some(expected_object_ty_id));
        }
        let Ok(primary_declaration) = primary_declaration.try_into_typed::<Declaration>() else {
            return Ok(Some(expected_object_ty_id));
        };
        let declaration_id: LocalNodeId<Declaration> = primary_declaration.into();
        let declaration = ctx.tree.get(declaration_id);

        // ensure the declaration is a struct or class
        if !matches!(
            declaration,
            Declaration::Struct(..) | Declaration::Class(..)
        ) {
            return Ok(Some(expected_object_ty_id));
        }

        // collect declared field keys from the struct or class
        let Some(member_ids) = declaration.member_ids() else {
            return Ok(Some(expected_object_ty_id));
        };
        let mut declared_field_keys = Vec::new();
        for member_id in member_ids {
            let member = ctx.tree.get(*member_id);
            match member {
                Member::Field { key, .. } => {
                    if let Some(static_key) = self.static_key_from_key(
                        ctx.compiler_context.revision(),
                        ctx.profile,
                        ctx.tree,
                        ctx.symbols,
                        ctx.types,
                        *key,
                    ) {
                        declared_field_keys.push(static_key);
                    }
                }
                Member::Embed { value, .. } => {
                    // include embedded fields in tagged literal filtering
                    let embed_shape =
                        self.embed_member_shape(&mut ctx.type_context_reborrow(), *value)?;
                    for field in embed_shape.fields {
                        declared_field_keys.push(field.key);
                    }
                }
                _ => {}
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
        } = ctx.types.get_type(expected_object_ty_id).clone()
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
        let filtered_type_id = ctx.types.insert_type_from(filtered_type, expression_id);

        Ok(Some(filtered_type_id))
    }

    /// Prefer a contextual tagged type when it supplies missing static arguments.
    /// Requires the tag expression to detect explicit static arguments.
    pub(crate) fn expected_tag_reference_type_from_context(
        &self,
        tag_expression_id: LocalNodeId<TypeExpression>,
        expected_ty_id: Option<LocalTypeId>,
        tag_ty_id: LocalTypeId,
        tree: &NodeTree,
        types: &TypeTable,
    ) -> LocalTypeId {
        // skip when no contextual type exists
        let Some(expected_ty_id) = self.expected_value_type(expected_ty_id, types) else {
            return tag_ty_id;
        };

        // keep explicit generic arguments on the tag
        let has_explicit_arguments = match tree.get(tag_expression_id) {
            TypeExpression::Reference {
                generic_arguments, ..
            }
            | TypeExpression::LocalReference {
                generic_arguments, ..
            }
            | TypeExpression::ModuleReference {
                generic_arguments, ..
            }
            | TypeExpression::GlobalReference {
                generic_arguments, ..
            }
            | TypeExpression::Member {
                generic_arguments, ..
            }
            | TypeExpression::Import {
                generic_arguments, ..
            } => !generic_arguments.is_empty(),
            _ => false,
        };
        if has_explicit_arguments {
            return tag_ty_id;
        }

        // resolve reference symbols for the tag and expected types
        let Type::Reference {
            symbol: tag_symbol,
            generic_arguments: _,
        } = types.get_type(tag_ty_id)
        else {
            return tag_ty_id;
        };
        let Type::Reference {
            symbol: expected_symbol,
            generic_arguments: expected_arguments,
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
    pub(crate) fn expected_field_type(
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
    pub(crate) fn expected_element_types(
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
    pub(crate) fn expected_array_element_type(
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
    pub(crate) fn expected_type_for_scalar_literal(
        &self,
        value: &ScalarLiteral,
        expected_ty_id: Option<LocalTypeId>,
        types: &TypeTable,
        options: &AnalyzeOptions,
    ) -> Option<LocalTypeId> {
        let expected_ty_id = self.expected_value_type(expected_ty_id, types)?;
        self.match_scalar_literal_expected(value, expected_ty_id, types, options)
    }

    /// Return true when the expected type is a scalar-literal union that should keep literal precision.
    pub(crate) fn expected_type_is_scalar_literal_union(
        &self,
        expected_ty_id: Option<LocalTypeId>,
        types: &TypeTable,
    ) -> bool {
        let Some(expected_ty_id) = self.expected_value_type(expected_ty_id, types) else {
            return false;
        };

        self.type_is_scalar_literal_union(expected_ty_id, types)
    }

    /// Strip a Type::Value wrapper from a type id.
    pub(crate) fn expected_value_type(
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

    /// Return true when a type is one scalar literal or a union of scalar literals.
    fn type_is_scalar_literal_union(&self, ty_id: LocalTypeId, types: &TypeTable) -> bool {
        match types.get_type(ty_id) {
            Type::TypeLiteral {
                value: TypeLiteral::ScalarLiteral(_),
            } => true,
            Type::Union { elements } => {
                !elements.is_empty()
                    && elements
                        .iter()
                        .all(|element| self.type_is_scalar_literal_union(*element, types))
            }
            _ => false,
        }
    }
}
