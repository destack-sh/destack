use std::sync::{Arc, OnceLock};

use destack_artifact::{
    ArtifactKey, ComponentGraph, IndexKind, InferenceComponentIndex, ModuleIndex, PackageGraph,
    ProgramIndex,
};
use destack_dir as dir;
use destack_repository::{ArtifactReader, ProviderError, Repository, RepositoryError, Revision};
use destack_source::{ModuleId, ProfileId};
use parking_lot::Mutex;
use rustc_hash::FxHashMap;

use crate::{Module, ModuleQueryContext, QueryError, QueryMethod, QueryResult};

/// Query context anchored to a program revision.
pub struct ProgramQueryContext<'a> {
    /// The repository used for this query.
    repository: &'a Repository,
    /// The revision used for this query.
    revision: Revision,
    /// The profile used for module artifacts.
    profile_id: ProfileId,
    /// Module ids in stable program order.
    module_ids: Box<[ModuleId]>,
    /// Module contexts read by this query.
    modules: Mutex<FxHashMap<ModuleId, Arc<ModuleQueryContext<'a>>>>,
    /// Exact artifact requirements for actual query reads.
    require_artifacts: &'a dyn Fn(&[ArtifactKey]) -> QueryResult<()>,
    /// Lazily read package graph for import completion.
    package_graph: OnceLock<Result<Arc<PackageGraph>, ProviderError>>,
    /// Lazily read component graph for semantic index families.
    component_graph: OnceLock<Result<Arc<ComponentGraph>, ProviderError>>,
    /// Lazily read program indexes by family.
    program_indexes: [OnceLock<Result<Arc<ProgramIndex>, ProviderError>>; IndexKind::ALL.len()],
    /// Lazily read module indexes by module and family.
    module_indexes: OnceLock<
        Box<[[OnceLock<Result<Arc<ModuleIndex>, ProviderError>>; IndexKind::MODULE_COUNT]]>,
    >,
    /// Lazily read component indexes by component and family.
    component_indexes: OnceLock<
        Box<
            [[OnceLock<Result<Arc<InferenceComponentIndex>, ProviderError>>;
                 IndexKind::INFERENCE_COMPONENT_COUNT]],
        >,
    >,
}

impl std::fmt::Debug for ProgramQueryContext<'_> {
    /// Format the visible program query state.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ProgramQueryContext")
            .field("revision", &self.revision)
            .field("profile_id", &self.profile_id)
            .field("module_count", &self.module_ids.len())
            .field("loaded_modules", &self.modules.lock().len())
            .finish_non_exhaustive()
    }
}

impl<'a> ProgramQueryContext<'a> {
    /// Return the artifact roots scheduled before one program query.
    pub fn initial_roots(
        repository: &Repository,
        revision: Revision,
        profile_id: ProfileId,
        method: QueryMethod,
    ) -> QueryResult<Vec<ArtifactKey>> {
        let mut artifacts = Vec::new();
        if method == QueryMethod::Completion {
            artifacts.push(ArtifactKey::package_graph(profile_id));
        }
        let kinds = Self::index_kinds(method);
        if kinds.iter().any(|kind| kind.is_inference_component_owned()) {
            artifacts.push(ArtifactKey::component_graph(profile_id));
        }
        for kind in kinds {
            artifacts.push(ArtifactKey::program_index(profile_id, *kind));
        }

        // schedule import resolution for every module read by file renames
        if method == QueryMethod::RenameFiles {
            for module_id in Self::authored_module_ids(repository, revision)? {
                artifacts.extend([
                    ArtifactKey::dir_parsed(module_id),
                    ArtifactKey::dir_imported(module_id, profile_id),
                    ArtifactKey::dir_expanded(module_id, profile_id),
                ]);
            }
        }

        Ok(artifacts)
    }

    /// Iterate over the exact modules covered by this program context.
    pub fn modules(&self) -> impl Iterator<Item = Module> + '_ {
        self.module_ids.iter().map(move |module_id| Module {
            module_id: *module_id,
            profile_id: self.profile_id,
        })
    }

    /// Return the repository for this query.
    pub fn repository(&self) -> &'a Repository {
        self.repository
    }

    /// Return the exact revision for this query.
    pub fn revision(&self) -> Revision {
        self.revision
    }

    /// Return the profile used for module artifacts.
    pub fn profile_id(&self) -> ProfileId {
        self.profile_id
    }

    /// Return module ids in stable program order.
    pub(crate) fn module_ids(&self) -> &[ModuleId] {
        &self.module_ids
    }

    /// Return one module context at this revision.
    pub fn module(&self, module_id: ModuleId) -> QueryResult<Arc<ModuleQueryContext<'a>>> {
        self.module_ordinal(module_id)?;
        if let Some(context) = self.modules.lock().get(&module_id) {
            return Ok(context.clone());
        }

        // require and compose the module only after its first actual read
        let context = ModuleQueryContext::new(
            self.repository,
            self.revision,
            module_id,
            self.profile_id,
            self.require_artifacts,
        )?;
        let context = Arc::new(context);
        let mut modules = self.modules.lock();
        let context = modules.entry(module_id).or_insert(context);

        Ok(context.clone())
    }

    /// Read one global type with its owning module.
    pub(crate) fn read_type<R>(
        &self,
        type_id: dir::GlobalTypeId,
        read: impl FnOnce(&dir::Type, &ModuleQueryContext<'_>) -> QueryResult<R>,
    ) -> QueryResult<R> {
        let module = self.module(type_id.module_id)?;
        let type_value = module.types().get_type(type_id.local_id);

        read(&type_value, &module)
    }

    /// Return whether one module belongs to authored workspace source.
    pub(crate) fn is_authored_module(&self, module_id: ModuleId) -> QueryResult<bool> {
        let package_id = module_id.package_id;
        let package = self.repository.package(self.revision, package_id)?.ok_or(
            RepositoryError::MissingPackage {
                package: package_id,
            },
        )?;

        Ok(package.kind.is_authored())
    }

    /// Return the authored modules covered by this program context.
    pub fn authored_modules(&self) -> QueryResult<Vec<Module>> {
        let module_ids = Self::authored_module_ids(self.repository(), self.revision())?;
        let modules = module_ids
            .into_iter()
            .map(|module_id| Module {
                module_id,
                profile_id: self.profile_id(),
            })
            .collect();

        Ok(modules)
    }

    /// Return module ids owned by authored workspace packages.
    fn authored_module_ids(
        repository: &Repository,
        revision: Revision,
    ) -> QueryResult<Vec<ModuleId>> {
        let mut authored = Vec::new();

        // retain modules from declared and implicit packages
        for module_id in repository.module_ids(revision)? {
            let package_id = module_id.package_id;
            let package = repository.package(revision, package_id)?.ok_or(
                RepositoryError::MissingPackage {
                    package: package_id,
                },
            )?;
            if package.kind.is_authored() {
                authored.push(module_id);
            }
        }

        Ok(authored)
    }

    /// Read the active package graph for this program.
    pub(crate) fn package_graph(&self) -> QueryResult<&PackageGraph> {
        let artifact = ArtifactKey::package_graph(self.profile_id());
        (self.require_artifacts)(&[artifact])?;
        let graph = self.package_graph.get_or_init(|| {
            let artifacts = ArtifactReader::new(self.repository(), self.revision());

            artifacts.package_graph(self.profile_id())
        });

        match graph {
            Ok(graph) => Ok(graph.as_ref()),
            Err(error) => Err(QueryError::from(error.clone())),
        }
    }

    /// Return symbol postings.
    pub(crate) fn symbol_postings(&self) -> QueryResult<&dir::SymbolPostings> {
        match self.program_index(IndexKind::Symbols)? {
            ProgramIndex::Symbols(postings) => Ok(postings),
            index => Err(QueryError::invalid(format!(
                "expected symbol index, found {:?}",
                index.kind()
            ))),
        }
    }

    /// Return export postings.
    pub(crate) fn export_postings(&self) -> QueryResult<&dir::ExportPostings> {
        match self.program_index(IndexKind::Exports)? {
            ProgramIndex::Exports(postings) => Ok(postings),
            index => Err(QueryError::invalid(format!(
                "expected export index, found {:?}",
                index.kind()
            ))),
        }
    }

    /// Return reference postings.
    pub(crate) fn reference_postings(&self) -> QueryResult<&dir::ReferencePostings> {
        match self.program_index(IndexKind::References)? {
            ProgramIndex::References(postings) => Ok(postings),
            index => Err(QueryError::invalid(format!(
                "expected reference index, found {:?}",
                index.kind()
            ))),
        }
    }

    /// Return call postings.
    pub(crate) fn call_postings(&self) -> QueryResult<&dir::CallPostings> {
        match self.program_index(IndexKind::Calls)? {
            ProgramIndex::Calls(postings) => Ok(postings),
            index => Err(QueryError::invalid(format!(
                "expected call index, found {:?}",
                index.kind()
            ))),
        }
    }

    /// Return heritage postings.
    pub(crate) fn heritage_postings(&self) -> QueryResult<&dir::HeritagePostings> {
        match self.program_index(IndexKind::Heritage)? {
            ProgramIndex::Heritage(postings) => Ok(postings),
            index => Err(QueryError::invalid(format!(
                "expected heritage index, found {:?}",
                index.kind()
            ))),
        }
    }

    /// Return decorator postings.
    pub(crate) fn decorator_postings(&self) -> QueryResult<&dir::DecoratorPostings> {
        match self.program_index(IndexKind::Decorators)? {
            ProgramIndex::Decorators(postings) => Ok(postings),
            index => Err(QueryError::invalid(format!(
                "expected decorator index, found {:?}",
                index.kind()
            ))),
        }
    }

    /// Return one module symbol index.
    pub(crate) fn symbol_index(&self, module_id: ModuleId) -> QueryResult<&dir::SymbolIndex> {
        match self.module_index(module_id, IndexKind::Symbols)? {
            ModuleIndex::Symbols(index) => Ok(index),
            index => Err(Self::unexpected_module_index(IndexKind::Symbols, index)),
        }
    }

    /// Return one module symbol index selected by a program ordinal.
    pub(crate) fn symbol_index_at(
        &self,
        ordinal: u32,
    ) -> QueryResult<(ModuleId, &dir::SymbolIndex)> {
        let (module_id, index) = self.module_index_at(ordinal, IndexKind::Symbols)?;
        let ModuleIndex::Symbols(index) = index else {
            return Err(Self::unexpected_module_index(IndexKind::Symbols, index));
        };

        Ok((module_id, index))
    }

    /// Return one module export index.
    pub(crate) fn export_index(&self, module_id: ModuleId) -> QueryResult<&dir::ExportIndex> {
        match self.module_index(module_id, IndexKind::Exports)? {
            ModuleIndex::Exports(index) => Ok(index),
            index => Err(Self::unexpected_module_index(IndexKind::Exports, index)),
        }
    }

    /// Return one module export index selected by a program ordinal.
    pub(crate) fn export_index_at(
        &self,
        ordinal: u32,
    ) -> QueryResult<(ModuleId, &dir::ExportIndex)> {
        let (module_id, index) = self.module_index_at(ordinal, IndexKind::Exports)?;
        let ModuleIndex::Exports(index) = index else {
            return Err(Self::unexpected_module_index(IndexKind::Exports, index));
        };

        Ok((module_id, index))
    }

    /// Return one module reference index selected by a program ordinal.
    pub(crate) fn reference_index_at(
        &self,
        ordinal: u32,
    ) -> QueryResult<(ModuleId, &dir::ReferenceIndex)> {
        let (module_id, index) = self.module_index_at(ordinal, IndexKind::References)?;
        let ModuleIndex::References(index) = index else {
            return Err(Self::unexpected_module_index(IndexKind::References, index));
        };

        Ok((module_id, index))
    }

    /// Return one module call index selected by a program ordinal.
    pub(crate) fn call_index_at(&self, ordinal: u32) -> QueryResult<(ModuleId, &dir::CallIndex)> {
        let (module_id, index) = self.module_index_at(ordinal, IndexKind::Calls)?;
        let ModuleIndex::Calls(index) = index else {
            return Err(Self::unexpected_module_index(IndexKind::Calls, index));
        };

        Ok((module_id, index))
    }

    /// Return one module heritage index selected by a program ordinal.
    pub(crate) fn heritage_index_at(
        &self,
        ordinal: u32,
    ) -> QueryResult<(ModuleId, &dir::HeritageIndex)> {
        let (module_id, index) = self.module_index_at(ordinal, IndexKind::Heritage)?;
        let ModuleIndex::Heritage(index) = index else {
            return Err(Self::unexpected_module_index(IndexKind::Heritage, index));
        };

        Ok((module_id, index))
    }

    /// Return one module decorator index selected by a program ordinal.
    pub(crate) fn decorator_index_at(
        &self,
        ordinal: u32,
    ) -> QueryResult<(ModuleId, &dir::DecoratorIndex)> {
        let (module_id, index) = self.module_index_at(ordinal, IndexKind::Decorators)?;
        let ModuleIndex::Decorators(index) = index else {
            return Err(Self::unexpected_module_index(IndexKind::Decorators, index));
        };

        Ok((module_id, index))
    }

    /// Build one program query context.
    pub fn new(
        repository: &'a Repository,
        revision: Revision,
        profile_id: ProfileId,
        require_artifacts: &'a dyn Fn(&[ArtifactKey]) -> QueryResult<()>,
    ) -> QueryResult<Self> {
        let mut module_ids = repository.module_ids(revision)?;
        module_ids.sort_unstable();
        module_ids.dedup();
        Ok(Self {
            repository,
            revision,
            profile_id,
            module_ids: module_ids.into_boxed_slice(),
            modules: Mutex::new(FxHashMap::default()),
            require_artifacts,
            package_graph: OnceLock::new(),
            component_graph: OnceLock::new(),
            program_indexes: std::array::from_fn(|_| OnceLock::new()),
            module_indexes: OnceLock::new(),
            component_indexes: OnceLock::new(),
        })
    }

    /// Read one exact program index artifact.
    fn program_index(&self, kind: IndexKind) -> QueryResult<&ProgramIndex> {
        let artifact = ArtifactKey::program_index(self.profile_id(), kind);
        (self.require_artifacts)(&[artifact])?;
        let index = self.program_indexes[kind.program_index_ordinal()].get_or_init(|| {
            let artifacts = ArtifactReader::new(self.repository(), self.revision());

            artifacts.program_index(self.profile_id(), kind)
        });

        match index {
            Ok(index) => Ok(index.as_ref()),
            Err(error) => Err(QueryError::from(error.clone())),
        }
    }

    /// Read one exact module index artifact.
    fn module_index(&self, module_id: ModuleId, kind: IndexKind) -> QueryResult<&ModuleIndex> {
        if kind.is_inference_component_owned() {
            return self.component_module_index(module_id, kind);
        }

        // require the exact module index before reading its payload
        let artifact = ArtifactKey::module_index(module_id, self.profile_id(), kind);
        (self.require_artifacts)(&[artifact])?;
        let kind_ordinal = kind
            .module_index_ordinal()
            .ok_or_else(|| QueryError::invalid(format!("non-module index family: {kind:?}")))?;
        let ordinal = self.module_ordinal(module_id)?;
        let module_indexes = self.module_indexes.get_or_init(|| {
            (0..self.module_ids.len())
                .map(|_| std::array::from_fn(|_| OnceLock::new()))
                .collect::<Vec<_>>()
                .into_boxed_slice()
        });
        let index = module_indexes[ordinal][kind_ordinal].get_or_init(|| {
            let artifacts = ArtifactReader::new(self.repository(), self.revision());

            artifacts.module_index(module_id, self.profile_id(), kind)
        });

        match index {
            Ok(index) => Ok(index.as_ref()),
            Err(error) => Err(QueryError::from(error.clone())),
        }
    }

    /// Read one exact component index row.
    fn component_module_index(
        &self,
        module_id: ModuleId,
        kind: IndexKind,
    ) -> QueryResult<&ModuleIndex> {
        let graph = self.component_graph()?;
        let component = graph
            .inference_component(module_id)
            .ok_or_else(|| QueryError::missing(format!("module component: {module_id:?}")))?;
        let ordinal = graph
            .inference_components()
            .binary_search(&component)
            .map_err(|_| QueryError::invalid(format!("inference component: {component:?}")))?;
        let kind_ordinal = kind
            .inference_component_index_ordinal()
            .ok_or_else(|| QueryError::invalid(format!("non-component index family: {kind:?}")))?;
        let artifact = ArtifactKey::inference_component_index(component, self.profile_id(), kind);
        (self.require_artifacts)(&[artifact])?;
        let component_indexes = self.component_indexes.get_or_init(|| {
            (0..graph.inference_components().len())
                .map(|_| std::array::from_fn(|_| OnceLock::new()))
                .collect::<Vec<_>>()
                .into_boxed_slice()
        });
        let index = component_indexes[ordinal][kind_ordinal].get_or_init(|| {
            let artifacts = ArtifactReader::new(self.repository(), self.revision());

            artifacts.inference_component_index(component, self.profile_id(), kind)
        });
        let index = match index {
            Ok(index) => index,
            Err(error) => return Err(QueryError::from(error.clone())),
        };
        if index.component != component || index.kind != kind {
            return Err(QueryError::invalid(format!(
                "component index differs from its request: component={component:?}, kind={kind:?}"
            )));
        }

        index
            .get(module_id)
            .map(Arc::as_ref)
            .ok_or_else(|| QueryError::missing(format!("component index module: {module_id:?}")))
    }

    /// Read the exact component graph artifact.
    fn component_graph(&self) -> QueryResult<&ComponentGraph> {
        let artifact = ArtifactKey::component_graph(self.profile_id());
        (self.require_artifacts)(&[artifact])?;
        let graph = self.component_graph.get_or_init(|| {
            let artifacts = ArtifactReader::new(self.repository(), self.revision());

            artifacts.component_graph(self.profile_id())
        });

        match graph {
            Ok(graph) => Ok(graph.as_ref()),
            Err(error) => Err(QueryError::from(error.clone())),
        }
    }

    /// Return one exact module index selected by a program ordinal.
    fn module_index_at(
        &self,
        ordinal: u32,
        kind: IndexKind,
    ) -> QueryResult<(ModuleId, &ModuleIndex)> {
        let ordinal = ordinal as usize;
        let module_id = *self
            .module_ids
            .get(ordinal)
            .ok_or(QueryError::invalid(format!("program ordinal: {ordinal:?}")))?;
        let index = self.module_index(module_id, kind)?;

        Ok((module_id, index))
    }

    /// Return the index families read by one query method.
    fn index_kinds(method: QueryMethod) -> &'static [IndexKind] {
        use IndexKind::{
            Calls, Decorators, Exports, Extensions, Heritage, Members, References, Symbols,
        };

        match method {
            QueryMethod::Completion => &[Symbols, Exports, Members, Extensions],
            QueryMethod::CodeLenses => &[References, Heritage],
            QueryMethod::GotoImplementation | QueryMethod::Supertypes | QueryMethod::Subtypes => {
                &[Heritage]
            }
            QueryMethod::Highlight
            | QueryMethod::FindReferences
            | QueryMethod::Rename
            | QueryMethod::Inline => &[References],
            QueryMethod::IncomingCalls | QueryMethod::OutgoingCalls => &[Calls],
            QueryMethod::Decorators => &[Decorators],
            QueryMethod::CodeActions => &[Symbols, Exports, References],
            QueryMethod::SearchSymbols => &[Symbols],
            QueryMethod::Hover
            | QueryMethod::SignatureHelp
            | QueryMethod::InlayHints
            | QueryMethod::FoldingRanges
            | QueryMethod::SemanticTokens
            | QueryMethod::SemanticTokensRange
            | QueryMethod::Outline
            | QueryMethod::Links
            | QueryMethod::SelectionRanges
            | QueryMethod::GotoDefinition
            | QueryMethod::GotoDeclaration
            | QueryMethod::GotoTypeDefinition
            | QueryMethod::CallItem
            | QueryMethod::TypeItem
            | QueryMethod::RenameTarget
            | QueryMethod::RenameFiles
            | QueryMethod::ExtractVariable => &[],
        }
    }

    /// Return the ordinal for one indexed module.
    fn module_ordinal(&self, module_id: ModuleId) -> QueryResult<usize> {
        self.module_ids
            .binary_search(&module_id)
            .map_err(|_| QueryError::missing(format!("program module: {module_id:?}")))
    }

    /// Report an index family mismatch.
    fn unexpected_module_index(expected: IndexKind, index: &ModuleIndex) -> QueryError {
        QueryError::invalid(format!(
            "expected {expected:?} module index, found {:?}",
            index.kind()
        ))
    }
}
