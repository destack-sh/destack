use std::marker::PhantomData;
use std::sync::Arc;

use destack_artifact::{ArtifactStore, Ast, DirAnalyzed, DirResolved};
use destack_ast as ast;
use destack_core::StringPool;
use destack_dir::{self as dir};
use destack_source::{FileId, ModuleId, NodeSourceMap, ProfileId};
use destack_workspace::{Module, Program, Session};

use super::{get_module_by_file_id, program_for_module};

/// Query context for a module.
///
/// Bundles the commonly-needed AST and DIR references for query functions.
/// Created via [`query_context`] or [`query_context_with_profile`].
#[derive(Debug)]
pub struct QueryContext<'a> {
    /// The owning program.
    pub program: Arc<Program>,
    /// The live artifact store for the program.
    pub artifacts: Arc<ArtifactStore>,
    /// The module AST (syntax tree and strings).
    pub ast: Arc<Ast>,
    /// The module DIR (semantic IR).
    pub dir: Arc<DirAnalyzed>,
    /// The resolved module linkage surface.
    pub resolved: Arc<DirResolved>,
    /// The profile used for this context.
    pub profile_id: ProfileId,
    /// The module id.
    pub module_id: ModuleId,
    /// The source file id.
    pub file_id: FileId,
    /// Tie the context lifetime to the call site without borrowing module state.
    marker: PhantomData<&'a ()>,
}

/// Source-facing query context.
#[derive(Debug, Clone, Copy)]
pub struct SourceContext<'a> {
    /// The file id for this source view.
    pub file_id: FileId,
    /// The module AST carrying source tokens and spans.
    pub ast: &'a Ast,
}

impl<'a> SourceContext<'a> {
    /// Execute a closure with the source text for this file.
    pub fn with_text<T>(self, session: &Session, f: impl FnOnce(&str) -> T) -> T {
        let source_file = session.files.get(self.file_id);

        f(source_file.text())
    }

    /// Return the token stream for this file.
    pub fn tokens(self) -> &'a [ast::TokenSpan] {
        &self.ast.tokens
    }

    /// Return the side token stream for this file.
    pub fn side_tokens(self) -> &'a [ast::TokenSpan] {
        &self.ast.side_tokens
    }
}

/// Ast-facing query context.
#[derive(Debug, Clone, Copy)]
pub struct AstContext<'a> {
    /// The file id for this ast view.
    pub file_id: FileId,
    /// The module AST.
    pub ast: &'a Ast,
}

impl<'a> AstContext<'a> {
    /// Return the AST node tree.
    pub fn tree(self) -> &'a ast::NodeTree {
        &self.ast.tree
    }

    /// Return the AST source map.
    pub fn source_map(self) -> &'a NodeSourceMap {
        &self.ast.tree.source_map
    }

    /// Return the AST parent index.
    pub fn parents(self) -> &'a ast::NodeParentIndex {
        &self.ast.parents
    }

    /// Return the top level AST roots.
    pub fn roots(self) -> &'a [ast::LocalNodeId<ast::Expression>] {
        &self.ast.roots
    }

    /// Return the module string pool.
    pub fn strings(self) -> &'a StringPool {
        &self.ast.strings
    }

    /// Return the main token stream for this file.
    pub fn tokens(self) -> &'a [ast::TokenSpan] {
        &self.ast.tokens
    }

    /// Return the side token stream for this file.
    pub fn side_tokens(self) -> &'a [ast::TokenSpan] {
        &self.ast.side_tokens
    }
}

/// Analyzed DIR-facing query context.
#[derive(Debug, Clone, Copy)]
pub struct DirAnalyzedContext<'a> {
    /// The module id for this dir view.
    pub module_id: ModuleId,
    /// The analyzed dir surface.
    pub dir: &'a DirAnalyzed,
}

impl<'a> DirAnalyzedContext<'a> {
    /// Return the DIR node tree.
    pub fn tree(self) -> &'a dir::NodeTree {
        &self.dir.tree
    }

    /// Return the DIR symbol table.
    pub fn symbols(self) -> &'a dir::SymbolTable {
        &self.dir.symbols
    }

    /// Return the DIR type table.
    pub fn types(self) -> &'a dir::TypeTable {
        &self.dir.types
    }

    /// Return the top level DIR roots.
    pub fn roots(self) -> &'a [dir::LocalNodeId<dir::Expression>] {
        self.dir.roots.as_ref()
    }
}

/// Resolved DIR-facing query context.
#[derive(Debug, Clone, Copy)]
pub struct DirResolvedContext<'a> {
    /// The module id for this resolved view.
    pub module_id: ModuleId,
    /// The resolved dir surface.
    pub dir: &'a DirResolved,
}

impl<'a> DirResolvedContext<'a> {
    /// Return the DIR node tree.
    pub fn tree(self) -> &'a dir::NodeTree {
        &self.dir.tree
    }

    /// Return the DIR symbol table.
    pub fn symbols(self) -> &'a dir::SymbolTable {
        &self.dir.symbols
    }

    /// Return the DIR type table.
    pub fn types(self) -> &'a dir::TypeTable {
        &self.dir.types
    }

    /// Return the top level DIR roots.
    pub fn roots(self) -> &'a [dir::LocalNodeId<dir::Expression>] {
        self.dir.roots.as_ref()
    }
}

impl<'a> QueryContext<'a> {
    /// Return the source-facing query context.
    pub fn source_context(&self) -> SourceContext<'_> {
        SourceContext {
            file_id: self.file_id,
            ast: self.ast.as_ref(),
        }
    }

    /// Return the ast-facing query context.
    pub fn ast_context(&self) -> AstContext<'_> {
        AstContext {
            file_id: self.file_id,
            ast: self.ast.as_ref(),
        }
    }

    /// Return the analyzed DIR-facing query context.
    pub fn dir_analyzed_context(&self) -> DirAnalyzedContext<'_> {
        DirAnalyzedContext {
            module_id: self.module_id,
            dir: self.dir.as_ref(),
        }
    }

    /// Return the resolved DIR-facing query context.
    pub fn dir_resolved_context(&self) -> DirResolvedContext<'_> {
        DirResolvedContext {
            module_id: self.module_id,
            dir: self.resolved.as_ref(),
        }
    }

    /// Return the resolved DIR.
    pub fn dir_resolved(&self) -> &DirResolved {
        self.resolved.as_ref()
    }

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
/// Returns `None` if AST, analyzed DIR, or resolved DIR is not available for the module.
pub fn query_context<'a>(session: &Session, module: &'a Module) -> Option<QueryContext<'a>> {
    // resolve the owning program and its default profile
    let program = program_for_module(session, module);
    let profile = program.default_profile_id_for_module(module.id);

    // build query context
    query_context_with_program_and_profile(session, module, program, profile)
}

/// Get query context for a module with an explicit profile.
///
/// Returns `None` if AST, analyzed DIR, or resolved DIR is not available for the module/profile.
pub fn query_context_with_profile<'a>(
    session: &Session,
    module: &'a Module,
    profile: ProfileId,
) -> Option<QueryContext<'a>> {
    // resolve the owning program and profile dir artifact
    let program = program_for_module(session, module);
    query_context_with_program_and_profile(session, module, program, profile)
}

/// Get query context for a module with an explicit program and profile.
fn query_context_with_program_and_profile<'a>(
    session: &Session,
    module: &'a Module,
    program: Arc<Program>,
    profile: ProfileId,
) -> Option<QueryContext<'a>> {
    // resolve module ast and profile dir artifact
    let artifacts = session.get_artifacts_for_program(program.as_ref())?;
    let ast = artifacts.ast(module.id)?;
    let dir = artifacts.dir_analyzed(module.id, profile)?;
    let resolved = artifacts.dir_resolved(module.id, profile)?;

    // build query context
    Some(QueryContext {
        program,
        artifacts,
        ast,
        dir,
        resolved,
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
    let ctx = query_context(session, module)?;

    // run the caller logic inside the query context
    Some(f(ctx))
}

/// Execute a closure with an AST context for a file.
pub fn with_ast_context_for_file<T>(
    session: &Session,
    file_id: FileId,
    f: impl FnOnce(AstContext<'_>) -> T,
) -> Option<T> {
    // resolve the module for the file id
    let module = get_module_by_file_id(session, file_id)?;
    let module = module.as_ref();

    // run the caller logic inside the AST context
    with_ast_context_for_module(session, module, f)
}

/// Execute a closure with an AST context for a module.
pub fn with_ast_context_for_module<T>(
    session: &Session,
    module: &Module,
    f: impl FnOnce(AstContext<'_>) -> T,
) -> Option<T> {
    // resolve the AST while the module guard is held
    let program = program_for_module(session, module);
    let artifacts = session.get_artifacts_for_program(program.as_ref())?;
    let ast = artifacts.ast(module.id)?;
    let ctx = AstContext {
        file_id: module.file_id,
        ast: ast.as_ref(),
    };

    // run the caller logic inside the AST context
    Some(f(ctx))
}

/// Execute a closure with a source context for a file.
pub fn with_source_context_for_file<T>(
    session: &Session,
    file_id: FileId,
    f: impl FnOnce(SourceContext<'_>) -> T,
) -> Option<T> {
    // resolve the module for the file id
    let module = get_module_by_file_id(session, file_id)?;
    let module = module.as_ref();

    // run the caller logic inside the source context
    with_source_context_for_module(session, module, f)
}

/// Execute a closure with a source context for a module.
pub fn with_source_context_for_module<T>(
    session: &Session,
    module: &Module,
    f: impl FnOnce(SourceContext<'_>) -> T,
) -> Option<T> {
    // resolve the AST while the module guard is held
    let program = program_for_module(session, module);
    let artifacts = session.get_artifacts_for_program(program.as_ref())?;
    let ast = artifacts.ast(module.id)?;
    let ctx = SourceContext {
        file_id: module.file_id,
        ast: ast.as_ref(),
    };

    // run the caller logic inside the source context
    Some(f(ctx))
}

/// Execute a closure with an analyzed DIR context for a file.
pub fn with_dir_analyzed_context_for_file<T>(
    session: &Session,
    file_id: FileId,
    f: impl FnOnce(DirAnalyzedContext<'_>) -> T,
) -> Option<T> {
    // resolve the module for the file id
    let module = get_module_by_file_id(session, file_id)?;
    let module = module.as_ref();

    // run the caller logic inside the analyzed DIR context
    with_dir_analyzed_context_for_module(session, module, f)
}

/// Execute a closure with an analyzed DIR context for a module.
pub fn with_dir_analyzed_context_for_module<T>(
    session: &Session,
    module: &Module,
    f: impl FnOnce(DirAnalyzedContext<'_>) -> T,
) -> Option<T> {
    // resolve the program and default profile while the module guard is held
    let program = program_for_module(session, module);
    let profile = program.default_profile_id_for_module(module.id);
    let artifacts = session.get_artifacts_for_program(program.as_ref())?;
    let dir = artifacts.dir_analyzed(module.id, profile)?;
    let ctx = DirAnalyzedContext {
        module_id: module.id,
        dir: dir.as_ref(),
    };

    // run the caller logic inside the analyzed DIR context
    Some(f(ctx))
}

/// Execute a closure with a resolved DIR context for a file.
pub fn with_dir_resolved_context_for_file<T>(
    session: &Session,
    file_id: FileId,
    f: impl FnOnce(DirResolvedContext<'_>) -> T,
) -> Option<T> {
    // resolve the module for the file id
    let module = get_module_by_file_id(session, file_id)?;
    let module = module.as_ref();

    // run the caller logic inside the resolved context
    with_dir_resolved_context_for_module(session, module, f)
}

/// Execute a closure with a resolved DIR context for a module.
pub fn with_dir_resolved_context_for_module<T>(
    session: &Session,
    module: &Module,
    f: impl FnOnce(DirResolvedContext<'_>) -> T,
) -> Option<T> {
    // resolve the program and default profile while the module guard is held
    let program = program_for_module(session, module);
    let profile = program.default_profile_id_for_module(module.id);
    let artifacts = session.get_artifacts_for_program(program.as_ref())?;
    let resolved = artifacts.dir_resolved(module.id, profile)?;
    let ctx = DirResolvedContext {
        module_id: module.id,
        dir: resolved.as_ref(),
    };

    // run the caller logic inside the resolved context
    Some(f(ctx))
}

/// Execute a closure with AST and resolved DIR contexts for a file.
pub fn with_ast_and_dir_resolved_context_for_file<T>(
    session: &Session,
    file_id: FileId,
    f: impl FnOnce(AstContext<'_>, DirResolvedContext<'_>) -> T,
) -> Option<T> {
    // resolve the module for the file id
    let module = get_module_by_file_id(session, file_id)?;
    let module = module.as_ref();

    // run the caller logic inside the shared contexts
    with_ast_and_dir_resolved_context_for_module(session, module, f)
}

/// Execute a closure with AST and resolved DIR contexts for a module.
pub fn with_ast_and_dir_resolved_context_for_module<T>(
    session: &Session,
    module: &Module,
    f: impl FnOnce(AstContext<'_>, DirResolvedContext<'_>) -> T,
) -> Option<T> {
    // resolve the program and default profile while the module guard is held
    let program = program_for_module(session, module);
    let profile = program.default_profile_id_for_module(module.id);
    let artifacts = session.get_artifacts_for_program(program.as_ref())?;
    let ast = artifacts.ast(module.id)?;
    let resolved = artifacts.dir_resolved(module.id, profile)?;

    let ast = AstContext {
        file_id: module.file_id,
        ast: ast.as_ref(),
    };
    let resolved = DirResolvedContext {
        module_id: module.id,
        dir: resolved.as_ref(),
    };

    // run the caller logic inside the shared contexts
    Some(f(ast, resolved))
}
