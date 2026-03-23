use super::*;
use crate::analyze::common::{ModuleTypeView, SymbolTypeView, TypeContext};

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Check if source_symbol is a subtype of target_symbol via lineage (follows inheritance chain).
    /// (This also checks visible extensions that add `implements` clauses to the source type.)
    pub(crate) fn is_type_lineage_assignable(
        &self,
        ctx: SymbolTypeView<'_>,
        source_symbol: GlobalSymbolId,
        target_symbol: GlobalSymbolId,
    ) -> bool {
        let mut visited = HashSet::new();
        self.is_type_lineage_assignable_inner(ctx, source_symbol, target_symbol, &mut visited)
    }

    /// Check if source_symbol is a subtype of target_symbol via lineage (follows inheritance chain).
    pub(super) fn is_type_lineage_assignable_inner(
        &self,
        ctx: SymbolTypeView<'_>,
        source_symbol: GlobalSymbolId,
        target_symbol: GlobalSymbolId,
        visited: &mut HashSet<GlobalSymbolId>,
    ) -> bool {
        // avoid cycles across inheritance graphs
        if !visited.insert(source_symbol) {
            return false;
        }

        // step 1: check the type's own lineage
        if let Some(lineage) = self.lineage_for_symbol(ctx.module_type_view(), source_symbol) {
            // check direct extends
            if let Some(extends) = lineage.extends {
                if extends == target_symbol {
                    return true;
                }
                if self.is_type_lineage_assignable_inner(ctx, extends, target_symbol, visited) {
                    return true;
                }
            }

            // check direct implements
            for &implements in &lineage.implements {
                if implements == target_symbol {
                    return true;
                }
                if self.is_type_lineage_assignable_inner(ctx, implements, target_symbol, visited) {
                    return true;
                }
            }

            // check embedded types (composition can also contribute to assignability)
            for &embedded in &lineage.embedded {
                if embedded == target_symbol {
                    return true;
                }
                if self.is_type_lineage_assignable_inner(ctx, embedded, target_symbol, visited) {
                    return true;
                }
            }
        }

        // step 2: check visible extensions that add implements clauses
        let Some(extension_symbols) =
            self.query_visible_extension_symbols_for_target(ctx, source_symbol)
        else {
            return false;
        };
        for extension_symbol in extension_symbols {
            let lineage = self.query_extension_lineage_for_symbol_in_module(
                ctx.module_type_view(),
                extension_symbol,
            );
            let Some(lineage) = lineage else {
                continue;
            };

            // extensions typically only add implements, but check all for completeness
            for &implements in &lineage.implements {
                if implements == target_symbol {
                    return true;
                }
                if self.is_type_lineage_assignable_inner(ctx, implements, target_symbol, visited) {
                    return true;
                }
            }
        }

        false
    }

    /// Return the lineage for a symbol.
    pub(super) fn lineage_for_symbol(
        &self,
        ctx: ModuleTypeView<'_>,
        symbol: GlobalSymbolId,
    ) -> Option<Lineage> {
        self.lineage_for_symbol_or_local_for_artifact(
            ctx.module,
            ctx.profile,
            symbol,
            ctx.types,
            destack_artifact::ArtifactKey::dir_declared,
        )
        .ok()
        .flatten()
    }

    /// Normalize a conditional type for assignability when it resolves in flow mode.
    pub(super) fn normalize_conditional_for_assignability(
        &self,
        ctx: &mut TypeContext<'_>,
        type_id: LocalTypeId,
    ) -> Option<LocalTypeId> {
        // only conditional types participate in flow normalization
        if !matches!(ctx.types.get_type(type_id), Type::Conditional { .. }) {
            return None;
        }
        // normalize in flow mode to resolve conditionals
        let normalized = {
            let mut normalize_visited = Vec::new();
            self.normalize_type_inner(
                &mut ctx.reborrow(),
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
