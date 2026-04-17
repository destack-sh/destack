use crate::analyze::common::{
    ModuleTypeView, SymbolTypeView, TreeSymbolView, TypeContext, TypeView,
};
use crate::{AnalyzeError, AnalyzeResult, Compiler};
use destack_dir::{
    Declaration, EnumBackingType, EnumField, EnumFieldValue, Expression, GlobalSymbolId, IntType,
    LocalNodeId, Name, NodeType, PrimitiveType, ScalarLiteral, StaticExpression, StaticKey,
    StringId, SymbolTable, Type, TypeLiteral,
};
use destack_workspace::{Module, ProfileId};

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Resolve enum field values from their declarations.
    pub(crate) fn infer_enum_field_values(
        &self,
        ctx: &mut TypeContext<'_>,
        enum_symbol: GlobalSymbolId,
        fields: &[LocalNodeId<EnumField>],
    ) -> AnalyzeResult<EnumBackingType> {
        // default to the configured integer width when unspecified
        let default_int_type = IntType::Arbitrary {
            width: self.options.default_int_width,
            is_signed: true,
        }
        .simplify();

        // track inferred backing type and next implicit value
        let mut backing_type: Option<EnumBackingType> = None;
        let mut next_value: i64 = 0;
        let mut has_next = false;

        // walk enum fields in declaration order
        let enum_scope = ctx.symbols.get_scope_by_symbol(enum_symbol.local_id);
        for field_id in fields {
            let field = ctx.tree.get(*field_id);
            let field_key = match field.name {
                Name::Identifier(name) | Name::String(name) => StaticKey::Name(name),
                Name::Number(name) => StaticKey::Number(name),
            };
            let Some(field_symbol) = ctx.symbols.find_active_symbol(enum_scope, field_key) else {
                return Err(AnalyzeError::InvalidEnumFieldValue {
                    node: field_id
                        .into_global_any(ctx.module.id)
                        .into_anchored(Some(ctx.profile)),
                });
            };
            let field_symbol = field_symbol.into_global(ctx.module.id);

            // resolve explicit values first
            let explicit_value = if let Some(value_id) = field.value {
                let static_value = self
                    .evaluate_static_expression_value(
                        &mut ctx.reborrow(),
                        value_id,
                        Some(enum_symbol),
                    )?
                    .ok_or(AnalyzeError::InvalidEnumFieldValue {
                        node: field_id
                            .into_global_any(ctx.module.id)
                            .into_anchored(Some(ctx.profile)),
                    })?;
                Some((
                    value_id,
                    self.enum_field_value_from_static(
                        &static_value,
                        *field_id,
                        ctx.module,
                        ctx.profile,
                    )?,
                ))
            } else {
                None
            };

            // select or validate backing type
            let field_backing_type = if let Some((value_id, value)) = explicit_value {
                match value {
                    EnumFieldValue::Int(_) => {
                        let int_type = self
                            .enum_int_type_for_expression(ctx.module_type_view(), value_id)?
                            .unwrap_or(default_int_type);
                        EnumBackingType::Int(int_type)
                    }
                    EnumFieldValue::String(_) => EnumBackingType::String,
                }
            } else {
                backing_type.unwrap_or(EnumBackingType::Int(default_int_type))
            };

            if let Some(existing) = backing_type {
                // reject mismatched backing types across enum fields
                let matches = match (existing, field_backing_type) {
                    (EnumBackingType::Int(left), EnumBackingType::Int(right)) => left == right,
                    (EnumBackingType::String, EnumBackingType::String) => true,
                    _ => false,
                };
                if !matches {
                    let backing_type_id = match field_backing_type {
                        EnumBackingType::Int(int_type) => ctx.types.insert_type_from(
                            Type::TypeLiteral {
                                value: TypeLiteral::Primitive(PrimitiveType::Int(int_type)),
                            },
                            *field_id,
                        ),
                        EnumBackingType::String => ctx.types.insert_type_from(
                            Type::TypeLiteral {
                                value: TypeLiteral::Primitive(PrimitiveType::String),
                            },
                            *field_id,
                        ),
                    };
                    return Err(AnalyzeError::InvalidEnumBackingType {
                        node: field_id
                            .into_global_any(ctx.module.id)
                            .into_anchored(Some(ctx.profile)),
                        ty: backing_type_id.into_global(ctx.module.id),
                    });
                }
            } else {
                backing_type = Some(field_backing_type);
            }

            // resolve the field value using the backing type
            let value = match (backing_type, explicit_value) {
                (Some(EnumBackingType::Int(int_type)), Some((_, EnumFieldValue::Int(value)))) => {
                    let is_signed = self.enum_int_type_is_signed(int_type);
                    if !is_signed && value < 0 {
                        return Err(AnalyzeError::InvalidEnumFieldValue {
                            node: field_id
                                .into_global_any(ctx.module.id)
                                .into_anchored(Some(ctx.profile)),
                        });
                    }
                    next_value =
                        value
                            .checked_add(1)
                            .ok_or(AnalyzeError::InvalidEnumFieldValue {
                                node: field_id
                                    .into_global_any(ctx.module.id)
                                    .into_anchored(Some(ctx.profile)),
                            })?;
                    has_next = true;
                    EnumFieldValue::Int(value)
                }
                (Some(EnumBackingType::Int(int_type)), None) => {
                    let value = if has_next { next_value } else { 0 };
                    next_value =
                        value
                            .checked_add(1)
                            .ok_or(AnalyzeError::InvalidEnumFieldValue {
                                node: field_id
                                    .into_global_any(ctx.module.id)
                                    .into_anchored(Some(ctx.profile)),
                            })?;
                    has_next = true;

                    let is_signed = self.enum_int_type_is_signed(int_type);
                    if !is_signed && value < 0 {
                        return Err(AnalyzeError::InvalidEnumFieldValue {
                            node: field_id
                                .into_global_any(ctx.module.id)
                                .into_anchored(Some(ctx.profile)),
                        });
                    }

                    EnumFieldValue::Int(value)
                }
                (Some(EnumBackingType::String), Some((_, EnumFieldValue::String(value)))) => {
                    EnumFieldValue::String(value)
                }
                (Some(EnumBackingType::String), _) => {
                    return Err(AnalyzeError::InvalidEnumFieldValue {
                        node: field_id
                            .into_global_any(ctx.module.id)
                            .into_anchored(Some(ctx.profile)),
                    });
                }
                _ => {
                    return Err(AnalyzeError::InvalidEnumFieldValue {
                        node: field_id
                            .into_global_any(ctx.module.id)
                            .into_anchored(Some(ctx.profile)),
                    });
                }
            };

            // record the resolved value
            ctx.types.set_enum_field_value(field_symbol, value);
        }

        let backing_type = backing_type.unwrap_or(EnumBackingType::Int(default_int_type));
        Ok(backing_type)
    }

    /// Resolve an enum field value from a static expression.
    fn enum_field_value_from_static(
        &self,
        value: &StaticExpression,
        field_id: LocalNodeId<EnumField>,
        module: &Module,
        profile: ProfileId,
    ) -> AnalyzeResult<EnumFieldValue> {
        match value {
            StaticExpression::ScalarLiteral { value } => match value {
                ScalarLiteral::Integer(value) => Ok(EnumFieldValue::Int(*value)),
                ScalarLiteral::String(value) => Ok(EnumFieldValue::String(*value)),
                _ => Err(AnalyzeError::InvalidEnumFieldValue {
                    node: field_id
                        .into_global_any(module.id)
                        .into_anchored(Some(profile)),
                }),
            },
            _ => Err(AnalyzeError::InvalidEnumFieldValue {
                node: field_id
                    .into_global_any(module.id)
                    .into_anchored(Some(profile)),
            }),
        }
    }

    /// Resolve an enum field value from a symbol reference.
    pub(crate) fn enum_field_value_for_symbol_reference(
        &self,
        ctx: &mut TypeContext<'_>,
        enum_symbol: GlobalSymbolId,
        target_symbol: GlobalSymbolId,
    ) -> AnalyzeResult<Option<EnumFieldValue>> {
        // prefer local symbol ctx when possible
        if target_symbol.module_id == ctx.module.id {
            return Ok(self.enum_field_value_for_symbol_reference_local(
                &mut ctx.reborrow(),
                enum_symbol,
                target_symbol,
            ));
        }

        let owner_dir = self
            .require_indexed_dir_declared(
                &ctx.index,
                ctx.compiler_context.revision(),
                target_symbol.module_id,
                ctx.profile,
            )
            .map_err(AnalyzeError::from)?;
        let owner_module = ctx.compiler_context.module(target_symbol.module_id);
        let owner_module = owner_module.as_ref();
        Ok(self.enum_field_value_for_symbol_reference_read(
            SymbolTypeView::new(
                ctx.compiler_context,
                owner_module,
                ctx.profile,
                &owner_dir.symbols,
                &owner_dir.types,
            ),
            enum_symbol,
            target_symbol,
        ))
    }

    /// Resolve an enum field value from symbol ctx and immutable type ctx.
    fn enum_field_value_for_symbol_reference_read(
        &self,
        ctx: SymbolTypeView<'_>,
        enum_symbol: GlobalSymbolId,
        target_symbol: GlobalSymbolId,
    ) -> Option<EnumFieldValue> {
        // validate module ownership for symbol table lookups
        debug_assert_eq!(target_symbol.module_id, ctx.module.id);

        // ensure the target is an enum field on this enum
        let target_entry = ctx.symbols.get_symbol(target_symbol.local_id);
        let primary = target_entry.primary_declaration?;
        if primary.local_id.ty != NodeType::EnumField {
            return None;
        }

        // confirm the field is owned by the enum or merge group
        let scope = ctx.symbols.get_scope_by_symbol(target_symbol.local_id);
        let scope_owner = scope.owner_id?;
        let scope_owner = scope_owner.into_global(ctx.module.id);
        if scope_owner != enum_symbol
            && !self.symbols_share_merge_group(enum_symbol, scope_owner, ctx.symbols)
        {
            return None;
        }

        // remote reads are publish-only: consume existing declared enum field values
        ctx.types.get_enum_field_value(target_symbol)
    }

    /// Resolve an enum field value from symbol ctx and type ctx.
    fn enum_field_value_for_symbol_reference_local(
        &self,
        ctx: &mut TypeContext<'_>,
        enum_symbol: GlobalSymbolId,
        target_symbol: GlobalSymbolId,
    ) -> Option<EnumFieldValue> {
        // validate module ownership for symbol table lookups
        debug_assert_eq!(target_symbol.module_id, ctx.module.id);

        // ensure the target is an enum field on this enum
        let target_entry = ctx.symbols.get_symbol(target_symbol.local_id);
        let primary = target_entry.primary_declaration?;
        if primary.local_id.ty != NodeType::EnumField {
            return None;
        }

        // confirm the field is owned by the enum or merge group
        let scope = ctx.symbols.get_scope_by_symbol(target_symbol.local_id);
        let scope_owner = scope.owner_id?;
        let scope_owner = scope_owner.into_global(ctx.module.id);
        if scope_owner != enum_symbol
            && !self.symbols_share_merge_group(enum_symbol, scope_owner, ctx.symbols)
        {
            return None;
        }

        // reuse cached values when possible
        if let Some(value) = ctx.types.get_enum_field_value(target_symbol) {
            return Some(value);
        }

        // ensure backing values are inferred
        let _ = self.enum_backing_type_for_symbol(&mut ctx.reborrow(), enum_symbol);

        ctx.types.get_enum_field_value(target_symbol)
    }

    /// Resolve the enum backing type for a local symbol using available ctx.
    pub(crate) fn enum_backing_type_for_symbol(
        &self,
        ctx: &mut TypeContext<'_>,
        enum_symbol: GlobalSymbolId,
    ) -> Option<EnumBackingType> {
        // reuse cached backing types
        if let Some(backing) = ctx.types.get_enum_backing_type(enum_symbol) {
            return Some(backing);
        }

        // consume already published values from remote modules
        if enum_symbol.module_id != ctx.module.id {
            let remote_dir = self.require_indexed_dir_declared(
                &ctx.index,
                ctx.compiler_context.revision(),
                enum_symbol.module_id,
                ctx.profile,
            );
            match remote_dir {
                Ok(remote_dir) => return remote_dir.types.get_enum_backing_type(enum_symbol),
                Err(error) => {
                    self.error(AnalyzeError::from(error));
                    return None;
                }
            }
        }

        // local enum lookups require local symbol ownership
        debug_assert_eq!(enum_symbol.module_id, ctx.module.id);

        // infer local enum field values to determine the backing type
        let fields = self.enum_fields_for_symbol_in_tree(ctx.tree_symbol_view(), enum_symbol);
        if fields.is_empty() {
            return None;
        }
        let backing_type = self.infer_enum_field_values(&mut ctx.reborrow(), enum_symbol, &fields);
        match backing_type {
            Ok(backing_type) => {
                ctx.types.set_enum_backing_type(enum_symbol, backing_type);
            }
            Err(error) => {
                self.error(error);
            }
        }

        ctx.types.get_enum_backing_type(enum_symbol)
    }

    /// Check whether two symbols share the same merge group.
    fn symbols_share_merge_group(
        &self,
        left: GlobalSymbolId,
        right: GlobalSymbolId,
        symbols: &SymbolTable,
    ) -> bool {
        if left.module_id != right.module_id {
            return false;
        }

        let left_entry = symbols.get_symbol(left.local_id);
        let right_entry = symbols.get_symbol(right.local_id);
        match (left_entry.merge_group, right_entry.merge_group) {
            (Some(left_group), Some(right_group)) => left_group == right_group,
            _ => false,
        }
    }

    /// Collect enum field ids for a symbol in a single tree.
    fn enum_fields_for_symbol_in_tree(
        &self,
        ctx: TreeSymbolView<'_>,
        enum_symbol: GlobalSymbolId,
    ) -> Vec<LocalNodeId<EnumField>> {
        let symbol_entry = ctx.symbols.get_symbol(enum_symbol.local_id);
        let candidate_ids = if let Some(group_id) = symbol_entry.merge_group {
            ctx.symbols.merge_group_symbols(group_id).to_vec()
        } else {
            vec![enum_symbol.local_id]
        };

        let mut fields = Vec::new();
        for candidate_id in candidate_ids {
            let candidate_entry = ctx.symbols.get_symbol(candidate_id);
            let mut declaration_ids = Vec::new();

            if let Some(primary) = candidate_entry.primary_declaration
                && primary.local_id.ty == NodeType::Declaration
            {
                declaration_ids.push(LocalNodeId::<Declaration>::new(primary.local_id.id));
            }

            if let Some(secondaries) = candidate_entry.secondary_declarations.as_deref() {
                for declaration in secondaries {
                    if declaration.local_id.ty == NodeType::Declaration {
                        declaration_ids
                            .push(LocalNodeId::<Declaration>::new(declaration.local_id.id));
                    }
                }
            }

            for declaration_id in declaration_ids {
                let declaration = ctx.tree.get(declaration_id);
                let Declaration::Enum(declaration) = declaration else {
                    continue;
                };
                fields.extend_from_slice(&declaration.fields);
            }
        }

        fields
    }

    /// Collect enum field symbols for a declaration symbol.
    pub(crate) fn enum_field_symbols_for_enum(
        &self,
        ctx: TypeView<'_>,
        enum_symbol: GlobalSymbolId,
    ) -> Vec<GlobalSymbolId> {
        self.with_module_tree_symbol_view_or_local_for_artifact(
            ctx.compiler_context,
            ctx.module,
            ctx.profile,
            enum_symbol.module_id,
            ctx.tree,
            ctx.symbols,
            destack_artifact::ArtifactKey::dir_declared,
            |view| {
                let fields = self.enum_fields_for_symbol_in_tree(view, enum_symbol);
                fields
                    .into_iter()
                    .filter_map(|field_id| {
                        let field = view.tree.get(field_id);
                        let field_key = match field.name {
                            Name::Identifier(name) | Name::String(name) => StaticKey::Name(name),
                            Name::Number(name) => StaticKey::Number(name),
                        };
                        let enum_scope = view.symbols.get_scope_by_symbol(enum_symbol.local_id);
                        view.symbols
                            .find_active_symbol(enum_scope, field_key)
                            .map(|symbol| symbol.into_global(view.module.id))
                    })
                    .collect()
            },
        )
        .unwrap_or_default()
    }

    /// Resolve the integer backing type for an enum expression.
    fn enum_int_type_for_expression(
        &self,
        ctx: ModuleTypeView<'_>,
        expression_id: LocalNodeId<Expression>,
    ) -> AnalyzeResult<Option<IntType>> {
        // this inference pass can run before expression types are committed
        let Some(value_type_id) = ctx
            .types
            .get_declared_or_inferred_type_id(expression_id.into_global_any(ctx.module.id))
        else {
            return Ok(None);
        };
        let value_type = ctx.types.get_type(value_type_id);

        // map the value type to a backing integer type
        let resolved = match value_type {
            Type::TypeLiteral {
                value: TypeLiteral::Primitive(PrimitiveType::Int(int_type)),
            } => Some(int_type.simplify()),
            Type::TypeLiteral {
                value: TypeLiteral::ScalarLiteral(ScalarLiteral::Integer(_)),
            } => Some(
                IntType::Arbitrary {
                    width: self.options.default_int_width,
                    is_signed: true,
                }
                .simplify(),
            ),
            _ => None,
        };

        Ok(resolved)
    }

    /// Check whether an integer backing type is signed.
    fn enum_int_type_is_signed(&self, int_type: IntType) -> bool {
        matches!(
            int_type.simplify(),
            IntType::Int8
                | IntType::Int16
                | IntType::Int32
                | IntType::Int64
                | IntType::Int128
                | IntType::Int256
                | IntType::Isize
                | IntType::Arbitrary {
                    is_signed: true,
                    ..
                }
        )
    }

    /// Resolve an enum field symbol for a member name.
    pub(crate) fn enum_field_symbol_for_name(
        &self,
        ctx: TypeView<'_>,
        enum_symbol: GlobalSymbolId,
        field_name: StringId,
    ) -> AnalyzeResult<Option<GlobalSymbolId>> {
        // resolve fields in remote modules when needed
        if enum_symbol.module_id != ctx.module.id {
            let module = ctx.compiler_context.module(enum_symbol.module_id);
            let module = module.as_ref();
            let dir = self
                .require_artifact_dir_declared(
                    ctx.compiler_context.revision(),
                    enum_symbol.module_id,
                    ctx.profile,
                )
                .map_err(AnalyzeError::from)?;

            return Ok(self.enum_field_symbol_for_name_in_tree(
                TreeSymbolView::new(
                    ctx.compiler_context,
                    module,
                    ctx.profile,
                    &dir.tree,
                    &dir.symbols,
                ),
                enum_symbol,
                field_name,
            ));
        }

        Ok(
            self.enum_field_symbol_for_name_in_tree(
                ctx.tree_symbol_view(),
                enum_symbol,
                field_name,
            ),
        )
    }

    /// Query one enum field symbol in non-AnalyzeResult paths.
    pub(crate) fn query_enum_field_symbol_for_name(
        &self,
        ctx: TypeView<'_>,
        enum_symbol: GlobalSymbolId,
        field_name: StringId,
    ) -> Option<GlobalSymbolId> {
        match self.enum_field_symbol_for_name(ctx, enum_symbol, field_name) {
            Ok(field_symbol) => field_symbol,
            Err(error) => {
                self.error(error);
                None
            }
        }
    }

    /// Scan enum declarations in a single tree for a field symbol.
    pub(crate) fn enum_field_symbol_for_name_in_tree(
        &self,
        ctx: TreeSymbolView<'_>,
        enum_symbol: GlobalSymbolId,
        field_name: StringId,
    ) -> Option<GlobalSymbolId> {
        let symbol_entry = ctx.symbols.get_symbol(enum_symbol.local_id);
        let candidate_ids = if let Some(group_id) = symbol_entry.merge_group {
            ctx.symbols.merge_group_symbols(group_id).to_vec()
        } else {
            vec![enum_symbol.local_id]
        };

        for candidate_id in candidate_ids {
            let candidate_entry = ctx.symbols.get_symbol(candidate_id);
            let mut declaration_ids = Vec::new();

            // include the primary declaration when available
            if let Some(primary) = candidate_entry.primary_declaration
                && primary.local_id.ty == NodeType::Declaration
            {
                declaration_ids.push(LocalNodeId::<Declaration>::new(primary.local_id.id));
            }

            // include secondary declarations when present
            if let Some(secondaries) = candidate_entry.secondary_declarations.as_deref() {
                for declaration in secondaries {
                    if declaration.local_id.ty == NodeType::Declaration {
                        declaration_ids
                            .push(LocalNodeId::<Declaration>::new(declaration.local_id.id));
                    }
                }
            }

            // scan enum members for the matching field
            for declaration_id in declaration_ids {
                let declaration = ctx.tree.get(declaration_id);
                let Declaration::Enum(declaration) = declaration else {
                    continue;
                };

                // return the symbol for the matching field name
                for field_id in &declaration.fields {
                    let field = ctx.tree.get(*field_id);
                    if field.name.string() == field_name {
                        let field_key = match field.name {
                            Name::Identifier(name) | Name::String(name) => StaticKey::Name(name),
                            Name::Number(name) => StaticKey::Number(name),
                        };
                        let enum_scope = ctx.symbols.get_scope_by_symbol(enum_symbol.local_id);
                        if let Some(field_symbol) =
                            ctx.symbols.find_active_symbol(enum_scope, field_key)
                        {
                            return Some(field_symbol.into_global(ctx.module.id));
                        }
                    }
                }
            }
        }

        None
    }
}
