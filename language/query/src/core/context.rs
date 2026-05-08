use std::sync::Arc;

use destack_artifact::{
    ArtifactKey, ArtifactPin, ArtifactVersion, Ast, DirChecked, DirDeclared, DirExpanded,
    DirExported, DirImported, GlobalEnvironment,
};
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
    /// The declared module DIR.
    dir_declared: Arc<DirDeclared>,
    /// The imported module DIR.
    dir_imported: Arc<DirImported>,
    /// The expanded module DIR.
    dir_expanded: Arc<DirExpanded>,
    /// The exported module DIR.
    dir_exported: Arc<DirExported>,
    /// The checked module DIR.
    dir_checked: Arc<DirChecked>,
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
pub(crate) struct AstQueryContext<'a> {
    /// The file id for this ast view.
    file_id: FileId,
    /// The module AST.
    ast: &'a Ast,
}

impl<'a> AstQueryContext<'a> {
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
pub(crate) struct DirQueryContext<'a> {
    /// The module id for this dir view.
    module_id: ModuleId,
    /// The revision for this dir view.
    revision: Revision,
    /// The declared DIR artifact.
    declared: &'a DirDeclared,
    /// The imported DIR artifact.
    imported: &'a DirImported,
    /// The expanded DIR artifact.
    expanded: &'a DirExpanded,
    /// The exported DIR artifact.
    exported: &'a DirExported,
    /// The checked DIR artifact.
    checked: &'a DirChecked,
}

impl<'a> DirQueryContext<'a> {
    /// Return the module id for this dir view.
    pub(crate) fn module_id(self) -> ModuleId {
        self.module_id
    }

    /// Return the revision for this dir view.
    pub(crate) fn revision(self) -> Revision {
        self.revision
    }

    /// Return the visible DIR tree view.
    pub(crate) fn view(self) -> dir::View<'a> {
        dir::View::patched(&self.declared.tree, &self.expanded.patch)
    }

    /// Return whether one DIR symbol is visible in this query view.
    pub(crate) fn symbol_is_active(self, symbol_id: dir::GlobalSymbolId) -> bool {
        if symbol_id.module_id != self.module_id {
            return true;
        }

        let symbol = self.symbols().get_symbol(symbol_id.local_id);
        let Some(declaration) = symbol.declaration else {
            return true;
        };

        self.view().is_active(declaration.local_id)
    }

    /// Return the DIR symbol table.
    pub(crate) fn symbols(self) -> &'a dir::SymbolTable {
        &self.declared.symbols
    }

    /// Return the DIR type table.
    pub(crate) fn types(self) -> &'a dir::TypeTable {
        &self.checked.types
    }

    /// Return the top level DIR roots.
    pub(crate) fn roots(self) -> &'a [dir::LocalNodeId<dir::Expression>] {
        self.declared.roots.as_ref()
    }

    /// Return the DIR string pool.
    pub(crate) fn strings(self) -> &'a StringPool {
        &self.declared.strings
    }

    /// Return the namespace scope for this module.
    pub(crate) fn namespace_scope(self) -> dir::LocalScopeId {
        self.declared.namespace_scope
    }

    /// Return the imported DIR artifact.
    pub(crate) fn imported(self) -> &'a DirImported {
        self.imported
    }

    /// Return the exported DIR artifact.
    pub(crate) fn exported(self) -> &'a DirExported {
        self.exported
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
    pub(crate) fn ast(&self) -> AstQueryContext<'_> {
        AstQueryContext {
            file_id: self.file_id,
            ast: self.ast.as_ref(),
        }
    }

    /// Return the dir query surface.
    pub(crate) fn dir(&self) -> DirQueryContext<'_> {
        DirQueryContext {
            module_id: self.module_id,
            revision: self.revision,
            declared: self.dir_declared.as_ref(),
            imported: self.dir_imported.as_ref(),
            expanded: self.dir_expanded.as_ref(),
            exported: self.dir_exported.as_ref(),
            checked: self.dir_checked.as_ref(),
        }
    }

    /// Return the global environment for this query profile.
    pub(crate) fn global_environment(
        &self,
        repository: &Repository,
    ) -> Option<Arc<GlobalEnvironment>> {
        let key = ArtifactKey::global_environment(self.profile_id);
        let version = artifact_version(repository, self.revision, key)?;

        repository.artifact_store().global_environment(&version)
    }
}

/// Get query context for a module with one explicit profile.
pub(crate) fn query_context_for_profile(
    repository: &Repository,
    revision: Revision,
    module_id: ModuleId,
    profile: ProfileId,
) -> Option<QueryContext> {
    let module = repository.module(revision, module_id).ok().flatten()?;
    let artifacts = repository.artifact_store().clone();
    let selected_profile = repository
        .module_profile_by_id(revision, module.id, profile)
        .ok()
        .flatten()?
        .id();

    // resolve and retain the exact source artifacts
    let ast_version = artifact_version(repository, revision, ArtifactKey::ast(module.id))?;
    let declared_version = artifact_version(
        repository,
        revision,
        ArtifactKey::dir_declared(module.id, selected_profile),
    )?;
    let imported_version = artifact_version(
        repository,
        revision,
        ArtifactKey::dir_imported(module.id, selected_profile),
    )?;
    let expanded_version = artifact_version(
        repository,
        revision,
        ArtifactKey::dir_expanded(module.id, selected_profile),
    )?;
    let exported_version = artifact_version(
        repository,
        revision,
        ArtifactKey::dir_exported(module.id, selected_profile),
    )?;
    let checked_version = artifact_version(
        repository,
        revision,
        ArtifactKey::dir_checked(module.id, selected_profile),
    )?;

    // resolve module ast and profile dir artifact
    let ast = artifacts.ast(&ast_version)?;
    let dir_declared = artifacts.dir_declared(&declared_version)?;
    let dir_imported = artifacts.dir_imported(&imported_version)?;
    let dir_expanded = artifacts.dir_expanded(&expanded_version)?;
    let dir_exported = artifacts.dir_exported(&exported_version)?;
    let dir_checked = artifacts.dir_checked(&checked_version)?;

    // artifact roots
    let ast_pin = artifacts.pin(&ast_version)?;
    let declared_pin = artifacts.pin(&declared_version)?;
    let imported_pin = artifacts.pin(&imported_version)?;
    let expanded_pin = artifacts.pin(&expanded_version)?;
    let exported_pin = artifacts.pin(&exported_version)?;
    let checked_pin = artifacts.pin(&checked_version)?;

    // build query context
    Some(QueryContext {
        _pins: vec![
            ast_pin,
            declared_pin,
            imported_pin,
            expanded_pin,
            exported_pin,
            checked_pin,
        ],
        ast,
        dir_declared,
        dir_imported,
        dir_expanded,
        dir_exported,
        dir_checked,
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
    let profile = repository.module_profile(revision, module_id).ok()?.id();

    query_context_for_profile(repository, revision, module_id, profile)
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
    f: impl FnOnce(AstQueryContext<'_>) -> T,
) -> Option<T> {
    let module = get_module_by_file_id(repository, revision, file_id)?;
    with_ast_query_for_module(repository, revision, module.id, f)
}

/// Execute a closure with an ast query surface for one module.
pub(crate) fn with_ast_query_for_module<T>(
    repository: &Repository,
    revision: Revision,
    module_id: ModuleId,
    f: impl FnOnce(AstQueryContext<'_>) -> T,
) -> Option<T> {
    let module = repository.module(revision, module_id).ok().flatten()?;
    let artifacts = repository.artifact_store().clone();
    let ast_version = artifact_version(repository, revision, ArtifactKey::ast(module.id))?;
    let ast = artifacts.ast(&ast_version)?;
    let query = AstQueryContext {
        file_id: module.file_id,
        ast: ast.as_ref(),
    };

    Some(f(query))
}

/// Return one recorded artifact version.
fn artifact_version(
    repository: &Repository,
    revision: Revision,
    artifact_key: ArtifactKey,
) -> Option<ArtifactVersion> {
    repository
        .artifact_version(revision, &artifact_key)
        .ok()
        .flatten()
}
