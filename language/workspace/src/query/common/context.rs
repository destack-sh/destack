use destack_dir::{self as dir};
use destack_source::{FileId, ModuleId};
use parking_lot::RwLockReadGuard;

use crate::{Module, ModuleAst, ModuleDir, ProfileId, Session};

/// Query context for a module.
///
/// Bundles the commonly-needed AST and DIR references for query functions.
/// Created via [`Session::query_context`] or [`Session::query_context_with_profile`].
#[derive(Debug)]
pub struct QueryContext<'a> {
    /// The module AST (syntax tree and strings).
    pub ast: &'a ModuleAst,
    /// The module DIR (semantic IR).
    pub dir: &'a ModuleDir,
    /// The profile used for this context.
    pub profile_id: ProfileId,
    /// The module id.
    pub module_id: ModuleId,
    /// The source file id.
    pub file_id: FileId,
}

impl<'a> QueryContext<'a> {
    /// Get a read guard on the DIR node tree.
    #[inline]
    pub fn tree(&self) -> RwLockReadGuard<'_, dir::NodeTree> {
        self.dir.tree.read()
    }

    /// Get a read guard on the symbol table.
    #[inline]
    pub fn symbols(&self) -> RwLockReadGuard<'_, dir::SymbolTable> {
        self.dir.symbols.read()
    }

    /// Get a read guard on the type table.
    #[inline]
    pub fn types(&self) -> RwLockReadGuard<'_, dir::TypeTable> {
        self.dir.types.read()
    }
}

impl Session {
    /// Get query context for a module using its default profile.
    ///
    /// Returns `None` if AST or DIR is not available for the module.
    ///
    /// # Example
    /// ```ignore
    /// let module = session.modules.get(module_id);
    /// let module = module.read();
    /// let Some(ctx) = session.query_context(&module) else {
    ///     return None;
    /// };
    /// let tree = ctx.tree();
    /// // ... use ctx.ast, tree, ctx.symbols(), etc.
    /// ```
    pub fn query_context<'a>(&self, module: &'a Module) -> Option<QueryContext<'a>> {
        let profile = self.default_profile_for_module(module.id);
        self.query_context_with_profile(module, profile)
    }

    /// Get query context for a module with an explicit profile.
    ///
    /// Returns `None` if AST or DIR is not available for the module/profile.
    pub fn query_context_with_profile<'a>(
        &self,
        module: &'a Module,
        profile: ProfileId,
    ) -> Option<QueryContext<'a>> {
        let ast = module.ast.as_ref()?;
        let dir = module.dir_maybe(profile)?;
        Some(QueryContext {
            ast,
            dir,
            profile_id: profile,
            module_id: module.id,
            file_id: module.file_id,
        })
    }
}
