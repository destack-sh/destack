use destack_dir::{LocalNodeId, Module, NodeTree, Pattern, PatternField, SymbolTable};

use crate::{Compiler, ResolveError, ResolveResult};

impl Compiler {
    /// Resolve a Pattern.
    pub(super) fn resolve_pattern(
        &self,
        module: &Module,
        pattern_id: LocalNodeId<Pattern>,
        tree: &mut NodeTree,
        _symbols: &mut SymbolTable,
    ) -> ResolveResult<()> {
        let pattern = tree.get(pattern_id);
        let pattern: Pattern = match pattern {
            Pattern::UnresolvedTuple { ty: _, fields } => Pattern::Tuple {
                target_symbol: None, // TODO #Incomplete: resolve patterns
                fields: fields.clone(),
            },
            Pattern::UnresolvedStruct { ty: _, fields: _ } => {
                return Err(ResolveError::UnsupportedNode {
                    node: pattern_id.into_global_any(module.id),
                });
            }
            _ => return Ok(()),
        };
        *tree.get_mut(pattern_id) = pattern;

        Ok(())
    }

    /// Resolve a PatternField.
    pub(super) fn resolve_pattern_field(
        &self,
        module: &Module,
        pattern_field_id: LocalNodeId<PatternField>,
        tree: &mut NodeTree,
        _symbols: &mut SymbolTable,
    ) -> ResolveResult<()> {
        let pattern_field = tree.get(pattern_field_id);

        let pattern_field: PatternField = match pattern_field {
            PatternField::UnresolvedNamed {
                mutability: _,
                name: _,
                pattern: _,
                default: _,
                symbol: _,
            } => {
                return Err(ResolveError::UnsupportedNode {
                    node: pattern_field_id.into_global_any(module.id),
                });
            }
            PatternField::UnresolvedAlias {
                mutability: _,
                name: _,
                alias: _,
                default: _,
                symbol: _,
            } => {
                return Err(ResolveError::UnsupportedNode {
                    node: pattern_field_id.into_global_any(module.id),
                });
            }
            PatternField::UnresolvedPositional { pattern } => {
                PatternField::Positional { pattern: *pattern }
            }

            _ => return Ok(()),
        };
        *tree.get_mut(pattern_field_id) = pattern_field;

        Ok(())
    }
}
