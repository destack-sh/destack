//! Module lowering from DIR to JS AST.
//!
//! The `ModuleLowerer` converts elaborated DIR (Destack IR) into a JavaScript AST.
//! This is the emission stage for JS and TS targets.

use destack_artifact::{Ast, DirChecked, DirDeclared};
use destack_core::StringPool;
use destack_dir::{BindingTable, GlobalSymbolId, TypeTable};
use destack_workspace::{Module, Target};
use {destack_dir as dir, destack_js as js};

use crate::{CodegenJsError, CodegenJsResult, CodegenJsWarning, ScriptSymbolId};

/// Context for lowering a DIR module to JS AST.
#[derive(Debug)]
#[allow(dead_code)]
pub struct ModuleLowerer<'a> {
    /// The source module.
    pub(crate) module: &'a Module,
    /// The source module AST.
    pub(crate) ast: &'a Ast,
    /// The source string pool for bound DIR nodes.
    pub(crate) source_strings: &'a StringPool,

    /// The DIR roots.
    pub(crate) dir_roots: &'a Vec<dir::LocalNodeId<dir::Expression>>,
    /// The DIR tree.
    pub(crate) dir_tree: &'a dir::Tree,
    /// The symbol table.
    pub(crate) symbols: &'a BindingTable,
    /// The type table.
    pub(crate) types: &'a TypeTable,
    /// The target configuration.
    pub(crate) target: &'a Target,

    /// The output JS AST tree.
    pub(crate) tree: js::Tree,
    /// Root nodes in the output.
    pub(crate) roots: Vec<js::LocalNodeIdAny>,
    /// String pool for the output.
    pub(crate) strings: StringPool,
    /// Collected warnings.
    pub(crate) warnings: Vec<CodegenJsWarning>,
    /// Collected non-fatal errors (treated as warnings for continued processing).
    pub(crate) errors: Vec<CodegenJsError>,
}

impl<'a> ModuleLowerer<'a> {
    /// Build one lowered script symbol id from one local DIR symbol.
    pub(crate) fn source_symbol_id(&self, symbol_id: dir::LocalSymbolId) -> ScriptSymbolId {
        ScriptSymbolId::Source(symbol_id.into_global(self.module.id))
    }

    /// Store one lowered script symbol id on one JS AST node.
    pub(crate) fn set_node_symbol<T>(
        &mut self,
        node_id: js::LocalNodeId<T>,
        symbol_id: ScriptSymbolId,
    ) where
        T: js::Node,
        js::Tree: js::TreeImpl<T>,
    {
        self.tree.set_symbol(node_id, symbol_id);
    }

    /// Store one source-backed symbol id on one JS AST node.
    pub(crate) fn set_source_node_symbol<T>(
        &mut self,
        node_id: js::LocalNodeId<T>,
        symbol_id: dir::LocalSymbolId,
    ) where
        T: js::Node,
        js::Tree: js::TreeImpl<T>,
    {
        self.set_node_symbol(node_id, self.source_symbol_id(symbol_id));
    }

    /// Store one global source-backed symbol id on one JS AST node.
    pub(crate) fn set_global_node_symbol<T>(
        &mut self,
        node_id: js::LocalNodeId<T>,
        symbol_id: GlobalSymbolId,
    ) where
        T: js::Node,
        js::Tree: js::TreeImpl<T>,
    {
        self.set_node_symbol(node_id, ScriptSymbolId::Source(symbol_id));
    }

    /// Create a new module lowerer.
    pub fn new(
        module: &'a Module,
        ast: &'a Ast,
        source_strings: &'a StringPool,
        declared: &'a DirDeclared,
        checked: &'a DirChecked,
        target: &'a Target,
    ) -> Self {
        let strings = StringPool::new();
        strings.ensure_all_from(source_strings);

        Self {
            module,
            ast,
            source_strings,
            dir_tree: &declared.tree,
            dir_roots: declared.roots.as_ref(),
            symbols: &declared.bindings,
            types: &checked.types,
            target,
            tree: js::Tree::new(),
            roots: Vec::new(),
            strings,
            warnings: Vec::new(),
            errors: Vec::new(),
        }
    }

    /// Record a non-fatal error (allows lowering to continue).
    pub(crate) fn error(&mut self, error: CodegenJsError) {
        self.errors.push(error);
    }

    /// Record a warning.
    pub(crate) fn warning(&mut self, warning: CodegenJsWarning) {
        self.warnings.push(warning);
    }

    /// Lower the module to JS AST.
    pub fn lower_module(&mut self) -> CodegenJsResult<()> {
        for expression_id in self.dir_roots.iter() {
            match self.lower_expression(*expression_id) {
                Ok(root_id) => self.roots.push(root_id),
                Err(error) => self.error(error),
            }
        }
        Ok(())
    }
}
