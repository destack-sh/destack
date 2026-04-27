use crate::{Compiler, ResolveResult};
use destack_artifact::ExportedSymbolTable;
use destack_dir::{
    Declaration, DependencyKind, ImportAliasTarget, LocalNodeId, LocalScopeId, LocalSymbolId,
    SymbolSpaceOrder, SymbolTable, Tree, TypeExpression,
};
use destack_workspace::{Module, ProfileId};

impl Compiler {
    /// Resolve a Declaration node (updates target_symbol if applicable).
    pub(crate) fn resolve_declaration(
        &self,
        revision: destack_workspace::Revision,
        module: &Module,
        profile: ProfileId,
        tree: &mut Tree,
        symbols: &mut SymbolTable,
        namespace_symbol: LocalSymbolId,
        namespace_scope: LocalScopeId,
        global_augmentation_scope: LocalScopeId,
        exported_symbols: &ExportedSymbolTable,
        declaration_id: LocalNodeId<Declaration>,
    ) -> ResolveResult<()> {
        let declaration = tree.get(declaration_id).clone();
        match declaration {
            Declaration::Extension(declaration) => {
                // bail if already resolved
                if declaration.target_symbol.is_some() {
                    return Ok(());
                }

                // resolve the target symbol for the extension target
                let resolved_target_symbol =
                    self.nominal_owner_symbol_for_type_expression(tree, declaration.target_type);
                if let Some(resolved_target_symbol) = resolved_target_symbol
                    && let Declaration::Extension(declaration) = tree.get_mut(declaration_id)
                {
                    declaration.target_symbol = Some(resolved_target_symbol);
                }

                Ok(())
            }

            Declaration::Type(declaration) => {
                let symbol_id = declaration.symbol;
                let symbol = symbols.get_symbol(symbol_id);

                // bail if symbol already has a target_symbol
                if symbol.target_symbol.is_some() {
                    return Ok(());
                }

                // only structural aliases should inherit target symbols
                if declaration.is_nominal {
                    return Ok(());
                }

                // extract the aliased type expression
                let value_expr: &TypeExpression = tree.get(declaration.value);

                // extract the target symbol from the resolved reference expressions
                if let Some(resolved_target_symbol) = value_expr.target_symbol() {
                    // set the type alias symbol's target_symbol
                    symbols
                        .get_symbol_mut(symbol_id)
                        .resolve_to(resolved_target_symbol);
                }

                Ok(())
            }

            Declaration::ImportAlias(declaration) => {
                let symbol_id = declaration.symbol;
                let symbol = symbols.get_symbol(symbol_id);

                // bail if symbol already has a target_symbol
                if symbol.target_symbol.is_some() {
                    return Ok(());
                }

                // resolve the alias target from the stored path syntax
                if let ImportAliasTarget::Path { path } = &declaration.target {
                    let space_order = match declaration.kind {
                        DependencyKind::Type => SymbolSpaceOrder::TypeOnly,
                        DependencyKind::Value => SymbolSpaceOrder::ValueThenType,
                    };

                    if let Some(resolved_target_symbol) = self.resolve_path_target_symbol(
                        revision,
                        module,
                        profile,
                        tree,
                        symbols,
                        namespace_symbol,
                        namespace_scope,
                        global_augmentation_scope,
                        exported_symbols,
                        declaration_id,
                        path,
                        space_order,
                    )? {
                        symbols
                            .get_symbol_mut(symbol_id)
                            .resolve_to(resolved_target_symbol);
                    }
                }

                Ok(())
            }

            _ => Ok(()),
        }
    }
}
