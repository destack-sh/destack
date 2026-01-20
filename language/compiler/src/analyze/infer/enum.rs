use crate::{AnalyzeError, AnalyzeResult, Compiler};
use destack_dir::{
    Declaration, EnumBackingType, EnumField, EnumFieldValue, Expression, GlobalSymbolId, IntType,
    LocalNodeId, NodeTree, NodeType, PrimitiveType, ScalarLiteral, StaticExpression, StringId,
    SymbolTable, Type, TypeLiteral, TypeTable,
};
use destack_workspace::{Module, ProfileId};

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Resolve enum field values from their declarations.
    pub(super) fn infer_enum_field_values(
        &self,
        module: &Module,
        profile: ProfileId,
        enum_symbol: GlobalSymbolId,
        fields: &[LocalNodeId<EnumField>],
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
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
            let field = tree.get(*field_id);
            let field_symbol = field.symbol.into_global(module.id);

            // resolve explicit values first
            let explicit_value = if let Some(value_id) = field.value {
                let static_value = self
                    .evaluate_static_expression_value(
                        module,
                        profile,
                        value_id,
                        tree,
                        symbols,
                        types,
                        Some(enum_symbol),
                    )?
                    .ok_or(AnalyzeError::InvalidEnumFieldValue {
                        node: field_id
                            .into_global_any(module.id)
                            .into_anchored(Some(profile)),
                    })?;
                Some((
                    value_id,
                    self.enum_field_value_from_static(&static_value, *field_id, module, profile)?,
                ))
            } else {
                None
            };

            // select or validate backing type
            let field_backing_type = if let Some((value_id, value)) = explicit_value {
                match value {
                    EnumFieldValue::Int(_) => {
                        let int_type = self
                            .enum_int_type_for_expression(module, profile, value_id, types)
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
                        EnumBackingType::Int(int_type) => types.insert_type_from(
                            Type::TypeLiteral {
                                value: TypeLiteral::Primitive(PrimitiveType::Int(int_type)),
                            },
                            *field_id,
                        ),
                        EnumBackingType::String => types.insert_type_from(
                            Type::TypeLiteral {
                                value: TypeLiteral::Primitive(PrimitiveType::String),
                            },
                            *field_id,
                        ),
                    };
                    return Err(AnalyzeError::InvalidEnumBackingType {
                        node: field_id
                            .into_global_any(module.id)
                            .into_anchored(Some(profile)),
                        ty: backing_type_id.into_global(module.id),
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
                                .into_global_any(module.id)
                                .into_anchored(Some(profile)),
                        });
                    }
                    next_value =
                        value
                            .checked_add(1)
                            .ok_or(AnalyzeError::InvalidEnumFieldValue {
                                node: field_id
                                    .into_global_any(module.id)
                                    .into_anchored(Some(profile)),
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
                                    .into_global_any(module.id)
                                    .into_anchored(Some(profile)),
                            })?;
                    has_next = true;

                    let is_signed = self.enum_int_type_is_signed(int_type);
                    if !is_signed && value < 0 {
                        return Err(AnalyzeError::InvalidEnumFieldValue {
                            node: field_id
                                .into_global_any(module.id)
                                .into_anchored(Some(profile)),
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
                            .into_global_any(module.id)
                            .into_anchored(Some(profile)),
                    });
                }
                _ => {
                    return Err(AnalyzeError::InvalidEnumFieldValue {
                        node: field_id
                            .into_global_any(module.id)
                            .into_anchored(Some(profile)),
                    });
                }
            };

            // record the resolved value
            types.set_enum_field_value(field_symbol, value);
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
        module: &Module,
        enum_symbol: GlobalSymbolId,
        target_symbol: GlobalSymbolId,
        symbols: &SymbolTable,
        types: &TypeTable,
    ) -> Option<EnumFieldValue> {
        // only allow local enum member references
        if target_symbol.module_id != module.id {
            return None;
        }

        // ensure the target is an enum field on this enum
        let target_entry = symbols.get_symbol(target_symbol.local_id);
        let primary = target_entry.primary_declaration?;
        if primary.local_id.ty != NodeType::EnumField {
            return None;
        }
        let scope = symbols.get_scope_by_symbol(target_symbol.local_id);
        let scope_owner = scope.owner_id?;
        if scope_owner.into_global(module.id) != enum_symbol {
            return None;
        }

        types.get_enum_field_value(target_symbol)
    }

    /// Resolve the integer backing type for an enum expression.
    fn enum_int_type_for_expression(
        &self,
        module: &Module,
        profile: ProfileId,
        expression_id: LocalNodeId<Expression>,
        types: &TypeTable,
    ) -> Option<IntType> {
        // resolve the enum member value type
        let value_type_id = types
            .get_declared_or_inferred_type_id(expression_id.into_global_any(module.id))
            .ok_or(AnalyzeError::MissingType {
                node: expression_id
                    .into_global_any(module.id)
                    .into_anchored(Some(profile)),
            })
            .ok()?;
        let value_type = types.get_type(value_type_id);

        // map the value type to a backing integer type
        match value_type {
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
        }
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
        enum_symbol: GlobalSymbolId,
        field_name: StringId,
        tree: &NodeTree,
        symbols: &SymbolTable,
    ) -> Option<GlobalSymbolId> {
        // only resolve within the same module
        if enum_symbol.module_id != module.id {
            return None;
        }

        // collect enum declarations for the symbol
        let symbol_entry = symbols.get_symbol(enum_symbol.local_id);
        let mut declaration_ids = Vec::new();

        // include the primary declaration when available
        if let Some(primary) = symbol_entry.primary_declaration
            && primary.local_id.ty == NodeType::Declaration
        {
            declaration_ids.push(LocalNodeId::<Declaration>::new(primary.local_id.id));
        }

        // include secondary declarations when present
        if let Some(secondaries) = symbol_entry.secondary_declarations.as_deref() {
            for declaration in secondaries {
                if declaration.local_id.ty == NodeType::Declaration {
                    declaration_ids.push(LocalNodeId::<Declaration>::new(declaration.local_id.id));
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
                    return Some(field.symbol.into_global(module.id));
                }
            }
        }

        None
    }
}
