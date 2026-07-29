use std::sync::Arc;

use destack_artifact::{
    ArtifactDependency, ArtifactKey, ArtifactOutcome, ArtifactProjection,
    ArtifactProjectionFingerprint, ArtifactProjectionKey, ArtifactTable, ArtifactVersion, Asset,
    Build, Bundle, ComponentGraph, Data, DirBound, DirCheckedComponent, DirCheckedModule,
    DirDeclaredComponent, DirExpanded, DirExported, DirImported, DirMaterialized, DirParsed,
    DirResolved, ExternalReferenceComponents, GlobalEnvironment, IndexKind,
    InferenceComponentIndex, MirAnalyzed, MirElaborated, MirLowered, MirOptimized, MirVerified,
    ModuleIndex, ModuleLinted, Object, PackageGraph, Product, ProgramAnalysis, ProgramIndex,
    ProgramLinted, Script,
};
use destack_program::Program;
use destack_source::{ComponentId, ModuleId, PackageId, ProductId, ProfileId, TargetId};
use rustc_hash::FxHashSet;

use crate::provider::ProviderError;
use crate::repository::{Repository, Revision};

/// Revision-bound read-only view over ready artifacts.
#[derive(Clone)]
pub struct ArtifactReader<'a> {
    /// The repository that binds artifact versions to the revision.
    repository: &'a Repository,
    /// The pinned revision the reader resolves against.
    revision: Revision,
    /// Frozen provider dependencies when reads are restricted.
    dependencies: Option<&'a [ArtifactDependency]>,
    /// The provider whose reads are restricted.
    provider: Option<ArtifactKey>,
}

/// Projection-checked read-only view over one component graph.
#[derive(Debug)]
pub struct ComponentGraphReader<'a> {
    /// The artifact reader enforcing provider dependencies.
    artifacts: &'a ArtifactReader<'a>,
    /// The component graph artifact key.
    key: ArtifactKey,
    /// The exact component graph payload.
    graph: Arc<ComponentGraph>,
}

impl std::fmt::Debug for ArtifactReader<'_> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ArtifactReader")
            .finish_non_exhaustive()
    }
}

impl<'a> ArtifactReader<'a> {
    /// Create a read-only reader for one repository revision.
    pub fn new(repository: &'a Repository, revision: Revision) -> Self {
        Self {
            repository,
            revision,
            dependencies: None,
            provider: None,
        }
    }

    /// Restrict provider reads to one frozen dependency set.
    pub fn restrict(
        mut self,
        provider: ArtifactKey,
        dependencies: Option<&'a [ArtifactDependency]>,
    ) -> Self {
        self.dependencies = dependencies;
        self.provider = Some(provider);

        self
    }

    /// Resolve one artifact to its exact ready version.
    pub fn version(&self, artifact_key: ArtifactKey) -> Result<ArtifactVersion, ProviderError> {
        if let Some(dependencies) = self.dependencies {
            let version = dependencies.iter().find_map(|dependency| match dependency {
                ArtifactDependency::Artifact(version) if version.key == artifact_key => {
                    Some(*version)
                }
                ArtifactDependency::Artifact(_)
                | ArtifactDependency::Projection(_)
                | ArtifactDependency::Source(_) => None,
            });

            return self.ready_version(artifact_key, version);
        }

        let version = self
            .repository
            .artifact_version(self.revision, &artifact_key)
            .map_err(|error| {
                ProviderError::internal(format!("failed to resolve artifact version: {error}"))
            })?;

        self.ready_version(artifact_key, version)
    }

    /// Resolve one declared artifact projection to its exact owner version.
    pub fn projection_version(
        &self,
        projection: ArtifactProjection,
    ) -> Result<ArtifactVersion, ProviderError> {
        if let Some(dependencies) = self.dependencies {
            let version = dependencies.iter().find_map(|dependency| match dependency {
                ArtifactDependency::Artifact(version) if version.key == projection.artifact => {
                    Some(*version)
                }
                ArtifactDependency::Projection(dependency)
                    if dependency.projection() == projection =>
                {
                    Some(dependency.version())
                }
                ArtifactDependency::Artifact(_)
                | ArtifactDependency::Projection(_)
                | ArtifactDependency::Source(_) => None,
            });

            return self.ready_version(projection.artifact, version);
        }

        self.version(projection.artifact)
    }

    /// Read one declared artifact projection fingerprint.
    pub fn projection_fingerprint(
        &self,
        projection: ArtifactProjection,
    ) -> Result<ArtifactProjectionFingerprint, ProviderError> {
        let version = self.projection_version(projection)?;
        let fingerprint = self
            .repository
            .artifact_table()
            .projection_fingerprint(&version, &projection)
            .ok_or(ProviderError::Corrupt { version })?;

        Ok(fingerprint)
    }

    /// Resolve one projected artifact owner without authorizing a projected value.
    fn projection_owner_version(
        &self,
        artifact_key: ArtifactKey,
    ) -> Result<ArtifactVersion, ProviderError> {
        if let Some(dependencies) = self.dependencies {
            let version = dependencies.iter().find_map(|dependency| match dependency {
                ArtifactDependency::Artifact(version) if version.key == artifact_key => {
                    Some(*version)
                }
                ArtifactDependency::Projection(dependency)
                    if dependency.projection().artifact == artifact_key =>
                {
                    Some(dependency.version())
                }
                ArtifactDependency::Artifact(_)
                | ArtifactDependency::Projection(_)
                | ArtifactDependency::Source(_) => None,
            });

            return self.ready_version(artifact_key, version);
        }

        self.version(artifact_key)
    }

    /// Require one resolved artifact version to carry a ready result.
    fn ready_version(
        &self,
        artifact_key: ArtifactKey,
        version: Option<ArtifactVersion>,
    ) -> Result<ArtifactVersion, ProviderError> {
        let Some(version) = version else {
            if self.dependencies.is_none() {
                return Err(ProviderError::blocked(artifact_key));
            }

            let provider = self.provider.ok_or_else(|| {
                ProviderError::internal("restricted artifact reader has no provider")
            })?;

            return Err(ProviderError::internal(format!(
                "artifact provider {provider:?} read undeclared artifact {artifact_key:?}"
            )));
        };
        match self.repository.artifact_table().outcome(&version) {
            Some(ArtifactOutcome::Ok) => Ok(version),
            Some(ArtifactOutcome::Failed(_)) => {
                Err(ProviderError::RequirementFailed { key: artifact_key })
            }
            None => Err(ProviderError::Corrupt { version }),
        }
    }

    /// Read one collected typed artifact payload.
    fn read<T>(
        &self,
        artifact_key: ArtifactKey,
        get: impl FnOnce(&ArtifactTable, &ArtifactVersion) -> Option<Arc<T>>,
    ) -> Result<Arc<T>, ProviderError> {
        let version = self.version(artifact_key)?;
        let payload = get(self.repository.artifact_table(), &version)
            .ok_or(ProviderError::Corrupt { version })?;

        Ok(payload)
    }

    /// Read one parsed DIR artifact.
    pub fn dir_parsed(&self, module: ModuleId) -> Result<Arc<DirParsed>, ProviderError> {
        self.read(ArtifactKey::dir_parsed(module), ArtifactTable::dir_parsed)
    }

    /// Read one data artifact.
    pub fn data(&self, module: ModuleId) -> Result<Arc<Data>, ProviderError> {
        self.read(ArtifactKey::data(module), ArtifactTable::data)
    }

    /// Read one global environment artifact.
    pub fn global_environment(
        &self,
        profile: ProfileId,
    ) -> Result<Arc<GlobalEnvironment>, ProviderError> {
        self.read(
            ArtifactKey::global_environment(profile),
            ArtifactTable::global_environment,
        )
    }

    /// Read one active package graph artifact.
    pub fn package_graph(&self, profile: ProfileId) -> Result<Arc<PackageGraph>, ProviderError> {
        self.read(
            ArtifactKey::package_graph(profile),
            ArtifactTable::package_graph,
        )
    }

    /// Read one component graph artifact.
    pub fn component_graph(
        &self,
        profile: ProfileId,
    ) -> Result<Arc<ComponentGraph>, ProviderError> {
        self.read(
            ArtifactKey::component_graph(profile),
            ArtifactTable::component_graph,
        )
    }

    /// Read one component graph through exact projected values.
    pub fn component_graph_reader(
        &'a self,
        profile: ProfileId,
    ) -> Result<ComponentGraphReader<'a>, ProviderError> {
        let key = ArtifactKey::component_graph(profile);
        let version = self.projection_owner_version(key)?;
        let graph = self
            .repository
            .artifact_table()
            .component_graph(&version)
            .ok_or(ProviderError::Corrupt { version })?;

        Ok(ComponentGraphReader {
            artifacts: self,
            key,
            graph,
        })
    }

    /// Read one bound DIR artifact.
    pub fn dir_bound(
        &self,
        module: ModuleId,
        profile: ProfileId,
    ) -> Result<Arc<DirBound>, ProviderError> {
        self.read(
            ArtifactKey::dir_bound(module, profile),
            ArtifactTable::dir_bound,
        )
    }

    /// Read one imported DIR artifact.
    pub fn dir_imported(
        &self,
        module: ModuleId,
        profile: ProfileId,
    ) -> Result<Arc<DirImported>, ProviderError> {
        self.read(
            ArtifactKey::dir_imported(module, profile),
            ArtifactTable::dir_imported,
        )
    }

    /// Read one expanded DIR artifact.
    pub fn dir_expanded(
        &self,
        module: ModuleId,
        profile: ProfileId,
    ) -> Result<Arc<DirExpanded>, ProviderError> {
        self.read(
            ArtifactKey::dir_expanded(module, profile),
            ArtifactTable::dir_expanded,
        )
    }

    /// Read one exported DIR artifact.
    pub fn dir_exported(
        &self,
        module: ModuleId,
        profile: ProfileId,
    ) -> Result<Arc<DirExported>, ProviderError> {
        self.read(
            ArtifactKey::dir_exported(module, profile),
            ArtifactTable::dir_exported,
        )
    }

    /// Read one exported DIR artifact through an exact projection.
    pub fn dir_exported_projection(
        &self,
        module: ModuleId,
        profile: ProfileId,
        projection: ArtifactProjectionKey,
    ) -> Result<Arc<DirExported>, ProviderError> {
        let key = ArtifactKey::dir_exported(module, profile);
        let projection = ArtifactProjection::new(key, projection);
        let version = self.projection_version(projection)?;
        let exported = self
            .repository
            .artifact_table()
            .dir_exported(&version)
            .ok_or(ProviderError::Corrupt { version })?;

        Ok(exported)
    }

    /// Read one resolved DIR artifact.
    pub fn dir_resolved(
        &self,
        module: ModuleId,
        profile: ProfileId,
    ) -> Result<Arc<DirResolved>, ProviderError> {
        self.read(
            ArtifactKey::dir_resolved(module, profile),
            ArtifactTable::dir_resolved,
        )
    }

    /// Read one resolved DIR artifact through an exact projection.
    pub fn dir_resolved_projection(
        &self,
        module: ModuleId,
        profile: ProfileId,
        projection: ArtifactProjectionKey,
    ) -> Result<Arc<DirResolved>, ProviderError> {
        let key = ArtifactKey::dir_resolved(module, profile);
        let projection = ArtifactProjection::new(key, projection);
        let version = self.projection_version(projection)?;
        let resolved = self
            .repository
            .artifact_table()
            .dir_resolved(&version)
            .ok_or(ProviderError::Corrupt { version })?;

        Ok(resolved)
    }

    /// Read one declared DIR component artifact.
    pub fn dir_declared_component(
        &self,
        component: ComponentId,
        profile: ProfileId,
    ) -> Result<Arc<DirDeclaredComponent>, ProviderError> {
        let key = ArtifactKey::dir_declared_component(component, profile);
        let version = self.projection_owner_version(key)?;
        let component = self
            .repository
            .artifact_table()
            .dir_declared_component(&version)
            .ok_or(ProviderError::Corrupt { version })?;

        // require every module row before exposing the complete component
        for module in &component.modules {
            let projection = ArtifactProjection::new(
                key,
                ArtifactProjectionKey::DirDeclaredModule(module.module),
            );
            let _version = self.projection_version(projection)?;
        }

        Ok(component)
    }

    /// Read one checked DIR component artifact.
    pub fn dir_checked_component(
        &self,
        component: ComponentId,
        profile: ProfileId,
    ) -> Result<Arc<DirCheckedComponent>, ProviderError> {
        self.read(
            ArtifactKey::dir_checked_component(component, profile),
            ArtifactTable::dir_checked_component,
        )
    }

    /// Read one projected checked DIR module.
    pub fn dir_checked_module(
        &self,
        component: ComponentId,
        module: ModuleId,
        profile: ProfileId,
    ) -> Result<Arc<DirCheckedModule>, ProviderError> {
        let key = ArtifactKey::dir_checked_component(component, profile);
        let projection =
            ArtifactProjection::new(key, ArtifactProjectionKey::DirCheckedModule(module));
        let version = self.projection_version(projection)?;
        let component = self
            .repository
            .artifact_table()
            .dir_checked_component(&version)
            .ok_or(ProviderError::Corrupt { version })?;
        let module = component.module(module).ok_or_else(|| {
            ProviderError::internal(format!(
                "checked component {key:?} does not contain module {module:?}"
            ))
        })?;

        Ok(Arc::new(module.clone()))
    }

    /// Read one checked DIR artifact.
    pub fn dir_checked(
        &self,
        module: ModuleId,
        profile: ProfileId,
    ) -> Result<Arc<DirCheckedModule>, ProviderError> {
        // read the facade that names the owning component
        let version = self.version(ArtifactKey::dir_checked(module, profile))?;
        let checked = self
            .repository
            .artifact_table()
            .dir_checked(&version)
            .ok_or(ProviderError::Corrupt { version })?;

        // require the current component to carry the selected module output
        let component_key = ArtifactKey::dir_checked_component(checked.component, profile);
        let projection = ArtifactProjection::new(
            component_key,
            ArtifactProjectionKey::DirCheckedModule(module),
        );
        let binding = self
            .repository
            .artifact_binding(self.revision, &version.key)
            .map_err(|error| ProviderError::internal(error.to_string()))?
            .ok_or(ProviderError::Corrupt { version })?;
        let dependency = binding
            .dependencies
            .iter()
            .find_map(|dependency| match dependency {
                ArtifactDependency::Projection(dependency)
                    if dependency.projection() == projection =>
                {
                    Some(dependency)
                }
                _ => None,
            })
            .ok_or_else(|| {
                ProviderError::internal(format!(
                    "checked facade has no module projection dependency: {projection:?}"
                ))
            })?;
        let component_version = dependency.version();
        let fingerprint = self
            .repository
            .artifact_table()
            .projection_fingerprint(&component_version, &projection)
            .ok_or(ProviderError::Corrupt {
                version: component_version,
            })?;
        if fingerprint != checked.fingerprint || fingerprint != dependency.fingerprint() {
            return Err(ProviderError::internal(format!(
                "checked facade projection does not match component: facade={:?}, dependency={:?}, component={fingerprint:?}",
                checked.fingerprint,
                dependency.fingerprint(),
            )));
        }

        // read the checked module entry from the owning component
        let component = self
            .repository
            .artifact_table()
            .dir_checked_component(&component_version)
            .ok_or(ProviderError::Corrupt {
                version: component_version,
            })?;
        let entry = component.module(module).ok_or_else(|| {
            ProviderError::internal(format!(
                "checked component {} does not contain module {module:?}",
                checked.component
            ))
        })?;

        Ok(Arc::new(entry.clone()))
    }

    /// Read one materialized DIR artifact.
    pub fn dir_materialized(
        &self,
        module: ModuleId,
        profile: ProfileId,
    ) -> Result<Arc<DirMaterialized>, ProviderError> {
        self.read(
            ArtifactKey::dir_materialized(module, profile),
            ArtifactTable::dir_materialized,
        )
    }

    /// Read one lowered MIR artifact.
    pub fn mir_lowered(
        &self,
        module: ModuleId,
        profile: ProfileId,
        target: TargetId,
    ) -> Result<Arc<MirLowered>, ProviderError> {
        self.read(
            ArtifactKey::mir_lowered(module, profile, target),
            ArtifactTable::mir_lowered,
        )
    }

    /// Read one verified MIR artifact.
    pub fn mir_verified(
        &self,
        module: ModuleId,
        profile: ProfileId,
        target: TargetId,
    ) -> Result<Arc<MirVerified>, ProviderError> {
        self.read(
            ArtifactKey::mir_verified(module, profile, target),
            ArtifactTable::mir_verified,
        )
    }

    /// Read one elaborated MIR artifact.
    pub fn mir_elaborated(
        &self,
        module: ModuleId,
        profile: ProfileId,
        target: TargetId,
    ) -> Result<Arc<MirElaborated>, ProviderError> {
        self.read(
            ArtifactKey::mir_elaborated(module, profile, target),
            ArtifactTable::mir_elaborated,
        )
    }

    /// Read one analyzed MIR link summary.
    pub fn mir_analyzed(
        &self,
        module: ModuleId,
        profile: ProfileId,
        target: TargetId,
    ) -> Result<Arc<MirAnalyzed>, ProviderError> {
        self.read(
            ArtifactKey::mir_analyzed(module, profile, target),
            ArtifactTable::mir_analyzed,
        )
    }

    /// Read one whole-program analysis artifact.
    pub fn program_analysis(
        &self,
        profile: ProfileId,
        target: TargetId,
    ) -> Result<Arc<ProgramAnalysis>, ProviderError> {
        self.read(
            ArtifactKey::program_analysis(profile, target),
            ArtifactTable::program_analysis,
        )
    }

    /// Read one optimized MIR artifact.
    pub fn mir_optimized(
        &self,
        module: ModuleId,
        profile: ProfileId,
        target: TargetId,
    ) -> Result<Arc<MirOptimized>, ProviderError> {
        self.read(
            ArtifactKey::mir_optimized(module, profile, target),
            ArtifactTable::mir_optimized,
        )
    }

    /// Read one module index artifact.
    pub fn module_index(
        &self,
        module: ModuleId,
        profile: ProfileId,
        kind: IndexKind,
    ) -> Result<Arc<ModuleIndex>, ProviderError> {
        self.read(
            ArtifactKey::module_index(module, profile, kind),
            ArtifactTable::module_index,
        )
    }

    /// Read one checked component query index artifact.
    pub fn inference_component_index(
        &self,
        component: ComponentId,
        profile: ProfileId,
        kind: IndexKind,
    ) -> Result<Arc<InferenceComponentIndex>, ProviderError> {
        self.read(
            ArtifactKey::inference_component_index(component, profile, kind),
            ArtifactTable::inference_component_index,
        )
    }

    /// Read one program index artifact.
    pub fn program_index(
        &self,
        profile: ProfileId,
        kind: IndexKind,
    ) -> Result<Arc<ProgramIndex>, ProviderError> {
        self.read(
            ArtifactKey::program_index(profile, kind),
            ArtifactTable::program_index,
        )
    }

    /// Read one structured script artifact.
    pub fn script(&self, module: ModuleId, target: TargetId) -> Result<Arc<Script>, ProviderError> {
        self.read(ArtifactKey::script(module, target), ArtifactTable::script)
    }

    /// Read one compiled-code object artifact.
    pub fn object(&self, module: ModuleId, target: TargetId) -> Result<Arc<Object>, ProviderError> {
        self.read(ArtifactKey::object(module, target), ArtifactTable::object)
    }

    /// Read one asset artifact.
    pub fn asset(&self, module: ModuleId, target: TargetId) -> Result<Arc<Asset>, ProviderError> {
        self.read(ArtifactKey::asset(module, target), ArtifactTable::asset)
    }

    /// Read one build payload.
    pub fn build(&self, target: TargetId) -> Result<Arc<Build>, ProviderError> {
        self.read(ArtifactKey::build(target), ArtifactTable::build)
    }

    /// Read one bundle artifact.
    pub fn bundle(
        &self,
        package: PackageId,
        target: TargetId,
    ) -> Result<Arc<Bundle>, ProviderError> {
        self.read(ArtifactKey::bundle(package, target), ArtifactTable::bundle)
    }

    /// Read one executable program artifact.
    pub fn program(
        &self,
        package: PackageId,
        target: TargetId,
    ) -> Result<Arc<Program>, ProviderError> {
        self.read(
            ArtifactKey::program(package, target),
            ArtifactTable::program,
        )
    }

    /// Read one product artifact.
    pub fn product(
        &self,
        package: PackageId,
        product: ProductId,
    ) -> Result<Arc<Product>, ProviderError> {
        self.read(
            ArtifactKey::product(package, product),
            ArtifactTable::product,
        )
    }

    /// Read one module lint marker artifact.
    pub fn module_linted(
        &self,
        module: ModuleId,
        profile: ProfileId,
        target: TargetId,
    ) -> Result<Arc<ModuleLinted>, ProviderError> {
        self.read(
            ArtifactKey::module_linted(module, profile, target),
            ArtifactTable::module_linted,
        )
    }

    /// Read one program lint marker artifact.
    pub fn program_linted(
        &self,
        profile: ProfileId,
        target: TargetId,
    ) -> Result<Arc<ProgramLinted>, ProviderError> {
        self.read(
            ArtifactKey::program_linted(profile, target),
            ArtifactTable::program_linted,
        )
    }
}

impl ComponentGraphReader<'_> {
    /// Return the reference component containing one module.
    pub fn reference_component(
        &self,
        module: ModuleId,
    ) -> Result<Option<ComponentId>, ProviderError> {
        self.require(ArtifactProjectionKey::ReferenceComponent(module))?;

        Ok(self.graph.reference_component(module))
    }

    /// Return the member modules of one reference component.
    pub fn reference_members(&self, component: ComponentId) -> Result<&[ModuleId], ProviderError> {
        self.require(ArtifactProjectionKey::ReferenceMembers(component))?;

        self.graph.reference_members(component).ok_or_else(|| {
            ProviderError::internal(format!(
                "reference component {component:?} is absent from the component graph"
            ))
        })
    }

    /// Return the direct external reference components of one component.
    pub fn reference_dependencies(
        &self,
        component: ComponentId,
    ) -> Result<&[ComponentId], ProviderError> {
        self.require(ArtifactProjectionKey::ReferenceDependencies(component))?;

        self.graph.reference_dependencies(component).ok_or_else(|| {
            ProviderError::internal(format!(
                "reference component {component:?} is absent from the component graph"
            ))
        })
    }

    /// Return the inference component containing one module.
    pub fn inference_component(
        &self,
        module: ModuleId,
    ) -> Result<Option<ComponentId>, ProviderError> {
        self.require(ArtifactProjectionKey::InferenceComponent(module))?;

        Ok(self.graph.inference_component(module))
    }

    /// Return the member modules of one inference component.
    pub fn inference_members(&self, component: ComponentId) -> Result<&[ModuleId], ProviderError> {
        self.require(ArtifactProjectionKey::InferenceMembers(component))?;

        self.graph.inference_members(component).ok_or_else(|| {
            ProviderError::internal(format!(
                "inference component {component:?} is absent from the component graph"
            ))
        })
    }

    /// Return upstream inference components of one inference component.
    pub fn inference_dependencies(
        &self,
        component: ComponentId,
    ) -> Result<&[ComponentId], ProviderError> {
        self.require(ArtifactProjectionKey::InferenceDependencies(component))?;

        self.graph.inference_dependencies(component).ok_or_else(|| {
            ProviderError::internal(format!(
                "inference component {component:?} is absent from the component graph"
            ))
        })
    }

    /// Return the entry module of one reference component.
    pub fn reference_entry(
        &self,
        component: ComponentId,
    ) -> Result<Option<ModuleId>, ProviderError> {
        Ok(self.reference_members(component)?.first().copied())
    }

    /// Return the entry module of one inference component.
    pub fn inference_entry(
        &self,
        component: ComponentId,
    ) -> Result<Option<ModuleId>, ProviderError> {
        Ok(self.inference_members(component)?.first().copied())
    }

    /// Return components reachable from module roots in stable breadth-first order.
    pub fn reachable_components(
        &self,
        roots: &[ModuleId],
    ) -> Result<Vec<ComponentId>, ProviderError> {
        let mut components = Vec::new();
        let mut seen = FxHashSet::default();

        // seed the walk with root components in caller order
        for root in roots {
            let component = self.reference_component(*root)?.ok_or_else(|| {
                ProviderError::internal(format!(
                    "module {root:?} is absent from the component graph"
                ))
            })?;
            if seen.insert(component) {
                components.push(component);
            }
        }

        // extend the ordered worklist through the condensation graph
        let mut index = 0;
        while index < components.len() {
            let component = components[index];
            index += 1;

            for dependency in self.reference_dependencies(component)? {
                if seen.insert(*dependency) {
                    components.push(*dependency);
                }
            }
        }

        Ok(components)
    }

    /// Return every transitive reference dependency in stable breadth-first order.
    pub fn transitive_reference_dependencies(
        &self,
        component: ComponentId,
    ) -> Result<Vec<ComponentId>, ProviderError> {
        let mut dependencies = Vec::new();
        let mut seen = FxHashSet::default();

        // seed the walk with direct dependencies
        for dependency in self.reference_dependencies(component)? {
            if seen.insert(*dependency) {
                dependencies.push(*dependency);
            }
        }

        // extend the ordered worklist through the condensation graph
        let mut index = 0;
        while index < dependencies.len() {
            let component = dependencies[index];
            index += 1;

            for dependency in self.reference_dependencies(component)? {
                if seen.insert(*dependency) {
                    dependencies.push(*dependency);
                }
            }
        }

        Ok(dependencies)
    }

    /// Return external reference components loaded with one component.
    pub fn external_reference_components(
        &self,
        component: ComponentId,
        implicit_modules: impl IntoIterator<Item = ModuleId>,
    ) -> Result<ExternalReferenceComponents, ProviderError> {
        let references = self.transitive_reference_dependencies(component)?;
        self.require(ArtifactProjectionKey::InherentExtensions)?;

        // prevent extension closures from recursively loading extensions
        if self.graph.is_extension_component(component) {
            return Ok(ExternalReferenceComponents {
                references,
                extensions: Vec::new(),
            });
        }

        // index ordinary references and every component loaded so far
        let reference_set = references.iter().copied().collect::<FxHashSet<_>>();
        let mut loaded = reference_set.clone();
        let implicit = implicit_modules.into_iter().collect::<FxHashSet<_>>();
        let mut extensions = Vec::new();

        // load extensions whose targets are referenced or implicit
        for extension in self.graph.cross_component_extensions() {
            let target = self
                .graph
                .reference_component(extension.target.module_id)
                .ok_or_else(|| {
                    ProviderError::internal(format!(
                        "inherent extension target is absent from the component graph: {:?}",
                        extension.target
                    ))
                })?;
            let is_target_loaded =
                implicit.contains(&extension.target.module_id) || reference_set.contains(&target);
            let source = self
                .graph
                .reference_component(extension.symbol.module_id)
                .ok_or_else(|| {
                    ProviderError::internal(format!(
                        "inherent extension source is absent from the component graph: {:?}",
                        extension.symbol
                    ))
                })?;
            if !is_target_loaded || source == component || !loaded.insert(source) {
                continue;
            }

            // load the extension source and everything it references
            extensions.push(source);
            for dependency in self.transitive_reference_dependencies(source)? {
                if loaded.insert(dependency) {
                    extensions.push(dependency);
                }
            }
        }

        Ok(ExternalReferenceComponents {
            references,
            extensions,
        })
    }

    /// Require one exact component graph projection.
    fn require(&self, projection: ArtifactProjectionKey) -> Result<(), ProviderError> {
        let projection = ArtifactProjection::new(self.key, projection);
        let _version = self.artifacts.projection_version(projection)?;

        Ok(())
    }
}
