use super::SignatureResolutionMode;
use super::argument::InheritedStaticArguments;
use crate::analyze::common::RelationMode;
use crate::timing::tags;
use crate::{AnalyzeError, AnalyzeOptions, AnalyzeResult, Compiler, InferContext};
use destack_base::StringId;
use destack_builtin::LanguageSymbol;
use destack_dir::{
    Argument, BindingAnchor, BindingModifier, Declaration, Expression, GlobalSymbolId, InferTable,
    LocalNodeId, LocalNodeIdAny, LocalTypeId, Member, NodeTree, NodeType, NormalizationMode,
    StaticArgument, StaticKey, SymbolSpace, SymbolTable, SymbolType, Type, TypeLiteral, TypeTable,
    Visibility,
};
use destack_source::ModuleId;
use destack_workspace::{Module, ModuleSource, ProfileId};
use std::collections::{HashMap, HashSet};

/// Describe the resolution outcome for a member lookup.
#[derive(Debug, Clone)]
pub(super) enum MemberResolution {
    /// No resolution is recorded for this lookup.
    None,
    /// A single target symbol is selected.
    Static { symbol: GlobalSymbolId },
    /// Multiple target symbols must be dispatched at runtime.
    Dynamic {
        candidates: Vec<MemberResolutionCandidate>,
    },
    /// A nominal lookup failed, but some candidates exist.
    Unresolved,
}

/// Candidate member target for dynamic union dispatch.
#[derive(Debug, Clone)]
pub(super) struct MemberResolutionCandidate {
    /// The receiver type to dispatch against.
    pub(super) receiver_ty_id: LocalTypeId,
    /// The resolved member symbol for this receiver type.
    pub(super) symbol: GlobalSymbolId,
}

/// Resolved extension metadata for a member lookup.
#[derive(Debug, Clone)]
pub(super) struct ExtensionMemberContext {
    /// The resolved static arguments for the extension parameters.
    pub(super) arguments: Vec<StaticArgument>,
    /// The substitutions for extension type parameters.
    pub(super) substitutions: HashMap<GlobalSymbolId, LocalTypeId>,
}

/// Select which members are visible during lookup.
#[derive(Debug, Copy, Clone)]
pub(super) enum MemberLookupMode {
    /// Look up instance members only.
    Instance,
    /// Look up static members only.
    Value,
    /// Look up members without filtering.
    Any,
}

/// Visibility metadata for a member symbol.
#[derive(Debug, Clone)]
struct MemberVisibilityContext {
    /// The visibility of the member symbol.
    visibility: Visibility,
    /// The owner symbol of the member symbol.
    owner_symbol: GlobalSymbolId,
}

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Infer a member access expression.
    pub(super) fn infer_member_expression(
        &self,
        module: &Module,
        expression_id: LocalNodeId<Expression>,
        left_id: LocalNodeId<Expression>,
        member_name: StringId,
        static_arguments: Option<&[LocalNodeId<Argument>]>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        infer: &mut InferTable,
        ctx: &mut InferContext,
    ) -> AnalyzeResult<LocalTypeId> {
        let _timing = self.timing_scope(tags::ANALYZE_INFER_EXPRESSION_MEMBER);

        // optional chain receivers must unwrap maybe before member lookup
        let optional_chain =
            self.infer_optional_chain_receiver(module, left_id, tree, symbols, types, infer, ctx)?;
        let (left_id, left_ty_id, optional_chain_has_nullish) =
            if let Some(optional_chain) = optional_chain {
                let Some(receiver_ty_id) = optional_chain.receiver_ty_id else {
                    let ty = Type::TypeLiteral {
                        value: TypeLiteral::Undefined,
                    };
                    return Ok(types.insert_type_from(ty, expression_id));
                };
                (
                    optional_chain.receiver_id,
                    receiver_ty_id,
                    optional_chain.has_nullish,
                )
            } else {
                let left_ty_id =
                    self.infer_expression(module, left_id, tree, symbols, types, infer, ctx)?;
                (left_id, left_ty_id, false)
            };
        let left_ty_id = self.materialize_infer_type_for_check(
            module,
            ctx.profile,
            symbols,
            left_ty_id,
            infer,
            types,
            &ctx.options,
        );

        // normalize apparent types before member lookup
        let left_ty_id = self.normalize_apparent_type(
            module,
            ctx.profile,
            left_ty_id,
            symbols,
            types,
            NormalizationMode::Assign,
            RelationMode::ASSIGN,
        );
        let left_ty = types.get_type(left_ty_id).clone();
        let mut member_instance_id = None;
        let finish_result = |type_id: LocalTypeId, types: &mut TypeTable| {
            self.optional_chain_result_type(
                expression_id,
                type_id,
                optional_chain_has_nullish,
                types,
            )
        };

        // short circuit member access on any
        if matches!(
            left_ty,
            Type::TypeLiteral {
                value: TypeLiteral::Any,
            }
        ) {
            let type_id = self.any_member_access_type(expression_id, types);
            return Ok(finish_result(type_id, types));
        }

        // resolve the member key for lookup
        let member_key = StaticKey::Name(member_name);

        // handle enum field access early to preserve nominal enum types
        if let Some(enum_reference_id) = self.resolve_enum_field_access(
            module,
            expression_id,
            left_id,
            left_ty_id,
            &left_ty,
            &member_key,
            ctx.profile,
            tree,
            symbols,
            types,
        )? {
            return Ok(finish_result(enum_reference_id, types));
        }

        // ensure instance types are available for reference receivers
        self.ensure_reference_instance_types_for_type(
            module,
            ctx.profile,
            expression_id.into_any(),
            left_ty_id,
            types,
        )?;

        // inherit static arguments and substitutions from the receiver
        let inherited = self.resolve_inherited_static_arguments(
            module,
            ctx.profile,
            left_id.into_any(),
            Some(left_ty_id),
            &left_ty,
            &ctx.options,
            tree,
            symbols,
            types,
        )?;

        // resolve member dispatch for the left type
        let member_resolution = self.resolve_member_symbol_for_receiver(
            module,
            left_id,
            &left_ty,
            &member_key,
            ctx.profile,
            tree,
            symbols,
            types,
        )?;

        // reject implicit dynamic dispatch when configured
        if ctx.options.no_implicit_dynamic_dispatch
            && matches!(module.source, ModuleSource::User)
            && matches!(member_resolution, MemberResolution::Dynamic { .. })
        {
            self.error(AnalyzeError::ImplicitDynamicDispatchDisabled {
                node: expression_id
                    .into_global_any(module.id)
                    .into_anchored(Some(ctx.profile)),
            });
        }
        let member_symbol = match &member_resolution {
            MemberResolution::Static { symbol } => Some(*symbol),
            _ => None,
        };

        // enforce member visibility for resolved symbols
        match &member_resolution {
            MemberResolution::Dynamic { candidates } => {
                for candidate in candidates {
                    self.check_member_visibility(
                        module,
                        expression_id,
                        candidate.symbol,
                        candidate.receiver_ty_id,
                        ctx.profile,
                        tree,
                        symbols,
                        types,
                        ctx,
                    );
                }
            }
            _ => {
                if let Some(member_symbol) = member_symbol {
                    self.check_member_visibility(
                        module,
                        expression_id,
                        member_symbol,
                        left_ty_id,
                        ctx.profile,
                        tree,
                        symbols,
                        types,
                        ctx,
                    );
                }
            }
        }

        // resolve enum field member symbol
        let member_symbol = self.resolve_enum_field_member_symbol(
            module,
            left_id,
            &member_key,
            member_symbol,
            ctx.profile,
            tree,
            symbols,
            types,
        )?;
        let enum_field_value_ty_id =
            self.enum_field_value_type_for_symbol(module, symbols, member_symbol, types);

        // resolve extension substitutions for member symbols
        let extension_context = if let Some(member_symbol) = member_symbol {
            self.resolve_extension_member_context(
                module,
                ctx.profile,
                expression_id.into_any(),
                member_symbol,
                &inherited.arguments,
                &ctx.options,
                tree,
                symbols,
                types,
            )?
        } else {
            None
        };

        // merge inherited and extension substitutions
        let substitutions = self.merge_member_substitutions(&inherited, extension_context.as_ref());

        // decide how to filter member lookups for this receiver
        let lookup_mode = self.member_lookup_mode_for_receiver_expression(
            module,
            left_id,
            &left_ty,
            ctx.profile,
            tree,
            symbols,
        );

        // infer the member type
        let mut member_type_visited = Vec::new();
        let member_ty_id = self.infer_member_of_type(
            module,
            ctx.profile,
            expression_id.into_any(),
            symbols,
            &left_ty,
            &member_key,
            lookup_mode,
            types,
            &mut member_type_visited,
        )?;
        // prefer declared or inferred symbol types for resolved members
        let member_ty_id = if let Some(member_symbol) = member_symbol {
            let mut member_ty_id = match (types.get_value_type_id(member_symbol), member_ty_id) {
                (Some(value_ty_id), Some(member_ty_id)) => {
                    if self.is_infer_var_type(value_ty_id, types) {
                        Some(member_ty_id)
                    } else {
                        Some(value_ty_id)
                    }
                }
                (Some(value_ty_id), None) => Some(value_ty_id),
                (None, Some(member_ty_id)) => Some(member_ty_id),
                (None, None) => None,
            };

            // import remote member types when needed
            if member_ty_id.is_none() && member_symbol.module_id != module.id {
                let remote_ty_id = self.resolve_remote_symbol_value_type(
                    module,
                    ctx.profile,
                    expression_id.into_any(),
                    member_symbol,
                    ctx.is_surface_inference,
                    types,
                )?;
                member_ty_id = Some(remote_ty_id);
            }

            member_ty_id
        } else {
            member_ty_id
        };

        let has_member = member_ty_id.is_some();
        let resolved_member_ty_id = if let Some(enum_field_value_ty_id) = enum_field_value_ty_id {
            enum_field_value_ty_id
        } else if let Some(member_ty_id) = member_ty_id {
            let member_ty_id = if !substitutions.is_empty() {
                let mut cache = HashMap::new();
                self.substitute_static_parameters(member_ty_id, &substitutions, types, &mut cache)
            } else {
                member_ty_id
            };

            let resolved_member_ty_id = if let Some(static_argument_ids) = static_arguments {
                match types.get_type(member_ty_id).clone() {
                    Type::Function {
                        asynchrony,
                        cardinality,
                        static_parameters,
                        this_parameter,
                        dynamic_parameters,
                        return_type,
                    } => {
                        let resolved = self.resolve_function_signature(
                            module,
                            expression_id.into_any(),
                            member_symbol,
                            Some(static_argument_ids),
                            None,
                            (!substitutions.is_empty()).then_some(&substitutions),
                            None,
                            &static_parameters,
                            &dynamic_parameters,
                            return_type,
                            None,
                            SignatureResolutionMode::Checking,
                            false,
                            ctx.profile,
                            &ctx.options,
                            tree,
                            symbols,
                            types,
                            infer,
                        )?;

                        let resolved_this_parameter = if substitutions.is_empty() {
                            this_parameter
                        } else {
                            let mut cache = HashMap::new();
                            this_parameter.map(|parameter| {
                                self.substitute_static_parameters(
                                    parameter,
                                    &substitutions,
                                    types,
                                    &mut cache,
                                )
                            })
                        };
                        let (resolved_dynamic_parameters, resolved_return_type) =
                            if substitutions.is_empty() {
                                (resolved.dynamic_parameters, resolved.return_type)
                            } else {
                                let mut cache = HashMap::new();
                                let dynamic_parameters = resolved
                                    .dynamic_parameters
                                    .iter()
                                    .map(|parameter| {
                                        self.substitute_static_parameters(
                                            *parameter,
                                            &substitutions,
                                            types,
                                            &mut cache,
                                        )
                                    })
                                    .collect::<Vec<_>>();
                                let return_type = resolved.return_type.map(|return_type| {
                                    self.substitute_static_parameters(
                                        return_type,
                                        &substitutions,
                                        types,
                                        &mut cache,
                                    )
                                });
                                (dynamic_parameters, return_type)
                            };

                        let instantiated_fn = Type::Function {
                            asynchrony,
                            cardinality,
                            static_parameters: Vec::new(),
                            this_parameter: resolved_this_parameter,
                            dynamic_parameters: resolved_dynamic_parameters,
                            return_type: resolved_return_type,
                        };

                        if let Some(member_symbol) = member_symbol {
                            member_instance_id = self.register_member_instance_for_arguments(
                                module,
                                expression_id,
                                member_symbol,
                                &inherited,
                                extension_context.as_ref(),
                                &resolved.static_arguments,
                                types,
                            );
                        }

                        types.insert_type_from(instantiated_fn, expression_id)
                    }
                    Type::Object {
                        call_signatures, ..
                    } => {
                        let mut resolved_signatures = Vec::new();
                        let mut resolved_static_arguments = Vec::new();

                        for signature_id in call_signatures {
                            let Type::Function {
                                asynchrony,
                                cardinality,
                                static_parameters,
                                this_parameter,
                                dynamic_parameters,
                                return_type,
                            } = types.get_type(signature_id).clone()
                            else {
                                continue;
                            };

                            let resolved = self.resolve_function_signature(
                                module,
                                expression_id.into_any(),
                                member_symbol,
                                Some(static_argument_ids),
                                None,
                                (!substitutions.is_empty()).then_some(&substitutions),
                                None,
                                &static_parameters,
                                &dynamic_parameters,
                                return_type,
                                None,
                                SignatureResolutionMode::Checking,
                                false,
                                ctx.profile,
                                &ctx.options,
                                tree,
                                symbols,
                                types,
                                infer,
                            )?;
                            if resolved_static_arguments.is_empty() {
                                resolved_static_arguments = resolved.static_arguments.clone();
                            }

                            let resolved_this_parameter = if substitutions.is_empty() {
                                this_parameter
                            } else {
                                let mut cache = HashMap::new();
                                this_parameter.map(|parameter| {
                                    self.substitute_static_parameters(
                                        parameter,
                                        &substitutions,
                                        types,
                                        &mut cache,
                                    )
                                })
                            };
                            let (resolved_dynamic_parameters, resolved_return_type) =
                                if substitutions.is_empty() {
                                    (resolved.dynamic_parameters, resolved.return_type)
                                } else {
                                    let mut cache = HashMap::new();
                                    let dynamic_parameters = resolved
                                        .dynamic_parameters
                                        .iter()
                                        .map(|parameter| {
                                            self.substitute_static_parameters(
                                                *parameter,
                                                &substitutions,
                                                types,
                                                &mut cache,
                                            )
                                        })
                                        .collect::<Vec<_>>();
                                    let return_type = resolved.return_type.map(|return_type| {
                                        self.substitute_static_parameters(
                                            return_type,
                                            &substitutions,
                                            types,
                                            &mut cache,
                                        )
                                    });
                                    (dynamic_parameters, return_type)
                                };

                            let instantiated_fn = Type::Function {
                                asynchrony,
                                cardinality,
                                static_parameters: Vec::new(),
                                this_parameter: resolved_this_parameter,
                                dynamic_parameters: resolved_dynamic_parameters,
                                return_type: resolved_return_type,
                            };
                            let signature_ty_id =
                                types.insert_type_from(instantiated_fn, expression_id);
                            resolved_signatures.push(signature_ty_id);
                        }

                        if let Some(member_symbol) = member_symbol {
                            member_instance_id = self.register_member_instance_for_arguments(
                                module,
                                expression_id,
                                member_symbol,
                                &inherited,
                                extension_context.as_ref(),
                                &resolved_static_arguments,
                                types,
                            );
                        }

                        if resolved_signatures.is_empty() {
                            self.error(AnalyzeError::MissingType {
                                node: expression_id
                                    .into_global_any(module.id)
                                    .into_anchored(Some(ctx.profile)),
                            });
                            member_ty_id
                        } else {
                            let overload_set = Type::Object {
                                fields: Vec::new(),
                                call_signatures: resolved_signatures,
                                construct_signatures: Vec::new(),
                                index_signatures: Vec::new(),
                            };
                            types.insert_type_from(overload_set, expression_id)
                        }
                    }
                    _ => {
                        self.error(AnalyzeError::MissingType {
                            node: expression_id
                                .into_global_any(module.id)
                                .into_anchored(Some(ctx.profile)),
                        });
                        member_ty_id
                    }
                }
            } else {
                member_ty_id
            };

            // record member resolution when possible
            self.record_member_resolution(
                expression_id.into_global_any(module.id),
                Some(left_ty_id),
                &member_resolution,
                member_instance_id,
                None,
                has_member,
                types,
            );

            resolved_member_ty_id
        } else {
            let mut index_visited = Vec::new();
            let index_signature_ty_id = self.infer_index_signature_value_type_for_key(
                module,
                ctx.profile,
                expression_id.into_any(),
                symbols,
                &left_ty,
                &member_key,
                types,
                &mut index_visited,
            );

            if let Some(index_signature_ty_id) = index_signature_ty_id {
                let options = ctx.options;
                if options.no_property_access_from_index_signature
                    && !self.is_import_meta_chain_member(tree, left_id, "env")
                {
                    self.error(AnalyzeError::PropertyAccessFromIndexSignature {
                        node: expression_id
                            .into_global_any(module.id)
                            .into_anchored(Some(ctx.profile)),
                        receiver_ty: left_ty_id.into_global(module.id),
                        member_key,
                    });
                }

                self.record_member_resolution(
                    expression_id.into_global_any(module.id),
                    Some(left_ty_id),
                    &member_resolution,
                    member_instance_id,
                    None,
                    true,
                    types,
                );

                index_signature_ty_id
            } else {
                self.error(AnalyzeError::MissingMember {
                    node: expression_id
                        .into_global_any(module.id)
                        .into_anchored(Some(ctx.profile)),
                    receiver_ty: left_ty_id.into_global(module.id),
                    member_key,
                });

                // record unresolved member resolution
                self.record_member_resolution(
                    expression_id.into_global_any(module.id),
                    Some(left_ty_id),
                    &member_resolution,
                    member_instance_id,
                    None,
                    has_member,
                    types,
                );

                let ty = Type::TypeLiteral {
                    value: TypeLiteral::Unknown,
                };
                types.insert_type_from(ty, expression_id)
            }
        };

        let resolved_member_ty_id = {
            let mut cache = HashMap::new();
            self.substitute_this_type(resolved_member_ty_id, left_ty_id, types, &mut cache)
        };

        Ok(finish_result(resolved_member_ty_id, types))
    }

    /// Return the member access type for `any` receivers.
    fn any_member_access_type(
        &self,
        expression_id: LocalNodeId<Expression>,
        types: &mut TypeTable,
    ) -> LocalTypeId {
        // preserve any when the receiver is any
        let ty = Type::TypeLiteral {
            value: TypeLiteral::Any,
        };
        types.insert_type_from(ty, expression_id)
    }

    /// Resolve enum field member access when the receiver is an enum.
    fn resolve_enum_field_access(
        &self,
        module: &Module,
        expression_id: LocalNodeId<Expression>,
        left_id: LocalNodeId<Expression>,
        left_ty_id: LocalTypeId,
        left_ty: &Type,
        member_key: &StaticKey,
        profile: ProfileId,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> AnalyzeResult<Option<LocalTypeId>> {
        // select the enum symbol for the receiver
        let enum_symbol = self
            .enum_symbol_for_receiver_symbol(module, left_id, profile, tree, symbols)
            .or_else(|| self.enum_symbol_for_type(left_ty, types));
        let Some(enum_symbol) = enum_symbol else {
            return Ok(None);
        };

        // resolve the enum field symbol for the requested member key
        let enum_field_symbol = self.enum_field_symbol_for_member_key(
            module,
            enum_symbol,
            member_key,
            profile,
            tree,
            symbols,
        )?;
        let Some(enum_field_symbol) = enum_field_symbol else {
            return Ok(None);
        };

        // record the member resolution for the enum field
        let resolution = MemberResolution::Static {
            symbol: enum_field_symbol,
        };
        self.record_member_resolution(
            expression_id.into_global_any(module.id),
            Some(left_ty_id),
            &resolution,
            None,
            None,
            true,
            types,
        );

        // return the nominal enum reference type
        let enum_reference = Type::Reference {
            symbol: enum_symbol,
            static_arguments: None,
        };
        Ok(Some(types.insert_type_from(enum_reference, expression_id)))
    }

    /// Merge inherited and extension substitutions for member lookup.
    fn merge_member_substitutions(
        &self,
        inherited: &InheritedStaticArguments,
        extension_context: Option<&ExtensionMemberContext>,
    ) -> HashMap<GlobalSymbolId, LocalTypeId> {
        let mut substitutions = inherited.substitutions.clone();
        if let Some(context) = extension_context {
            for (symbol, ty_id) in &context.substitutions {
                substitutions.insert(*symbol, *ty_id);
            }
        }
        substitutions
    }

    /// Register instance arguments for a resolved member symbol.
    fn register_member_instance_for_arguments(
        &self,
        module: &Module,
        expression_id: LocalNodeId<Expression>,
        member_symbol: GlobalSymbolId,
        inherited: &InheritedStaticArguments,
        extension_context: Option<&ExtensionMemberContext>,
        resolved_arguments: &[StaticArgument],
        types: &mut TypeTable,
    ) -> Option<destack_dir::LocalInstanceId> {
        let mut instance_arguments = match extension_context {
            Some(context) => context.arguments.clone(),
            None => inherited.arguments.clone(),
        };
        instance_arguments.extend_from_slice(resolved_arguments);
        if instance_arguments.is_empty() {
            return None;
        }

        Some(self.register_instance_for_node(
            expression_id.into_global_any(module.id),
            member_symbol,
            instance_arguments,
            types,
        ))
    }

    /// Resolve the enum symbol that owns an enum field symbol.
    fn enum_symbol_for_enum_field_symbol(
        &self,
        module: &Module,
        symbols: &SymbolTable,
        member_symbol: GlobalSymbolId,
    ) -> Option<GlobalSymbolId> {
        // resolve the enum field symbol entry
        let (is_enum_field, scope_owner) = self.with_module_symbols_base_or_local(
            module,
            member_symbol.module_id,
            symbols,
            |_, owner_symbols| {
                let member_entry = owner_symbols.get_symbol(member_symbol.local_id);
                let scope = owner_symbols.get_scope_by_symbol(member_symbol.local_id);
                let is_enum_field = member_entry
                    .primary_declaration
                    .is_some_and(|declaration| declaration.local_id.ty == NodeType::EnumField);
                scope.owner_id.map(|owner_id| (is_enum_field, owner_id))
            },
        )?;

        // ensure the symbol is an enum field
        if !is_enum_field {
            return None;
        }

        // ensure the owning symbol is an enum
        if scope_owner.ty != SymbolType::Enum {
            return None;
        }

        Some(scope_owner.into_global(member_symbol.module_id))
    }

    /// Resolve enum field member symbols when the receiver is an enum reference.
    fn resolve_enum_field_member_symbol(
        &self,
        module: &Module,
        left_id: LocalNodeId<Expression>,
        member_key: &StaticKey,
        member_symbol: Option<GlobalSymbolId>,
        profile: ProfileId,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> AnalyzeResult<Option<GlobalSymbolId>> {
        // keep already resolved member symbols
        if member_symbol.is_some() {
            return Ok(member_symbol);
        }

        // only direct enum references can resolve enum field symbols
        let Some(left_symbol) =
            self.reference_symbol_for_expression(module, left_id, profile, tree, symbols)
        else {
            return Ok(None);
        };
        if left_symbol.ty() != SymbolType::Enum {
            return Ok(None);
        }

        // resolve the member symbol from the enum declaration
        let mut visited = Vec::new();
        self.resolve_member_symbol_for_symbol(
            module,
            left_symbol,
            member_key,
            MemberLookupMode::Value,
            profile,
            tree,
            symbols,
            types,
            &mut visited,
        )
    }

    /// Resolve the value type for an enum field symbol when possible.
    fn enum_field_value_type_for_symbol(
        &self,
        module: &Module,
        symbols: &SymbolTable,
        member_symbol: Option<GlobalSymbolId>,
        types: &TypeTable,
    ) -> Option<LocalTypeId> {
        let member_symbol = member_symbol?;
        self.enum_symbol_for_enum_field_symbol(module, symbols, member_symbol)?;
        types.get_value_type_id(member_symbol)
    }

    /// Resolve enum symbols from a receiver expression when it is a direct reference.
    fn enum_symbol_for_receiver_symbol(
        &self,
        module: &Module,
        left_id: LocalNodeId<Expression>,
        profile: ProfileId,
        tree: &NodeTree,
        symbols: &SymbolTable,
    ) -> Option<GlobalSymbolId> {
        let left_symbol =
            self.reference_symbol_for_expression(module, left_id, profile, tree, symbols)?;
        if left_symbol.ty() == SymbolType::Enum {
            Some(left_symbol)
        } else {
            None
        }
    }

    /// Resolve an enum field symbol matching a member key.
    fn enum_field_symbol_for_member_key(
        &self,
        module: &Module,
        enum_symbol: GlobalSymbolId,
        member_key: &StaticKey,
        profile: ProfileId,
        tree: &NodeTree,
        symbols: &SymbolTable,
    ) -> AnalyzeResult<Option<GlobalSymbolId>> {
        // resolve local declarations when possible
        if enum_symbol.module_id == module.id {
            return Ok(self.enum_field_symbol_for_member_key_in_tree(
                enum_symbol,
                member_key,
                tree,
                symbols,
            ));
        }

        // ensure remote declarations are available
        self.require_analyze_module_declare(enum_symbol.module_id, profile)
            .map_err(AnalyzeError::from)?;

        Ok(self.with_module_tree_symbols(
            module,
            profile,
            enum_symbol.module_id,
            |_, owner_tree, owner_symbols| {
                self.enum_field_symbol_for_member_key_in_tree(
                    enum_symbol,
                    member_key,
                    owner_tree,
                    owner_symbols,
                )
            },
        ))
    }

    /// Resolve enum declarations for a matching field key.
    fn enum_field_symbol_for_member_key_in_tree(
        &self,
        enum_symbol: GlobalSymbolId,
        member_key: &StaticKey,
        tree: &NodeTree,
        symbols: &SymbolTable,
    ) -> Option<GlobalSymbolId> {
        // ensure we are scanning an enum symbol
        let symbol_entry = symbols.get_symbol(enum_symbol.local_id);
        if symbol_entry.ty != SymbolType::Enum {
            return None;
        }

        // collect the enum declarations for the symbol
        let mut declaration_ids = Vec::new();
        if let Some(primary_declaration) = symbol_entry.primary_declaration {
            declaration_ids.push(primary_declaration);
        }
        if let Some(secondary_declarations) = symbol_entry.secondary_declarations.as_deref() {
            declaration_ids.extend(secondary_declarations.iter().copied());
        }

        // scan enum fields for a matching key
        for declaration_id in declaration_ids {
            let Ok(declaration_id) = declaration_id.try_into_local_typed::<Declaration>() else {
                continue;
            };
            let Declaration::Enum { fields, .. } = tree.get(declaration_id) else {
                continue;
            };
            for field_id in fields {
                let field = tree.get(*field_id);
                let field_key = StaticKey::Name(field.name);
                if field_key.matches(member_key) {
                    return Some(field.symbol.into_global(enum_symbol.module_id));
                }
            }
        }

        None
    }

    /// Resolve extension arguments and substitutions for a member lookup.
    pub(super) fn resolve_extension_member_context(
        &self,
        module: &Module,
        profile: ProfileId,
        source_id: LocalNodeIdAny,
        member_symbol: GlobalSymbolId,
        inherited_arguments: &[StaticArgument],
        options: &AnalyzeOptions,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> AnalyzeResult<Option<ExtensionMemberContext>> {
        // locate the extension symbol that owns the member
        let extension_symbol =
            self.extension_symbol_for_member(module, profile, member_symbol, symbols);
        let Some(extension_symbol) = extension_symbol else {
            return Ok(None);
        };

        // ensure extension instance types are available for parameter kind resolution
        if types.get_instance_type_id(extension_symbol).is_none()
            && extension_symbol.module_id != module.id
        {
            self.import_instance_type_for_symbol(profile, source_id, extension_symbol, types)?;
        }

        // resolve extension static parameter symbols
        let extension_parameters = self
            .collect_static_parameter_symbols(module, extension_symbol, profile, tree, symbols)
            .unwrap_or_default();

        // skip argument resolution when the extension has no parameters
        if extension_parameters.is_empty() {
            return Ok(Some(ExtensionMemberContext {
                arguments: Vec::new(),
                substitutions: HashMap::new(),
            }));
        }

        // avoid defaulting unresolved extension parameters to unknown
        if inherited_arguments.is_empty() {
            return Ok(Some(ExtensionMemberContext {
                arguments: Vec::new(),
                substitutions: HashMap::new(),
            }));
        }

        // map inherited arguments to extension parameters using the target type argument order
        let mut positional_arguments = self.map_extension_inherited_arguments(
            extension_symbol,
            &extension_parameters,
            inherited_arguments,
            profile,
        );
        if positional_arguments.is_empty() {
            positional_arguments = inherited_arguments.to_vec();
        }

        // normalize inherited arguments for positional mapping
        for argument in positional_arguments.iter_mut() {
            if let StaticArgument::Evaluated { name, .. } = argument {
                *name = None;
            }
        }

        // resolve arguments and defaults against extension parameters
        let resolved_arguments: Option<Vec<StaticArgument>> = self
            .resolve_type_reference_static_arguments(
                module,
                profile,
                source_id,
                extension_symbol,
                Some(&positional_arguments),
                true,
                options,
                tree,
                symbols,
                types,
            )?;
        let resolved_arguments = resolved_arguments.unwrap_or_default();

        // build substitutions for extension type parameters
        let substitutions = self.build_type_parameter_substitutions_for_symbol(
            module,
            profile,
            extension_symbol,
            source_id,
            &resolved_arguments,
            tree,
            symbols,
            types,
        );

        Ok(Some(ExtensionMemberContext {
            arguments: resolved_arguments,
            substitutions,
        }))
    }

    /// Map receiver static arguments into extension parameter order.
    fn map_extension_inherited_arguments(
        &self,
        extension_symbol: GlobalSymbolId,
        extension_parameters: &[GlobalSymbolId],
        inherited_arguments: &[StaticArgument],
        profile: ProfileId,
    ) -> Vec<StaticArgument> {
        // skip mapping when no parameters are declared
        if extension_parameters.is_empty() {
            return Vec::new();
        }

        // resolve the target type argument mapping from the extension declaration
        let target_mapping =
            self.extension_target_argument_mapping(extension_symbol, extension_parameters, profile);
        let Some(target_mapping) = target_mapping else {
            return Vec::new();
        };

        // map receiver arguments into extension parameter order
        let mut reordered = vec![None; extension_parameters.len()];
        for (target_index, parameter_index) in target_mapping.into_iter().enumerate() {
            if parameter_index >= reordered.len() {
                return Vec::new();
            }
            if reordered[parameter_index].is_some() {
                return Vec::new();
            }

            reordered[parameter_index] = Some(target_index);
        }

        let mut mapped = Vec::with_capacity(reordered.len());
        for maybe_target_index in reordered {
            let Some(target_index) = maybe_target_index else {
                return Vec::new();
            };

            if let Some(argument) = inherited_arguments.get(target_index) {
                mapped.push(argument.clone());
            } else {
                return Vec::new();
            }
        }

        mapped
    }

    /// Resolve the extension symbol that owns a member symbol.
    pub(super) fn extension_symbol_for_member(
        &self,
        module: &Module,
        profile: ProfileId,
        member_symbol: GlobalSymbolId,
        symbols: &SymbolTable,
    ) -> Option<GlobalSymbolId> {
        // locate the scope owner for the member symbol
        self.with_module_symbols_or_local(
            module,
            profile,
            member_symbol.module_id,
            symbols,
            |owner_module, owner_symbols| {
                let member_entry = owner_symbols.get_symbol(member_symbol.local_id);
                let scope = owner_symbols.get_scope_by_id(member_entry.scope.0);
                let owner_id = scope.owner_id?;
                let owner_entry = owner_symbols.get_symbol(owner_id);
                if owner_entry.ty != SymbolType::Extension {
                    return None;
                }

                let extension_id = owner_id.with_type(owner_entry.ty);
                Some(extension_id.into_global(owner_module.id))
            },
        )
    }

    /// Resolve the visibility context for a member symbol.
    fn member_visibility_context_for_symbol(
        &self,
        module: &Module,
        member_symbol: GlobalSymbolId,
        profile: ProfileId,
        tree: &NodeTree,
        symbols: &SymbolTable,
    ) -> Option<MemberVisibilityContext> {
        self.with_module_tree_symbols_or_local(
            module,
            profile,
            member_symbol.module_id,
            tree,
            symbols,
            |owner_module, owner_tree, owner_symbols| {
                self.member_visibility_context_for_symbol_in_tree(
                    owner_module.id,
                    member_symbol,
                    owner_tree,
                    owner_symbols,
                )
            },
        )
    }

    /// Resolve the visibility context for a member symbol inside a known tree.
    fn member_visibility_context_for_symbol_in_tree(
        &self,
        module_id: ModuleId,
        member_symbol: GlobalSymbolId,
        tree: &NodeTree,
        symbols: &SymbolTable,
    ) -> Option<MemberVisibilityContext> {
        // resolve the member symbol entry
        let member_entry = symbols.get_symbol(member_symbol.local_id);
        let member_node = member_entry.primary_declaration?;
        let member_local = member_node.local_id;
        if member_local.ty != NodeType::Member {
            return None;
        }

        // resolve the owning declaration symbol
        let scope = symbols.get_scope_by_id(member_entry.scope.0);
        let owner_id = scope.owner_id?;
        let owner_entry = symbols.get_symbol(owner_id);
        if !matches!(owner_entry.ty, SymbolType::Class | SymbolType::Struct) {
            return None;
        }

        let owner_symbol = owner_id.with_type(owner_entry.ty).into_global(module_id);
        let member_id = member_local.into_typed::<Member>();
        let member = tree.get(member_id);

        // extract visibility from modifiers
        let modifiers = match member {
            Member::Type { modifiers, .. }
            | Member::Field { modifiers, .. }
            | Member::Method { modifiers, .. }
            | Member::Embed { modifiers, .. }
            | Member::StaticBlock { modifiers, .. }
            | Member::ComptimeBlock { modifiers, .. } => modifiers.as_ref(),
        };
        let visibility = modifiers
            .and_then(|modifier| modifier.visibility)
            .unwrap_or(Visibility::Public);

        Some(MemberVisibilityContext {
            visibility,
            owner_symbol,
        })
    }

    /// Enforce visibility for a resolved member symbol.
    fn check_member_visibility(
        &self,
        module: &Module,
        expression_id: LocalNodeId<Expression>,
        member_symbol: GlobalSymbolId,
        receiver_ty_id: LocalTypeId,
        profile: ProfileId,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &TypeTable,
        ctx: &InferContext,
    ) {
        let Some(context) = self.member_visibility_context_for_symbol(
            module,
            member_symbol,
            profile,
            tree,
            symbols,
        ) else {
            return;
        };

        // public members are always accessible
        if context.visibility == Visibility::Public {
            return;
        }

        // require a class/struct context for private and protected access
        let Some(current_class) = ctx.in_nominal_symbol else {
            self.error(AnalyzeError::InaccessibleSymbol {
                node: expression_id
                    .into_global_any(module.id)
                    .into_anchored(Some(profile)),
                visibility: context.visibility,
                symbol: member_symbol,
            });
            return;
        };

        // private members require the declaring class
        if context.visibility == Visibility::Private && current_class != context.owner_symbol {
            self.error(AnalyzeError::InaccessibleSymbol {
                node: expression_id
                    .into_global_any(module.id)
                    .into_anchored(Some(profile)),
                visibility: context.visibility,
                symbol: member_symbol,
            });
            return;
        }

        // protected members require a subclass context
        if context.visibility == Visibility::Protected
            && current_class != context.owner_symbol
            && !self.is_type_lineage_assignable(
                module,
                profile,
                current_class,
                context.owner_symbol,
                symbols,
                types,
            )
        {
            self.error(AnalyzeError::InaccessibleSymbol {
                node: expression_id
                    .into_global_any(module.id)
                    .into_anchored(Some(profile)),
                visibility: context.visibility,
                symbol: member_symbol,
            });
            return;
        }

        // protected members must be accessed through the current class lineage
        if context.visibility == Visibility::Protected {
            let receiver_symbol = self.receiver_symbol_for_visibility(receiver_ty_id, types);
            if let Some(receiver_symbol) = receiver_symbol
                && receiver_symbol != current_class
                && !self.is_type_lineage_assignable(
                    module,
                    profile,
                    receiver_symbol,
                    current_class,
                    symbols,
                    types,
                )
            {
                self.error(AnalyzeError::InaccessibleSymbol {
                    node: expression_id
                        .into_global_any(module.id)
                        .into_anchored(Some(profile)),
                    visibility: context.visibility,
                    symbol: member_symbol,
                });
            }
        }
    }

    /// Resolve a nominal symbol for visibility checks from a receiver type.
    fn receiver_symbol_for_visibility(
        &self,
        receiver_ty_id: LocalTypeId,
        types: &TypeTable,
    ) -> Option<GlobalSymbolId> {
        // resolve nominal symbols for reference-like receiver types
        match types.get_type(receiver_ty_id) {
            Type::Reference { symbol, .. } => Some(*symbol),
            Type::Value { value } => types.get_type(*value).symbol(),
            _ => None,
        }
    }

    /// Resolve member symbols for a receiver type when nominal dispatch is possible.
    pub(super) fn resolve_member_symbol(
        &self,
        module: &Module,
        receiver_ty: &Type,
        member_key: &StaticKey,
        profile: ProfileId,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &TypeTable,
    ) -> AnalyzeResult<MemberResolution> {
        let resolution = match receiver_ty {
            Type::Reference { .. } => {
                // resolve nominal members first
                let mut visited = Vec::new();
                let member_symbol = self.resolve_member_symbol_for_type(
                    module,
                    receiver_ty,
                    member_key,
                    profile,
                    tree,
                    symbols,
                    types,
                    &mut visited,
                    true,
                )?;
                if let Some(symbol) = member_symbol {
                    MemberResolution::Static { symbol }
                } else {
                    MemberResolution::Unresolved
                }
            }
            Type::TypeLiteral { .. } => {
                // resolve implicit members for primitive and literal receivers
                let mut visited = Vec::new();
                let member_symbol = self.resolve_member_symbol_for_type(
                    module,
                    receiver_ty,
                    member_key,
                    profile,
                    tree,
                    symbols,
                    types,
                    &mut visited,
                    true,
                )?;
                if let Some(symbol) = member_symbol {
                    MemberResolution::Static { symbol }
                } else {
                    MemberResolution::Unresolved
                }
            }
            Type::Union { elements } => {
                // resolve member symbols for each union element
                let mut candidates = Vec::new();
                for element_id in elements {
                    let element_ty = types.get_type(*element_id).clone();
                    let mut visited = Vec::new();
                    let mut member_symbol = self.resolve_member_symbol_for_type(
                        module,
                        &element_ty,
                        member_key,
                        profile,
                        tree,
                        symbols,
                        types,
                        &mut visited,
                        true,
                    )?;
                    if member_symbol.is_none() {
                        // fall back to instance type owners when possible
                        if let Some(instance_symbol) = types.symbol_for_instance_type(*element_id) {
                            member_symbol = self.resolve_member_symbol_for_symbol(
                                module,
                                instance_symbol,
                                member_key,
                                MemberLookupMode::Instance,
                                profile,
                                tree,
                                symbols,
                                types,
                                &mut visited,
                            )?;
                        }
                    }
                    let Some(member_symbol) = member_symbol else {
                        return Ok(MemberResolution::None);
                    };
                    candidates.push(MemberResolutionCandidate {
                        receiver_ty_id: *element_id,
                        symbol: member_symbol,
                    });
                }

                // map resolution type depending on variants
                match candidates.len() {
                    0 => MemberResolution::None,
                    1 => MemberResolution::Static {
                        symbol: candidates[0].symbol,
                    },
                    _ => MemberResolution::Dynamic { candidates },
                }
            }
            _ => {
                // resolve implicit well known member resolution
                let mut visited = Vec::new();
                let member_symbol = self.resolve_member_symbol_for_type(
                    module,
                    receiver_ty,
                    member_key,
                    profile,
                    tree,
                    symbols,
                    types,
                    &mut visited,
                    true,
                )?;
                if let Some(symbol) = member_symbol {
                    MemberResolution::Static { symbol }
                } else {
                    MemberResolution::None
                }
            }
        };

        Ok(resolution)
    }

    /// Resolve member symbols using the receiver expression when available.
    pub(super) fn resolve_member_symbol_for_receiver(
        &self,
        module: &Module,
        receiver_id: LocalNodeId<Expression>,
        receiver_ty: &Type,
        member_key: &StaticKey,
        profile: ProfileId,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &TypeTable,
    ) -> AnalyzeResult<MemberResolution> {
        // prefer static-only lookup for direct class values
        if let Some(nominal_symbol) =
            self.nominal_value_symbol_for_expression(module, receiver_id, profile, tree, symbols)
        {
            let mut visited = Vec::new();
            let member_symbol = self.resolve_member_symbol_for_symbol(
                module,
                nominal_symbol,
                member_key,
                MemberLookupMode::Value,
                profile,
                tree,
                symbols,
                types,
                &mut visited,
            )?;
            return Ok(member_symbol
                .map(|symbol| MemberResolution::Static { symbol })
                .unwrap_or(MemberResolution::None));
        }

        // fall back to regular member lookup
        self.resolve_member_symbol(
            module,
            receiver_ty,
            member_key,
            profile,
            tree,
            symbols,
            types,
        )
    }

    /// Select the lookup mode for a receiver expression.
    pub(super) fn member_lookup_mode_for_receiver_expression(
        &self,
        module: &Module,
        receiver_id: LocalNodeId<Expression>,
        receiver_ty: &Type,
        profile: ProfileId,
        tree: &NodeTree,
        symbols: &SymbolTable,
    ) -> MemberLookupMode {
        // nominal values only expose static members
        if self
            .nominal_value_symbol_for_expression(module, receiver_id, profile, tree, symbols)
            .is_some()
        {
            return MemberLookupMode::Value;
        }

        // instance receivers should never surface static members
        if matches!(receiver_ty, Type::Reference { .. }) {
            return MemberLookupMode::Instance;
        }

        // fall back to unfiltered lookup for non-instance receivers
        MemberLookupMode::Any
    }

    /// Return a nominal symbol when the expression refers to a type value.
    fn nominal_value_symbol_for_expression(
        &self,
        module: &Module,
        receiver_id: LocalNodeId<Expression>,
        profile: ProfileId,
        tree: &NodeTree,
        symbols: &SymbolTable,
    ) -> Option<GlobalSymbolId> {
        // peel parenthesized receivers to their core symbol
        let receiver_id = self.unwrap_parenthesized_expression(receiver_id, tree);

        // resolve the direct reference symbol for the receiver
        let symbol =
            self.reference_symbol_for_expression(module, receiver_id, profile, tree, symbols)?;

        // keep only nominal symbols in value space
        if !matches!(
            symbol.ty(),
            SymbolType::Class | SymbolType::Struct | SymbolType::Enum | SymbolType::Newtype
        ) {
            return None;
        }
        let space = self.infer_symbol_space_for_global(module, profile, symbol, symbols);
        if matches!(space, SymbolSpace::Value | SymbolSpace::TypeValue) {
            Some(symbol)
        } else {
            None
        }
    }

    /// Resolve the symbol space for a global symbol.
    fn infer_symbol_space_for_global(
        &self,
        module: &Module,
        profile: ProfileId,
        symbol: GlobalSymbolId,
        symbols: &SymbolTable,
    ) -> SymbolSpace {
        self.with_module_symbols_or_local(
            module,
            profile,
            symbol.module_id,
            symbols,
            |_, owner_symbols| owner_symbols.get_symbol(symbol.local_id).space,
        )
    }

    /// Return true when the expression is rooted at import.meta.
    fn is_import_meta_chain(
        &self,
        tree: &NodeTree,
        mut expression_id: LocalNodeId<Expression>,
    ) -> bool {
        loop {
            match tree.get(expression_id) {
                Expression::ImportMeta => return true,
                Expression::Member { left, .. } => {
                    expression_id = *left;
                }
                _ => return false,
            }
        }
    }

    /// Return true when the expression is rooted at import.meta.<member>.
    fn is_import_meta_chain_member(
        &self,
        tree: &NodeTree,
        expression_id: LocalNodeId<Expression>,
        member: &str,
    ) -> bool {
        let member_key = self.program.strings.intern(member);
        match tree.get(expression_id) {
            Expression::Member { left, name, .. } => {
                *name == member_key && self.is_import_meta_chain(tree, *left)
            }
            _ => false,
        }
    }

    /// Resolve the member symbol for a type and member key.
    pub(super) fn resolve_member_symbol_for_type(
        &self,
        module: &Module,
        receiver_ty: &Type,
        member_key: &StaticKey,
        profile: ProfileId,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &TypeTable,
        visited: &mut Vec<GlobalSymbolId>,
        allow_implicit: bool,
    ) -> AnalyzeResult<Option<GlobalSymbolId>> {
        // pick a lookup mode based on the receiver type
        let lookup_mode = if matches!(receiver_ty, Type::Reference { .. }) {
            MemberLookupMode::Instance
        } else {
            MemberLookupMode::Any
        };

        let resolved = match receiver_ty {
            Type::Value { .. } => {
                let Some(type_symbol) = self.get_language_symbol(profile, LanguageSymbol::Type)
                else {
                    return Ok(None);
                };
                self.resolve_member_symbol_for_symbol(
                    module,
                    type_symbol,
                    member_key,
                    lookup_mode,
                    profile,
                    tree,
                    symbols,
                    types,
                    visited,
                )?
            }
            Type::Intersection { elements } => {
                let element_ids = elements.clone();
                let mut resolved = None;
                for element_id in element_ids {
                    let element_ty = types.get_type(element_id).clone();
                    resolved = self.resolve_member_symbol_for_type(
                        module,
                        &element_ty,
                        member_key,
                        profile,
                        tree,
                        symbols,
                        types,
                        visited,
                        allow_implicit,
                    )?;
                    if resolved.is_some() {
                        break;
                    }
                }
                resolved
            }
            Type::Reference { symbol, .. } => self.resolve_member_symbol_for_symbol(
                module,
                *symbol,
                member_key,
                lookup_mode,
                profile,
                tree,
                symbols,
                types,
                visited,
            )?,
            _ => None,
        };

        if resolved.is_some() || !allow_implicit {
            return Ok(resolved);
        }

        let Some(well_known_symbol) = self.well_known_symbol_for_type(receiver_ty, types) else {
            return Ok(None);
        };

        let Some(symbol) = self.get_well_known_type_symbol(profile, well_known_symbol) else {
            return Ok(None);
        };
        self.resolve_member_symbol_for_symbol(
            module,
            symbol,
            member_key,
            lookup_mode,
            profile,
            tree,
            symbols,
            types,
            visited,
        )
    }

    /// Resolve the member symbol for a nominal type symbol.
    pub(super) fn resolve_member_symbol_for_symbol(
        &self,
        module: &Module,
        symbol: GlobalSymbolId,
        member_key: &StaticKey,
        lookup_mode: MemberLookupMode,
        profile: ProfileId,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &TypeTable,
        visited: &mut Vec<GlobalSymbolId>,
    ) -> AnalyzeResult<Option<GlobalSymbolId>> {
        // NOTE #Suspicious: member lookup order mixes merge groups, lineage, and extensions with implicit precedence
        // stop on cycles in symbol lookup
        if visited.contains(&symbol) {
            return Ok(None);
        }
        visited.push(symbol);

        // resolve members from the local module data
        if symbol.module_id == module.id {
            let allow_merge = module.language_type.supports_declaration_merging();
            return self.resolve_member_symbol_in_module(
                module,
                symbol,
                member_key,
                lookup_mode,
                profile,
                tree,
                symbols,
                types,
                allow_merge,
                visited,
            );
        }

        if self.module_is_ambient_lib(module) {
            return Ok(None);
        }

        // ensure the remote module is declared before reading its DIR
        self.require_analyze_module_declare(symbol.module_id, profile)
            .map_err(AnalyzeError::from)?;

        let resolved = self.with_module_tree_symbols(
            module,
            profile,
            symbol.module_id,
            |owner_module, owner_tree, owner_symbols| {
                let owner_types = owner_module.dir(profile).types.read();
                let allow_merge = owner_module.language_type.supports_declaration_merging();
                self.resolve_member_symbol_in_module(
                    module,
                    symbol,
                    member_key,
                    lookup_mode,
                    profile,
                    owner_tree,
                    owner_symbols,
                    &owner_types,
                    allow_merge,
                    visited,
                )
            },
        )?;
        if resolved.is_some() {
            return Ok(resolved);
        }

        // check locally visible extensions for remote targets
        self.resolve_member_symbol_in_extensions(
            module,
            symbol,
            member_key,
            lookup_mode,
            profile,
            tree,
            symbols,
            types,
        )
    }

    /// Resolve member symbols using module-local declarations and merges.
    /// This follows infer_member_of_symbol lookup order using module data.
    fn resolve_member_symbol_in_module(
        &self,
        module: &Module,
        symbol: GlobalSymbolId,
        member_key: &StaticKey,
        lookup_mode: MemberLookupMode,
        profile: ProfileId,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &TypeTable,
        allow_merge: bool,
        visited: &mut Vec<GlobalSymbolId>,
    ) -> AnalyzeResult<Option<GlobalSymbolId>> {
        let symbol_entry = symbols.get_symbol(symbol.local_id);

        // step 1: check members declared directly on this symbol
        if let Some(member_symbol) = self.find_member_symbol_in_declaration(
            symbol.module_id,
            profile,
            symbol,
            member_key,
            lookup_mode,
            tree,
            symbols,
            types,
        ) {
            return Ok(Some(member_symbol));
        }

        // step 2: check merge groups and global augmentations
        if allow_merge {
            // scan merge group peers for members
            if let Some(group_id) = symbol_entry.merge_group {
                for group_symbol in symbols.merge_group_symbols(group_id) {
                    if *group_symbol == symbol.local_id {
                        continue;
                    }

                    if let Some(member_symbol) = self.resolve_member_symbol_for_symbol(
                        module,
                        group_symbol.into_global(symbol.module_id),
                        member_key,
                        lookup_mode,
                        profile,
                        tree,
                        symbols,
                        types,
                        visited,
                    )? {
                        return Ok(Some(member_symbol));
                    }
                }
            }

            // scan global augmentations for additional members
            if let Some(key) = symbol_entry.key
                && !self.module_is_ambient_lib(module)
            {
                let mut merge_symbols = Vec::new();
                if let Some(global_symbols) =
                    self.get_global_symbol_group(module.id, profile, key, symbol_entry.space)
                {
                    merge_symbols.extend(global_symbols);
                }
                if !self.module_is_ambient_lib(module)
                    && let Some(ambient_symbols) = self.get_ambient_lib_symbol_sources_for_merge(
                        profile,
                        key,
                        symbol_entry.space,
                    )
                {
                    merge_symbols.extend(ambient_symbols);
                }

                if !merge_symbols.is_empty() {
                    let mut seen = HashSet::new();
                    for merge_symbol in merge_symbols {
                        if !seen.insert(merge_symbol) || merge_symbol == symbol {
                            continue;
                        }

                        if let Some(member_symbol) = self.resolve_member_symbol_for_symbol(
                            module,
                            merge_symbol,
                            member_key,
                            lookup_mode,
                            profile,
                            tree,
                            symbols,
                            types,
                            visited,
                        )? {
                            return Ok(Some(member_symbol));
                        }
                    }
                }
            }
        }

        // step 3: check inherited members and visible extensions
        if let Some(lineage) = types.get_lineage_for_symbol(symbol).cloned() {
            // follow extends first
            if let Some(extends) = lineage.extends
                && let Some(member_symbol) = self.resolve_member_symbol_for_symbol(
                    module,
                    extends,
                    member_key,
                    lookup_mode,
                    profile,
                    tree,
                    symbols,
                    types,
                    visited,
                )?
            {
                return Ok(Some(member_symbol));
            }

            // check embedded types
            for embedded in &lineage.embedded {
                if let Some(member_symbol) = self.resolve_member_symbol_for_symbol(
                    module,
                    *embedded,
                    member_key,
                    lookup_mode,
                    profile,
                    tree,
                    symbols,
                    types,
                    visited,
                )? {
                    return Ok(Some(member_symbol));
                }
            }
        }

        // check extensions
        self.resolve_member_symbol_in_extensions(
            module,
            symbol,
            member_key,
            lookup_mode,
            profile,
            tree,
            symbols,
            types,
        )
    }

    /// Resolve members from extensions visible in the current module.
    fn resolve_member_symbol_in_extensions(
        &self,
        module: &Module,
        symbol: GlobalSymbolId,
        member_key: &StaticKey,
        lookup_mode: MemberLookupMode,
        profile: ProfileId,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &TypeTable,
    ) -> AnalyzeResult<Option<GlobalSymbolId>> {
        // check visible extensions for this symbol
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

            let member_symbol = self.find_member_symbol_in_extension(
                module,
                profile,
                extension_symbol,
                member_key,
                lookup_mode,
                tree,
                symbols,
                types,
            )?;
            if let Some(member_symbol) = member_symbol {
                return Ok(Some(member_symbol));
            }
        }

        Ok(None)
    }

    /// Find a member symbol inside an extension declaration.
    fn find_member_symbol_in_extension(
        &self,
        module: &Module,
        profile: ProfileId,
        extension_symbol: GlobalSymbolId,
        member_key: &StaticKey,
        lookup_mode: MemberLookupMode,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &TypeTable,
    ) -> AnalyzeResult<Option<GlobalSymbolId>> {
        // reuse local module data when the extension is local
        if extension_symbol.module_id == module.id {
            return Ok(self.find_member_symbol_in_declaration(
                module.id,
                profile,
                extension_symbol,
                member_key,
                lookup_mode,
                tree,
                symbols,
                types,
            ));
        }

        // ensure the extension module is declared before reading it
        self.require_analyze_module_declare(extension_symbol.module_id, profile)
            .map_err(AnalyzeError::from)?;

        Ok(self.with_module_tree_symbols(
            module,
            profile,
            extension_symbol.module_id,
            |owner_module, owner_tree, owner_symbols| {
                let owner_types = owner_module.dir(profile).types.read();
                self.find_member_symbol_in_declaration(
                    owner_module.id,
                    profile,
                    extension_symbol,
                    member_key,
                    lookup_mode,
                    owner_tree,
                    owner_symbols,
                    &owner_types,
                )
            },
        ))
    }

    /// Find a member symbol inside a declaration for a key.
    pub(super) fn find_member_symbol_in_declaration(
        &self,
        module_id: destack_source::ModuleId,
        profile: ProfileId,
        symbol: GlobalSymbolId,
        member_key: &StaticKey,
        lookup_mode: MemberLookupMode,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &TypeTable,
    ) -> Option<GlobalSymbolId> {
        let symbol_entry = symbols.get_symbol(symbol.local_id);

        // collect primary and secondary declarations to scan
        let mut declaration_ids = Vec::new();
        if let Some(primary_declaration) = symbol_entry.primary_declaration {
            declaration_ids.push(primary_declaration);
        }
        if let Some(secondary_declarations) = symbol_entry.secondary_declarations.as_deref() {
            declaration_ids.extend(secondary_declarations.iter().copied());
        }

        // scan declarations for a matching member
        for declaration_id in declaration_ids {
            let Ok(declaration_id) = declaration_id.try_into_local_typed::<Declaration>() else {
                continue;
            };
            let declaration = tree.get(declaration_id);

            if let Declaration::Enum {
                fields, members, ..
            } = declaration
            {
                // only expose enum fields through value lookups
                if matches!(lookup_mode, MemberLookupMode::Value | MemberLookupMode::Any) {
                    for field_id in fields {
                        let field = tree.get(*field_id);
                        let field_key = StaticKey::Name(field.name);
                        if field_key.matches(member_key) {
                            return Some(field.symbol.into_global(module_id));
                        }
                    }
                }

                // check enum methods and members
                for member_id in members {
                    let member = tree.get(*member_id);
                    // honor static versus instance lookup modes
                    if !self.member_visible_for_lookup(member, lookup_mode) {
                        continue;
                    }

                    let static_key = member.key().and_then(|key| {
                        self.static_key_from_dynamic_key(profile, *key, tree, symbols, types)
                    });

                    if let Some(static_key) = static_key
                        && static_key.matches(member_key)
                    {
                        return Some(member.symbol().into_global(module_id));
                    }
                }

                continue;
            }

            // check type members on structured declarations
            let Some(members) = declaration.member_ids() else {
                continue;
            };

            for member_id in members {
                let member = tree.get(*member_id);
                // honor static versus instance lookup modes
                if !self.member_visible_for_lookup(member, lookup_mode) {
                    continue;
                }

                let static_key = member.key().and_then(|key| {
                    self.static_key_from_dynamic_key(profile, *key, tree, symbols, types)
                });

                if let Some(static_key) = static_key
                    && static_key.matches(member_key)
                {
                    return Some(member.symbol().into_global(module_id));
                }
            }
        }

        None
    }

    /// Return true when a member matches the requested lookup mode.
    fn member_visible_for_lookup(&self, member: &Member, lookup_mode: MemberLookupMode) -> bool {
        // resolve modifiers from the member node
        let modifiers = match member {
            Member::Type { modifiers, .. } => modifiers.as_ref(),
            Member::Field { modifiers, .. } => modifiers.as_ref(),
            Member::Method { modifiers, .. } => modifiers.as_ref(),
            Member::Embed { modifiers, .. } => modifiers.as_ref(),
            Member::StaticBlock { modifiers, .. } => modifiers.as_ref(),
            Member::ComptimeBlock { modifiers, .. } => modifiers.as_ref(),
        };

        // filter by anchor for the lookup mode
        self.modifiers_visible_for_lookup(modifiers, lookup_mode)
    }

    /// Return true when modifiers allow access for a lookup mode.
    fn modifiers_visible_for_lookup(
        &self,
        modifiers: Option<&BindingModifier>,
        lookup_mode: MemberLookupMode,
    ) -> bool {
        // normalize the binding anchor for comparison
        let anchor = modifiers.and_then(|modifiers| modifiers.anchor);

        // match anchors to the requested lookup mode
        match lookup_mode {
            MemberLookupMode::Any => true,
            MemberLookupMode::Instance => !matches!(anchor, Some(BindingAnchor::Static)),
            MemberLookupMode::Value => matches!(anchor, Some(BindingAnchor::Static)),
        }
    }
}
