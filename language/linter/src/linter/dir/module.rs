use std::slice;
use std::sync::Arc;

use destack_artifact::{DiagnosticAnchor, DirExpanded, DirExported, DirParsed, DirResolved};
use destack_dir as dir;
use destack_repository::{ArtifactReader, Module, ProfileId, ProviderError, Repository, Revision};
use destack_source::{
    File, FileId, ModuleId, NodeSpanBoundary, NodeSpanRegion, NodeSpanType, Span,
};

use super::Dir;

/// A borrowed checked DIR module.
#[derive(Debug, Clone, Copy)]
pub struct DirModule<'a> {
    /// The indexed checked DIR.
    pub dir: &'a Dir<'a>,
    /// The module id.
    pub id: ModuleId,
    /// The contributing source files in parsed order.
    pub files: &'a [Arc<File>],
    /// The parsed DIR artifact.
    pub parsed: &'a DirParsed,
    /// The expanded DIR artifact.
    pub expanded: &'a DirExpanded,
    /// The resolved import and source-reference artifact.
    pub resolved: &'a DirResolved,
    /// The resolved export artifact.
    pub exported: &'a DirExported,
    /// The binding table.
    pub bindings: &'a dir::BindingTable<'static>,
    /// The module dependency table.
    pub modules: &'a dir::ModuleTable<'static>,
    /// The type table.
    pub types: &'a dir::TypeTable<'static>,
    /// The static table.
    pub statics: &'a dir::StaticTable<'static>,
    /// The checked decorator table.
    pub decorators: &'a dir::DecoratorTable<'static>,
    /// The auto implementation table.
    pub auto: &'a dir::AutoTable<'static>,
    /// The resolution table.
    pub resolutions: &'a dir::ResolutionTable<'static>,
    /// The generic table.
    pub generics: &'a dir::GenericTable<'static>,
    /// The definition table.
    pub definitions: &'a dir::DefinitionTable<'static>,
    /// The coercion table.
    pub coercions: &'a dir::CoercionTable<'static>,
    /// The capture table.
    pub captures: &'a dir::CaptureTable<'static>,
    /// The top-level expression roots.
    pub roots: &'a [dir::LocalNodeId<dir::Expression>],
    /// The stable module node.
    pub module_node: dir::LocalNodeIdAny,
    /// The module namespace scope.
    pub namespace_scope: dir::LocalScopeId,
}

/// Owned checked DIR storage for one module.
#[derive(Debug)]
pub(super) struct DirModuleStorage {
    /// The module id.
    pub(super) id: ModuleId,
    /// The contributing source files in parsed order.
    files: Box<[Arc<File>]>,
    /// The parsed DIR artifact.
    parsed: Arc<DirParsed>,
    /// The expanded DIR artifact.
    expanded: Arc<DirExpanded>,
    /// The resolved import and source-reference artifact.
    resolved: Arc<DirResolved>,
    /// The resolved export artifact.
    exported: Arc<DirExported>,
    /// The binding table.
    pub(super) bindings: dir::BindingTable<'static>,
    /// The module dependency table.
    modules: dir::ModuleTable<'static>,
    /// The type table.
    pub(super) types: dir::TypeTable<'static>,
    /// The static table.
    pub(super) statics: dir::StaticTable<'static>,
    /// The checked decorator table.
    decorators: dir::DecoratorTable<'static>,
    /// The auto implementation table.
    auto: dir::AutoTable<'static>,
    /// The resolution table.
    resolutions: dir::ResolutionTable<'static>,
    /// The generic table.
    pub(super) generics: dir::GenericTable<'static>,
    /// The definition table.
    pub(super) definitions: dir::DefinitionTable<'static>,
    /// The coercion table.
    coercions: dir::CoercionTable<'static>,
    /// The capture table.
    captures: dir::CaptureTable<'static>,
    /// The top-level expression roots.
    roots: Vec<dir::LocalNodeId<dir::Expression>>,
    /// The stable module node.
    module_node: dir::LocalNodeIdAny,
    /// The module namespace scope.
    namespace_scope: dir::LocalScopeId,
}

impl<'a> DirModule<'a> {
    /// Create a borrowed checked DIR module.
    pub(super) fn new(dir: &'a Dir<'a>, storage: &'a DirModuleStorage) -> Self {
        Self {
            dir,
            id: storage.id,
            files: &storage.files,
            parsed: &storage.parsed,
            expanded: &storage.expanded,
            resolved: &storage.resolved,
            exported: &storage.exported,
            bindings: &storage.bindings,
            modules: &storage.modules,
            types: &storage.types,
            statics: &storage.statics,
            decorators: &storage.decorators,
            auto: &storage.auto,
            resolutions: &storage.resolutions,
            generics: &storage.generics,
            definitions: &storage.definitions,
            coercions: &storage.coercions,
            captures: &storage.captures,
            roots: &storage.roots,
            module_node: storage.module_node,
            namespace_scope: storage.namespace_scope,
        }
    }

    /// Return the reduced checked type of one local node.
    pub fn node_type(&self, node: dir::LocalNodeIdAny) -> Result<dir::Type, ProviderError> {
        let type_id = self.node_type_id(node)?;

        self.dir.get_type(type_id)
    }

    /// Return the reduced checked type id of one local node.
    pub fn node_type_id(
        &self,
        node: dir::LocalNodeIdAny,
    ) -> Result<dir::GlobalTypeId, ProviderError> {
        let node = node.into_global(self.id);
        self.types.get_reduced_node_type_id(node).ok_or_else(|| {
            ProviderError::internal(format!("checked DIR node {node:?} has no reduced type"))
        })
    }

    /// Return one checked node's type after its selected adjustments.
    pub fn adjusted_type(&self, node: dir::LocalNodeIdAny) -> Result<dir::Type, ProviderError> {
        let global = node.into_global(self.id);
        let type_id = match self.coercions.coercion(global) {
            Some(coercion) => coercion.target(),
            None => self.node_type_id(node)?,
        };

        self.dir.get_type(type_id)
    }

    /// Return the required source span for one DIR node.
    pub fn span(&self, node: dir::LocalNodeIdAny) -> Result<Span, ProviderError> {
        self.view()
            .get_span_by_id(node.id)
            .ok_or_else(|| ProviderError::Internal {
                message: format!(
                    "DIR node {} in module {:?} has no source span",
                    node.id, self.id
                ),
            })
    }

    /// Return the main source span for one DIR node.
    pub fn main_span(&self, node: dir::LocalNodeIdAny) -> Result<Span, ProviderError> {
        self.view()
            .get_side_span_by_id(node.id, NodeSpanType::Main)
            .ok_or_else(|| ProviderError::Internal {
                message: format!(
                    "DIR node {} in module {:?} has no main source span",
                    node.id, self.id
                ),
            })
    }

    /// Return the complete authored source span for one DIR node.
    pub fn source_extent(&self, node: dir::LocalNodeIdAny) -> Result<Span, ProviderError> {
        self.view()
            .get_source_extent_by_id(node.id)
            .ok_or_else(|| ProviderError::Internal {
                message: format!(
                    "DIR node {} in module {:?} has no source extent",
                    node.id, self.id
                ),
            })
    }

    /// Return the authored parentheses around one DIR node.
    pub fn source_parentheses(&self, node: dir::LocalNodeIdAny) -> Option<Span> {
        self.view()
            .get_side_span_by_id(node.id, NodeSpanType::Region(NodeSpanRegion::Parentheses))
    }

    /// Return the source text covered by one span.
    pub fn source(&self, span: Span) -> Result<&str, ProviderError> {
        let source = self.file(span.file)?.text();
        let range = span.start as usize..span.end as usize;

        source.get(range).ok_or_else(|| ProviderError::Internal {
            message: format!("source span {span:?} is not a valid UTF-8 range"),
        })
    }

    /// Return whether an extent contains a comment outside the retained spans.
    pub fn has_unretained_comment(
        &self,
        extent: Span,
        retained: &[Span],
    ) -> Result<bool, ProviderError> {
        let parsed_file = self.parsed.file(extent.file).ok_or_else(|| {
            ProviderError::internal(format!(
                "source file {:?} is absent from parsed lint module {:?}",
                extent.file, self.id
            ))
        })?;
        let has_comment = parsed_file.comments.iter().any(|comment| {
            extent.contains_span(comment.span)
                && !retained
                    .iter()
                    .any(|retained| retained.contains_span(comment.span))
        });

        Ok(has_comment)
    }

    /// Return the source span and trailing boundary for one DIR statement.
    pub fn statement_span(
        &self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> Result<Span, ProviderError> {
        let view = self.view();
        let span = self.span(expression.into_any())?;
        let trailing = view.get_side_span(
            expression,
            NodeSpanType::Boundary(NodeSpanBoundary::Trailing),
        );

        Ok(trailing.map_or(span, |trailing| span.merge(trailing)))
    }

    /// Return the source span removed with one DIR statement.
    pub fn statement_removal_span(
        &self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> Result<Span, ProviderError> {
        let span = self.statement_span(expression)?;
        let source = self.file(span.file)?.text().as_bytes();
        let mut start = span.start as usize;

        // absorb the horizontal separator immediately before the statement
        while start > 0 && matches!(source[start - 1], b' ' | b'\t') {
            start -= 1;
        }

        // absorb the line break when only the block close follows
        let mut end = span.end as usize;
        if (start == 0 || source[start - 1] == b'\n') && source.get(end) == Some(&b'\n') {
            let mut next = end + 1;
            while next < source.len() && matches!(source[next], b' ' | b'\t') {
                next += 1;
            }
            if source.get(next) == Some(&b'}') {
                end += 1;
            }
        }

        Ok(Span::new(span.file, start as u32, end as u32))
    }

    /// Return a source anchor for one DIR node.
    pub fn anchor(&self, node: dir::LocalNodeIdAny) -> Result<DiagnosticAnchor, ProviderError> {
        let span = self.span(node)?;

        Ok(DiagnosticAnchor::Span(span))
    }

    /// Return the checked DIR tree.
    pub fn view(&self) -> dir::View<'_> {
        dir::View::with_patches(&self.parsed.tree, slice::from_ref(&self.expanded.patch))
    }

    /// Return one source file.
    pub fn file(&self, file_id: FileId) -> Result<&File, ProviderError> {
        self.files
            .iter()
            .find(|file| file.id == file_id)
            .map(AsRef::as_ref)
            .ok_or_else(|| ProviderError::Internal {
                message: format!("file {file_id:?} is outside lint module {:?}", self.id),
            })
    }
}

impl DirModuleStorage {
    /// Load one module's checked DIR.
    pub(super) fn load(
        repository: &Repository,
        revision: Revision,
        profile: ProfileId,
        module: Arc<Module>,
        artifacts: &ArtifactReader<'_>,
    ) -> Result<Self, ProviderError> {
        let module_id = module.id;

        // read the DIR artifacts
        let parsed = artifacts.dir_parsed(module_id)?;
        let bound = artifacts.dir_bound(module_id, profile)?;
        let imported = artifacts.dir_imported(module_id, profile)?;
        let resolved = artifacts.dir_resolved(module_id, profile)?;
        let expanded = artifacts.dir_expanded(module_id, profile)?;
        let exported = artifacts.dir_exported(module_id, profile)?;
        let checked = artifacts.dir_checked(module_id, profile)?;
        let declared = artifacts.dir_declared(module_id, profile)?;
        let expected_files = module
            .files
            .iter()
            .map(|file| file.file_id)
            .collect::<Vec<_>>();
        let parsed_files = parsed
            .files
            .iter()
            .map(|file| file.file_id)
            .collect::<Vec<_>>();
        if parsed_files != expected_files {
            return Err(ProviderError::Internal {
                message: format!(
                    "lint module {module_id:?} parsed files {parsed_files:?} do not match repository files {expected_files:?}"
                ),
            });
        }

        // load source files
        let mut files = Vec::with_capacity(parsed_files.len());
        for file_id in parsed_files {
            let file = repository
                .file(revision, file_id)
                .map_err(|error| ProviderError::Internal {
                    message: error.to_string(),
                })?
                .ok_or_else(|| ProviderError::Internal {
                    message: format!("missing lint file {file_id:?}"),
                })?;
            files.push(file);
        }

        // compose the checked tables
        let bindings = checked.binding_table(&bound, &expanded, &declared);
        let modules = expanded.module_table(&imported);
        let types = checked.type_table(&bound, &expanded, &declared);
        let statics = checked.static_table(&bound, &expanded, &declared);
        let decorators = checked.decorator_table(&declared);
        let auto = checked.auto_table();
        let resolutions = checked.resolution_table(&declared);
        let generics = checked.generic_table(&declared);
        let definitions = checked.definition_table(&declared);
        let coercions = checked.coercion_table();
        let captures = checked.capture_table();
        let roots = expanded.roots.clone();
        let module_node = bound.module_node;
        let namespace_scope = bound.namespace_scope;

        Ok(Self {
            id: module_id,
            files: files.into_boxed_slice(),
            parsed,
            expanded,
            resolved,
            exported,
            bindings,
            modules,
            types,
            statics,
            decorators,
            auto,
            resolutions,
            generics,
            definitions,
            coercions,
            captures,
            roots,
            module_node,
            namespace_scope,
        })
    }
}
