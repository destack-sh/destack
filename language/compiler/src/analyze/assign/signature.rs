use super::*;

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Check assignability of signature sets (target signatures must be matched).
    pub(super) fn is_signature_set_assignable(
        &self,
        module: &Module,
        profile: ProfileId,
        symbols: &SymbolTable,
        target_signatures: &[LocalTypeId],
        source_signatures: &[LocalTypeId],
        types: &mut TypeTable,
        options: &AnalyzeOptions,
    ) -> bool {
        if target_signatures.is_empty() {
            return true;
        }
        for target_signature in target_signatures {
            let mut matched = false;
            for source_signature in source_signatures {
                if self
                    .is_type_assignable(
                        module,
                        profile,
                        symbols,
                        *target_signature,
                        *source_signature,
                        types,
                        options,
                    )
                    .is_assignable()
                {
                    matched = true;
                    break;
                }
            }
            if !matched {
                return false;
            }
        }
        true
    }

    /// Check assignability of call signatures against a single function source.
    pub(super) fn is_call_signatures_assignable_from_function(
        &self,
        module: &Module,
        profile: ProfileId,
        symbols: &SymbolTable,
        target_signatures: &[LocalTypeId],
        source_params: &[LocalTypeId],
        source_this: &Option<LocalTypeId>,
        source_return: &Option<LocalTypeId>,
        types: &mut TypeTable,
        options: &AnalyzeOptions,
    ) -> bool {
        if target_signatures.is_empty() {
            return true;
        }

        for target_signature in target_signatures {
            let signature = types.get_type(*target_signature).clone();
            let Type::Function {
                dynamic_parameters: target_params,
                this_parameter: target_this,
                return_type: target_return,
                ..
            } = signature
            else {
                return false;
            };

            if !self
                .is_function_type_assignable(
                    module,
                    profile,
                    symbols,
                    types.get_type_source(*target_signature),
                    &target_params,
                    &target_this,
                    &target_return,
                    source_params,
                    source_this,
                    source_return,
                    types,
                    options,
                )
                .is_assignable()
            {
                return false;
            }
        }

        true
    }

    /// Check assignability of index signatures.
    pub(super) fn is_index_signatures_assignable(
        &self,
        module: &Module,
        profile: ProfileId,
        symbols: &SymbolTable,
        target_signatures: &[TypeIndexSignature],
        source_signatures: &[TypeIndexSignature],
        source_fields: &[TypeField],
        types: &mut TypeTable,
        options: &AnalyzeOptions,
    ) -> bool {
        if target_signatures.is_empty() {
            return true;
        }
        for target_signature in target_signatures {
            let mut matched = false;
            let mut has_matching_key_kind = false;
            for source_signature in source_signatures {
                let target_kind = index_key_kind_for_type(target_signature.key_type, types);
                let source_kind = index_key_kind_for_type(source_signature.key_type, types);
                if !index_key_kinds_compatible_for_assignability(target_kind, source_kind) {
                    continue;
                }
                has_matching_key_kind = true;
                if self.is_index_signature_assignable(
                    module,
                    profile,
                    symbols,
                    target_signature,
                    source_signature,
                    types,
                    options,
                ) {
                    matched = true;
                    break;
                }
            }
            if matched {
                continue;
            }
            if has_matching_key_kind {
                return false;
            }
            if !self.are_fields_assignable_to_index_signature(
                module,
                profile,
                symbols,
                target_signature,
                source_fields,
                types,
                options,
            ) {
                return false;
            }
        }
        true
    }

    /// Check assignability for a single index signature.
    pub(super) fn is_index_signature_assignable(
        &self,
        module: &Module,
        profile: ProfileId,
        symbols: &SymbolTable,
        target_signature: &TypeIndexSignature,
        source_signature: &TypeIndexSignature,
        types: &mut TypeTable,
        options: &AnalyzeOptions,
    ) -> bool {
        let target_kind = index_key_kind_for_type(target_signature.key_type, types);
        let source_kind = index_key_kind_for_type(source_signature.key_type, types);
        if !index_key_kinds_compatible_for_assignability(target_kind, source_kind) {
            return false;
        }

        if !self
            .is_type_assignable(
                module,
                profile,
                symbols,
                target_signature.value_type,
                source_signature.value_type,
                types,
                options,
            )
            .is_assignable()
        {
            return false;
        }

        true
    }

    /// Check if any source field violates a target index signature.
    pub(super) fn are_fields_assignable_to_index_signature(
        &self,
        module: &Module,
        profile: ProfileId,
        symbols: &SymbolTable,
        target_signature: &TypeIndexSignature,
        source_fields: &[TypeField],
        types: &mut TypeTable,
        options: &AnalyzeOptions,
    ) -> bool {
        let key_kind = index_key_kind_for_type(target_signature.key_type, types);
        for field in source_fields {
            if !field_key_matches_index_kind(&field.key, key_kind) {
                continue;
            }
            if !self.is_field_type_assignable_to_index_signature(
                module,
                profile,
                symbols,
                target_signature.value_type,
                field,
                types,
                options,
            ) {
                return false;
            }
        }
        true
    }

    /// Check if a field type is compatible with an index signature value type.
    pub(super) fn is_field_type_assignable_to_index_signature(
        &self,
        module: &Module,
        profile: ProfileId,
        symbols: &SymbolTable,
        value_type: LocalTypeId,
        field: &TypeField,
        types: &mut TypeTable,
        options: &AnalyzeOptions,
    ) -> bool {
        let undefined_ty_id = types.insert_type_from_type(
            Type::TypeLiteral {
                value: TypeLiteral::Undefined,
            },
            value_type,
        );

        let mut is_assignable = self
            .is_type_assignable(
                module, profile, symbols, value_type, field.ty, types, options,
            )
            .is_assignable();

        if !options.exact_optional_property_types && field.is_optional {
            let undefined_assignable = self
                .is_type_assignable(
                    module,
                    profile,
                    symbols,
                    value_type,
                    undefined_ty_id,
                    types,
                    options,
                )
                .is_assignable();
            is_assignable &= undefined_assignable;
        }

        is_assignable
    }

    /// Check function type assignability (contravariant params, covariant return).
    pub(super) fn is_function_type_assignable(
        &self,
        module: &Module,
        profile: ProfileId,
        symbols: &SymbolTable,
        assignment_anchor: LocalNodeIdAny,
        target_params: &[LocalTypeId],
        target_this: &Option<LocalTypeId>,
        target_return: &Option<LocalTypeId>,
        source_params: &[LocalTypeId],
        source_this: &Option<LocalTypeId>,
        source_return: &Option<LocalTypeId>,
        types: &mut TypeTable,
        options: &AnalyzeOptions,
    ) -> Assignability {
        // track unsound variance diagnostics
        let mut reported_unsound_variance = false;

        // this parameter: contravariant when strict, bivariant otherwise
        if let (Some(target_this), Some(source_this)) = (target_this, source_this) {
            let strict_assignable = self
                .is_type_assignable(
                    module,
                    profile,
                    symbols,
                    *source_this,
                    *target_this,
                    types,
                    options,
                )
                .is_assignable();
            let loose_assignable = self
                .is_type_assignable(
                    module,
                    profile,
                    symbols,
                    *target_this,
                    *source_this,
                    types,
                    options,
                )
                .is_assignable();
            if options.strict_function_types {
                if !strict_assignable {
                    return Assignability::NotAssignable;
                }
            } else if !strict_assignable && !loose_assignable {
                return Assignability::NotAssignable;
            } else if options.no_unsound_variance
                && !strict_assignable
                && loose_assignable
                && !reported_unsound_variance
            {
                self.report_unsound_variance(module, profile, assignment_anchor, options);
                reported_unsound_variance = true;
            }
        }

        // handle variadic top parameters like (...args: unknown[])
        if let Some(rest_element) = self.variadic_top_parameter_type(target_params, types) {
            for source_param in source_params {
                let strict_assignable = self
                    .is_type_assignable(
                        module,
                        profile,
                        symbols,
                        rest_element,
                        *source_param,
                        types,
                        options,
                    )
                    .is_assignable();
                let loose_assignable = self
                    .is_type_assignable(
                        module,
                        profile,
                        symbols,
                        *source_param,
                        rest_element,
                        types,
                        options,
                    )
                    .is_assignable();
                if options.strict_function_types {
                    if !strict_assignable {
                        return Assignability::NotAssignable;
                    }
                } else if !strict_assignable && !loose_assignable {
                    return Assignability::NotAssignable;
                } else if options.no_unsound_variance
                    && !strict_assignable
                    && loose_assignable
                    && !reported_unsound_variance
                {
                    self.report_unsound_variance(module, profile, assignment_anchor, options);
                    reported_unsound_variance = true;
                }
            }
        } else {
            // source must not require more parameters than target provides
            if source_params.len() > target_params.len() {
                return Assignability::NotAssignable;
            }

            // parameters: contravariant when strict, bivariant otherwise
            for (target_param, source_param) in target_params.iter().zip(source_params.iter()) {
                let strict_assignable = self
                    .is_type_assignable(
                        module,
                        profile,
                        symbols,
                        *source_param,
                        *target_param,
                        types,
                        options,
                    )
                    .is_assignable();
                let loose_assignable = self
                    .is_type_assignable(
                        module,
                        profile,
                        symbols,
                        *target_param,
                        *source_param,
                        types,
                        options,
                    )
                    .is_assignable();
                if options.strict_function_types {
                    if !strict_assignable {
                        return Assignability::NotAssignable;
                    }
                } else if !strict_assignable && !loose_assignable {
                    return Assignability::NotAssignable;
                } else if options.no_unsound_variance
                    && !strict_assignable
                    && loose_assignable
                    && !reported_unsound_variance
                {
                    self.report_unsound_variance(module, profile, assignment_anchor, options);
                    reported_unsound_variance = true;
                }
            }
        }

        // return type: covariant (target return must be assignable from source return)
        match (target_return, source_return) {
            // callback targets returning void accept any source return type
            (Some(target_ret), Some(_))
                if matches!(
                    types.get_type(types.unwrap_value_type_id(*target_ret)),
                    Type::TypeLiteral {
                        value: TypeLiteral::Void,
                    }
                ) =>
            {
                Assignability::Assignable
            }
            (Some(target_ret), Some(source_ret)) => self.is_type_assignable(
                module,
                profile,
                symbols,
                *target_ret,
                *source_ret,
                types,
                options,
            ),
            (None, _) => Assignability::Assignable,
            (Some(_), None) => Assignability::NotAssignable,
        }
    }

    /// Return the element type for variadic top parameters like (...args: unknown[]).
    pub(super) fn variadic_top_parameter_type(
        &self,
        target_params: &[LocalTypeId],
        types: &TypeTable,
    ) -> Option<LocalTypeId> {
        // only treat a single array parameter with any/unknown element as variadic
        if target_params.len() != 1 {
            return None;
        }

        let param_type = types.get_type(target_params[0]);
        let element = match param_type {
            Type::Array { element, .. } => *element,
            Type::ArraySized { element, .. } => Some(*element),
            _ => None,
        };
        let element = element?;

        match types.get_type(element) {
            Type::TypeLiteral {
                value: TypeLiteral::Any | TypeLiteral::Unknown,
            } => Some(element),
            _ => None,
        }
    }
}
