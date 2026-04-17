use std::collections::HashSet;

use destack_artifact::ArtifactKey;
use destack_dir::{
    DependencyItem, GlobalSymbolId, LocalNodeIdAny, LocalTypeId, NodeType, StaticArgument,
    StaticExpression, SymbolKind, SymbolSpace, SymbolType, Type, TypeLiteral, TypeTable,
    WellKnownSymbol,
};
use destack_source::ModuleId;
use destack_workspace::{ProfileId, Revision};

use crate::analyze::common::{ModuleSymbolView, TypeContext};
use crate::{Compiler, RequirementError};

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
    /// Resolve one merged namespace symbol into the type space using one symbol table.
    fn merged_type_symbol_in_table(
        &self,
        module_id: ModuleId,
        symbols: &destack_dir::SymbolTable,
        symbol: GlobalSymbolId,
    ) -> GlobalSymbolId {
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
            })
            .map(|candidate| candidate.into_global(module_id))
            .unwrap_or(symbol)
    }

    /// Resolve the canonical symbol from committed declared artifact state in one explicit revision.
    pub fn canonical_declared_artifact_symbol_for_revision(
        &self,
        revision: Revision,
        profile: ProfileId,
        symbol: GlobalSymbolId,
    ) -> GlobalSymbolId {
        let mut current_symbol = symbol;
        let mut visited = HashSet::new();

        // follow the committed declared artifact chain
        loop {
            if !visited.insert(current_symbol) {
                return current_symbol;
            }

            let Some(dir) =
                self.repository
                    .dir_declared(revision, current_symbol.module_id, profile)
            else {
                return current_symbol;
            };

            let symbol_entry = dir.symbols.get_symbol(current_symbol.local_id);
            let normalized_symbol = GlobalSymbolId::new(
                current_symbol.module_id,
                current_symbol.local_id.with_type(symbol_entry.ty),
            );
            if let Some(canonical_symbol) = symbol_entry.canonical_symbol
                && canonical_symbol != normalized_symbol
            {
                current_symbol = canonical_symbol;
                continue;
            }

            if let Some(target_symbol) = symbol_entry.target_symbol
                && target_symbol != normalized_symbol
            {
                current_symbol = target_symbol;
                continue;
            }

            if let Some(primary_declaration) = symbol_entry.primary_declaration
                && primary_declaration.local_id.ty == NodeType::DependencyItem
            {
                let dependency_id = primary_declaration
                    .try_into_typed::<DependencyItem>()
                    .unwrap_or_else(|_| panic!("dependency item conversion failed"));
                let dependency = dir.tree.get::<DependencyItem>(dependency_id.local_id);
                if let Some(target_symbol) = dependency.target_symbol()
                    && target_symbol != normalized_symbol
                {
                    current_symbol = target_symbol;
                    continue;
                }
            }

            return normalized_symbol;
        }
    }

    /// Resolve the canonical symbol from committed declared artifact state.
    pub fn canonical_declared_artifact_symbol(
        &self,
        profile: ProfileId,
        symbol: GlobalSymbolId,
    ) -> GlobalSymbolId {
        let revision = self.current_context().revision();

        self.canonical_declared_artifact_symbol_for_revision(revision, profile, symbol)
    }

    /// Resolve the canonical symbol for a reference with one exact DIR artifact family.
    pub(crate) fn canonical_symbol_id_for_artifact(
        &self,
        view: ModuleSymbolView<'_>,
        symbol: GlobalSymbolId,
        mode: CanonicalSymbolMode,
        artifact_key: fn(ModuleId, ProfileId) -> ArtifactKey,
    ) -> Result<GlobalSymbolId, RequirementError> {
        let mut current_symbol = symbol;
        let mut visited = Vec::new();

        // walk target and canonical chains until we stabilize
        loop {
            if visited.contains(&current_symbol) {
                break Ok(current_symbol);
            }
            visited.push(current_symbol);

            let (symbol_ty, canonical_symbol, target_symbol) = self
                .with_module_symbols_or_local_for_artifact(
                    view.compiler_context,
                    view.module,
                    view.profile,
                    current_symbol.module_id,
                    view.symbols,
                    artifact_key,
                    |_, owner_symbols| {
                        let symbol_entry = owner_symbols.get_symbol(current_symbol.local_id);
                        (
                            symbol_entry.ty,
                            symbol_entry.canonical_symbol,
                            symbol_entry.target_symbol,
                        )
                    },
                )?;

            // preserve alias identity
            if matches!(mode, CanonicalSymbolMode::PreserveAliases)
                && matches!(symbol_ty, SymbolType::TypeAlias | SymbolType::Newtype)
            {
                break Ok(current_symbol);
            }

            // otherwise, follow canonical links
            if let Some(canonical_symbol) = canonical_symbol
                && matches!(mode, CanonicalSymbolMode::FollowAliases)
            {
                break Ok(canonical_symbol);
            }

            if let Some(target_symbol) = target_symbol {
                current_symbol = target_symbol;
            } else {
                break Ok(current_symbol);
            }
        }
    }

    /// Resolve a symbol to the declaration owner symbol with one exact DIR artifact family.
    pub(crate) fn declaration_symbol_id_for_artifact(
        &self,
        view: ModuleSymbolView<'_>,
        symbol: GlobalSymbolId,
        artifact_key: fn(ModuleId, ProfileId) -> ArtifactKey,
    ) -> Result<Option<GlobalSymbolId>, RequirementError> {
        let mut current_symbol = self.canonical_symbol_id_for_artifact(
            view,
            symbol,
            CanonicalSymbolMode::FollowAliases,
            artifact_key,
        )?;
        let mut visited_symbols = HashSet::new();

        loop {
            if !visited_symbols.insert(current_symbol) {
                return Ok(None);
            }

            let (normalized_symbol, is_declaration, target_symbol, canonical_symbol) = self
                .with_module_symbols_or_local_for_artifact(
                    view.compiler_context,
                    view.module,
                    view.profile,
                    current_symbol.module_id,
                    view.symbols,
                    artifact_key,
                    |owner_module, owner_symbols| {
                        let symbol_entry = owner_symbols.get_symbol(current_symbol.local_id);
                        let normalized_symbol = GlobalSymbolId::new(
                            owner_module.id,
                            current_symbol.local_id.with_type(symbol_entry.ty),
                        );
                        let is_declaration =
                            symbol_entry.primary_declaration.is_some_and(|declaration| {
                                declaration.local_id.ty == NodeType::Declaration
                                    && symbol_entry.ty != SymbolType::Void
                            });
                        (
                            normalized_symbol,
                            is_declaration,
                            symbol_entry.target_symbol,
                            symbol_entry.canonical_symbol,
                        )
                    },
                )?;

            if is_declaration {
                return Ok(Some(normalized_symbol));
            }

            let Some(next_symbol) = target_symbol.or(canonical_symbol) else {
                return Ok(None);
            };
            current_symbol = self.canonical_symbol_id_for_artifact(
                view,
                next_symbol,
                CanonicalSymbolMode::FollowAliases,
                artifact_key,
            )?;
        }
    }

    /// Resolve the canonical symbol for a reference with explicit alias handling.
    pub(crate) fn canonical_symbol_id(
        &self,
        view: ModuleSymbolView<'_>,
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

            let Some((symbol_ty, canonical_symbol, target_symbol)) = self
                .with_module_symbols_or_local_for_artifact(
                    view.compiler_context,
                    view.module,
                    view.profile,
                    current_symbol.module_id,
                    view.symbols,
                    destack_artifact::ArtifactKey::dir_declared,
                    |_, owner_symbols| {
                        let symbol_entry = owner_symbols.get_symbol(current_symbol.local_id);
                        (
                            symbol_entry.ty,
                            symbol_entry.canonical_symbol,
                            symbol_entry.target_symbol,
                        )
                    },
                )
                .ok()
            else {
                break current_symbol;
            };

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

    /// Resolve a symbol to the declaration owner symbol when one exists.
    pub(crate) fn declaration_symbol_id(
        &self,
        view: ModuleSymbolView<'_>,
        symbol: GlobalSymbolId,
    ) -> Option<GlobalSymbolId> {
        let mut current_symbol =
            self.canonical_symbol_id(view, symbol, CanonicalSymbolMode::FollowAliases);
        let mut visited_symbols = HashSet::new();

        loop {
            if !visited_symbols.insert(current_symbol) {
                return None;
            }

            let (normalized_symbol, is_declaration, target_symbol, canonical_symbol) = self
                .with_module_symbols_or_local_for_artifact(
                    view.compiler_context,
                    view.module,
                    view.profile,
                    current_symbol.module_id,
                    view.symbols,
                    destack_artifact::ArtifactKey::dir_declared,
                    |owner_module, owner_symbols| {
                        let symbol_entry = owner_symbols.get_symbol(current_symbol.local_id);
                        let normalized_symbol = GlobalSymbolId::new(
                            owner_module.id,
                            current_symbol.local_id.with_type(symbol_entry.ty),
                        );
                        let is_declaration =
                            symbol_entry.primary_declaration.is_some_and(|declaration| {
                                declaration.local_id.ty == NodeType::Declaration
                                    && symbol_entry.ty != SymbolType::Void
                            });
                        (
                            normalized_symbol,
                            is_declaration,
                            symbol_entry.target_symbol,
                            symbol_entry.canonical_symbol,
                        )
                    },
                )
                .ok()?;

            if is_declaration {
                return Some(normalized_symbol);
            }

            let next_symbol = target_symbol.or(canonical_symbol)?;
            current_symbol =
                self.canonical_symbol_id(view, next_symbol, CanonicalSymbolMode::FollowAliases);
        }
    }

    /// Return the array well-known kind for a symbol when applicable.
    pub(crate) fn well_known_array_kind(
        &self,
        profile: ProfileId,
        symbol: GlobalSymbolId,
    ) -> Option<WellKnownSymbol> {
        if self.is_well_known_symbol(profile, symbol, WellKnownSymbol::FixedArray) {
            return Some(WellKnownSymbol::FixedArray);
        }
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
        view: ModuleSymbolView<'_>,
        symbol: GlobalSymbolId,
    ) -> GlobalSymbolId {
        self.merged_type_symbol_id_for_artifact(
            view,
            symbol,
            destack_artifact::ArtifactKey::dir_declared,
        )
    }

    /// Resolve merged namespace symbols into the type space from one exact DIR artifact family.
    pub(crate) fn merged_type_symbol_id_for_artifact(
        &self,
        view: ModuleSymbolView<'_>,
        symbol: GlobalSymbolId,
        artifact_key: fn(ModuleId, ProfileId) -> ArtifactKey,
    ) -> GlobalSymbolId {
        // reuse local symbol table when possible
        if symbol.module_id == view.module.id {
            return self.merged_type_symbol_in_table(view.module.id, view.symbols, symbol);
        }

        self.with_module_symbols_or_local_for_artifact(
            view.compiler_context,
            view.module,
            view.profile,
            symbol.module_id,
            view.symbols,
            artifact_key,
            |_, symbols| self.merged_type_symbol_in_table(symbol.module_id, symbols, symbol),
        )
        .unwrap_or(symbol)
    }

    /// Normalize well-known type references into structural types when possible.
    pub(crate) fn normalize_well_known_type_reference(
        &self,
        ctx: &mut TypeContext<'_>,
        source_id: LocalNodeIdAny,
        well_known: WellKnownSymbol,
        generic_arguments: Option<&[StaticArgument]>,
    ) -> Option<Type> {
        let element = generic_arguments
            .and_then(|arguments| arguments.first())
            .map(|argument| self.generic_argument_type(argument, source_id, ctx.types));

        match well_known {
            WellKnownSymbol::FixedArray => {
                let arguments = generic_arguments?;
                let element_argument = arguments.first()?;
                let count_argument = arguments.get(1)?;

                let element = self.generic_argument_type(element_argument, source_id, ctx.types);
                let count = self.generic_argument_type(count_argument, source_id, ctx.types);
                Some(Type::ArraySized {
                    element,
                    count,
                    is_readonly: false,
                })
            }
            WellKnownSymbol::Array => Some(Type::Array {
                element,
                is_readonly: false,
            }),
            WellKnownSymbol::ReadonlyArray => Some(Type::Array {
                element,
                is_readonly: true,
            }),
            _ => None,
        }
    }

    /// Convert a static argument into a type id for type evaluation.
    fn generic_argument_type(
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
