pub(super) use crate::TestProgram;
pub(super) use destack_dir::{
    Expression, GlobalSymbolId, LocalNodeId, LocalScopeId, LocalScopeMark, LocalTypeId, NodeTree,
    StaticKey, SymbolTable, TypeTable,
};
pub(super) use destack_source::ModuleId;

/// Cached module data for declare tests.
pub(super) struct DeclareTestView<'a> {
    /// The owning test program.
    pub(super) test: &'a TestProgram,
    /// The module id under test.
    pub(super) module_id: ModuleId,
    /// The module root expressions.
    roots: Vec<LocalNodeId<Expression>>,
    /// The module node tree.
    tree: NodeTree,
    /// The module symbol table.
    symbols: SymbolTable,
    /// The module type table.
    types: TypeTable,
    /// The module namespace scope id.
    namespace_scope: LocalScopeId,
}

impl TestProgram {
    /// Add, analyze, and check a declare module in one step.
    pub(super) fn analyze_declare_module_with_source(&self, name: &str, source: &str) -> ModuleId {
        let module_id = self.add_module(name, source);
        self.analyze_module(module_id);
        self.compile_check_clean();
        module_id
    }

    /// Create a cached declare view for a module default profile.
    pub(super) fn declare_view(&self, module_id: ModuleId) -> DeclareTestView<'_> {
        let profile = self.default_profile_id(module_id);
        let module = self.program.modules.get(module_id);
        let module = module.read();
        let dir = module.dir(profile);

        let roots = dir.roots.clone();
        let tree = dir.tree.read().clone();
        let symbols = dir.symbols.read().clone();
        let types = dir.types.read().clone();
        let namespace_scope = dir.namespace_scope;

        DeclareTestView {
            test: self,
            module_id,
            roots,
            tree,
            symbols,
            types,
            namespace_scope,
        }
    }
}

impl<'a> DeclareTestView<'a> {
    /// Read root expressions.
    pub(super) fn roots(&self) -> &[LocalNodeId<Expression>] {
        &self.roots
    }

    /// Read the node tree.
    pub(super) fn tree(&self) -> &NodeTree {
        &self.tree
    }

    /// Read the type table.
    pub(super) fn types(&self) -> &TypeTable {
        &self.types
    }

    /// Build an interned static key for a name.
    pub(super) fn key(&self, name: &str) -> StaticKey {
        StaticKey::Name(self.test.program.strings.intern(name))
    }

    /// Resolve a namespace symbol by name.
    pub(super) fn expect_namespace_symbol(&self, name: &str) -> GlobalSymbolId {
        let namespace_scope = self.symbols.get_scope_by_id(self.namespace_scope);
        self.symbols
            .find_active_symbol_up_to(namespace_scope, self.key(name), LocalScopeMark::end())
            .map(|symbol| symbol.into_global(self.module_id))
            .unwrap_or_else(|| panic!("expected namespace symbol `{name}`"))
    }

    /// Resolve a namespace value type id by name.
    pub(super) fn expect_namespace_value_type_id(&self, name: &str) -> LocalTypeId {
        let symbol = self.expect_namespace_symbol(name);
        self.types
            .get_value_type_id(symbol)
            .unwrap_or_else(|| panic!("expected value type id for `{name}`"))
    }
}
