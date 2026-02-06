use destack_dir::{
    GlobalSymbolId, LocalNodeIdAny, LocalTypeId, StaticArgument, StaticExpression, SymbolKind,
    SymbolSpace, SymbolTable, SymbolType, Type, TypeLiteral, TypeTable, WellKnownSymbol,
};
use destack_workspace::{Module, ProfileId};

use crate::Compiler;

/// Control how canonical symbol resolution treats aliases.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub(crate) enum CanonicalSymbolMode {
    /// Follow target and canonical links without preserving aliases.
    FollowAliases,
    /// Preserve alias identity when walking targets.
    PreserveAliases,
}

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Resolve the canonical symbol for a reference with explicit alias handling.
    pub(crate) fn canonical_symbol_id(
        &self,
        module: &Module,
        symbols: &SymbolTable,
        profile: ProfileId,
        symbol: GlobalSymbolId,
        mode: CanonicalSymbolMode,
    ) -> GlobalSymbolId {
        let mut current_symbol = symbol;
        let mut visited = Vec::new();

        // walk target and canonical chains until we stabilize
        loop {
            if visited.contains(&current_symbol) {
                break current_symbol;
            }
            visited.push(current_symbol);

            let (symbol_ty, canonical_symbol, target_symbol) = self.with_module_symbols_or_local(
                module,
                profile,
                current_symbol.module_id,
                symbols,
                |_, owner_symbols| {
                    let symbol_entry = owner_symbols.get_symbol(current_symbol.local_id);
                    (
                        symbol_entry.ty,
                        symbol_entry.canonical_symbol,
                        symbol_entry.target_symbol,
                    )
                },
            );

            // preserve alias identity
            if matches!(mode, CanonicalSymbolMode::PreserveAliases)
                && matches!(symbol_ty, SymbolType::TypeAlias | SymbolType::Newtype)
            {
                break current_symbol;
            }

            // otherwise, follow canonical links
            if let Some(canonical_symbol) = canonical_symbol
                && matches!(mode, CanonicalSymbolMode::FollowAliases)
            {
                break canonical_symbol;
            }

            if let Some(target_symbol) = target_symbol {
                current_symbol = target_symbol;
            } else {
                break current_symbol;
            }
        }
    }

    /// Return the array well-known kind for a symbol when applicable.
    pub(crate) fn well_known_array_kind(
        &self,
        profile: ProfileId,
        symbol: GlobalSymbolId,
    ) -> Option<WellKnownSymbol> {
        if self.is_well_known_symbol(profile, symbol, WellKnownSymbol::Array) {
            return Some(WellKnownSymbol::Array);
        }
        if self.is_well_known_symbol(profile, symbol, WellKnownSymbol::ReadonlyArray) {
            return Some(WellKnownSymbol::ReadonlyArray);
        }
        None
    }

    /// Resolve merged namespace symbols into the type space when possible.
    pub(crate) fn merged_type_symbol_id(
        &self,
        module: &Module,
        symbols: &SymbolTable,
        profile: ProfileId,
        symbol: GlobalSymbolId,
    ) -> GlobalSymbolId {
        // reuse local symbol table when possible
        if symbol.module_id == module.id {
            let symbol_entry = symbols.get_symbol(symbol.local_id);
            // stop when the symbol is not a namespace
            if symbol_entry.kind != SymbolKind::Namespace {
                return symbol;
            }

            // stop when the namespace has no merge group
            let Some(group_id) = symbol_entry.merge_group else {
                return symbol;
            };

            // select a merged type or type value symbol when available
            let candidate =
                symbols
                    .merge_group_symbols(group_id)
                    .iter()
                    .copied()
                    .find(|group_symbol| {
                        let merged_symbol = symbols.get_symbol(*group_symbol);
                        merged_symbol.kind != SymbolKind::Namespace
                            && matches!(
                                merged_symbol.space,
                                SymbolSpace::Type | SymbolSpace::TypeValue
                            )
                    });

            return candidate
                .map(|candidate| candidate.into_global(module.id))
                .unwrap_or(symbol);
        }

        self.with_module_symbols(
            module,
            profile,
            symbol.module_id,
            |owner_module, owner_symbols| {
                let symbol_entry = owner_symbols.get_symbol(symbol.local_id);
                // stop when the symbol is not a namespace
                if symbol_entry.kind != SymbolKind::Namespace {
                    return symbol;
                }

                // stop when the namespace has no merge group
                let Some(group_id) = symbol_entry.merge_group else {
                    return symbol;
                };

                // select a merged type or type value symbol when available
                let candidate = owner_symbols
                    .merge_group_symbols(group_id)
                    .iter()
                    .copied()
                    .find(|group_symbol| {
                        let merged_symbol = owner_symbols.get_symbol(*group_symbol);
                        merged_symbol.kind != SymbolKind::Namespace
                            && matches!(
                                merged_symbol.space,
                                SymbolSpace::Type | SymbolSpace::TypeValue
                            )
                    });

                candidate
                    .map(|candidate| candidate.into_global(owner_module.id))
                    .unwrap_or(symbol)
            },
        )
    }

    /// Normalize well-known type references into structural types when possible.
    pub(crate) fn normalize_well_known_type_reference(
        &self,
        _module: &Module,
        _symbols: &SymbolTable,
        profile: ProfileId,
        source_id: LocalNodeIdAny,
        symbol: GlobalSymbolId,
        well_known: WellKnownSymbol,
        static_arguments: Option<&[StaticArgument]>,
        types: &mut TypeTable,
    ) -> Option<Type> {
        let element = static_arguments
            .and_then(|arguments| arguments.first())
            .map(|argument| self.static_argument_type(argument, source_id, types));

        match well_known {
            WellKnownSymbol::Array => Some(Type::Array {
                element,
                is_readonly: false,
            }),
            WellKnownSymbol::ReadonlyArray => Some(Type::Array {
                element,
                is_readonly: true,
            }),
            _ => {
                if self.is_well_known_symbol(profile, symbol, well_known) {
                    Some(Type::Array {
                        element,
                        is_readonly: false,
                    })
                } else {
                    None
                }
            }
        }
    }

    /// Convert a static argument into a type id for type evaluation.
    fn static_argument_type(
        &self,
        argument: &StaticArgument,
        source_id: LocalNodeIdAny,
        types: &mut TypeTable,
    ) -> LocalTypeId {
        let ty = match argument {
            StaticArgument::Evaluated { value, .. } => match value {
                StaticExpression::Type { ty } => return *ty,
                StaticExpression::TypeLiteral { value } => Type::TypeLiteral {
                    value: value.clone(),
                },
                StaticExpression::ScalarLiteral { value } => Type::TypeLiteral {
                    value: TypeLiteral::ScalarLiteral(value.clone()),
                },
                _ => Type::TypeLiteral {
                    value: TypeLiteral::Unknown,
                },
            },
            StaticArgument::Unevaluated { .. } => Type::TypeLiteral {
                value: TypeLiteral::Unknown,
            },
        };

        types.insert_type_from_any(ty, source_id)
    }
}
