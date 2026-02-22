use super::*;

impl Compiler {
    /// Resolve and commit the inferred type for a resolved member symbol.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn resolve_member_access_type_for_symbol(
        &self,
        module: &Module,
        expression_id: LocalNodeId<Expression>,
        member_symbol: Option<GlobalSymbolId>,
        member_ty_id: LocalTypeId,
        static_arguments: Option<&[LocalNodeId<Argument>]>,
        substitutions: &HashMap<GlobalSymbolId, LocalTypeId>,
        profile: ProfileId,
        options: &AnalyzeOptions,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        infer: &mut InferTable,
    ) -> AnalyzeResult<ResolvedMemberAccessType> {
        // apply receiver and extension substitutions
        let member_ty_id = if substitutions.is_empty() {
            member_ty_id
        } else {
            let mut cache = HashMap::new();
            self.substitute_static_parameters(member_ty_id, substitutions, types, &mut cache)
        };

        // rewrite owner scoped associated aliases after substitution
        let member_ty_id = if let Some(member_symbol) = member_symbol {
            if let Some(owner_symbol) =
                self.query_owner_symbol_for_member_symbol(module, profile, member_symbol, symbols)?
            {
                self.rewrite_associated_aliases_for_owner(
                    module,
                    profile,
                    expression_id.into_any(),
                    owner_symbol,
                    substitutions,
                    member_ty_id,
                    tree,
                    symbols,
                    types,
                )
            } else {
                member_ty_id
            }
        } else {
            member_ty_id
        };

        // instantiate member static arguments when present
        let (resolved_member_ty_id, resolved_static_arguments, resolved_static_parameter_symbols) =
            if let Some(static_argument_ids) = static_arguments {
                let (
                    resolved_member_ty_id,
                    resolved_static_arguments,
                    resolved_static_parameter_symbols,
                ) = self.apply_member_static_arguments(
                    module,
                    expression_id,
                    member_symbol,
                    member_ty_id,
                    static_argument_ids,
                    substitutions,
                    profile,
                    options,
                    tree,
                    symbols,
                    types,
                    infer,
                )?;
                (
                    resolved_member_ty_id,
                    resolved_static_arguments,
                    resolved_static_parameter_symbols,
                )
            } else {
                (member_ty_id, Vec::new(), Vec::new())
            };

        Ok(ResolvedMemberAccessType {
            type_id: resolved_member_ty_id,
            static_arguments: resolved_static_arguments,
            static_parameter_symbols: resolved_static_parameter_symbols,
        })
    }

    /// Apply static arguments to a member type when the receiver uses `receiver.member<...>`.
    pub(crate) fn apply_member_static_arguments(
        &self,
        module: &Module,
        expression_id: LocalNodeId<Expression>,
        member_symbol: Option<GlobalSymbolId>,
        member_ty_id: LocalTypeId,
        static_argument_ids: &[LocalNodeId<Argument>],
        substitutions: &HashMap<GlobalSymbolId, LocalTypeId>,
        profile: ProfileId,
        options: &AnalyzeOptions,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        infer: &mut InferTable,
    ) -> AnalyzeResult<(LocalTypeId, Vec<StaticArgument>, Vec<GlobalSymbolId>)> {
        match types.get_type(member_ty_id).clone() {
            Type::Function { .. } => {
                let instantiated = self.instantiate_member_signature_for_static_arguments(
                    module,
                    expression_id,
                    member_symbol,
                    member_ty_id,
                    static_argument_ids,
                    substitutions,
                    profile,
                    options,
                    tree,
                    symbols,
                    types,
                    infer,
                )?;
                Ok((
                    instantiated.type_id,
                    instantiated.static_arguments,
                    instantiated.static_parameter_symbols,
                ))
            }
            Type::Object {
                call_signatures, ..
            } => {
                let mut resolved_signatures = Vec::new();
                let mut resolved_static_arguments = Vec::new();
                let mut resolved_static_parameter_symbols = Vec::new();

                for signature_id in call_signatures {
                    if !matches!(types.get_type(signature_id), Type::Function { .. }) {
                        continue;
                    }

                    let instantiated = self.instantiate_member_signature_for_static_arguments(
                        module,
                        expression_id,
                        member_symbol,
                        signature_id,
                        static_argument_ids,
                        substitutions,
                        profile,
                        options,
                        tree,
                        symbols,
                        types,
                        infer,
                    )?;
                    if resolved_static_arguments.is_empty() {
                        resolved_static_arguments = instantiated.static_arguments.clone();
                        resolved_static_parameter_symbols =
                            instantiated.static_parameter_symbols.clone();
                    }
                    resolved_signatures.push(instantiated.type_id);
                }

                if resolved_signatures.is_empty() {
                    self.error_invalid_member_static_arguments(module, profile, expression_id);
                    Ok((member_ty_id, Vec::new(), Vec::new()))
                } else {
                    let overload_set = Type::Object {
                        fields: Vec::new(),
                        call_signatures: resolved_signatures,
                        construct_signatures: Vec::new(),
                        index_signatures: Vec::new(),
                    };
                    let overload_set_ty_id = types.insert_type_from(overload_set, expression_id);
                    Ok((
                        overload_set_ty_id,
                        resolved_static_arguments,
                        resolved_static_parameter_symbols,
                    ))
                }
            }
            _ => {
                self.error_invalid_member_static_arguments(module, profile, expression_id);
                Ok((member_ty_id, Vec::new(), Vec::new()))
            }
        }
    }

    /// Instantiate one function signature for member static argument checking.
    pub(crate) fn instantiate_member_signature_for_static_arguments(
        &self,
        module: &Module,
        expression_id: LocalNodeId<Expression>,
        member_symbol: Option<GlobalSymbolId>,
        signature_ty_id: LocalTypeId,
        static_argument_ids: &[LocalNodeId<Argument>],
        substitutions: &HashMap<GlobalSymbolId, LocalTypeId>,
        profile: ProfileId,
        options: &AnalyzeOptions,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        infer: &mut InferTable,
    ) -> AnalyzeResult<InstantiatedMemberSignature> {
        let Type::Function {
            asynchrony,
            cardinality,
            static_parameters,
            this_parameter,
            dynamic_parameters,
            return_type,
        } = types.get_type(signature_ty_id).clone()
        else {
            self.error_invalid_member_static_arguments(module, profile, expression_id);
            return Ok(InstantiatedMemberSignature {
                type_id: signature_ty_id,
                static_arguments: Vec::new(),
                static_parameter_symbols: Vec::new(),
            });
        };

        if static_parameters.is_empty() {
            if let Some(member_symbol) = member_symbol {
                let parameter_symbols = self
                    .collect_static_parameter_symbols(
                        module,
                        member_symbol,
                        profile,
                        tree,
                        symbols,
                        types,
                    )
                    .unwrap_or_default();
                if !parameter_symbols.is_empty() {
                    return Err(AnalyzeError::Internal {
                        message: format!(
                            "missing signature static parameters for generic member {member_symbol:?}"
                        ),
                    });
                }
            }
        }

        let static_parameter_symbols =
            self.static_parameter_symbols_for_type_ids(&static_parameters, types);

        let resolved = self.resolve_function_signature(
            module,
            expression_id.into_any(),
            member_symbol,
            Some(static_argument_ids),
            None,
            (!substitutions.is_empty()).then_some(substitutions),
            None,
            &static_parameters,
            &dynamic_parameters,
            return_type,
            None,
            SignatureResolutionMode::Check,
            false,
            profile,
            options,
            tree,
            symbols,
            types,
            infer,
        )?;

        let (resolved_this_parameter, resolved_dynamic_parameters, resolved_return_type) = self
            .substitute_member_signature_parts(
                this_parameter,
                resolved.dynamic_parameters,
                resolved.return_type,
                substitutions,
                types,
            );

        let instantiated_fn = Type::Function {
            asynchrony,
            cardinality,
            static_parameters: Vec::new(),
            this_parameter: resolved_this_parameter,
            dynamic_parameters: resolved_dynamic_parameters,
            return_type: resolved_return_type,
        };
        let instantiated_type_id = types.insert_type_from(instantiated_fn, expression_id);

        Ok(InstantiatedMemberSignature {
            type_id: instantiated_type_id,
            static_arguments: resolved.static_arguments,
            static_parameter_symbols,
        })
    }

    /// Apply receiver substitutions to resolved function signature parts.
    pub(crate) fn substitute_member_signature_parts(
        &self,
        this_parameter: Option<LocalTypeId>,
        dynamic_parameters: Vec<LocalTypeId>,
        return_type: Option<LocalTypeId>,
        substitutions: &HashMap<GlobalSymbolId, LocalTypeId>,
        types: &mut TypeTable,
    ) -> (Option<LocalTypeId>, Vec<LocalTypeId>, Option<LocalTypeId>) {
        if substitutions.is_empty() {
            return (this_parameter, dynamic_parameters, return_type);
        }

        let mut cache = HashMap::new();
        let resolved_this_parameter = this_parameter.map(|parameter| {
            self.substitute_static_parameters(parameter, substitutions, types, &mut cache)
        });
        let resolved_dynamic_parameters = dynamic_parameters
            .iter()
            .map(|parameter| {
                self.substitute_static_parameters(*parameter, substitutions, types, &mut cache)
            })
            .collect::<Vec<_>>();
        let resolved_return_type = return_type.map(|return_type| {
            self.substitute_static_parameters(return_type, substitutions, types, &mut cache)
        });

        (
            resolved_this_parameter,
            resolved_dynamic_parameters,
            resolved_return_type,
        )
    }

    /// Report invalid static arguments on a member access.
    pub(crate) fn error_invalid_member_static_arguments(
        &self,
        module: &Module,
        profile: ProfileId,
        expression_id: LocalNodeId<Expression>,
    ) {
        self.error(AnalyzeError::InvalidStaticArgument {
            node: expression_id
                .into_global_any(module.id)
                .into_anchored(Some(profile)),
            message: "member does not accept static arguments".to_string(),
        });
    }
}
