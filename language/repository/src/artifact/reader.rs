use std::sync::Arc;

use destack_artifact::{
    ArtifactDependency, ArtifactKey, ArtifactOutcome, ArtifactProjection, ArtifactProjectionKey,
    ArtifactTable, ArtifactVersion, Asset, Build, Bundle, ComponentGraph, Data, DirBound,
    DirCheckedComponent, DirCheckedModule, DirDeclared, DirExpanded, DirExported, DirImported,
    DirMaterialized, DirParsed, DirResolved, GlobalEnvironment, MirAnalyzed, MirElaborated,
    MirLowered, MirOptimized, MirVerified, ModuleIndex, ModuleLinted, Object, PackageIndex,
    Product, ProgramAnalysis, ProgramIndex, ProgramLinted, Script,
};
use destack_program::Program;
use destack_source::{ComponentId, ModuleId, PackageId, ProductId, ProfileId, TargetId};

use crate::provider::ProviderError;
use crate::repository::{Repository, Revision};

/// Revision-bound read-only view over ready artifacts.
pub struct ArtifactReader<'a> {
    /// The repository that binds artifact versions to the revision.
    repository: &'a Repository,
    /// The pinned revision the reader resolves against.
    revision: Revision,
    /// The artifact table that owns typed payloads.
    table: Arc<ArtifactTable>,
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
        let table = repository.artifact_table().clone();

        Self {
            repository,
            revision,
            table,
        }
    }

    /// Resolve one artifact to its exact ready version.
    fn version(&self, artifact_key: ArtifactKey) -> Result<ArtifactVersion, ProviderError> {
        let version = self
            .repository
            .artifact_binding(self.revision, &artifact_key)
            .map_err(|error| {
                ProviderError::internal(format!("failed to resolve artifact version: {error}"))
            })?;

        // a not yet built dependency reads as blocked
        version
            .filter(|version| matches!(self.table.outcome(version), Some(ArtifactOutcome::Ok)))
            .ok_or_else(|| ProviderError::blocked(artifact_key))
    }

    /// Return the artifact version that supplied one projection dependency.
    fn projection_dependency_version(
        &self,
        version: &ArtifactVersion,
        projection: ArtifactProjection,
    ) -> Result<ArtifactVersion, ProviderError> {
        let dependencies = self
            .table
            .dependencies(version)
            .ok_or(ProviderError::Corrupt { version: *version })?;

        // find the dependency that satisfied the projection requirement
        for dependency in dependencies.iter() {
            let ArtifactDependency::Projection(dependency) = dependency else {
                continue;
            };
            if dependency.projection == projection {
                return Ok(dependency.version);
            }
        }

        Err(ProviderError::internal(format!(
            "artifact {version:?} has no projection dependency {projection:?}"
        )))
    }

    /// Read one collected typed artifact payload.
    fn read<T>(
        &self,
        artifact_key: ArtifactKey,
        get: impl FnOnce(&ArtifactTable, &ArtifactVersion) -> Option<Arc<T>>,
    ) -> Result<Arc<T>, ProviderError> {
        let version = self.version(artifact_key)?;
        let payload = get(&self.table, &version).ok_or(ProviderError::Corrupt { version })?;

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

    /// Read one declared DIR environment artifact.
    pub fn dir_declared(
        &self,
        entry: ModuleId,
        component: ComponentId,
        profile: ProfileId,
    ) -> Result<Arc<DirDeclared>, ProviderError> {
        self.read(
            ArtifactKey::dir_declared(entry, component, profile),
            ArtifactTable::dir_declared,
        )
    }

    /// Read one checked DIR component artifact.
    pub fn dir_checked_component(
        &self,
        entry: ModuleId,
        component: ComponentId,
        profile: ProfileId,
    ) -> Result<Arc<DirCheckedComponent>, ProviderError> {
        self.read(
            ArtifactKey::dir_checked_component(entry, component, profile),
            ArtifactTable::dir_checked_component,
        )
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
            .table
            .dir_checked(&version)
            .ok_or(ProviderError::Corrupt { version })?;

        // resolve the exact component version behind this module projection
        let component_key =
            ArtifactKey::dir_checked_component(checked.entry, checked.component, profile);
        let projection =
            ArtifactProjection::new(component_key, ArtifactProjectionKey::DirChecked(module));
        let component_version = self.projection_dependency_version(&version, projection)?;

        // read the checked module entry from the owning component
        let component =
            self.table
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

        Ok(Arc::new(entry.checked.clone()))
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
    ) -> Result<Arc<ModuleIndex>, ProviderError> {
        self.read(
            ArtifactKey::module_index(module, profile),
            ArtifactTable::module_index,
        )
    }

    /// Read one program index artifact.
    pub fn program_index(&self, profile: ProfileId) -> Result<Arc<ProgramIndex>, ProviderError> {
        self.read(
            ArtifactKey::program_index(profile),
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
