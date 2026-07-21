use std::slice;
use std::sync::Arc;

use destack_artifact::{
    DiagnosticAnchor, DirExpanded, DirExported, DirParsed, DirResolved, GlobalEnvironment,
};
use destack_core::StringPool;
use destack_dir as dir;
use destack_repository::{ArtifactReader, Module, ProfileId, ProviderError, Repository, Revision};
use destack_source::{File, FileId, ModuleId, NodeSpanBoundary, NodeSpanType, Span, TargetId};

/// One module's checked DIR.
#[derive(Debug)]
pub struct DirModule {
    /// The module id.
    pub id: ModuleId,
    /// The active profile.
    pub profile: ProfileId,
    /// The active target.
    pub target: TargetId,
    /// The global language environment.
    pub environment: Arc<GlobalEnvironment>,
    /// The contributing source files in parsed order.
    pub files: Box<[Arc<File>]>,
    /// The parsed DIR artifact.
    pub parsed: Arc<DirParsed>,
    /// The expanded DIR artifact.
    pub expanded: Arc<DirExpanded>,
    /// The resolved import and source-reference artifact.
    pub resolved: Arc<DirResolved>,
    /// The resolved export artifact.
    pub exported: Arc<DirExported>,
    /// The DIR string pool.
    pub strings: Arc<StringPool>,
    /// The binding table.
    pub bindings: dir::BindingTable<'static>,
    /// The module dependency table.
    pub modules: dir::ModuleTable<'static>,
    /// The type table.
    pub types: dir::TypeTable<'static>,
    /// The static table.
    pub statics: dir::StaticTable<'static>,
    /// The checked decorator table.
    pub decorators: dir::DecoratorTable<'static>,
    /// The auto implementation table.
    pub auto: dir::AutoTable<'static>,
    /// The resolution table.
    pub resolutions: dir::ResolutionTable<'static>,
    /// The generic table.
    pub generics: dir::GenericTable<'static>,
    /// The definition table.
    pub definitions: dir::DefinitionTable<'static>,
    /// The coercion table.
    pub coercions: dir::CoercionTable<'static>,
    /// The capture table.
    pub captures: dir::CaptureTable<'static>,
    /// The top-level expression roots.
    pub roots: Vec<dir::LocalNodeId<dir::Expression>>,
    /// The stable module node.
    pub module_node: dir::LocalNodeIdAny,
    /// The module namespace scope.
    pub namespace_scope: dir::LocalScopeId,
}

impl DirModule {
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

        Ok(Span::new(span.file, start as u32, span.end))
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

    /// Load one module's checked DIR.
    pub(crate) fn load(
        repository: &Repository,
        revision: Revision,
        profile: ProfileId,
        target: TargetId,
        environment: Arc<GlobalEnvironment>,
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
        let bindings = checked.binding_table(&bound, &expanded);
        let modules = expanded.module_table(&imported);
        let types = checked.type_table(&bound, &expanded);
        let statics = checked.static_table(&bound, &expanded);
        let decorators = checked.decorator_table();
        let auto = checked.auto_table();
        let resolutions = checked.resolution_table();
        let generics = checked.generic_table();
        let definitions = checked.definition_table();
        let coercions = checked.coercion_table();
        let captures = checked.capture_table();
        let roots = expanded.roots.clone();
        let module_node = bound.module_node;
        let namespace_scope = bound.namespace_scope;

        Ok(Self {
            id: module_id,
            profile,
            target,
            environment,
            files: files.into_boxed_slice(),
            parsed,
            expanded,
            resolved,
            exported,
            strings: repository.string_pool().clone(),
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
