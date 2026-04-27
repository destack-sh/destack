use std::sync::Arc;

use destack_artifact::{ArtifactKey, ArtifactPin, Ast, DirAnalyzed, DirResolved};
use destack_ast as ast;
use destack_core::StringPool;
use destack_dir::{self as dir};
use destack_source::{FileId, ModuleId, NodeSourceMap, ProfileId};
use destack_workspace::{Repository, Revision};

use crate::ast::get_module_by_file_id;

/// Query context for a module.
///
/// Bundles the commonly-needed AST and DIR references for query functions.
/// Created via [`query_context`].
#[derive(Debug)]
pub(crate) struct QueryContext {
    /// The exact artifact pins retained for this query.
    _pins: Vec<ArtifactPin>,
    /// The module AST (syntax tree and strings).
    ast: Arc<Ast>,
    /// The module DIR (semantic IR).
    dir_analyzed: Arc<DirAnalyzed>,
    /// The resolved module linkage surface.
    dir_resolved: Arc<DirResolved>,
    /// The revision used for this context.
    revision: Revision,
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

    /// Return the AST tree.
    pub(crate) fn tree(self) -> &'a ast::Tree {
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
    /// The revision for this dir view.
    revision: Revision,
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

    /// Return the revision for this dir view.
    pub(crate) fn revision(self) -> Revision {
        self.revision
    }

    /// Return the DIR tree.
    pub(crate) fn tree(self) -> &'a dir::Tree {
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

impl QueryContext {
    /// Return the profile id for this query context.
    pub(crate) fn profile_id(&self) -> ProfileId {
        self.profile_id
    }

    /// Return the revision for this query context.
    pub(crate) fn revision(&self) -> Revision {
        self.revision
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
            revision: self.revision,
            analyzed: self.dir_analyzed.as_ref(),
            resolved: self.dir_resolved.as_ref(),
        }
    }
}

/// Get query context for a module with one explicit profile.
fn query_context_with_profile(
    repository: &Repository,
    revision: Revision,
    module_id: ModuleId,
    profile: ProfileId,
) -> Option<QueryContext> {
    let module = repository.module(revision, module_id).ok().flatten()?;
    let artifacts = repository.artifact_store().clone();

    // resolve and retain the exact live query artifacts
    let ast_version = repository.artifact_version(revision, &ArtifactKey::ast(module.id));
    let selected_profile =
        repository.available_profile_id_for_module(revision, module.id, profile, true)?;
    let resolved_version = repository.artifact_version(
        revision,
        &ArtifactKey::dir_resolved(module.id, selected_profile),
    );
    let analyzed_version = repository.artifact_version(
        revision,
        &ArtifactKey::dir_analyzed(module.id, selected_profile),
    );

    // resolve module ast and profile dir artifact
    let ast = repository.ast(revision, module.id)?;
    let dir_analyzed = repository.dir_analyzed(revision, module.id, selected_profile)?;
    let dir_resolved = repository.dir_resolved(revision, module.id, selected_profile)?;

    // query artifact roots
    let ast_pin = artifacts.pin(&ast_version)?;
    let resolved_pin = artifacts.pin(&resolved_version)?;
    let analyzed_pin = artifacts.pin(&analyzed_version)?;

    // build query context
    Some(QueryContext {
        _pins: vec![ast_pin, resolved_pin, analyzed_pin],
        ast,
        dir_analyzed,
        dir_resolved,
        revision,
        profile_id: selected_profile,
        module_id: module.id,
        file_id: module.file_id,
    })
}

/// Get query context for one module.
pub(crate) fn query_context(
    repository: &Repository,
    revision: Revision,
    module_id: ModuleId,
) -> Option<QueryContext> {
    let profile = repository
        .default_profile_id_for_module(revision, module_id)
        .ok()?;

    query_context_with_profile(repository, revision, module_id, profile)
}

/// Get query context for a module id through the owning repository.
pub(crate) fn query_context_for_module_id(
    repository: &Repository,
    revision: Revision,
    module_id: ModuleId,
) -> Option<QueryContext> {
    query_context(repository, revision, module_id)
}

/// Execute a closure with a query context for one file.
pub(crate) fn with_query_context_for_file<T>(
    repository: &Repository,
    revision: Revision,
    file_id: FileId,
    f: impl FnOnce(QueryContext) -> T,
) -> Option<T> {
    let module = get_module_by_file_id(repository, revision, file_id)?;
    let ctx = query_context(repository, revision, module.id)?;

    Some(f(ctx))
}

/// Execute a closure with a query context for a module.
pub(crate) fn with_query_context_for_module<T>(
    repository: &Repository,
    revision: Revision,
    module_id: ModuleId,
    f: impl FnOnce(QueryContext) -> T,
) -> Option<T> {
    let ctx = query_context(repository, revision, module_id)?;
    Some(f(ctx))
}

/// Execute a closure with an ast query surface for a file.
pub(crate) fn with_ast_query_for_file<T>(
    repository: &Repository,
    revision: Revision,
    file_id: FileId,
    f: impl FnOnce(AstQuery<'_>) -> T,
) -> Option<T> {
    let module = get_module_by_file_id(repository, revision, file_id)?;
    with_ast_query_for_module(repository, revision, module.id, f)
}

/// Execute a closure with an ast query surface for one module.
pub(crate) fn with_ast_query_for_module<T>(
    repository: &Repository,
    revision: Revision,
    module_id: ModuleId,
    f: impl FnOnce(AstQuery<'_>) -> T,
) -> Option<T> {
    let module = repository.module(revision, module_id).ok().flatten()?;
    let ast = repository.ast(revision, module.id)?;
    let query = AstQuery {
        file_id: module.file_id,
        ast: ast.as_ref(),
    };

    Some(f(query))
}
