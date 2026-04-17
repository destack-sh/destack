use super::super::expression::call::SignatureStaticResolutionContext;
use super::*;
use crate::analyze::common::InferContext;
use destack_dir::{GenericArgument, ResolvedSignature};

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Resolve and commit the inferred type for a resolved member symbol.
    pub(crate) fn resolve_member_access_type_for_symbol(
        &self,
        ctx: &mut InferContext<'_>,
        expression_id: LocalNodeId<Expression>,
        member_symbol: Option<GlobalSymbolId>,
        receiver_symbol: Option<GlobalSymbolId>,
        receiver_arguments: &[StaticArgument],
        member_ty_id: LocalTypeId,
        generic_arguments: Option<&[LocalNodeId<GenericArgument>]>,
        substitutions: &HashMap<GlobalSymbolId, LocalTypeId>,
    ) -> AnalyzeResult<ResolvedMemberAccessType> {
        // apply receiver and extension substitutions
        let member_ty_id = if substitutions.is_empty() {
            member_ty_id
        } else {
            let mut cache = HashMap::new();
            self.substitute_static_parameters(member_ty_id, substitutions, ctx.types, &mut cache)
        };

        // rewrite owner scoped associated aliases after substitution
        let member_ty_id = if let Some(member_symbol) = member_symbol {
            if let Some(owner_symbol) =
                self.query_owner_symbol_for_member_symbol(ctx.module_symbol_view(), member_symbol)?
            {
                self.rewrite_associated_aliases_for_owner(
                    &mut ctx.type_context_reborrow(),
                    expression_id.into_any(),
                    owner_symbol,
                    receiver_symbol,
                    receiver_arguments,
                    substitutions,
                    member_ty_id,
                )
            } else {
                member_ty_id
            }
        } else {
            member_ty_id
        };

        // instantiate member static arguments when present
        let (resolved_member_ty_id, resolved_static_arguments, resolved_static_parameter_symbols) =
            if let Some(generic_argument_ids) = generic_arguments {
                let (
                    resolved_member_ty_id,
                    resolved_static_arguments,
                    resolved_static_parameter_symbols,
                ) = self.apply_member_static_arguments(
                    &mut ctx.reborrow(),
                    expression_id,
                    member_symbol,
                    member_ty_id,
                    generic_argument_ids,
                    substitutions,
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
            generic_arguments: resolved_static_arguments,
            generic_parameter_symbols: resolved_static_parameter_symbols,
        })
    }

    /// Apply static arguments to a member type when the receiver uses `receiver.member<...>`.
    pub(crate) fn apply_member_static_arguments(
        &self,
        ctx: &mut InferContext<'_>,
        expression_id: LocalNodeId<Expression>,
        member_symbol: Option<GlobalSymbolId>,
        member_ty_id: LocalTypeId,
        generic_argument_ids: &[LocalNodeId<GenericArgument>],
        substitutions: &HashMap<GlobalSymbolId, LocalTypeId>,
    ) -> AnalyzeResult<(LocalTypeId, Vec<StaticArgument>, Vec<GlobalSymbolId>)> {
        match ctx.types.get_type(member_ty_id).clone() {
            Type::Function { .. } => {
                let instantiated = self.instantiate_member_signature_for_static_arguments(
                    &mut ctx.reborrow(),
                    expression_id,
                    member_symbol,
                    member_ty_id,
                    generic_argument_ids,
                    substitutions,
                )?;
                Ok((
                    instantiated.type_id,
                    instantiated.generic_arguments,
                    instantiated.generic_parameter_symbols,
                ))
            }
            Type::Object {
                call_signatures, ..
            } => {
                let mut resolved_signatures = Vec::new();
                let mut resolved_static_arguments = Vec::new();
                let mut resolved_static_parameter_symbols = Vec::new();

                for signature_id in call_signatures {
                    if !matches!(ctx.types.get_type(signature_id), Type::Function { .. }) {
                        continue;
                    }

                    let instantiated = self.instantiate_member_signature_for_static_arguments(
                        &mut ctx.reborrow(),
                        expression_id,
                        member_symbol,
                        signature_id,
                        generic_argument_ids,
                        substitutions,
                    )?;
                    if resolved_static_arguments.is_empty() {
                        resolved_static_arguments = instantiated.generic_arguments.clone();
                        resolved_static_parameter_symbols =
                            instantiated.generic_parameter_symbols.clone();
                    }
                    resolved_signatures.push(instantiated.type_id);
                }

                if resolved_signatures.is_empty() {
                    self.error_invalid_member_static_arguments(
                        ctx.module,
                        ctx.profile,
                        expression_id,
                    );
                    Ok((member_ty_id, Vec::new(), Vec::new()))
                } else {
                    let overload_set = Type::Object {
                        fields: Vec::new(),
                        call_signatures: resolved_signatures,
                        construct_signatures: Vec::new(),
                        index_signatures: Vec::new(),
                    };
                    let overload_set_ty_id =
                        ctx.types.insert_type_from(overload_set, expression_id);
                    Ok((
                        overload_set_ty_id,
                        resolved_static_arguments,
                        resolved_static_parameter_symbols,
                    ))
                }
            }
            Type::Union { elements } => {
                let mut resolved_elements = Vec::with_capacity(elements.len());
                let mut resolved_static_arguments = Vec::new();
                let mut resolved_static_parameter_symbols = Vec::new();

                for element_id in elements {
                    let (
                        resolved_element_id,
                        element_static_arguments,
                        element_static_parameter_symbols,
                    ) = self.apply_member_static_arguments(
                        &mut ctx.reborrow(),
                        expression_id,
                        member_symbol,
                        element_id,
                        generic_argument_ids,
                        substitutions,
                    )?;

                    if resolved_static_arguments.is_empty() {
                        resolved_static_arguments = element_static_arguments;
                        resolved_static_parameter_symbols = element_static_parameter_symbols;
                    }

                    resolved_elements.push(resolved_element_id);
                }

                let resolved_union_ty_id =
                    self.union_type_from_list(resolved_elements, member_ty_id, ctx.types);

                Ok((
                    resolved_union_ty_id,
                    resolved_static_arguments,
                    resolved_static_parameter_symbols,
                ))
            }
            Type::Intersection { elements } => {
                let mut resolved_elements = Vec::with_capacity(elements.len());
                let mut resolved_static_arguments = Vec::new();
                let mut resolved_static_parameter_symbols = Vec::new();

                for element_id in elements {
                    let (
                        resolved_element_id,
                        element_static_arguments,
                        element_static_parameter_symbols,
                    ) = self.apply_member_static_arguments(
                        &mut ctx.reborrow(),
                        expression_id,
                        member_symbol,
                        element_id,
                        generic_argument_ids,
                        substitutions,
                    )?;

                    if resolved_static_arguments.is_empty() {
                        resolved_static_arguments = element_static_arguments;
                        resolved_static_parameter_symbols = element_static_parameter_symbols;
                    }

                    resolved_elements.push(resolved_element_id);
                }

                let resolved_intersection_ty_id =
                    self.intersection_type_from_list(resolved_elements, member_ty_id, ctx.types);

                Ok((
                    resolved_intersection_ty_id,
                    resolved_static_arguments,
                    resolved_static_parameter_symbols,
                ))
            }
            _ => {
                self.error_invalid_member_static_arguments(ctx.module, ctx.profile, expression_id);
                Ok((member_ty_id, Vec::new(), Vec::new()))
            }
        }
    }

    /// Instantiate one function signature for member static argument checking.
    pub(crate) fn instantiate_member_signature_for_static_arguments(
        &self,
        ctx: &mut InferContext<'_>,
        expression_id: LocalNodeId<Expression>,
        member_symbol: Option<GlobalSymbolId>,
        signature_ty_id: LocalTypeId,
        generic_argument_ids: &[LocalNodeId<GenericArgument>],
        substitutions: &HashMap<GlobalSymbolId, LocalTypeId>,
    ) -> AnalyzeResult<InstantiatedMemberSignature> {
        let Type::Function {
            asynchrony,
            cardinality,
            generic_parameters,
            this_parameter,
            parameters,
            return_type,
        } = ctx.types.get_type(signature_ty_id).clone()
        else {
            self.error_invalid_member_static_arguments(ctx.module, ctx.profile, expression_id);
            return Ok(InstantiatedMemberSignature {
                type_id: signature_ty_id,
                generic_arguments: Vec::new(),
                generic_parameter_symbols: Vec::new(),
            });
        };

        if generic_parameters.is_empty()
            && let Some(member_symbol) = member_symbol
        {
            let parameter_symbols = self
                .collect_static_parameter_symbols(ctx.type_view(), member_symbol)
                .unwrap_or_default();
            if !parameter_symbols.is_empty() {
                return Err(AnalyzeError::Internal {
                    message: format!(
                        "missing signature static parameters for generic member {member_symbol:?}"
                    ),
                });
            }
        }

        let generic_parameter_symbols =
            self.generic_parameter_symbols_for_type_ids(&generic_parameters, ctx.types);

        let resolved = self
            .resolve_function_static_arguments(
                &mut ctx.reborrow(),
                SignatureStaticResolutionContext {
                    node_id: expression_id.into_any(),
                    owner_symbol: member_symbol,
                    generic_argument_ids: Some(generic_argument_ids),
                    prefilled_static_arguments: None,
                    bound_substitutions: (!substitutions.is_empty()).then_some(substitutions),
                    argument_ids: None,
                    generic_parameter_type_ids: &generic_parameters,
                    parameter_type_ids: &parameters,
                    return_type,
                    expected_return_type: None,
                    mode: SignatureResolutionMode::Check,
                    allow_missing_value_arguments: false,
                },
            )?
            .unwrap_or(ResolvedSignature {
                parameters,
                return_type,
                generic_arguments: Vec::new(),
            });

        let (resolved_this_parameter, resolved_parameters, resolved_return_type) = self
            .substitute_member_signature_parts(
                this_parameter,
                resolved.parameters,
                resolved.return_type,
                substitutions,
                ctx.types,
            );

        let instantiated_fn = Type::Function {
            asynchrony,
            cardinality,
            generic_parameters: Vec::new(),
            this_parameter: resolved_this_parameter,
            parameters: resolved_parameters,
            return_type: resolved_return_type,
        };
        let instantiated_type_id = ctx.types.insert_type_from(instantiated_fn, expression_id);

        Ok(InstantiatedMemberSignature {
            type_id: instantiated_type_id,
            generic_arguments: resolved.generic_arguments,
            generic_parameter_symbols,
        })
    }

    /// Apply receiver substitutions to resolved function signature parts.
    pub(crate) fn substitute_member_signature_parts(
        &self,
        this_parameter: Option<LocalTypeId>,
        parameters: Vec<LocalTypeId>,
        return_type: Option<LocalTypeId>,
        substitutions: &HashMap<GlobalSymbolId, LocalTypeId>,
        types: &mut TypeTable,
    ) -> (Option<LocalTypeId>, Vec<LocalTypeId>, Option<LocalTypeId>) {
        if substitutions.is_empty() {
            return (this_parameter, parameters, return_type);
        }

        let mut cache = HashMap::new();
        let resolved_this_parameter = this_parameter.map(|parameter| {
            self.substitute_static_parameters(parameter, substitutions, types, &mut cache)
        });
        let resolved_parameters = parameters
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
            resolved_parameters,
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
