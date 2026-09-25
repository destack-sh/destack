use std::collections::BTreeMap;
use std::sync::Arc;

use tspp_artifact::{
    ArtifactDependency, ArtifactDependencySet, ArtifactKey, ArtifactPayload, ArtifactVersion,
    DirAnalyzed, DirBound, DirChecked, DirDeclared, DirElaborated, DirExpanded, DirExported,
    DirImported, DirMaterialized, DirParsed, DirResolved, DirView, IndexKind, ModuleIndex,
    ProgramIndex, SourceDependencyKey,
};
use tspp_core::FxIndexMap;
use tspp_repository::{ArtifactReader, ProviderContext, ProviderError, ProviderResult, Repository};
use tspp_source::{ModuleId, ProfileId};

use crate::module::{
    CallIndexer, CodeIndexer, DecoratorIndexer, ExportIndexer, HeritageIndexer, MemberIndexer,
    ModuleIndexContext, ReferenceIndexer, SymbolIndexer,
};
use crate::program::ProgramIndexer;

/// Provider that builds index artifacts for one repository.
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
            } => self.collect_module_index(context, module, profile, kind),
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
        context: &dyn ProviderContext,
        module_id: ModuleId,
        profile_id: ProfileId,
        kind: IndexKind,
    ) -> ProviderResult<ArtifactDependencySet> {
        let mut dependencies = ArtifactDependencySet::default();

        // index declarations directly from expanded DIR
        if kind == IndexKind::Symbols {
            dependencies.require(ArtifactKey::dir_parsed(module_id));
            dependencies.require(ArtifactKey::dir_bound(module_id, profile_id));
            dependencies.require(ArtifactKey::dir_imported(module_id, profile_id));
            dependencies.require(ArtifactKey::dir_expanded(module_id, profile_id));
        }
        // index exports from exported declarations and resolved dependencies,
        //  reading the import closure's export tables for star exports
        else if kind == IndexKind::Exports {
            dependencies.require(ArtifactKey::dir_exported(module_id, profile_id));
            dependencies.require(ArtifactKey::dir_resolved(module_id, profile_id));
            self.require_star_exports(context, module_id, profile_id, &mut dependencies)?;
        }
        // index checked families over the module's declared, elaborated, and checked DIR
        else {
            dependencies.require(ArtifactKey::dir_parsed(module_id));
            dependencies.require(ArtifactKey::dir_bound(module_id, profile_id));
            dependencies.require(ArtifactKey::dir_imported(module_id, profile_id));
            dependencies.require(ArtifactKey::dir_expanded(module_id, profile_id));
            dependencies.require(ArtifactKey::dir_resolved(module_id, profile_id));
            dependencies.require(ArtifactKey::dir_declared(module_id, profile_id));
            dependencies.require(ArtifactKey::dir_elaborated(module_id, profile_id));
            dependencies.require(ArtifactKey::dir_checked(module_id, profile_id));
            dependencies.require(ArtifactKey::dir_materialized(module_id, profile_id));
            dependencies.require(ArtifactKey::dir_analyzed(module_id, profile_id));
        }

        Ok(dependencies)
    }

    /// Require the export tables behind one module's star exports.
    fn require_star_exports(
        &self,
        context: &dyn ProviderContext,
        module_id: ModuleId,
        profile_id: ProfileId,
        dependencies: &mut ArtifactDependencySet,
    ) -> ProviderResult<()> {
        // follow star edges, requiring each reached module's export tables
        let artifacts = ArtifactReader::new(self.repository(), context.revision());
        let mut queue = vec![module_id];
        let mut visited = Vec::new();
        while let Some(module) = queue.pop() {
            if visited.contains(&module) {
                continue;
            }
            visited.push(module);
            if module != module_id {
                dependencies.require(ArtifactKey::dir_exported(module, profile_id));
                dependencies.require(ArtifactKey::dir_resolved(module, profile_id));
            }

            // a blocked table defers the remaining walk to the next attempt
            let exported = match artifacts.read::<DirExported>((module, profile_id)) {
                Ok(exported) => exported,
                Err(ProviderError::Blocked { .. }) => {
                    dependencies.mark_partial();

                    return Ok(());
                }
                Err(error) => return Err(error.into()),
            };
            for star in exported.exports.star_exports() {
                if let Some(target) = star.target {
                    queue.push(target);
                }
            }
        }

        Ok(())
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

        // require every module's index of this family
        let mut dependencies = ArtifactDependencySet::default();
        for module in &module_ids {
            dependencies.require(ArtifactKey::module_index(*module, profile_id, kind));
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
        let revision = context.revision();
        let artifacts = ArtifactReader::new(self.repository(), revision).restrict(
            context.artifact_dependencies().ok_or_else(|| {
                ProviderError::internal("module index provider has no frozen dependencies")
            })?,
        );

        // build the exact selected index family
        let payload = match kind {
            IndexKind::Symbols => {
                let view = DirView::expanded(
                    artifacts.read::<DirParsed>(module_id)?,
                    artifacts.read::<DirBound>((module_id, profile_id))?,
                    artifacts.read::<DirImported>((module_id, profile_id))?,
                    artifacts.read::<DirExpanded>((module_id, profile_id))?,
                );
                let strings = self.repository().string_pool();

                ModuleIndex::Symbols(SymbolIndexer::build(&view, strings)?)
            }
            IndexKind::Exports => {
                let exported = artifacts.read::<DirExported>((module_id, profile_id))?;
                let resolved = artifacts.read::<DirResolved>((module_id, profile_id))?;
                let strings = self.repository().string_pool();

                // load the export views behind any star exports
                let mut closure = FxIndexMap::default();
                let mut queue: Vec<ModuleId> = exported
                    .exports
                    .star_exports()
                    .filter_map(|star| star.target)
                    .collect();
                while let Some(module) = queue.pop() {
                    if module == module_id || closure.contains_key(&module) {
                        continue;
                    }
                    let dep_exported = artifacts.read::<DirExported>((module, profile_id))?;
                    let dep_resolved = artifacts.read::<DirResolved>((module, profile_id))?;
                    for star in dep_exported.exports.star_exports() {
                        if let Some(target) = star.target {
                            queue.push(target);
                        }
                    }
                    closure.insert(module, (dep_exported, dep_resolved));
                }

                ModuleIndex::Exports(ExportIndexer::build(
                    module_id, &exported, &resolved, &closure, strings,
                )?)
            }
            kind => {
                let module = self.module_index_context(&artifacts, module_id, profile_id)?;

                match kind {
                    IndexKind::Code => ModuleIndex::Code(CodeIndexer::build(&module)?),
                    IndexKind::Members => ModuleIndex::Members(MemberIndexer::build(&module)),
                    IndexKind::References => {
                        ModuleIndex::References(ReferenceIndexer::build(&module)?)
                    }
                    IndexKind::Calls => ModuleIndex::Calls(CallIndexer::build(&module)?),
                    IndexKind::Heritage => ModuleIndex::Heritage(HeritageIndexer::build(&module)?),
                    IndexKind::Decorators => {
                        ModuleIndex::Decorators(DecoratorIndexer::build(&module))
                    }
                    IndexKind::Symbols | IndexKind::Exports => {
                        return Err(ProviderError::internal(format!(
                            "checked module index kind: {kind:?}"
                        ))
                        .into());
                    }
                }
            }
        };

        Ok(ArtifactPayload::ModuleIndex(Arc::new(payload)))
    }

    /// Read checked DIR for one module index entry.
    fn module_index_context<'a>(
        &'a self,
        artifacts: &ArtifactReader<'_>,
        module_id: ModuleId,
        profile_id: ProfileId,
    ) -> ProviderResult<ModuleIndexContext<'a>> {
        let view = DirView::analyzed(
            artifacts.read::<DirParsed>(module_id)?,
            artifacts.read::<DirBound>((module_id, profile_id))?,
            artifacts.read::<DirImported>((module_id, profile_id))?,
            artifacts.read::<DirExpanded>((module_id, profile_id))?,
            artifacts.read::<DirResolved>((module_id, profile_id))?,
            artifacts.read::<DirDeclared>((module_id, profile_id))?,
            artifacts.read::<DirElaborated>((module_id, profile_id))?,
            artifacts.read::<DirChecked>((module_id, profile_id))?,
            artifacts.read::<DirMaterialized>((module_id, profile_id))?,
            artifacts.read::<DirAnalyzed>((module_id, profile_id))?,
        );
        let strings = self.repository().string_pool();

        Ok(ModuleIndexContext::new(strings, module_id, view))
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
            context.record_span("modules", started);
        }
        context.record_counter("modules", module_ids.len() as u64);

        // read the exact current module versions
        let started = repository.host().clock().now();
        let dependencies = context.artifact_dependencies().ok_or_else(|| {
            ProviderError::internal("program index provider has no frozen dependencies")
        })?;
        let versions = Self::program_module_versions(dependencies, profile_id, kind)?;
        Self::require_program_modules(&versions, &module_ids)?;
        if let Some(started) = started {
            context.record_span("index_owners", started);
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
            context.record_span("index", started);
        }

        Ok(ArtifactPayload::ProgramIndex(Arc::new(payload)))
    }

    /// Collect each program module's index owner version.
    fn program_module_versions(
        dependencies: &[ArtifactDependency],
        profile_id: ProfileId,
        kind: IndexKind,
    ) -> ProviderResult<BTreeMap<ModuleId, ArtifactVersion>> {
        let mut versions = BTreeMap::<ModuleId, ArtifactVersion>::new();

        // collect one owner version for every indexed module
        for dependency in dependencies {
            let version = match dependency {
                ArtifactDependency::Artifact(version) => version,
                ArtifactDependency::Source(source)
                    if source.key() == SourceDependencyKey::Modules =>
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

            // collect module index owners
            if let ArtifactKey::ModuleIndex {
                module,
                profile,
                kind: dependency_kind,
            } = version.key
            {
                Self::require_index_family(profile_id, kind, profile, dependency_kind)?;
                if versions.insert(module, *version).is_some() {
                    return Err(ProviderError::internal(format!(
                        "program index contains duplicate module dependencies: module={module:?}"
                    ))
                    .into());
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
        let base_version = base.version();
        if base_version.key != context.artifact_key() {
            return Err(ProviderError::internal(format!(
                "program index predecessor has another key: expected={:?}, found={:?}",
                context.artifact_key(),
                base_version.key
            ))
            .into());
        }

        // rebuild when program membership changed
        let base_versions = Self::program_module_versions(base.dependencies(), profile_id, kind)?;
        let base_modules = base_versions.keys().copied().collect::<Vec<_>>();
        if base_modules != module_ids {
            return Ok(None);
        }

        // load the predecessor program index
        let base_index = base
            .artifact::<ProgramIndex>()
            .ok_or(ProviderError::Corrupt {
                version: base_version,
            })?;
        if base_index.kind() != kind {
            return Err(ProviderError::internal(format!(
                "program {kind:?} index has predecessor {:?}",
                base_index.kind()
            ))
            .into());
        }

        // load only module entries whose owner versions changed
        let mut changed = Vec::new();
        for (ordinal, module_id) in module_ids.iter().enumerate() {
            let version = versions[module_id];
            if base_versions[module_id] == version {
                continue;
            }

            let index = Self::load_program_module(repository, *module_id, version, kind)?;
            changed.push((ordinal as u32, index));
        }
        context.record_counter("changed_modules", changed.len() as u64);

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
                    .artifact::<ModuleIndex>(&version)
                    .map_err(|error| ProviderError::internal(error.to_string()))?
                    .ok_or(ProviderError::Corrupt { version })?
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
