use super::*;

#[allow(clippy::too_many_arguments)]
impl Compiler {
    pub(crate) fn type_has_property(
        &self,
        module: &Module,
        profile: ProfileId,
        type_id: LocalTypeId,
        key: &StaticKey,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> bool {
        // anchor apparent type resolution to the current type source
        let source_id = types.get_type_source(type_id);

        // walk through shapes that can carry fields
        let ty = types.get_type(type_id).clone();
        match ty {
            Type::Object { fields, .. } => fields.iter().any(|field| field.key.matches(key)),
            Type::Reference { symbol, .. } => {
                // follow apparent instance types for nominal references
                self.apparent_instance_type(module, profile, source_id, symbol, symbols, types)
                    .is_some_and(|instance_id| {
                        self.type_has_property(module, profile, instance_id, key, symbols, types)
                    })
            }
            Type::Intersection { elements } => {
                // accept any intersection member that matches
                elements.iter().any(|element_id| {
                    self.type_has_property(module, profile, *element_id, key, symbols, types)
                })
            }
            _ => false,
        }
    }

    /// Resolve the type for a field with the given key.
    pub(crate) fn type_field_type_for_key(
        &self,
        module: &Module,
        profile: ProfileId,
        type_id: LocalTypeId,
        key: &StaticKey,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> AnalyzeResult<Option<(LocalTypeId, bool)>> {
        // unwrap aliases before walking fields
        let type_id =
            self.unwrap_type_alias_reference(module, profile, type_id, tree, symbols, types)?;
        let mut field_types = Vec::new();
        let mut is_optional = true;

        // collect matching field types for the key
        let mut pending_type_ids = vec![type_id];
        let mut visited_type_ids = Vec::new();
        while let Some(current_type_id) = pending_type_ids.pop() {
            if visited_type_ids.contains(&current_type_id) {
                continue;
            }
            visited_type_ids.push(current_type_id);
            match types.get_type(current_type_id) {
                Type::Object { fields, .. } => {
                    // collect all matching fields from the object
                    for field in fields {
                        if field.key.matches(key) {
                            field_types.push(field.ty);
                            is_optional = is_optional && field.is_optional;
                        }
                    }
                }
                Type::Reference { symbol, .. } => {
                    // prefer apparent instance types for nominal references
                    let source_id = types.get_type_source(current_type_id);
                    if let Some(instance_id) = self
                        .apparent_instance_type(module, profile, source_id, *symbol, symbols, types)
                    {
                        pending_type_ids.push(instance_id);
                    }
                }
                Type::Intersection { elements } => {
                    // gather fields from every element
                    for element_id in elements {
                        pending_type_ids.push(*element_id);
                    }
                }
                _ => {}
            }
        }
        if field_types.is_empty() {
            return Ok(None);
        }

        // combine multiple field types with intersection
        let field_type_id = match field_types.len() {
            1 => field_types[0],
            _ => {
                let source_type_id = field_types[0];
                types.insert_type_from_type(
                    Type::Intersection {
                        elements: field_types,
                    },
                    source_type_id,
                )
            }
        };

        Ok(Some((field_type_id, is_optional)))
    }

    /// Check whether a type id is any or unknown.

    pub(crate) fn type_is_object_like(
        &self,
        module: &Module,
        profile: ProfileId,
        type_id: LocalTypeId,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> bool {
        // anchor apparent type resolution to the current type source
        let source_id = types.get_type_source(type_id);

        // match shapes that would produce typeof object
        let ty = types.get_type(type_id).clone();
        match ty {
            Type::TypeLiteral {
                value: TypeLiteral::Null,
            } => true,
            Type::Object { .. }
            | Type::Array { .. }
            | Type::ArraySized { .. }
            | Type::Tuple { .. }
            | Type::Value { .. } => true,
            Type::Reference { symbol, .. } => {
                // prefer apparent instance types when available
                if let Some(instance_id) =
                    self.apparent_instance_type(module, profile, source_id, symbol, symbols, types)
                {
                    return self.type_is_object_like(module, profile, instance_id, symbols, types);
                }

                matches!(
                    symbol.local_id.ty,
                    SymbolType::Class
                        | SymbolType::Struct
                        | SymbolType::Interface
                        | SymbolType::Extension
                        | SymbolType::Enum
                )
            }
            Type::Intersection { elements } => elements.iter().any(|element_id| {
                self.type_is_object_like(module, profile, *element_id, symbols, types)
            }),
            _ => false,
        }
    }

    /// Check whether a type is function like for typeof guards.
    pub(crate) fn type_is_function_like(
        &self,
        module: &Module,
        profile: ProfileId,
        type_id: LocalTypeId,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> bool {
        // anchor apparent type resolution to the current type source
        let source_id = types.get_type_source(type_id);

        // match callable shapes for typeof function
        let ty = types.get_type(type_id).clone();
        match ty {
            Type::Function { .. } => true,
            Type::Object {
                call_signatures,
                construct_signatures,
                ..
            } => !call_signatures.is_empty() || !construct_signatures.is_empty(),
            Type::Reference { symbol, .. } => {
                // prefer apparent instance types when available
                if let Some(instance_id) =
                    self.apparent_instance_type(module, profile, source_id, symbol, symbols, types)
                {
                    return self.type_is_function_like(
                        module,
                        profile,
                        instance_id,
                        symbols,
                        types,
                    );
                }

                symbol.local_id.ty == SymbolType::Function
            }
            Type::Intersection { elements } => elements.iter().any(|element_id| {
                self.type_is_function_like(module, profile, *element_id, symbols, types)
            }),
            _ => false,
        }
    }
}
