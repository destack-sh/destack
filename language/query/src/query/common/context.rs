use std::marker::PhantomData;
use std::sync::Arc;

use destack_dir::{self as dir};
use destack_source::{FileId, ModuleId, ProfileId};
use destack_workspace::{Module, ModuleAst, ModuleDir, Program, Session};

use super::{get_module_by_file_id, program_for_module};

/// Query context for a module.
///
/// Bundles the commonly-needed AST and DIR references for query functions.
/// Created via [`query_context`] or [`query_context_with_profile`].
#[derive(Debug)]
pub struct QueryContext<'a> {
    /// The owning program.
    pub program: Arc<Program>,
    /// The module AST (syntax tree and strings).
    pub ast: Arc<ModuleAst>,
    /// The module DIR (semantic IR).
    pub dir: Arc<ModuleDir>,
    /// The profile used for this context.
    pub profile_id: ProfileId,
    /// The module id.
    pub module_id: ModuleId,
    /// The source file id.
    pub file_id: FileId,
    /// Tie the context lifetime to the call site without borrowing module state.
    marker: PhantomData<&'a ()>,
}

impl<'a> QueryContext<'a> {
    /// Get a read guard on the DIR node tree.
    #[inline]
    pub fn tree(&self) -> &dir::NodeTree {
        &self.dir.tree
    }

    /// Get a read guard on the symbol table.
    #[inline]
    pub fn symbols(&self) -> &dir::SymbolTable {
        &self.dir.symbols
    }

    /// Get a read guard on the type table.
    #[inline]
    pub fn types(&self) -> &dir::TypeTable {
        &self.dir.types
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

/// Get query context for a module using its default profile.
///
/// Returns `None` if AST or DIR is not available for the module.
pub fn query_context<'a>(session: &Session, module: &'a Module) -> Option<QueryContext<'a>> {
    // resolve the owning program and its default profile
    let program = program_for_module(session, module);
    let profile = program.default_profile_id_for_module(module.id);

    // build query context
    query_context_with_program_and_profile(module, program, profile)
}

/// Get query context for a module with an explicit profile.
///
/// Returns `None` if AST or DIR is not available for the module/profile.
pub fn query_context_with_profile<'a>(
    session: &Session,
    module: &'a Module,
    profile: ProfileId,
) -> Option<QueryContext<'a>> {
    // resolve the owning program and profile dir artifact
    let program = program_for_module(session, module);
    query_context_with_program_and_profile(module, program, profile)
}

/// Get query context for a module with an explicit program and profile.
fn query_context_with_program_and_profile<'a>(
    module: &'a Module,
    program: Arc<Program>,
    profile: ProfileId,
) -> Option<QueryContext<'a>> {
    // resolve module ast and profile dir artifact
    let ast = program.artifacts.ast(module.id)?;
    let dir = program.artifacts.dir_analyzed(module.id, profile)?;

    // build query context
    Some(QueryContext {
        program,
        ast,
        dir,
        profile_id: profile,
        module_id: module.id,
        file_id: module.file_id,
        marker: PhantomData,
    })
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
    let module = module.as_ref();
    let ctx = query_context(session, &module)?;

    // run the caller logic inside the query context
    Some(f(ctx))
}
