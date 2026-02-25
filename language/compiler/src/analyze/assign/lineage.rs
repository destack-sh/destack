use super::*;
use crate::analyze::common::TypeTablesContext;

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Check if source_symbol is a subtype of target_symbol via lineage (follows inheritance chain).
    /// (This also checks visible extensions that add `implements` clauses to the source type.)
    pub fn is_type_lineage_assignable(
        &self,
        module: &Module,
        profile: ProfileId,
        source_symbol: GlobalSymbolId,
        target_symbol: GlobalSymbolId,
        symbols: &SymbolTable,
        types: &TypeTable,
    ) -> bool {
        let mut visited = HashSet::new();
        self.is_type_lineage_assignable_inner(
            module,
            profile,
            source_symbol,
            target_symbol,
            symbols,
            types,
            &mut visited,
        )
    }

    /// Check if source_symbol is a subtype of target_symbol via lineage (follows inheritance chain).
    pub(super) fn is_type_lineage_assignable_inner(
        &self,
        module: &Module,
        profile: ProfileId,
        source_symbol: GlobalSymbolId,
        target_symbol: GlobalSymbolId,
        symbols: &SymbolTable,
        types: &TypeTable,
        visited: &mut HashSet<GlobalSymbolId>,
    ) -> bool {
        // avoid cycles across inheritance graphs
        if !visited.insert(source_symbol) {
            return false;
        }

        // step 1: check the type's own lineage
        if let Some(lineage) = self.lineage_for_symbol(module, profile, source_symbol, types) {
            // check direct extends
            if let Some(extends) = lineage.extends {
                if extends == target_symbol {
                    return true;
                }
                if self.is_type_lineage_assignable_inner(
                    module,
                    profile,
                    extends,
                    target_symbol,
                    symbols,
                    types,
                    visited,
                ) {
                    return true;
                }
            }

            // check direct implements
            for &implements in &lineage.implements {
                if implements == target_symbol {
                    return true;
                }
                if self.is_type_lineage_assignable_inner(
                    module,
                    profile,
                    implements,
                    target_symbol,
                    symbols,
                    types,
                    visited,
                ) {
                    return true;
                }
            }

            // check embedded types (composition can also contribute to assignability)
            for &embedded in &lineage.embedded {
                if embedded == target_symbol {
                    return true;
                }
                if self.is_type_lineage_assignable_inner(
                    module,
                    profile,
                    embedded,
                    target_symbol,
                    symbols,
                    types,
                    visited,
                ) {
                    return true;
                }
            }
        }

        // step 2: check visible extensions that add implements clauses
        let extension_symbols = match self.visible_extension_symbols_for_target(
            module,
            profile,
            symbols,
            types,
            source_symbol,
        ) {
            Ok(symbols) => symbols,
            Err(AnalyzeError::Yield { .. }) => return false,
            Err(error) => {
                self.error(error);
                return false;
            }
        };
        for extension_symbol in extension_symbols {
            let extension =
                match self.extension_for_symbol_in_module(module, profile, extension_symbol, types)
                {
                    Ok(extension) => extension,
                    Err(AnalyzeError::Yield { .. }) => return false,
                    Err(error) => {
                        self.error(error);
                        return false;
                    }
                };
            let Some(extension) = extension else {
                continue;
            };
            let extension_is_visible = match extension.kind {
                ExtensionKind::Inherent => true,
                ExtensionKind::Local => extension.symbol.module_id == module.id,
                ExtensionKind::Nominal => true,
            };
            if !extension_is_visible {
                continue;
            }
            let lineage = match self.extension_lineage_for_symbol_in_module(
                module,
                profile,
                extension_symbol,
                types,
            ) {
                Ok(lineage) => lineage,
                Err(AnalyzeError::Yield { .. }) => return false,
                Err(error) => {
                    self.error(error);
                    return false;
                }
            };
            let Some(lineage) = lineage else {
                continue;
            };

            // extensions typically only add implements, but check all for completeness
            for &implements in &lineage.implements {
                if implements == target_symbol {
                    return true;
                }
                if self.is_type_lineage_assignable_inner(
                    module,
                    profile,
                    implements,
                    target_symbol,
                    symbols,
                    types,
                    visited,
                ) {
                    return true;
                }
            }
        }

        false
    }

    /// Return the lineage for a symbol.
    pub(super) fn lineage_for_symbol(
        &self,
        module: &Module,
        profile: ProfileId,
        symbol: GlobalSymbolId,
        types: &TypeTable,
    ) -> Option<Lineage> {
        if let Some(lineage) = types.get_lineage_for_symbol(symbol) {
            return Some(lineage.clone());
        }

        // bail when the symbol is in the same module (lineage above is already cached)
        if symbol.module_id == module.id {
            return None;
        }

        self.with_module_types_at_stage(
            module,
            profile,
            symbol.module_id,
            AnalyzeDependencyStage::Declare,
            |_, remote_types| remote_types.get_lineage_for_symbol(symbol).cloned(),
        )
        .ok()
        .flatten()
    }

    /// Normalize a conditional type for assignability when it resolves in flow mode.
    pub(super) fn normalize_conditional_for_assignability(
        &self,
        tables: &mut TypeTablesContext<'_>,
        type_id: LocalTypeId,
    ) -> Option<LocalTypeId> {
        // only conditional types participate in flow normalization
        if !matches!(tables.types.get_type(type_id), Type::Conditional { .. }) {
            return None;
        }
        // normalize in flow mode to resolve conditionals
        let normalized = {
            let mut normalize_visited = Vec::new();
            self.normalize_type_inner(
                &mut tables.reborrow(),
                type_id,
                NormalizationMode::Flow,
                RelationMode::ASSIGN,
                &mut normalize_visited,
            )
        };
        if normalized == type_id {
            return None;
        }
        Some(normalized)
    }

    /// Check whether enum backing coercions are disallowed for assignability.
    pub(super) fn blocks_enum_backing_assignability(
        &self,
        source: &Type,
        target: &Type,
        types: &TypeTable,
    ) -> bool {
        // only reject enum backing coercions
        if self.enum_symbol_for_type(source, types).is_none() {
            return false;
        }

        matches!(
            target,
            Type::TypeLiteral {
                value: TypeLiteral::Primitive(
                    PrimitiveType::Number
                        | PrimitiveType::Int(_)
                        | PrimitiveType::Float(_)
                        | PrimitiveType::String
                )
            } | Type::TypeLiteral {
                value: TypeLiteral::ScalarLiteral(
                    ScalarLiteral::Integer(_) | ScalarLiteral::String(_)
                )
            }
        )
    }
}
