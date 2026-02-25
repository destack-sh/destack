use super::*;
use crate::analyze::common::TypeTablesContext;

impl Compiler {
    pub(crate) fn promise_type(
        &self,
        profile: ProfileId,
        value_type: Option<LocalTypeId>,
        source_id: LocalNodeIdAny,
        types: &mut TypeTable,
    ) -> Option<LocalTypeId> {
        let promise_symbol = self.get_well_known_type_symbol(profile, WellKnownSymbol::Promise)?;

        // default missing type arguments to unknown
        let value_type = value_type.unwrap_or_else(|| {
            types.insert_type_from_any(
                Type::TypeLiteral {
                    value: TypeLiteral::Unknown,
                },
                source_id,
            )
        });

        // make promise type
        let static_arguments = vec![StaticArgument::Evaluated {
            name: None,
            value: StaticExpression::Type { ty: value_type },
        }];
        Some(types.insert_type_from_any(
            Type::Reference {
                symbol: promise_symbol,
                static_arguments: Some(static_arguments),
            },
            source_id,
        ))
    }

    /// Resolve generator context types from a declared return type.
    pub(crate) fn generator_context_types(
        &self,
        tables: &mut TypeTablesContext<'_>,
        source_id: LocalNodeIdAny,
        return_type: Option<LocalTypeId>,
    ) -> (LocalTypeId, LocalTypeId, LocalTypeId) {
        let unknown_ty_id = tables.types.insert_type_from_any(
            Type::TypeLiteral {
                value: TypeLiteral::Unknown,
            },
            source_id,
        );
        let Some(return_type_id) = return_type else {
            return (unknown_ty_id, unknown_ty_id, unknown_ty_id);
        };

        if let Some((yield_ty_id, return_ty_id, next_ty_id)) =
            self.generator_type_arguments(&mut tables.reborrow(), return_type_id)
        {
            return (yield_ty_id, return_ty_id, next_ty_id);
        }

        (unknown_ty_id, return_type_id, unknown_ty_id)
    }

    /// Unwrap a Promise reference into its value type when possible.
    pub(crate) fn unwrap_promise_type(
        &self,
        tables: &mut TypeTablesContext<'_>,
        type_id: LocalTypeId,
    ) -> Option<LocalTypeId> {
        let ty = tables.types.get_type(type_id);
        let symbol = ty.symbol()?;
        let static_arguments = match ty {
            Type::Reference {
                static_arguments, ..
            } => static_arguments.clone(),
            _ => return None,
        };

        // compare canonical symbols to avoid alias mismatches
        let canonical_symbol = self.canonical_symbol_id(
            tables.module,
            tables.symbols,
            tables.profile,
            symbol,
            CanonicalSymbolMode::FollowAliases,
        );
        let is_promise_symbol =
            self.is_well_known_symbol(tables.profile, canonical_symbol, WellKnownSymbol::Promise);
        if !is_promise_symbol {
            return None;
        }

        // resolve unevaluated promise arguments when possible
        let static_arguments = if let Some(static_arguments) = static_arguments.as_ref()
            && static_arguments
                .iter()
                .any(|arg| matches!(arg, StaticArgument::Unevaluated { .. }))
        {
            let source_id = tables.types.get_type_source(type_id);
            self.resolve_type_reference_static_arguments(
                &mut tables.reborrow(),
                source_id,
                symbol,
                Some(static_arguments.as_slice()),
                true,
            )
            .ok()
            .flatten()
            .or_else(|| Some(static_arguments.clone()))
        } else {
            static_arguments
        };

        let Some(first_argument) = static_arguments
            .as_ref()
            .and_then(|arguments| arguments.first())
        else {
            let ty = Type::TypeLiteral {
                value: TypeLiteral::Unknown,
            };
            return Some(tables.types.insert_type_from_type(ty, type_id));
        };

        // evaluate remaining unevaluated arguments as types when possible
        let mut argument = first_argument.clone();
        if let StaticArgument::Unevaluated { node } = argument {
            if let Ok(Some(evaluated)) =
                self.evaluate_static_argument_as_type(&mut tables.reborrow(), node)
            {
                argument = evaluated;
            }
        }

        Some(self.convert_static_argument_type(
            &argument,
            tables.types.get_type_source(type_id),
            tables.types,
        ))
    }

    /// Extract generator type arguments from a reference when possible.
    pub(crate) fn generator_type_arguments(
        &self,
        tables: &mut TypeTablesContext<'_>,
        type_id: LocalTypeId,
    ) -> Option<(LocalTypeId, LocalTypeId, LocalTypeId)> {
        let (symbol, static_arguments) = {
            let Type::Reference {
                symbol,
                static_arguments,
            } = tables.types.get_type(type_id)
            else {
                return None;
            };

            (*symbol, static_arguments.clone())
        };
        let source_id = tables.types.get_type_source(type_id);

        let canonical_symbol = self.canonical_symbol_id(
            tables.module,
            tables.symbols,
            tables.profile,
            symbol,
            CanonicalSymbolMode::FollowAliases,
        );

        let generator_name = self.program.strings.intern("Generator");
        let generator_symbol = self.get_declared_lib_symbol_from(
            tables.profile,
            generator_name,
            SymbolSpaceOrder::TypeThenValue,
        );
        let iterator_symbol =
            self.get_well_known_type_symbol(tables.profile, WellKnownSymbol::Iterator);

        let is_generator = generator_symbol.is_some_and(|symbol| symbol == canonical_symbol)
            || iterator_symbol.is_some_and(|symbol| symbol == canonical_symbol);
        if !is_generator {
            return None;
        }

        let resolved_arguments = self
            .resolve_type_reference_static_arguments(
                &mut tables.reborrow(),
                source_id,
                symbol,
                static_arguments.as_deref(),
                true,
            )
            .ok()
            .flatten();
        let arguments = resolved_arguments
            .as_ref()
            .or(static_arguments.as_ref())
            .map(|arguments| arguments.as_slice())
            .unwrap_or(&[]);

        let unknown_ty_id = tables.types.insert_type_from_any(
            Type::TypeLiteral {
                value: TypeLiteral::Unknown,
            },
            source_id,
        );

        let yield_ty_id = arguments
            .first()
            .map(|argument| self.convert_static_argument_type(argument, source_id, tables.types))
            .unwrap_or(unknown_ty_id);
        let return_ty_id = arguments
            .get(1)
            .map(|argument| self.convert_static_argument_type(argument, source_id, tables.types))
            .unwrap_or(unknown_ty_id);
        let next_ty_id = arguments
            .get(2)
            .map(|argument| self.convert_static_argument_type(argument, source_id, tables.types))
            .unwrap_or(unknown_ty_id);

        Some((yield_ty_id, return_ty_id, next_ty_id))
    }

    /// Resolve the awaited type for a value.
    pub(crate) fn unwrap_awaited_type(
        &self,
        tables: &mut TypeTablesContext<'_>,
        type_id: LocalTypeId,
    ) -> LocalTypeId {
        let mut visited = Vec::new();
        self.unwrap_awaited_type_inner(&mut tables.reborrow(), type_id, &mut visited)
    }

    /// Resolve the awaited type for a value with cycle detection.
    fn unwrap_awaited_type_inner(
        &self,
        tables: &mut TypeTablesContext<'_>,
        type_id: LocalTypeId,
        visited: &mut Vec<LocalTypeId>,
    ) -> LocalTypeId {
        // avoid infinite recursion in cyclic types
        if visited.contains(&type_id) {
            return type_id;
        }
        visited.push(type_id);

        // expand alias references so await sees concrete promise targets
        let mut expanded_id = type_id;
        let mut visited_aliases = HashSet::new();
        loop {
            let Type::Reference {
                symbol,
                static_arguments,
            } = tables.types.get_type(expanded_id).clone()
            else {
                break;
            };
            let symbol = self.normalize_reference_symbol_id(tables.module, tables.profile, symbol);
            if symbol.ty() != SymbolType::TypeAlias {
                break;
            }
            if !visited_aliases.insert(symbol) {
                break;
            }

            let arguments = static_arguments.as_deref().unwrap_or(&[]);
            let mut normalize_visited = Vec::new();
            let source_id = tables.types.get_type_source(expanded_id);
            let Some(next_id) = self.normalize_type_alias_reference_with_arguments(
                &mut tables.reborrow(),
                source_id,
                symbol,
                arguments,
                NormalizationMode::Assign,
                RelationMode::ALIAS_EXPANSION,
                &mut normalize_visited,
            ) else {
                break;
            };
            if next_id == expanded_id {
                break;
            }
            expanded_id = next_id;
        }
        let type_id = expanded_id;

        // materialize static arguments before awaiting promise targets
        let mut materialize_cache = TypeRewriteCache::new();
        let type_id = self.materialize_static_arguments_in_type(
            &mut tables.reborrow(),
            type_id,
            &mut materialize_cache,
        );

        // normalize after alias expansion to avoid cached alias results
        let normalized_id = self.normalize_type_with_relation(
            &mut tables.reborrow(),
            type_id,
            NormalizationMode::Assign,
            RelationMode::ALIAS_EXPANSION,
        );
        let type_id = if normalized_id != type_id {
            normalized_id
        } else {
            type_id
        };
        if !visited.contains(&type_id) {
            visited.push(type_id);
        }

        // keep any/unknown as-is
        if self.type_is_semantic_top_like(type_id, tables.types) {
            return type_id;
        }

        // distribute await across unions
        if let Type::Union { elements } = tables.types.get_type(type_id).clone() {
            let mut awaited_elements = Vec::new();

            // evaluate each union element independently
            for element_id in elements {
                let awaited_id =
                    self.unwrap_awaited_type_inner(&mut tables.reborrow(), element_id, visited);
                awaited_elements.push(awaited_id);
            }

            return self.union_type_from_list(awaited_elements, type_id, tables.types);
        }

        // unwrap promises when possible
        if let Some(inner_id) = self.unwrap_promise_type(&mut tables.reborrow(), type_id) {
            return self.unwrap_awaited_type_inner(&mut tables.reborrow(), inner_id, visited);
        }

        type_id
    }
}
