use std::sync::Arc;

use destack_artifact::{ArtifactStore, Ast, DirAnalyzed, DirResolved};
use destack_ast as ast;
use destack_core::StringPool;
use destack_dir::{self as dir};
use destack_source::{FileId, ModuleId, NodeSourceMap, ProfileId};
use destack_workspace::{Module, Program, Session};

use crate::ast::get_module_by_file_id;
use crate::dir::program_for_module;

/// Query context for a module.
///
/// Bundles the commonly-needed AST and DIR references for query functions.
/// Created via [`query_context`].
#[derive(Debug)]
pub(crate) struct QueryContext {
    /// The live artifact store for the program.
    artifacts: Arc<ArtifactStore>,
    /// The module AST (syntax tree and strings).
    ast: Arc<Ast>,
    /// The module DIR (semantic IR).
    dir: Arc<DirAnalyzed>,
    /// The resolved module linkage surface.
    resolved: Arc<DirResolved>,
    /// The profile used for this context.
    profile_id: ProfileId,
    /// The module id.
    module_id: ModuleId,
    /// The source file id.
    file_id: FileId,
}

/// Ast-facing query surface.
#[derive(Debug, Clone, Copy)]
pub(crate) struct AstQuery<'a> {
    /// The file id for this ast view.
    file_id: FileId,
    /// The module AST.
    ast: &'a Ast,
}

impl<'a> AstQuery<'a> {
    /// Return the file id for this ast view.
    pub(crate) fn file_id(self) -> FileId {
        self.file_id
    }

    /// Return the AST node tree.
    pub(crate) fn tree(self) -> &'a ast::NodeTree {
        &self.ast.tree
    }

    /// Return the AST source map.
    pub(crate) fn source_map(self) -> &'a NodeSourceMap {
        &self.ast.tree.source_map
    }

    /// Return the AST parent index.
    pub(crate) fn parents(self) -> &'a ast::NodeParentIndex {
        &self.ast.parents
    }

    /// Return the top level AST roots.
    pub(crate) fn roots(self) -> &'a [ast::LocalNodeId<ast::Expression>] {
        &self.ast.roots
    }

    /// Return the module string pool.
    pub(crate) fn strings(self) -> &'a StringPool {
        &self.ast.strings
    }

    /// Return the main token stream for this file.
    pub(crate) fn tokens(self) -> &'a [ast::TokenSpan] {
        &self.ast.tokens
    }

    /// Return the side token stream for this file.
    pub(crate) fn side_tokens(self) -> &'a [ast::TokenSpan] {
        &self.ast.side_tokens
    }
}

/// DIR-facing query surface.
#[derive(Debug, Clone, Copy)]
pub(crate) struct DirQuery<'a> {
    /// The module id for this dir view.
    module_id: ModuleId,
    /// The analyzed dir surface.
    analyzed: &'a DirAnalyzed,
    /// The resolved dir surface.
    resolved: &'a DirResolved,
}

impl<'a> DirQuery<'a> {
    /// Return the module id for this dir view.
    pub(crate) fn module_id(self) -> ModuleId {
        self.module_id
    }

    /// Return the DIR node tree.
    pub(crate) fn tree(self) -> &'a dir::NodeTree {
        &self.analyzed.tree
    }

    /// Return the DIR symbol table.
    pub(crate) fn symbols(self) -> &'a dir::SymbolTable {
        &self.analyzed.symbols
    }

    /// Return the DIR type table.
    pub(crate) fn types(self) -> &'a dir::TypeTable {
        &self.analyzed.types
    }

    /// Return the top level DIR roots.
    pub(crate) fn roots(self) -> &'a [dir::LocalNodeId<dir::Expression>] {
        self.analyzed.roots.as_ref()
    }

    /// Return the namespace scope for this module.
    pub(crate) fn namespace_scope(self) -> dir::LocalScopeId {
        self.analyzed.namespace_scope
    }

    /// Return the resolved DIR symbol table.
    pub(crate) fn resolved_symbols(self) -> &'a dir::SymbolTable {
        &self.resolved.symbols
    }

    /// Return the resolved DIR artifact.
    pub(crate) fn resolved(self) -> &'a DirResolved {
        self.resolved
    }

    /// Get the inferred type id for a node.
    pub(crate) fn expression_type_id(
        self,
        node_id: dir::LocalNodeIdAny,
    ) -> Option<dir::LocalTypeId> {
        let global_node_id = node_id.into_global(self.module_id);
        self.types().get_inferred_type_id(global_node_id)
    }

    /// Get the declared or inferred type id for a node.
    pub(crate) fn node_type_id(self, node_id: dir::LocalNodeIdAny) -> Option<dir::LocalTypeId> {
        let global_node_id = node_id.into_global(self.module_id);
        self.types()
            .get_declared_or_inferred_type_id(global_node_id)
    }
}

/// Select a profile that has the requested query artifacts for one module.
fn select_profile_for_module(
    artifacts: &ArtifactStore,
    module_id: ModuleId,
    requested_profile: ProfileId,
    require_analyzed: bool,
) -> Option<ProfileId> {
    let profile_is_available = if require_analyzed {
        artifacts
            .dir_analyzed(module_id, requested_profile)
            .is_some()
            && artifacts
                .dir_resolved(module_id, requested_profile)
                .is_some()
    } else {
        artifacts
            .dir_resolved(module_id, requested_profile)
            .is_some()
    };
    if profile_is_available {
        return Some(requested_profile);
    }

    let mut profile_ids: Vec<_> = artifacts
        .profile_ids_for_module(module_id)
        .into_iter()
        .collect();
    profile_ids.sort_unstable();

    profile_ids.into_iter().find(|candidate| {
        if require_analyzed {
            artifacts.dir_analyzed(module_id, *candidate).is_some()
                && artifacts.dir_resolved(module_id, *candidate).is_some()
        } else {
            artifacts.dir_resolved(module_id, *candidate).is_some()
        }
    })
}

impl QueryContext {
    /// Return the artifact store for this query context.
    pub(crate) fn artifacts(&self) -> &ArtifactStore {
        self.artifacts.as_ref()
    }

    /// Return the profile id for this query context.
    pub(crate) fn profile_id(&self) -> ProfileId {
        self.profile_id
    }

    /// Return the module id for this query context.
    pub(crate) fn module_id(&self) -> ModuleId {
        self.module_id
    }

    /// Return the file id for this query context.
    pub(crate) fn file_id(&self) -> FileId {
        self.file_id
    }

    /// Return the ast query surface.
    pub(crate) fn ast(&self) -> AstQuery<'_> {
        AstQuery {
            file_id: self.file_id,
            ast: self.ast.as_ref(),
        }
    }

    /// Return the dir query surface.
    pub(crate) fn dir(&self) -> DirQuery<'_> {
        DirQuery {
            module_id: self.module_id,
            analyzed: self.dir.as_ref(),
            resolved: self.resolved.as_ref(),
        }
    }
}

/// Get query context for a module using its default profile.
///
/// Returns `None` if AST, analyzed DIR, or resolved DIR is not available for the module.
pub(crate) fn query_context(session: &Session, module: &Module) -> Option<QueryContext> {
    // resolve the owning program and its default profile
    let program = program_for_module(session, module);
    let profile = program.default_profile_id_for_module(module.id);

    // build query context
    query_context_with_program_and_profile(session, module, program, profile)
}

/// Get query context for a module with an explicit program and profile.
fn query_context_with_program_and_profile(
    session: &Session,
    module: &Module,
    program: Arc<Program>,
    profile: ProfileId,
) -> Option<QueryContext> {
    // resolve module ast and profile dir artifact
    let artifacts = session.get_artifacts_for_program(program.as_ref())?;
    let ast = artifacts.ast(module.id)?;
    let selected_profile = select_profile_for_module(&artifacts, module.id, profile, true)?;
    let dir = artifacts.dir_analyzed(module.id, selected_profile)?;
    let resolved = artifacts.dir_resolved(module.id, selected_profile)?;

    // build query context
    Some(QueryContext {
        artifacts,
        ast,
        dir,
        resolved,
        profile_id: selected_profile,
        module_id: module.id,
        file_id: module.file_id,
    })
}

/// Execute a closure with a query context for a file.
pub(crate) fn with_query_context_for_file<T>(
    session: &Session,
    file_id: FileId,
    f: impl FnOnce(QueryContext) -> T,
) -> Option<T> {
    // resolve the module for the file id
    let module = get_module_by_file_id(session, file_id)?;

    // build a query context while the module guard is held
    let module = module.as_ref();
    let ctx = query_context(session, module)?;

    // run the caller logic inside the query context
    Some(f(ctx))
}

/// Execute a closure with a query context for a module.
pub(crate) fn with_query_context_for_module<T>(
    session: &Session,
    module: &Module,
    f: impl FnOnce(QueryContext) -> T,
) -> Option<T> {
    let ctx = query_context(session, module)?;
    Some(f(ctx))
}

/// Execute a closure with an ast query surface for a file.
pub(crate) fn with_ast_query_for_file<T>(
    session: &Session,
    file_id: FileId,
    f: impl FnOnce(AstQuery<'_>) -> T,
) -> Option<T> {
    let module = get_module_by_file_id(session, file_id)?;
    with_ast_query_for_module(session, module.as_ref(), f)
}

/// Execute a closure with an ast query surface for a module.
pub(crate) fn with_ast_query_for_module<T>(
    session: &Session,
    module: &Module,
    f: impl FnOnce(AstQuery<'_>) -> T,
) -> Option<T> {
    let program = program_for_module(session, module);
    let artifacts = session.get_artifacts_for_program(program.as_ref())?;
    let ast = artifacts.ast(module.id)?;
    let query = AstQuery {
        file_id: module.file_id,
        ast: ast.as_ref(),
    };

    Some(f(query))
}

/// Execute a closure with ast and resolved dir query surfaces for a module.
pub(crate) fn with_ast_and_resolved_for_module<T>(
    session: &Session,
    module: &Module,
    f: impl FnOnce(AstQuery<'_>, ModuleId, &DirResolved) -> T,
) -> Option<T> {
    let program = program_for_module(session, module);
    let profile = program.default_profile_id_for_module(module.id);
    let artifacts = session.get_artifacts_for_program(program.as_ref())?;
    let ast = artifacts.ast(module.id)?;
    let selected_profile = select_profile_for_module(&artifacts, module.id, profile, false)?;
    let resolved = artifacts.dir_resolved(module.id, selected_profile)?;

    let ast_query = AstQuery {
        file_id: module.file_id,
        ast: ast.as_ref(),
    };

    Some(f(ast_query, module.id, resolved.as_ref()))
}
