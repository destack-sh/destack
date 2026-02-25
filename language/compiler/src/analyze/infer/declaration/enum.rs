use crate::analyze::common::{AnalyzeDependencyStage, TypeTablesContext};
use crate::{AnalyzeError, AnalyzeResult, Compiler};
use destack_dir::{
    Declaration, EnumBackingType, EnumField, EnumFieldValue, Expression, GlobalSymbolId, IntType,
    LocalNodeId, NodeTree, NodeType, PrimitiveType, ScalarLiteral, StaticExpression, StringId,
    SymbolTable, Type, TypeLiteral, TypeTable,
};
use destack_source::ModuleId;
use destack_workspace::{Module, ProfileId};

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Resolve enum field values from their declarations.
    pub(crate) fn infer_enum_field_values(
        &self,
        tables: &mut TypeTablesContext<'_>,
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
        for field_id in fields {
            let field = tables.tree.get(*field_id);
            let field_symbol = field.symbol.into_global(tables.module.id);

            // resolve explicit values first
            let explicit_value = if let Some(value_id) = field.value {
                let static_value = self
                    .evaluate_static_expression_value(
                        &mut tables.reborrow(),
                        value_id,
                        Some(enum_symbol),
                    )?
                    .ok_or(AnalyzeError::InvalidEnumFieldValue {
                        node: field_id
                            .into_global_any(tables.module.id)
                            .into_anchored(Some(tables.profile)),
                    })?;
                Some((
                    value_id,
                    self.enum_field_value_from_static(
                        &static_value,
                        *field_id,
                        tables.module,
                        tables.profile,
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
                            .enum_int_type_for_expression(
                                tables.module,
                                tables.profile,
                                value_id,
                                tables.types,
                            )?
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
                        EnumBackingType::Int(int_type) => tables.types.insert_type_from(
                            Type::TypeLiteral {
                                value: TypeLiteral::Primitive(PrimitiveType::Int(int_type)),
                            },
                            *field_id,
                        ),
                        EnumBackingType::String => tables.types.insert_type_from(
                            Type::TypeLiteral {
                                value: TypeLiteral::Primitive(PrimitiveType::String),
                            },
                            *field_id,
                        ),
                    };
                    return Err(AnalyzeError::InvalidEnumBackingType {
                        node: field_id
                            .into_global_any(tables.module.id)
                            .into_anchored(Some(tables.profile)),
                        ty: backing_type_id.into_global(tables.module.id),
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
                                .into_global_any(tables.module.id)
                                .into_anchored(Some(tables.profile)),
                        });
                    }
                    next_value =
                        value
                            .checked_add(1)
                            .ok_or(AnalyzeError::InvalidEnumFieldValue {
                                node: field_id
                                    .into_global_any(tables.module.id)
                                    .into_anchored(Some(tables.profile)),
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
                                    .into_global_any(tables.module.id)
                                    .into_anchored(Some(tables.profile)),
                            })?;
                    has_next = true;

                    let is_signed = self.enum_int_type_is_signed(int_type);
                    if !is_signed && value < 0 {
                        return Err(AnalyzeError::InvalidEnumFieldValue {
                            node: field_id
                                .into_global_any(tables.module.id)
                                .into_anchored(Some(tables.profile)),
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
                            .into_global_any(tables.module.id)
                            .into_anchored(Some(tables.profile)),
                    });
                }
                _ => {
                    return Err(AnalyzeError::InvalidEnumFieldValue {
                        node: field_id
                            .into_global_any(tables.module.id)
                            .into_anchored(Some(tables.profile)),
                    });
                }
            };

            // record the resolved value
            tables.types.set_enum_field_value(field_symbol, value);
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
        tables: &mut TypeTablesContext<'_>,
        enum_symbol: GlobalSymbolId,
        target_symbol: GlobalSymbolId,
    ) -> AnalyzeResult<Option<EnumFieldValue>> {
        // prefer local symbol tables when possible
        if target_symbol.module_id == tables.module.id {
            return Ok(self.enum_field_value_for_symbol_reference_in_tables(
                &mut tables.reborrow(),
                enum_symbol,
                target_symbol,
            ));
        }

        self.with_module_symbols_at_stage(
            tables.module,
            tables.profile,
            target_symbol.module_id,
            AnalyzeDependencyStage::Declare,
            |owner_module, owner_symbols| {
                let owner_types = owner_module.dir(tables.profile).types.read();
                self.enum_field_value_for_symbol_reference_read(
                    owner_module,
                    enum_symbol,
                    target_symbol,
                    owner_symbols,
                    &owner_types,
                )
            },
        )
        .map_err(AnalyzeError::from)
    }

    /// Resolve an enum field value from symbol tables and immutable type tables.
    fn enum_field_value_for_symbol_reference_read(
        &self,
        module: &Module,
        enum_symbol: GlobalSymbolId,
        target_symbol: GlobalSymbolId,
        symbols: &SymbolTable,
        types: &TypeTable,
    ) -> Option<EnumFieldValue> {
        // validate module ownership for symbol table lookups
        debug_assert_eq!(target_symbol.module_id, module.id);

        // ensure the target is an enum field on this enum
        let target_entry = symbols.get_symbol(target_symbol.local_id);
        let primary = target_entry.primary_declaration?;
        if primary.local_id.ty != NodeType::EnumField {
            return None;
        }

        // confirm the field is owned by the enum or merge group
        let scope = symbols.get_scope_by_symbol(target_symbol.local_id);
        let scope_owner = scope.owner_id?;
        let scope_owner = scope_owner.into_global(module.id);
        if scope_owner != enum_symbol
            && !self.symbols_share_merge_group(enum_symbol, scope_owner, symbols)
        {
            return None;
        }

        // remote reads are publish-only: consume existing declared enum field values
        types.get_enum_field_value(target_symbol)
    }

    /// Resolve an enum field value from symbol tables and type tables.
    fn enum_field_value_for_symbol_reference_in_tables(
        &self,
        tables: &mut TypeTablesContext<'_>,
        enum_symbol: GlobalSymbolId,
        target_symbol: GlobalSymbolId,
    ) -> Option<EnumFieldValue> {
        // validate module ownership for symbol table lookups
        debug_assert_eq!(target_symbol.module_id, tables.module.id);

        // ensure the target is an enum field on this enum
        let target_entry = tables.symbols.get_symbol(target_symbol.local_id);
        let primary = target_entry.primary_declaration?;
        if primary.local_id.ty != NodeType::EnumField {
            return None;
        }

        // confirm the field is owned by the enum or merge group
        let scope = tables.symbols.get_scope_by_symbol(target_symbol.local_id);
        let scope_owner = scope.owner_id?;
        let scope_owner = scope_owner.into_global(tables.module.id);
        if scope_owner != enum_symbol
            && !self.symbols_share_merge_group(enum_symbol, scope_owner, tables.symbols)
        {
            return None;
        }

        // reuse cached values when possible
        if let Some(value) = tables.types.get_enum_field_value(target_symbol) {
            return Some(value);
        }

        // ensure backing values are inferred
        let _ = self.enum_backing_type_for_symbol_in_tables(&mut tables.reborrow(), enum_symbol);

        tables.types.get_enum_field_value(target_symbol)
    }

    /// Resolve the enum backing type for a local symbol using available tables.
    pub(crate) fn enum_backing_type_for_symbol_in_tables(
        &self,
        tables: &mut TypeTablesContext<'_>,
        enum_symbol: GlobalSymbolId,
    ) -> Option<EnumBackingType> {
        // validate module ownership for symbol table lookups
        debug_assert_eq!(enum_symbol.module_id, tables.module.id);

        // reuse cached backing types
        if let Some(backing) = tables.types.get_enum_backing_type(enum_symbol) {
            return Some(backing);
        }

        // consume already published values from remote modules
        if enum_symbol.module_id != tables.module.id {
            let remote_backing = self
                .with_module_types_at_stage(
                    tables.module,
                    tables.profile,
                    enum_symbol.module_id,
                    AnalyzeDependencyStage::Declare,
                    |_owner_module, owner_types| owner_types.get_enum_backing_type(enum_symbol),
                )
                .map_err(AnalyzeError::from);
            match remote_backing {
                Ok(backing) => return backing,
                Err(error) => {
                    self.error(error);
                    return None;
                }
            }
        }

        // infer local enum field values to determine the backing type
        let fields = self.enum_fields_for_symbol_in_tree(enum_symbol, tables.tree, tables.symbols);
        if fields.is_empty() {
            return None;
        }
        let backing_type =
            self.infer_enum_field_values(&mut tables.reborrow(), enum_symbol, &fields);
        match backing_type {
            Ok(backing_type) => {
                tables
                    .types
                    .set_enum_backing_type(enum_symbol, backing_type);
            }
            Err(error) => {
                self.error(error);
            }
        }

        tables.types.get_enum_backing_type(enum_symbol)
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
        enum_symbol: GlobalSymbolId,
        tree: &NodeTree,
        symbols: &SymbolTable,
    ) -> Vec<LocalNodeId<EnumField>> {
        let symbol_entry = symbols.get_symbol(enum_symbol.local_id);
        let candidate_ids = if let Some(group_id) = symbol_entry.merge_group {
            symbols.merge_group_symbols(group_id).to_vec()
        } else {
            vec![enum_symbol.local_id]
        };

        let mut fields = Vec::new();
        for candidate_id in candidate_ids {
            let candidate_entry = symbols.get_symbol(candidate_id);
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
                let declaration = tree.get(declaration_id);
                let Declaration::Enum {
                    fields: enum_fields,
                    ..
                } = declaration
                else {
                    continue;
                };
                fields.extend_from_slice(enum_fields);
            }
        }

        fields
    }

    /// Collect enum field symbols for a declaration symbol.
    pub(crate) fn enum_field_symbols_for_enum(
        &self,
        module: &Module,
        profile: ProfileId,
        enum_symbol: GlobalSymbolId,
        tree: &NodeTree,
        symbols: &SymbolTable,
    ) -> Vec<GlobalSymbolId> {
        self.with_module_tree_symbols_or_local_at_stage(
            module,
            profile,
            enum_symbol.module_id,
            tree,
            symbols,
            AnalyzeDependencyStage::Declare,
            |owner_module, owner_tree, owner_symbols| {
                let fields =
                    self.enum_fields_for_symbol_in_tree(enum_symbol, owner_tree, owner_symbols);
                fields
                    .into_iter()
                    .map(|field_id| owner_tree.get(field_id).symbol.into_global(owner_module.id))
                    .collect()
            },
        )
        .unwrap_or_default()
    }

    /// Resolve the integer backing type for an enum expression.
    fn enum_int_type_for_expression(
        &self,
        module: &Module,
        _profile: ProfileId,
        expression_id: LocalNodeId<Expression>,
        types: &TypeTable,
    ) -> AnalyzeResult<Option<IntType>> {
        // this inference pass can run before expression types are committed
        let Some(value_type_id) =
            types.get_declared_or_inferred_type_id(expression_id.into_global_any(module.id))
        else {
            return Ok(None);
        };
        let value_type = types.get_type(value_type_id);

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
        module: &Module,
        profile: ProfileId,
        enum_symbol: GlobalSymbolId,
        field_name: StringId,
        tree: &NodeTree,
        symbols: &SymbolTable,
    ) -> AnalyzeResult<Option<GlobalSymbolId>> {
        // resolve fields in remote modules when needed
        if enum_symbol.module_id != module.id {
            return self
                .with_module_tree_symbols_at_stage(
                    module,
                    profile,
                    enum_symbol.module_id,
                    AnalyzeDependencyStage::Declare,
                    |owner_module, owner_tree, owner_symbols| {
                        self.enum_field_symbol_for_name_in_tree(
                            enum_symbol,
                            field_name,
                            owner_tree,
                            owner_symbols,
                            owner_module.id,
                        )
                    },
                )
                .map_err(AnalyzeError::from);
        }

        Ok(self.enum_field_symbol_for_name_in_tree(
            enum_symbol,
            field_name,
            tree,
            symbols,
            module.id,
        ))
    }

    /// Query one enum field symbol in non-AnalyzeResult paths.
    pub(crate) fn query_enum_field_symbol_for_name(
        &self,
        module: &Module,
        profile: ProfileId,
        enum_symbol: GlobalSymbolId,
        field_name: StringId,
        tree: &NodeTree,
        symbols: &SymbolTable,
    ) -> Option<GlobalSymbolId> {
        match self.enum_field_symbol_for_name(
            module,
            profile,
            enum_symbol,
            field_name,
            tree,
            symbols,
        ) {
            Ok(field_symbol) => field_symbol,
            Err(error) => {
                self.error(error);
                None
            }
        }
    }

    /// Scan enum declarations in a single tree for a field symbol.
    fn enum_field_symbol_for_name_in_tree(
        &self,
        enum_symbol: GlobalSymbolId,
        field_name: StringId,
        tree: &NodeTree,
        symbols: &SymbolTable,
        module_id: ModuleId,
    ) -> Option<GlobalSymbolId> {
        let symbol_entry = symbols.get_symbol(enum_symbol.local_id);
        let candidate_ids = if let Some(group_id) = symbol_entry.merge_group {
            symbols.merge_group_symbols(group_id).to_vec()
        } else {
            vec![enum_symbol.local_id]
        };

        for candidate_id in candidate_ids {
            let candidate_entry = symbols.get_symbol(candidate_id);
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
                let declaration = tree.get(declaration_id);
                let Declaration::Enum { fields, .. } = declaration else {
                    continue;
                };

                // return the symbol for the matching field name
                for field_id in fields {
                    let field = tree.get(*field_id);
                    if field.name == field_name {
                        return Some(field.symbol.into_global(module_id));
                    }
                }
            }
        }

        None
    }
}
