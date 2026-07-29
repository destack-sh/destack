use std::collections::BTreeMap;
use std::sync::Arc;

use destack_artifact::{
    ArtifactDependency, ArtifactDependencySet, ArtifactKey, ArtifactPayload, ArtifactProjectionKey,
    ArtifactVersion, DirCheckedModule, IndexKind, InferenceComponentIndex,
    InferenceComponentModule, ModuleIndex, ProgramIndex, SourceDependency,
};
use destack_repository::{
    ArtifactReader, ProviderContext, ProviderError, ProviderResult, Repository,
};
use destack_source::{ComponentId, ModuleId, ProfileId};

use super::module::{
    CallIndexer, DecoratorIndexer, ExportIndexer, ExtensionIndexer, HeritageIndexer, MemberIndexer,
    ModuleIndexContext, ReferenceIndexer, SymbolIndexer,
};
use super::program::ProgramIndexer;

/// Provider that builds query index artifacts for one repository.
#[derive(Debug, Clone)]
pub struct Indexer {
    /// The repository being indexed.
    repository: Arc<Repository>,
}

impl Indexer {
    /// Create an index provider for one repository.
    pub fn new(repository: Arc<Repository>) -> Self {
        Self { repository }
    }

    /// Return the repository being indexed.
    pub(crate) fn repository(&self) -> &Repository {
        &self.repository
    }

    /// Collect the dependency closure for one index artifact.
    pub fn collect(&self, context: &dyn ProviderContext) -> ProviderResult<ArtifactDependencySet> {
        match context.artifact_key() {
            ArtifactKey::ModuleIndex {
                module,
                profile,
                kind,
            } => self.collect_module_index(module, profile, kind),
            ArtifactKey::InferenceComponentIndex {
                component,
                profile,
                kind,
            } => self.collect_component_index(context, component, profile, kind),
            ArtifactKey::ProgramIndex { profile, kind } => {
                self.collect_program_index(context, profile, kind)
            }
            artifact_key => Err(ProviderError::internal(format!(
                "non index artifact key reached index provider: {artifact_key:?}"
            ))
            .into()),
        }
    }

    /// Collect dependencies for one module index artifact.
    fn collect_module_index(
        &self,
        module_id: ModuleId,
        profile_id: ProfileId,
        kind: IndexKind,
    ) -> ProviderResult<ArtifactDependencySet> {
        if !kind.is_module_owned() {
            return Err(
                ProviderError::internal(format!("{kind:?} is not a module index family")).into(),
            );
        }

        let mut dependencies = ArtifactDependencySet::default();

        // index declarations directly from expanded DIR
        if kind == IndexKind::Symbols {
            dependencies.require(ArtifactKey::dir_parsed(module_id));
            dependencies.require(ArtifactKey::dir_bound(module_id, profile_id));
            dependencies.require(ArtifactKey::dir_expanded(module_id, profile_id));
        }
        // index exports from exported declarations and resolved dependencies
        else if kind == IndexKind::Exports {
            dependencies.require(ArtifactKey::dir_exported(module_id, profile_id));
            dependencies.require(ArtifactKey::dir_resolved(module_id, profile_id));
        } else {
            return Err(ProviderError::internal(format!(
                "unsupported module index family: {kind:?}"
            ))
            .into());
        }

        Ok(dependencies)
    }

    /// Collect dependencies for one checked component index artifact.
    fn collect_component_index(
        &self,
        context: &dyn ProviderContext,
        component_id: ComponentId,
        profile_id: ProfileId,
        kind: IndexKind,
    ) -> ProviderResult<ArtifactDependencySet> {
        if !kind.is_inference_component_owned() {
            return Err(ProviderError::internal(format!(
                "{kind:?} is not a component index family"
            ))
            .into());
        }

        let mut dependencies = ArtifactDependencySet::default();
        let graph_key = ArtifactKey::component_graph(profile_id);
        dependencies.require_projection(
            graph_key,
            ArtifactProjectionKey::InferenceMembers(component_id),
        );

        // resolve component membership before naming module inputs
        let artifacts = ArtifactReader::new(self.repository(), context.revision());
        let graph = match artifacts.component_graph_reader(profile_id) {
            Ok(graph) => graph,
            Err(ProviderError::Blocked { .. }) => {
                dependencies.mark_partial();

                return Ok(dependencies);
            }
            Err(error) => return Err(error.into()),
        };
        let module_ids = graph.inference_members(component_id)?;
        if module_ids.is_empty() {
            return Err(ProviderError::internal(format!(
                "query index component has no modules: {component_id}"
            ))
            .into());
        }

        // index every checked module row in this component
        dependencies.require(ArtifactKey::dir_checked_component(component_id, profile_id));
        for module_id in module_ids {
            dependencies.require(ArtifactKey::dir_parsed(*module_id));
            dependencies.require(ArtifactKey::dir_bound(*module_id, profile_id));
            dependencies.require(ArtifactKey::dir_expanded(*module_id, profile_id));
            dependencies.require(ArtifactKey::dir_resolved(*module_id, profile_id));
        }

        Ok(dependencies)
    }

    /// Collect dependencies for one program index artifact.
    fn collect_program_index(
        &self,
        context: &dyn ProviderContext,
        profile_id: ProfileId,
        kind: IndexKind,
    ) -> ProviderResult<ArtifactDependencySet> {
        let revision = context.revision();
        let mut module_ids = self.repository().module_ids(revision).map_err(|error| {
            ProviderError::internal(format!("failed to read program modules: {error}"))
        })?;
        module_ids.sort_unstable();
        module_ids.dedup();
        let mut dependencies = ArtifactDependencySet::default();
        let keys = if kind.is_module_owned() {
            module_ids
                .iter()
                .map(|module| ArtifactKey::module_index(*module, profile_id, kind))
                .collect::<Vec<_>>()
        } else {
            let graph_key = ArtifactKey::component_graph(profile_id);
            dependencies.require_projection(graph_key, ArtifactProjectionKey::InferenceComponents);

            // resolve component identities before naming component indexes
            let artifacts = ArtifactReader::new(self.repository(), revision);
            let graph = match artifacts.component_graph_reader(profile_id) {
                Ok(graph) => graph,
                Err(ProviderError::Blocked { .. }) => {
                    dependencies.mark_partial();

                    return Ok(dependencies);
                }
                Err(error) => return Err(error.into()),
            };
            graph
                .inference_components()?
                .iter()
                .map(|component| {
                    ArtifactKey::inference_component_index(*component, profile_id, kind)
                })
                .collect::<Vec<_>>()
        };

        // require every matching index owner
        for key in keys {
            dependencies.require(key);
        }
        dependencies.observe_modules(&module_ids);

        Ok(dependencies)
    }

    /// Provide one index artifact.
    pub fn provide(&self, context: &dyn ProviderContext) -> ProviderResult<ArtifactPayload> {
        match context.artifact_key() {
            ArtifactKey::ModuleIndex {
                module,
                profile,
                kind,
            } => self.provide_module_index(context, module, profile, kind),
            ArtifactKey::InferenceComponentIndex {
                component,
                profile,
                kind,
            } => self.provide_component_index(context, component, profile, kind),
            ArtifactKey::ProgramIndex { profile, kind } => {
                self.provide_program_index(context, profile, kind)
            }
            artifact_key => Err(ProviderError::internal(format!(
                "non index artifact key reached index provider: {artifact_key:?}"
            ))
            .into()),
        }
    }

    /// Provide one module index artifact.
    fn provide_module_index(
        &self,
        context: &dyn ProviderContext,
        module_id: ModuleId,
        profile_id: ProfileId,
        kind: IndexKind,
    ) -> ProviderResult<ArtifactPayload> {
        if !kind.is_module_owned() {
            return Err(
                ProviderError::internal(format!("{kind:?} is not a module index family")).into(),
            );
        }

        let revision = context.revision();
        let artifacts = ArtifactReader::new(self.repository(), revision)
            .restrict(context.artifact_key(), context.artifact_dependencies());

        // build the exact selected index family
        let payload = match kind {
            IndexKind::Symbols => {
                let parsed = artifacts.dir_parsed(module_id)?;
                let bound = artifacts.dir_bound(module_id, profile_id)?;
                let expanded = artifacts.dir_expanded(module_id, profile_id)?;
                let strings = self.repository().string_pool();

                ModuleIndex::Symbols(SymbolIndexer::build(&parsed, &bound, &expanded, strings)?)
            }
            IndexKind::Exports => {
                let exported = artifacts.dir_exported(module_id, profile_id)?;
                let resolved = artifacts.dir_resolved(module_id, profile_id)?;
                let strings = self.repository().string_pool();

                ModuleIndex::Exports(ExportIndexer::build(
                    module_id, &exported, &resolved, strings,
                )?)
            }
            kind => {
                return Err(ProviderError::internal(format!(
                    "unsupported module index family: {kind:?}"
                ))
                .into());
            }
        };

        Ok(ArtifactPayload::ModuleIndex(Arc::new(payload)))
    }

    /// Provide one checked component index artifact.
    fn provide_component_index(
        &self,
        context: &dyn ProviderContext,
        component_id: ComponentId,
        profile_id: ProfileId,
        kind: IndexKind,
    ) -> ProviderResult<ArtifactPayload> {
        if !kind.is_inference_component_owned() {
            return Err(ProviderError::internal(format!(
                "{kind:?} is not a component index family"
            ))
            .into());
        }

        let artifacts = ArtifactReader::new(self.repository(), context.revision())
            .restrict(context.artifact_key(), context.artifact_dependencies());
        let checked = artifacts.dir_checked_component(component_id, profile_id)?;
        if checked.component != component_id {
            return Err(ProviderError::internal(format!(
                "component index received checked component {:?}",
                checked.component
            ))
            .into());
        }
        let graph = artifacts.component_graph_reader(profile_id)?;
        let expected_modules = graph.inference_members(component_id)?;
        let checked_modules = checked
            .modules
            .iter()
            .map(|module| module.module)
            .collect::<Vec<_>>();
        if checked_modules != expected_modules {
            return Err(ProviderError::internal(format!(
                "checked component modules differ from component graph: component={component_id}"
            ))
            .into());
        }

        // build one matching index row for every checked module
        let mut modules = Vec::with_capacity(checked.modules.len());
        for checked_module in &checked.modules {
            let module_id = checked_module.module;
            let module =
                self.module_index_context(&artifacts, module_id, profile_id, checked_module)?;
            let index = match kind {
                IndexKind::Members => ModuleIndex::Members(MemberIndexer::build(&module)),
                IndexKind::References => ModuleIndex::References(ReferenceIndexer::build(&module)?),
                IndexKind::Calls => ModuleIndex::Calls(CallIndexer::build(&module)?),
                IndexKind::Heritage => ModuleIndex::Heritage(HeritageIndexer::build(&module)?),
                IndexKind::Extensions => ModuleIndex::Extensions(ExtensionIndexer::build(&module)?),
                IndexKind::Decorators => ModuleIndex::Decorators(DecoratorIndexer::build(&module)),
                kind => {
                    return Err(ProviderError::internal(format!(
                        "unsupported component index family: {kind:?}"
                    ))
                    .into());
                }
            };
            modules.push(InferenceComponentModule {
                module: module_id,
                index: Arc::new(index),
            });
        }

        let index = InferenceComponentIndex {
            component: component_id,
            kind,
            modules,
        };

        Ok(ArtifactPayload::InferenceComponentIndex(Arc::new(index)))
    }

    /// Read semantic DIR state for one module index row.
    fn module_index_context<'a>(
        &'a self,
        artifacts: &ArtifactReader<'_>,
        module_id: ModuleId,
        profile_id: ProfileId,
        checked: &DirCheckedModule,
    ) -> ProviderResult<ModuleIndexContext<'a>> {
        let parsed = artifacts.dir_parsed(module_id)?;
        let bound = artifacts.dir_bound(module_id, profile_id)?;
        let expanded = artifacts.dir_expanded(module_id, profile_id)?;
        let resolved = artifacts.dir_resolved(module_id, profile_id)?;
        let strings = self.repository().string_pool();

        Ok(ModuleIndexContext::new(
            strings, parsed, &bound, expanded, resolved, checked,
        ))
    }

    /// Provide one program index artifact.
    fn provide_program_index(
        &self,
        context: &dyn ProviderContext,
        profile_id: ProfileId,
        kind: IndexKind,
    ) -> ProviderResult<ArtifactPayload> {
        let repository = self.repository();
        let revision = context.revision();
        let started = repository.host().clock().now();
        let mut module_ids = repository.module_ids(revision).map_err(|error| {
            ProviderError::internal(format!("failed to read program modules: {error}"))
        })?;
        module_ids.sort_unstable();
        module_ids.dedup();
        if let Some(started) = started {
            context.emit_span("modules", started);
        }
        context.emit_counter("modules", module_ids.len() as u64);

        // read the exact current module versions
        let started = repository.host().clock().now();
        let dependencies = context.artifact_dependencies().ok_or_else(|| {
            ProviderError::internal("program index provider has no frozen dependencies")
        })?;
        let versions = Self::program_module_versions(repository, dependencies, profile_id, kind)?;
        Self::require_program_modules(&versions, &module_ids)?;
        if let Some(started) = started {
            context.emit_span("index_owners", started);
        }

        // update a matching predecessor or build the complete index
        let started = repository.host().clock().now();
        let payload = match Self::update_program_index(
            repository,
            context,
            profile_id,
            kind,
            &module_ids,
            &versions,
        )? {
            Some(index) => index,
            None => {
                let modules = Self::load_program_modules(repository, &versions, kind, &module_ids)?;
                let modules = modules.iter().map(Arc::as_ref).collect();
                let indexer = ProgramIndexer { modules };

                indexer.build(kind)?
            }
        };
        if let Some(started) = started {
            context.emit_span("index", started);
        }

        Ok(ArtifactPayload::ProgramIndex(Arc::new(payload)))
    }

    /// Collect each program module's index owner version.
    fn program_module_versions(
        repository: &Repository,
        dependencies: &[ArtifactDependency],
        profile_id: ProfileId,
        kind: IndexKind,
    ) -> ProviderResult<BTreeMap<ModuleId, ArtifactVersion>> {
        let mut versions = BTreeMap::<ModuleId, ArtifactVersion>::new();
        let graph_key = ArtifactKey::component_graph(profile_id);

        // collect one owner version for every indexed module
        for dependency in dependencies {
            let version = match dependency {
                ArtifactDependency::Artifact(version) => version,
                ArtifactDependency::Source(SourceDependency::Modules { .. }) => {
                    continue;
                }
                ArtifactDependency::Projection(dependency)
                    if kind.is_inference_component_owned()
                        && dependency.projection().artifact == graph_key
                        && dependency.projection().key
                            == ArtifactProjectionKey::InferenceComponents =>
                {
                    continue;
                }
                ArtifactDependency::Source(dependency) => {
                    return Err(ProviderError::internal(format!(
                        "program index has an unrelated source dependency: {dependency:?}"
                    ))
                    .into());
                }
                ArtifactDependency::Projection(dependency) => {
                    return Err(ProviderError::internal(format!(
                        "program index has a projected dependency: {:?}",
                        dependency.projection()
                    ))
                    .into());
                }
            };

            // collect pre-check module index owners
            if let ArtifactKey::ModuleIndex {
                module,
                profile,
                kind: dependency_kind,
            } = version.key
            {
                if !kind.is_module_owned() {
                    return Err(ProviderError::internal(format!(
                        "program {kind:?} index has a module index dependency"
                    ))
                    .into());
                }
                Self::require_index_family(profile_id, kind, profile, dependency_kind)?;
                if versions.insert(module, *version).is_some() {
                    return Err(ProviderError::internal(format!(
                        "program index contains duplicate module dependencies: module={module:?}"
                    ))
                    .into());
                }

                continue;
            }

            // collect checked component index owners
            if let ArtifactKey::InferenceComponentIndex {
                component,
                profile,
                kind: dependency_kind,
            } = version.key
            {
                if !kind.is_inference_component_owned() {
                    return Err(ProviderError::internal(format!(
                        "program {kind:?} index has a component index dependency"
                    ))
                    .into());
                }
                Self::require_index_family(profile_id, kind, profile, dependency_kind)?;
                let index = repository
                    .artifact_table()
                    .inference_component_index(version)
                    .ok_or(ProviderError::Corrupt { version: *version })?;
                if index.component != component || index.kind != kind {
                    return Err(ProviderError::internal(format!(
                        "component index payload differs from its key: {:?}",
                        version.key
                    ))
                    .into());
                }

                // bind every component row to the owning artifact version
                for module in &index.modules {
                    if module.index.kind() != kind {
                        return Err(ProviderError::internal(format!(
                            "component {component} {kind:?} index contains {:?}",
                            module.index.kind()
                        ))
                        .into());
                    }
                    if versions.insert(module.module, *version).is_some() {
                        return Err(ProviderError::internal(format!(
                            "program index contains duplicate module rows: module={:?}",
                            module.module
                        ))
                        .into());
                    }
                }

                continue;
            }

            return Err(ProviderError::internal(format!(
                "program index has an unrelated artifact dependency: {:?}",
                version.key
            ))
            .into());
        }

        Ok(versions)
    }

    /// Require an index owner to match the requested profile and family.
    fn require_index_family(
        profile_id: ProfileId,
        kind: IndexKind,
        profile: ProfileId,
        dependency_kind: IndexKind,
    ) -> ProviderResult<()> {
        if profile != profile_id {
            return Err(ProviderError::internal(format!(
                "program index depends on another profile: expected={profile_id:?}, found={profile:?}"
            ))
            .into());
        }
        if dependency_kind != kind {
            return Err(ProviderError::internal(format!(
                "program {kind:?} index depends on {dependency_kind:?}"
            ))
            .into());
        }

        Ok(())
    }

    /// Require one module version for every program module.
    fn require_program_modules(
        versions: &BTreeMap<ModuleId, ArtifactVersion>,
        module_ids: &[ModuleId],
    ) -> ProviderResult<()> {
        let version_modules = versions.keys().copied().collect::<Vec<_>>();
        if version_modules != module_ids {
            return Err(ProviderError::internal(format!(
                "program index module dependencies differ from the repository: expected={module_ids:?}, found={version_modules:?}"
            ))
            .into());
        }

        Ok(())
    }

    /// Update one program index from its matching predecessor.
    fn update_program_index(
        repository: &Repository,
        context: &dyn ProviderContext,
        profile_id: ProfileId,
        kind: IndexKind,
        module_ids: &[ModuleId],
        versions: &BTreeMap<ModuleId, ArtifactVersion>,
    ) -> ProviderResult<Option<ProgramIndex>> {
        let Some(base) = context.artifact_base() else {
            return Ok(None);
        };
        if base.version.key != context.artifact_key() {
            return Err(ProviderError::internal(format!(
                "program index predecessor has another key: expected={:?}, found={:?}",
                context.artifact_key(),
                base.version.key
            ))
            .into());
        }

        // rebuild when program membership changed
        let base_versions =
            Self::program_module_versions(repository, &base.dependencies, profile_id, kind)?;
        let base_modules = base_versions.keys().copied().collect::<Vec<_>>();
        if base_modules != module_ids {
            return Ok(None);
        }

        // load the predecessor program index
        let base_index = repository
            .artifact_table()
            .program_index(&base.version)
            .ok_or(ProviderError::Corrupt {
                version: base.version,
            })?;
        if base_index.kind() != kind {
            return Err(ProviderError::internal(format!(
                "program {kind:?} index has predecessor {:?}",
                base_index.kind()
            ))
            .into());
        }

        // load only module rows whose owner versions changed
        let mut changed = Vec::new();
        for (ordinal, module_id) in module_ids.iter().enumerate() {
            let version = versions[module_id];
            if base_versions[module_id] == version {
                continue;
            }

            let index = Self::load_program_module(repository, *module_id, version, kind)?;
            changed.push((ordinal as u32, index));
        }
        context.emit_counter("changed_modules", changed.len() as u64);

        // replace changed postings in the predecessor
        let changed = changed
            .iter()
            .map(|(ordinal, index)| (*ordinal, index.as_ref()))
            .collect::<Vec<_>>();
        let index = ProgramIndexer::update(base_index.as_ref().clone(), &changed)?;

        Ok(Some(index))
    }

    /// Load module indexes in stable program order.
    fn load_program_modules(
        repository: &Repository,
        versions: &BTreeMap<ModuleId, ArtifactVersion>,
        kind: IndexKind,
        module_ids: &[ModuleId],
    ) -> ProviderResult<Vec<Arc<ModuleIndex>>> {
        // load every module index in stable program order
        let mut modules = Vec::with_capacity(module_ids.len());
        for module_id in module_ids {
            let version = versions[module_id];
            let index = Self::load_program_module(repository, *module_id, version, kind)?;
            modules.push(index);
        }

        Ok(modules)
    }

    /// Load one exact module index.
    fn load_program_module(
        repository: &Repository,
        module_id: ModuleId,
        version: ArtifactVersion,
        kind: IndexKind,
    ) -> ProviderResult<Arc<ModuleIndex>> {
        let index = match version.key {
            ArtifactKey::ModuleIndex { module, .. } => {
                if module != module_id {
                    return Err(ProviderError::internal(format!(
                        "program module {module_id:?} has owner for {module:?}"
                    ))
                    .into());
                }
                repository
                    .artifact_table()
                    .module_index(&version)
                    .ok_or(ProviderError::Corrupt { version })?
            }
            ArtifactKey::InferenceComponentIndex { .. } => {
                let component = repository
                    .artifact_table()
                    .inference_component_index(&version)
                    .ok_or(ProviderError::Corrupt { version })?;
                component.get(module_id).cloned().ok_or_else(|| {
                    ProviderError::internal(format!(
                        "component index does not contain program module {module_id:?}"
                    ))
                })?
            }
            key => {
                return Err(ProviderError::internal(format!(
                    "program module has unrelated owner: {key:?}"
                ))
                .into());
            }
        };
        if index.kind() != kind {
            return Err(ProviderError::internal(format!(
                "program {kind:?} index loaded module {:?}",
                index.kind()
            ))
            .into());
        }

        Ok(index)
    }
}
