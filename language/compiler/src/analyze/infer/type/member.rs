use super::*;

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Normalize one well known reference symbol before member inference recursion.
    fn normalize_well_known_reference_for_member_inference(
        &self,
        module: &Module,
        profile: ProfileId,
        reference_ty: Type,
    ) -> AnalyzeResult<Type> {
        let Type::Reference {
            symbol,
            static_arguments,
        } = reference_ty
        else {
            return Ok(reference_ty);
        };

        // normalize to declared symbol typing first
        let symbol = self
            .remap_typevalue_symbol_to_type_space(module, profile, symbol)
            .map_err(AnalyzeError::from)?;

        Ok(Type::Reference {
            symbol,
            static_arguments,
        })
    }

    pub(crate) fn infer_member_of_type(
        &self,
        module: &Module,
        profile: ProfileId,
        node_id: LocalNodeIdAny,
        symbols: &SymbolTable,
        receiver_ty: &Type,
        member_key: &StaticKey,
        lookup_mode: MemberLookupMode,
        types: &mut TypeTable,
        visited: &mut Vec<GlobalSymbolId>,
    ) -> AnalyzeResult<Option<LocalTypeId>> {
        match receiver_ty {
            // object type: look up field directly
            Type::Object {
                fields,
                call_signatures,
                ..
            } => {
                // check explicit object fields first
                if let Some(field_ty) =
                    self.member_type_from_fields(fields, member_key, node_id, types)
                {
                    return Ok(Some(field_ty));
                }

                // override bind/call/apply signatures based on strictness policy
                let options = self.analyze_context_options_for_module(module.id);
                if !call_signatures.is_empty()
                    && let Some(synthetic) = self.bind_call_apply_member_type(
                        node_id,
                        receiver_ty,
                        member_key,
                        options.strict_bind_call_apply,
                        types,
                    )
                {
                    return Ok(Some(synthetic));
                }

                // fall back to implicit Object members
                let Some(reference_ty) = self.well_known_type(profile, receiver_ty, types) else {
                    return Ok(None);
                };
                let reference_ty = self.normalize_well_known_reference_for_member_inference(
                    module,
                    profile,
                    reference_ty,
                )?;
                self.infer_member_of_type(
                    module,
                    profile,
                    node_id,
                    symbols,
                    &reference_ty,
                    member_key,
                    lookup_mode,
                    types,
                    visited,
                )
            }

            // value type: unwrap to the underlying type
            Type::Value { .. } => {
                let Some(reference_ty) = self.well_known_type(profile, receiver_ty, types) else {
                    return Ok(None);
                };
                let reference_ty = self.normalize_well_known_reference_for_member_inference(
                    module,
                    profile,
                    reference_ty,
                )?;
                self.infer_member_of_type(
                    module,
                    profile,
                    node_id,
                    symbols,
                    &reference_ty,
                    member_key,
                    lookup_mode,
                    types,
                    visited,
                )
            }

            // reference to a nominal type: expand alias arguments before member lookup
            Type::Reference {
                symbol,
                static_arguments,
            } => {
                if static_arguments.is_some() && matches!(symbol.ty(), SymbolType::TypeAlias) {
                    let mut normalize_visited = Vec::new();
                    if let Some(expanded_id) = self.normalize_type_alias_reference_with_arguments(
                        module,
                        profile,
                        node_id,
                        *symbol,
                        static_arguments.as_deref().unwrap_or(&[]),
                        symbols,
                        types,
                        NormalizationMode::Assign,
                        RelationMode::ASSIGN,
                        &mut normalize_visited,
                    ) {
                        let expanded_ty = types.get_type(expanded_id).clone();
                        return self.infer_member_of_type(
                            module,
                            profile,
                            node_id,
                            symbols,
                            &expanded_ty,
                            member_key,
                            lookup_mode,
                            types,
                            visited,
                        );
                    }
                }

                match lookup_mode {
                    MemberLookupMode::Instance | MemberLookupMode::Any => self
                        .infer_member_of_symbol(
                            module,
                            profile,
                            node_id,
                            symbols,
                            *symbol,
                            member_key,
                            lookup_mode,
                            types,
                            visited,
                        ),
                    MemberLookupMode::Value => {
                        let Some(value_ty_id) = types.get_value_type_id(*symbol) else {
                            return Ok(None);
                        };
                        let value_ty = types.get_type(value_ty_id).clone();
                        self.infer_member_of_type(
                            module,
                            profile,
                            node_id,
                            symbols,
                            &value_ty,
                            member_key,
                            lookup_mode,
                            types,
                            visited,
                        )
                    }
                }
            }

            // array like types: fall back to well known Array members
            Type::Array { .. } | Type::ArraySized { .. } | Type::Tuple { .. } => {
                let Some(reference_ty) = self.well_known_type(profile, receiver_ty, types) else {
                    return Ok(None);
                };
                let reference_ty = self.normalize_well_known_reference_for_member_inference(
                    module,
                    profile,
                    reference_ty,
                )?;
                self.infer_member_of_type(
                    module,
                    profile,
                    node_id,
                    symbols,
                    &reference_ty,
                    member_key,
                    lookup_mode,
                    types,
                    visited,
                )
            }

            // unary wrappers: unwrap before resolving members
            Type::Unary { right, .. }
            | Type::ValueOf { right, .. }
            | Type::ReferenceOf { right, .. }
            | Type::PointerOf { right, .. } => {
                let inner_ty = types.get_type(*right).clone();
                self.infer_member_of_type(
                    module,
                    profile,
                    node_id,
                    symbols,
                    &inner_ty,
                    member_key,
                    lookup_mode,
                    types,
                    visited,
                )
            }

            // union type: require all elements to have the field, return union of field types
            Type::Union { elements } => {
                let element_ids = elements.clone();
                let mut field_types: Vec<LocalTypeId> = Vec::new();
                for element_id in element_ids {
                    let element_ty = types.get_type(element_id).clone();
                    if let Some(field_ty) = self.infer_member_of_type(
                        module,
                        profile,
                        node_id,
                        symbols,
                        &element_ty,
                        member_key,
                        lookup_mode,
                        types,
                        visited,
                    )? {
                        field_types.push(field_ty);
                    } else {
                        return Ok(None);
                    }
                }
                // if all field types are the same, return that type
                // otherwise, return a union of the field types
                if field_types.is_empty() {
                    Ok(None)
                } else if field_types.len() == 1 {
                    Ok(Some(field_types[0]))
                } else {
                    // check if all types are identical
                    let first = field_types[0];
                    if field_types.iter().all(|&t| t == first) {
                        Ok(Some(first))
                    } else {
                        Ok(Some(types.insert_type_from_any(
                            Type::Union {
                                elements: field_types,
                            },
                            node_id,
                        )))
                    }
                }
            }

            // intersection type: first match wins
            Type::Intersection { elements } => {
                let element_ids = elements.clone();
                for element_id in element_ids {
                    let element_ty = types.get_type(element_id).clone();
                    if let Some(field_ty) = self.infer_member_of_type(
                        module,
                        profile,
                        node_id,
                        symbols,
                        &element_ty,
                        member_key,
                        lookup_mode,
                        types,
                        visited,
                    )? {
                        return Ok(Some(field_ty));
                    }
                }
                Ok(None)
            }

            Type::Function { .. } => {
                let options = self.analyze_context_options_for_module(module.id);

                // override bind/call/apply signatures based on strictness policy
                if let Some(synthetic) = self.bind_call_apply_member_type(
                    node_id,
                    receiver_ty,
                    member_key,
                    options.strict_bind_call_apply,
                    types,
                ) {
                    return Ok(Some(synthetic));
                }

                let Some(reference_ty) = self.well_known_type(profile, receiver_ty, types) else {
                    return Ok(None);
                };
                let reference_ty = self.normalize_well_known_reference_for_member_inference(
                    module,
                    profile,
                    reference_ty,
                )?;
                self.infer_member_of_type(
                    module,
                    profile,
                    node_id,
                    symbols,
                    &reference_ty,
                    member_key,
                    lookup_mode,
                    types,
                    visited,
                )
            }

            Type::TypeLiteral { .. } => {
                let Some(reference_ty) = self.well_known_type(profile, receiver_ty, types) else {
                    return Ok(None);
                };
                let reference_ty = self.normalize_well_known_reference_for_member_inference(
                    module,
                    profile,
                    reference_ty,
                )?;
                self.infer_member_of_type(
                    module,
                    profile,
                    node_id,
                    symbols,
                    &reference_ty,
                    member_key,
                    lookup_mode,
                    types,
                    visited,
                )
            }

            _ => Ok(None),
        }
    }

    /// Build a member type from fields that share the same key.
    fn member_type_from_fields(
        &self,
        fields: &[TypeField],
        member_key: &StaticKey,
        node_id: LocalNodeIdAny,
        types: &mut TypeTable,
    ) -> Option<LocalTypeId> {
        let mut matching = Vec::new();
        for field in fields {
            if field.key.matches(member_key) {
                matching.push(field.ty);
            }
        }

        if matching.is_empty() {
            return None;
        }

        if matching.len() == 1 {
            return Some(matching[0]);
        }

        let all_functions = matching
            .iter()
            .all(|ty_id| matches!(types.get_type(*ty_id), Type::Function { .. }));
        if all_functions {
            let overload_set = Type::Object {
                fields: Vec::new(),
                call_signatures: matching,
                construct_signatures: Vec::new(),
                index_signatures: Vec::new(),
            };
            return Some(types.insert_type_from_any(overload_set, node_id));
        }

        let source_type_id = matching[0];
        Some(self.union_type_from_list(matching, source_type_id, types))
    }

    /// Build bind/call/apply member types for callable receivers.
    fn bind_call_apply_member_type(
        &self,
        node_id: LocalNodeIdAny,
        receiver_ty: &Type,
        member_key: &StaticKey,
        is_strict_bind_call_apply: bool,
        types: &mut TypeTable,
    ) -> Option<LocalTypeId> {
        let member_name = member_key.name()?;

        // only override call/apply/bind with strict signatures
        let member_name = self.program.strings.get(member_name);
        let member_name = member_name.as_ref();
        if member_name != "call" && member_name != "apply" && member_name != "bind" {
            return None;
        }

        let mut member_signatures = Vec::new();
        match receiver_ty {
            Type::Function { .. } => {
                if let Some(signature_id) = self.bind_call_apply_signature_from_type(
                    node_id,
                    receiver_ty,
                    member_name,
                    is_strict_bind_call_apply,
                    types,
                ) {
                    member_signatures.push(signature_id);
                }
            }
            Type::Object {
                call_signatures, ..
            } => {
                for signature_id in call_signatures {
                    let signature_ty = types.get_type(*signature_id).clone();
                    if let Some(member_signature_id) = self.bind_call_apply_signature_from_type(
                        node_id,
                        &signature_ty,
                        member_name,
                        is_strict_bind_call_apply,
                        types,
                    ) {
                        member_signatures.push(member_signature_id);
                    }
                }
            }
            _ => return None,
        }

        if member_signatures.is_empty() {
            return None;
        }

        if member_signatures.len() == 1 {
            return Some(member_signatures[0]);
        }

        let overload_set = Type::Object {
            fields: Vec::new(),
            call_signatures: member_signatures,
            construct_signatures: Vec::new(),
            index_signatures: Vec::new(),
        };
        Some(types.insert_type_from_any(overload_set, node_id))
    }

    fn bind_call_apply_signature_from_type(
        &self,
        node_id: LocalNodeIdAny,
        receiver_ty: &Type,
        member_name: &str,
        is_strict_bind_call_apply: bool,
        types: &mut TypeTable,
    ) -> Option<LocalTypeId> {
        let Type::Function {
            static_parameters,
            this_parameter,
            dynamic_parameters,
            return_type,
            ..
        } = receiver_ty
        else {
            return None;
        };

        // resolve the strict this argument type
        let strict_this_arg = if let Some(this_parameter) = this_parameter {
            *this_parameter
        } else {
            let ty = Type::TypeLiteral {
                value: TypeLiteral::Unknown,
            };
            types.insert_type_from_any(ty, node_id)
        };
        let non_strict_arg = {
            let ty = Type::TypeLiteral {
                value: TypeLiteral::Unknown,
            };
            types.insert_type_from_any(ty, node_id)
        };

        // use strict or permissive bind/call/apply argument typing
        let this_arg = if is_strict_bind_call_apply {
            strict_this_arg
        } else {
            non_strict_arg
        };
        let call_parameters = if is_strict_bind_call_apply {
            dynamic_parameters.clone()
        } else {
            vec![non_strict_arg]
        };

        // build shared function metadata
        let asynchrony = Asynchrony::Sync;
        let cardinality = FunctionCardinality::Scalar;

        // build the strict member signature
        let member_ty = match member_name {
            "call" => {
                // call(thisArg, ...args) -> return_type
                let mut params = Vec::with_capacity(call_parameters.len() + 1);
                params.push(this_arg);
                params.extend_from_slice(&call_parameters);

                Type::Function {
                    asynchrony,
                    cardinality,
                    static_parameters: static_parameters.clone(),
                    this_parameter: None,
                    dynamic_parameters: params,
                    return_type: *return_type,
                }
            }
            "apply" => {
                // apply(thisArg, argsTuple) -> return_type
                let tuple_elements = call_parameters
                    .iter()
                    .map(|ty| TypeElement::new(*ty))
                    .collect();
                let tuple_ty = Type::Tuple {
                    elements: tuple_elements,
                    is_readonly: false,
                };
                let tuple_ty_id = types.insert_type_from_any(tuple_ty, node_id);

                Type::Function {
                    asynchrony,
                    cardinality,
                    static_parameters: static_parameters.clone(),
                    this_parameter: None,
                    dynamic_parameters: vec![this_arg, tuple_ty_id],
                    return_type: *return_type,
                }
            }
            "bind" => {
                // bind(thisArg, ...args) -> bound function
                let mut params = Vec::with_capacity(call_parameters.len() + 1);
                params.push(this_arg);
                params.extend_from_slice(&call_parameters);

                let bound_function = Type::Function {
                    asynchrony,
                    cardinality,
                    static_parameters: static_parameters.clone(),
                    this_parameter: None,
                    dynamic_parameters: call_parameters,
                    return_type: *return_type,
                };
                let bound_function_id = types.insert_type_from_any(bound_function, node_id);

                Type::Function {
                    asynchrony,
                    cardinality,
                    static_parameters: static_parameters.clone(),
                    this_parameter: None,
                    dynamic_parameters: params,
                    return_type: Some(bound_function_id),
                }
            }
            _ => return None,
        };

        Some(types.insert_type_from_any(member_ty, node_id))
    }

    /// Infer the index signature value type for a member key.
    pub(crate) fn resolve_index_signature_value_type_for_key(
        &self,
        module: &Module,
        profile: ProfileId,
        node_id: LocalNodeIdAny,
        symbols: &SymbolTable,
        receiver_ty: &Type,
        member_key: &StaticKey,
        types: &mut TypeTable,
        visited: &mut Vec<GlobalSymbolId>,
    ) -> Option<LocalTypeId> {
        match receiver_ty {
            Type::Object {
                index_signatures, ..
            } => self.index_signature_value_type_for_key(index_signatures, member_key, types),
            Type::Value { value } => {
                let value_ty = types.get_type(*value).clone();
                self.resolve_index_signature_value_type_for_key(
                    module, profile, node_id, symbols, &value_ty, member_key, types, visited,
                )
            }
            Type::Reference { symbol, .. } => self.resolve_index_signature_value_type_for_symbol(
                module, profile, node_id, symbols, *symbol, member_key, types, visited,
            ),
            Type::Unary { right, .. }
            | Type::ValueOf { right, .. }
            | Type::ReferenceOf { right, .. }
            | Type::PointerOf { right, .. } => {
                let inner_ty = types.get_type(*right).clone();
                self.resolve_index_signature_value_type_for_key(
                    module, profile, node_id, symbols, &inner_ty, member_key, types, visited,
                )
            }
            Type::Union { elements } => {
                let mut value_types = Vec::new();
                for element_id in elements.clone() {
                    let element_ty = types.get_type(element_id).clone();
                    if let Some(value_ty) = self.resolve_index_signature_value_type_for_key(
                        module,
                        profile,
                        node_id,
                        symbols,
                        &element_ty,
                        member_key,
                        types,
                        visited,
                    ) {
                        value_types.push(value_ty);
                    } else {
                        return None;
                    }
                }
                match value_types.len() {
                    0 => None,
                    1 => Some(value_types[0]),
                    _ => {
                        let source_type_id = value_types[0];
                        Some(self.union_type_from_list(value_types, source_type_id, types))
                    }
                }
            }
            Type::Intersection { elements } => {
                for element_id in elements.clone() {
                    let element_ty = types.get_type(element_id).clone();
                    if let Some(value_ty) = self.resolve_index_signature_value_type_for_key(
                        module,
                        profile,
                        node_id,
                        symbols,
                        &element_ty,
                        member_key,
                        types,
                        visited,
                    ) {
                        return Some(value_ty);
                    }
                }
                None
            }
            _ => None,
        }
    }

    /// Infer member type for a nominal type symbol, traversing lineage and extensions.
    fn infer_member_of_symbol(
        &self,
        module: &Module,
        profile: ProfileId,
        node_id: LocalNodeIdAny,
        symbols: &SymbolTable,
        symbol: GlobalSymbolId,
        member_key: &StaticKey,
        lookup_mode: MemberLookupMode,
        types: &mut TypeTable,
        visited: &mut Vec<GlobalSymbolId>,
    ) -> AnalyzeResult<Option<LocalTypeId>> {
        // cycle detection: if we've already visited this symbol, stop
        // NOTE #Suspicious: should we really just return None for already visited symbol types?
        if visited.contains(&symbol) {
            return Ok(None);
        }
        visited.push(symbol);

        // resolve instance types before walking members and extensions
        self.resolve_instance_type_for_symbol(module, profile, node_id, symbol, types)?;

        // step 1: look up in the type's own instance type
        if let Some(ty_id) =
            self.apparent_instance_type(module, profile, node_id, symbol, symbols, types)
        {
            let ty = types.get_type(ty_id).clone();
            if let Some(member_ty) = self.infer_member_of_type(
                module,
                profile,
                node_id,
                symbols,
                &ty,
                member_key,
                lookup_mode,
                types,
                visited,
            )? {
                return Ok(Some(member_ty));
            }
        }

        // step 2: traverse lineage (extends, implements, embedded)
        let lineage = types.get_lineage_for_symbol(symbol).cloned();
        if let Some(lineage) = lineage {
            // check parent type (extends)
            if let Some(extends) = lineage.extends
                && let Some(member_ty) = self.infer_member_of_symbol(
                    module,
                    profile,
                    node_id,
                    symbols,
                    extends,
                    member_key,
                    lookup_mode,
                    types,
                    visited,
                )?
            {
                return Ok(Some(member_ty));
            }

            // check implemented interfaces
            for implements in &lineage.implements {
                if let Some(member_ty) = self.infer_member_of_symbol(
                    module,
                    profile,
                    node_id,
                    symbols,
                    *implements,
                    member_key,
                    lookup_mode,
                    types,
                    visited,
                )? {
                    return Ok(Some(member_ty));
                }
            }

            // check embedded types
            for embedded in &lineage.embedded {
                if let Some(member_ty) = self.infer_member_of_symbol(
                    module,
                    profile,
                    node_id,
                    symbols,
                    *embedded,
                    member_key,
                    lookup_mode,
                    types,
                    visited,
                )? {
                    return Ok(Some(member_ty));
                }
            }
        }

        // step 3: check visible extensions
        let extension_symbols =
            self.visible_extension_symbols_for_target(module, profile, symbols, types, symbol)?;
        for extension_symbol in extension_symbols {
            let Some(extension) =
                self.extension_for_symbol_in_module(module, profile, extension_symbol, types)?
            else {
                continue;
            };
            if !self.is_extension_visible(module, &extension) {
                continue;
            }

            // ensure the extension instance type is available
            if let Some(ty_id) = self.apparent_instance_type(
                module,
                profile,
                node_id,
                extension_symbol,
                symbols,
                types,
            ) {
                let ty = types.get_type(ty_id).clone();
                if let Type::Object { fields, .. } = ty
                    && let Some(field_ty) =
                        self.member_type_from_fields(&fields, member_key, node_id, types)
                {
                    return Ok(Some(field_ty));
                }
            }
        }

        Ok(None)
    }

    /// Infer the index signature value type for a symbol.
    fn resolve_index_signature_value_type_for_symbol(
        &self,
        module: &Module,
        profile: ProfileId,
        node_id: LocalNodeIdAny,
        symbols: &SymbolTable,
        symbol: GlobalSymbolId,
        member_key: &StaticKey,
        types: &mut TypeTable,
        visited: &mut Vec<GlobalSymbolId>,
    ) -> Option<LocalTypeId> {
        if visited.contains(&symbol) {
            return None;
        }
        visited.push(symbol);

        // step 1: look up in the type's own instance type
        if let Some(ty_id) =
            self.apparent_instance_type(module, profile, node_id, symbol, symbols, types)
        {
            let ty = types.get_type(ty_id).clone();
            if let Some(value_ty) = self.resolve_index_signature_value_type_for_key(
                module, profile, node_id, symbols, &ty, member_key, types, visited,
            ) {
                return Some(value_ty);
            }
        }

        // step 2: traverse lineage (extends, implements, embedded)
        if let Some(lineage) = types.get_lineage_for_symbol(symbol).cloned() {
            if let Some(extends) = lineage.extends
                && let Some(value_ty) = self.resolve_index_signature_value_type_for_symbol(
                    module, profile, node_id, symbols, extends, member_key, types, visited,
                )
            {
                return Some(value_ty);
            }

            for implements in &lineage.implements {
                if let Some(value_ty) = self.resolve_index_signature_value_type_for_symbol(
                    module,
                    profile,
                    node_id,
                    symbols,
                    *implements,
                    member_key,
                    types,
                    visited,
                ) {
                    return Some(value_ty);
                }
            }

            for embedded in &lineage.embedded {
                if let Some(value_ty) = self.resolve_index_signature_value_type_for_symbol(
                    module, profile, node_id, symbols, *embedded, member_key, types, visited,
                ) {
                    return Some(value_ty);
                }
            }
        }

        // step 3: check visible extensions
        let extension_symbols = match self
            .visible_extension_symbols_for_target(module, profile, symbols, types, symbol)
        {
            Ok(symbols) => symbols,
            Err(AnalyzeError::Yield { .. }) => return None,
            Err(error) => {
                self.error(error);
                return None;
            }
        };
        for extension_symbol in extension_symbols {
            let extension =
                match self.extension_for_symbol_in_module(module, profile, extension_symbol, types)
                {
                    Ok(extension) => extension,
                    Err(AnalyzeError::Yield { .. }) => return None,
                    Err(error) => {
                        self.error(error);
                        return None;
                    }
                };
            let Some(extension) = extension else {
                continue;
            };
            if !self.is_extension_visible(module, &extension) {
                continue;
            }
            if let Some(ty_id) = self.apparent_instance_type(
                module,
                profile,
                node_id,
                extension_symbol,
                symbols,
                types,
            ) {
                let ty = types.get_type(ty_id).clone();
                if let Some(value_ty) = self.resolve_index_signature_value_type_for_key(
                    module, profile, node_id, symbols, &ty, member_key, types, visited,
                ) {
                    return Some(value_ty);
                }
            }
        }

        None
    }

    /// Get the idnex signature value type for a member key.
    fn index_signature_value_type_for_key(
        &self,
        index_signatures: &[TypeIndexSignature],
        member_key: &StaticKey,
        types: &mut TypeTable,
    ) -> Option<LocalTypeId> {
        let key_kind = index_key_kind_for_member(member_key);
        let mut value_types = Vec::new();

        for signature in index_signatures {
            let signature_kind = index_key_kind_for_type(signature.key_type, types);
            if index_key_kinds_compatible_for_access(signature_kind, key_kind) {
                value_types.push(signature.value_type);
            }
        }

        match value_types.len() {
            0 => None,
            1 => Some(value_types[0]),
            _ => {
                let source_type_id = value_types[0];
                Some(self.union_type_from_list(value_types, source_type_id, types))
            }
        }
    }

    /// Check if an extension is visible from the given module:
    /// Native: Extension in same module as target type, always visible wherever type is used.
    /// Anonymous: Extension on foreign type, only visible in the file where it is declared.
    /// Named: Extension on foreign type, must be explicitly imported to use.
    pub(crate) fn is_extension_visible(&self, module: &Module, extension: &Extension) -> bool {
        match extension.kind {
            ExtensionKind::Inherent => true,
            ExtensionKind::Local => extension.symbol.module_id == module.id,
            ExtensionKind::Nominal => true,
        }
    }
}
