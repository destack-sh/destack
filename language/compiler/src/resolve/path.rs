use dyst_dir::{
    LocalNodeIdAny, LocalSymbolId, Module, NodeTree, Path, Scope, SymbolKey, SymbolTable,
};

use crate::{Compiler, ResolveError, ResolveResult};

impl<'a> Compiler<'a> {
    /// Resolve a Path.
    pub(super) fn resolve_path(
        &self,
        module: &Module,
        node: LocalNodeIdAny,
        scope: &Scope,
        path: &Path,
        _tree: &NodeTree,
        symbols: &SymbolTable,
    ) -> ResolveResult<LocalSymbolId> {
        assert!(path.segments.len() > 0);
        let root_segment = path.segments[0];
        let root_key = SymbolKey::Name(root_segment);

        let mut scope = scope;
        loop {
            // find symbol
            if let Some(symbol_id) = scope.find_symbol(root_key) {
                return Ok(symbol_id);
            } 
            // go to parent scope
            else if let Some(parent_scope_id) = scope.parent_id {
                scope = symbols.get_scope_by_id(parent_scope_id);
            } 
            // no more scopes
            else {
                break;
            }
        }

        Err(ResolveError::MissingSymbol {
            node: node.into_global(module.id),
            scope: scope.id.into_global(module.id),
            name: path.segments[0],
        })
    }
}
