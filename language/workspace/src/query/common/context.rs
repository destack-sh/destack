use std::cell::Cell;

use destack_dir::{self as dir};
use destack_source::{FileId, ModuleId};
use parking_lot::RwLockReadGuard;

use super::get_module_by_file_id;
use crate::{Module, ModuleAst, ModuleDir, ProfileId, Session};

thread_local! {
    static QUERY_ALLOW_STALE: Cell<bool> = const { Cell::new(false) };
}

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

    /// Get the inferred type id for a node (expression, declaration, etc.).
    ///
    /// Returns the type that was inferred during type checking for the given node.
    /// This is the primary way to get the type of an arbitrary expression.
    pub fn get_expression_type(&self, node_id: dir::LocalNodeIdAny) -> Option<dir::LocalTypeId> {
        let global_node_id = node_id.into_global(self.module_id);
        self.types().get_inferred_type_id(global_node_id)
    }

    /// Get the declared or inferred type id for a node.
    ///
    /// Prefers declared type (from type annotation) over inferred type.
    pub fn get_node_type(&self, node_id: dir::LocalNodeIdAny) -> Option<dir::LocalTypeId> {
        let global_node_id = node_id.into_global(self.module_id);
        self.types()
            .get_declared_or_inferred_type_id(global_node_id)
    }
}

impl Session {
    /// Execute a closure with query context configuration applied.
    pub fn with_query_context_mode<T>(&self, allow_stale: bool, f: impl FnOnce() -> T) -> T {
        // swap in query context mode
        let previous = QUERY_ALLOW_STALE.with(|flag| {
            let previous = flag.get();
            flag.set(allow_stale);
            previous
        });

        // run the query work
        let result = f();

        // restore previous mode
        QUERY_ALLOW_STALE.with(|flag| flag.set(previous));

        result
    }

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
        // resolve default profile
        let profile = self.default_profile_for_module(module.id);

        // read stale query mode
        let allow_stale = QUERY_ALLOW_STALE.with(|flag| flag.get());

        // build query context
        self.query_context_with_profile_mode(module, profile, allow_stale)
    }

    /// Get query context for a module with an explicit profile.
    ///
    /// Returns `None` if AST or DIR is not available for the module/profile.
    pub fn query_context_with_profile<'a>(
        &self,
        module: &'a Module,
        profile: ProfileId,
    ) -> Option<QueryContext<'a>> {
        // read stale query mode
        let allow_stale = QUERY_ALLOW_STALE.with(|flag| flag.get());

        // build query context
        self.query_context_with_profile_mode(module, profile, allow_stale)
    }

    /// Build query context for a module with optional stale fallback.
    fn query_context_with_profile_mode<'a>(
        &self,
        module: &'a Module,
        profile: ProfileId,
        allow_stale: bool,
    ) -> Option<QueryContext<'a>> {
        // resolve module ast
        let ast = module.ast_maybe()?;

        // resolve module dir
        let dir = if let Some(dir) = module.dir_maybe(profile) {
            Some(dir)
        } else if allow_stale {
            module.dir_base_maybe()
        } else {
            None
        }?;

        // build query context
        Some(QueryContext {
            ast,
            dir,
            profile_id: profile,
            module_id: module.id,
            file_id: module.file_id,
        })
    }
}

/// Execute a closure with a query context for a file.
pub fn with_query_context_for_file<T>(
    session: &Session,
    file_id: FileId,
    f: impl FnOnce(QueryContext<'_>) -> T,
) -> Option<T> {
    // resolve the module for the file id
    let module = get_module_by_file_id(session, file_id)?;

    // build a query context while the module guard is held
    let module = module.read();
    let ctx = session.query_context(&module)?;

    // run the caller logic inside the query context
    Some(f(ctx))
}
