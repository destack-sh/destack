use super::*;
use crate::analyze::common::TypeContext;

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Check object type assignability (structural subtyping).
    pub(super) fn is_object_type_assignable(
        &self,
        ctx: &mut AssignContext<'_>,
        target_fields: &[TypeField],
        target_call_signatures: &[LocalTypeId],
        target_construct_signatures: &[LocalTypeId],
        target_index_signatures: &[TypeIndexSignature],
        source_fields: &[TypeField],
        source_call_signatures: &[LocalTypeId],
        source_construct_signatures: &[LocalTypeId],
        source_index_signatures: &[TypeIndexSignature],
    ) -> Assignability {
        if self.is_object_fields_assignable(ctx, target_fields, source_fields)
            == Assignability::NotAssignable
        {
            return Assignability::NotAssignable;
        }

        if !self.is_signature_set_assignable(ctx, target_call_signatures, source_call_signatures) {
            return Assignability::NotAssignable;
        }

        if !self.is_signature_set_assignable(
            ctx,
            target_construct_signatures,
            source_construct_signatures,
        ) {
            return Assignability::NotAssignable;
        }

        if !self.is_index_signatures_assignable(
            ctx,
            target_index_signatures,
            source_index_signatures,
            source_fields,
        ) {
            return Assignability::NotAssignable;
        }

        Assignability::Assignable
    }

    /// Check callable object assignability from a function type.
    pub(super) fn is_object_assignable_from_function(
        &self,
        ctx: &mut AssignContext<'_>,
        target_fields: &[TypeField],
        target_call_signatures: &[LocalTypeId],
        target_construct_signatures: &[LocalTypeId],
        target_index_signatures: &[TypeIndexSignature],
        source_params: &[LocalTypeId],
        source_this: &Option<LocalTypeId>,
        source_return: &Option<LocalTypeId>,
    ) -> Assignability {
        if self.is_object_fields_assignable(ctx, target_fields, &[]) == Assignability::NotAssignable
        {
            return Assignability::NotAssignable;
        }

        if !self.is_call_signatures_assignable_from_function(
            ctx,
            target_call_signatures,
            source_params,
            source_this,
            source_return,
        ) {
            return Assignability::NotAssignable;
        }

        if !self.is_signature_set_assignable(ctx, target_construct_signatures, &[]) {
            return Assignability::NotAssignable;
        }

        if !self.is_index_signatures_assignable(ctx, target_index_signatures, &[], &[]) {
            return Assignability::NotAssignable;
        }

        Assignability::Assignable
    }

    /// Resolve a record-like index signature for the target symbol.
    pub(super) fn record_like_index_signature_for_target(
        &self,
        ctx: &mut TypeContext<'_>,
        target_symbol: GlobalSymbolId,
        static_arguments: Option<&[StaticArgument]>,
        target_id: LocalTypeId,
    ) -> Option<TypeIndexSignature> {
        // resolve record and map symbols
        let record_symbol =
            self.get_well_known_type_symbol(ctx.profile, WellKnownSymbol::Record)?;
        let map_symbol = self.get_well_known_type_symbol(ctx.profile, WellKnownSymbol::Map)?;
        let is_record_like = target_symbol == record_symbol || target_symbol == map_symbol;
        if !is_record_like {
            return None;
        }

        // resolve key and value arguments when available
        let static_arguments = static_arguments.unwrap_or(&[]);
        let unknown_literal_type_id = ctx
            .types
            .intern_literal_type(target_id, TypeLiteral::Unknown);
        let key_type_id = static_arguments
            .first()
            .and_then(|argument| {
                self.record_like_type_id_for_static_argument(argument, ctx.types, target_id)
            })
            .unwrap_or(unknown_literal_type_id);
        let value_type_id = static_arguments
            .get(1)
            .and_then(|argument| {
                self.record_like_type_id_for_static_argument(argument, ctx.types, target_id)
            })
            .unwrap_or(unknown_literal_type_id);

        // emit a synthetic index signature
        let name = self.repository.strings.intern("key");
        Some(TypeIndexSignature {
            name,
            key_type: key_type_id,
            value_type: value_type_id,
            is_optional: false,
            is_readonly: false,
        })
    }

    /// Resolve a static type argument to a type id.
    pub(super) fn record_like_type_id_for_static_argument(
        &self,
        argument: &StaticArgument,
        types: &mut TypeTable,
        target_id: LocalTypeId,
    ) -> Option<LocalTypeId> {
        // resolve type expressions to concrete ids when possible
        match argument {
            StaticArgument::Unevaluated { .. } => {
                Some(types.intern_literal_type(target_id, TypeLiteral::Unknown))
            }
            StaticArgument::Evaluated { value, .. } => match value {
                StaticExpression::Type { ty } => Some(*ty),
                StaticExpression::TypeLiteral { value } => {
                    Some(types.intern_literal_type(target_id, value.clone()))
                }
                _ => Some(types.intern_literal_type(target_id, TypeLiteral::Unknown)),
            },
        }
    }

    /// Read an object type shape for record-like assignability.
    pub(super) fn record_like_source_object_parts(
        &self,
        source: &Type,
        types: &TypeTable,
    ) -> Option<RecordLikeObjectParts> {
        // map record-like sources to object type parts
        match source {
            Type::Object {
                fields,
                call_signatures,
                construct_signatures,
                index_signatures,
            } => Some((
                fields.clone(),
                call_signatures.clone(),
                construct_signatures.clone(),
                index_signatures.clone(),
            )),
            Type::Reference { symbol, .. }
                if matches!(
                    symbol.ty(),
                    SymbolType::Interface
                        | SymbolType::Class
                        | SymbolType::Struct
                        | SymbolType::Newtype
                ) =>
            {
                if symbol.ty() == SymbolType::Newtype {
                    let alias_id = types.get_alias_target_type_id(*symbol)?;
                    let alias_type = types.get_type(alias_id);
                    return self.record_like_source_object_parts(alias_type, types);
                }

                let instance_id = types.get_instance_type_id(*symbol)?;
                let instance = types.get_type(instance_id);
                let Type::Object {
                    fields,
                    call_signatures,
                    construct_signatures,
                    index_signatures,
                } = instance
                else {
                    return None;
                };

                Some((
                    fields.clone(),
                    call_signatures.clone(),
                    construct_signatures.clone(),
                    index_signatures.clone(),
                ))
            }
            _ => None,
        }
    }

    /// Read one record-like object surface from a reference type.
    pub(super) fn record_like_object_parts_for_reference(
        &self,
        ctx: &mut TypeContext<'_>,
        source_id: LocalNodeIdAny,
        symbol: GlobalSymbolId,
        static_arguments: Option<&[StaticArgument]>,
    ) -> Option<RecordLikeObjectParts> {
        let instance_id = self.specialized_instance_type_for_reference(
            &mut ctx.reborrow(),
            source_id,
            symbol,
            static_arguments,
        )?;
        let instance = ctx.types.get_type(instance_id);

        self.record_like_source_object_parts(instance, ctx.types)
    }

    /// Check function assignability from callable object signatures.
    pub(super) fn is_function_assignable_from_object(
        &self,
        ctx: &mut AssignContext<'_>,
        assignment_anchor: LocalNodeIdAny,
        target_params: &[LocalTypeId],
        target_this: &Option<LocalTypeId>,
        target_return: &Option<LocalTypeId>,
        source_call_signatures: &[LocalTypeId],
    ) -> Assignability {
        if source_call_signatures.is_empty() {
            return Assignability::NotAssignable;
        }

        for source_signature in source_call_signatures {
            let signature = ctx.types.get_type(*source_signature).clone();
            let Type::Function {
                dynamic_parameters: source_params,
                this_parameter: source_this,
                return_type: source_return,
                ..
            } = signature
            else {
                continue;
            };

            if self
                .is_function_type_assignable(
                    ctx,
                    assignment_anchor,
                    target_params,
                    target_this,
                    target_return,
                    &source_params,
                    &source_this,
                    &source_return,
                )
                .is_assignable()
            {
                return Assignability::Assignable;
            }
        }

        Assignability::NotAssignable
    }

    /// Check object field assignability (structural subtyping).
    pub(super) fn is_object_fields_assignable(
        &self,
        ctx: &mut AssignContext<'_>,
        target_fields: &[TypeField],
        source_fields: &[TypeField],
    ) -> Assignability {
        // for each target field, find matching source field
        for target_field in target_fields {
            let undefined_ty_id = ctx.types.insert_type_from_type(
                Type::TypeLiteral {
                    value: TypeLiteral::Undefined,
                },
                target_field.ty,
            );
            let source_field = source_fields
                .iter()
                .find(|field| field.key.matches(&target_field.key));

            match source_field {
                Some(source_field) => {
                    let target_field_ty_id = self.prepare_assignability_type(
                        &mut ctx.type_context_reborrow(),
                        target_field.ty,
                    );
                    let source_field_ty_id = self.prepare_assignability_type(
                        &mut ctx.type_context_reborrow(),
                        source_field.ty,
                    );
                    let target_field_ty = ctx.types.get_type(target_field_ty_id);
                    let source_field_ty = ctx.types.get_type(source_field_ty_id);
                    let fields_are_method_like = matches!(target_field_ty, Type::Function { .. })
                        && matches!(source_field_ty, Type::Function { .. });

                    // reject readonly source fields when the target is mutable
                    if source_field.is_readonly
                        && !target_field.is_readonly
                        && !fields_are_method_like
                    {
                        return Assignability::NotAssignable;
                    }

                    // reject optional source fields when the target is required
                    if !target_field.is_optional && source_field.is_optional {
                        return Assignability::NotAssignable;
                    }

                    let mut is_assignable = self
                        .is_type_assignable(
                            &mut ctx.type_context_reborrow(),
                            target_field_ty_id,
                            source_field_ty_id,
                        )
                        .is_assignable();

                    if !ctx.options.exact_optional_property_types && target_field.is_optional {
                        is_assignable |= self
                            .is_type_assignable(
                                &mut ctx.type_context_reborrow(),
                                undefined_ty_id,
                                source_field_ty_id,
                            )
                            .is_assignable();
                    }

                    if !ctx.options.exact_optional_property_types && source_field.is_optional {
                        let undefined_assignable = self
                            .is_type_assignable(
                                &mut ctx.type_context_reborrow(),
                                target_field_ty_id,
                                undefined_ty_id,
                            )
                            .is_assignable();
                        is_assignable &= undefined_assignable;
                    }

                    if !is_assignable {
                        return Assignability::NotAssignable;
                    }
                }
                None => {
                    // field missing: only okay if target field is optional
                    if !target_field.is_optional {
                        return Assignability::NotAssignable;
                    }
                }
            }
        }

        Assignability::Assignable
    }
}
