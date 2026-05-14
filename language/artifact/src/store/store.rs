use std::sync::Arc;

use dashmap::DashMap;
use destack_source::DiagnosticCollection;

use super::entry::{ArtifactEntry, ArtifactOutcome};
use super::pin::ArtifactPin;
use crate::{
    ArtifactDependency, ArtifactFailure, ArtifactKey, ArtifactPayload, ArtifactVersion, Data,
    DirBound, DirChecked, DirElaborated, DirExpanded, DirExported, DirImported, DirMaterialized,
    DirParsed, GlobalEnvironment, MirLowered, MirOptimized, MirVerified, ModuleLinted,
    ModuleOutput, ModuleQueryIndex, PackageLinted, PackageOutput, WorkspaceLinted,
    WorkspaceQueryIndex,
};

/// One versioned artifact family map.
type ArtifactMap<T> = DashMap<ArtifactVersion, Arc<T>>;

/// Store of published semantic artifacts.
#[derive(Debug, Default)]
pub struct ArtifactStore {
    /// The exact artifact version entries.
    entries: DashMap<ArtifactVersion, ArtifactEntry>,
    /// The live retain count for each exact artifact version.
    retained_versions: DashMap<ArtifactVersion, usize>,

    /// DIR artifacts by module.
    dir_parsed: ArtifactMap<DirParsed>,
    /// Parsed data artifacts by module.
    data: ArtifactMap<Data>,

    /// Global environment by profile.
    global_environment: ArtifactMap<GlobalEnvironment>,

    /// Bound DIR artifacts by module and profile.
    dir_bound: ArtifactMap<DirBound>,
    /// Imported DIR artifacts by module and profile.
    dir_imported: ArtifactMap<DirImported>,
    /// Expanded DIR artifacts by module and profile.
    dir_expanded: ArtifactMap<DirExpanded>,
    /// Exported DIR artifacts by module and profile.
    dir_exported: ArtifactMap<DirExported>,
    /// Checked DIR artifacts by module and profile.
    dir_checked: ArtifactMap<DirChecked>,
    /// Materialized DIR artifacts by module and profile.
    dir_materialized: ArtifactMap<DirMaterialized>,
    /// Elaborated DIR artifacts by module and profile.
    dir_elaborated: ArtifactMap<DirElaborated>,

    /// Lowered MIR artifacts by module, profile, and target.
    mir_lowered: ArtifactMap<MirLowered>,
    /// Verified MIR markers by module, profile, and target.
    mir_verified: ArtifactMap<MirVerified>,
    /// Optimized MIR artifacts by module, profile, and target.
    mir_optimized: ArtifactMap<MirOptimized>,

    /// Query indexes by module and profile.
    module_query_index: ArtifactMap<ModuleQueryIndex>,
    /// Query indexes by workspace and profile.
    workspace_query_index: ArtifactMap<WorkspaceQueryIndex>,

    /// Generated module outputs by module and target.
    module_output: ArtifactMap<ModuleOutput>,
    /// Output entries by package and target.
    package_output: ArtifactMap<PackageOutput>,
    /// Module lint surfaces by module and profile.
    module_linted: ArtifactMap<ModuleLinted>,
    /// Package lint surfaces by package.
    package_linted: ArtifactMap<PackageLinted>,
    /// Workspace lint surfaces.
    workspace_linted: ArtifactMap<WorkspaceLinted>,
}

impl ArtifactStore {
    /// Create a new semantic artifact store.
    pub fn new() -> Self {
        Self::default()
    }

    /// Increase the live reference count for one exact artifact version.
    pub(crate) fn increase_ref_count(&self, version: &ArtifactVersion) {
        assert!(
            self.exists(version),
            "cannot increase ref count for missing artifact version: {version:?}"
        );

        let mut retain_count = self.retained_versions.entry(*version).or_insert(0);
        *retain_count += 1;
    }

    /// Retain one exact live artifact version with RAII release on drop.
    pub fn pin(self: &Arc<Self>, version: &ArtifactVersion) -> Option<ArtifactPin> {
        if !self.exists(version) {
            return None;
        }

        self.increase_ref_count(version);

        Some(ArtifactPin::new(Arc::clone(self), *version))
    }

    /// Decrease the live reference count for one exact artifact version.
    pub(crate) fn decrease_ref_count(&self, version: &ArtifactVersion) {
        let Some(mut retain_count) = self.retained_versions.get_mut(version) else {
            panic!("cannot decrease ref count for unpinned artifact version: {version:?}");
        };

        if *retain_count == 1 {
            drop(retain_count);
            self.retained_versions.remove(version);
        } else {
            *retain_count -= 1;
        }
    }

    /// Return the recorded diagnostics for one exact artifact version.
    pub fn diagnostics(&self, version: &ArtifactVersion) -> Option<Arc<DiagnosticCollection>> {
        self.entries
            .get(version)
            .map(|entry| Arc::clone(&entry.diagnostics))
    }

    /// Return the exact dependencies for one artifact version.
    pub fn dependencies(&self, version: &ArtifactVersion) -> Option<Arc<[ArtifactDependency]>> {
        self.entries
            .get(version)
            .map(|entry| Arc::clone(&entry.dependencies))
    }

    /// Return whether one exact artifact version entry exists.
    fn exists(&self, version: &ArtifactVersion) -> bool {
        self.entries.contains_key(version)
    }

    /// Return the exact terminal outcome for one artifact version.
    pub fn outcome(&self, version: &ArtifactVersion) -> Option<ArtifactOutcome> {
        self.entries.get(version).map(|entry| entry.outcome.clone())
    }

    /// Return the provider failure for one exact artifact version.
    pub fn failure(&self, version: &ArtifactVersion) -> Option<ArtifactFailure> {
        match self.outcome(version) {
            Some(ArtifactOutcome::Failed(failure)) => Some(failure),
            Some(ArtifactOutcome::Ok) | None => None,
        }
    }

    /// Return whether one exact artifact payload is published.
    pub fn has(&self, version: &ArtifactVersion) -> bool {
        match &version.key {
            ArtifactKey::GlobalEnvironment { .. } => self.global_environment.contains_key(version),
            ArtifactKey::DirParsed { .. } => self.dir_parsed.contains_key(version),
            ArtifactKey::Data { .. } => self.data.contains_key(version),
            ArtifactKey::DirBound { .. } => self.dir_bound.contains_key(version),
            ArtifactKey::DirImported { .. } => self.dir_imported.contains_key(version),
            ArtifactKey::DirExpanded { .. } => self.dir_expanded.contains_key(version),
            ArtifactKey::DirExported { .. } => self.dir_exported.contains_key(version),
            ArtifactKey::DirChecked { .. } => self.dir_checked.contains_key(version),
            ArtifactKey::DirMaterialized { .. } => self.dir_materialized.contains_key(version),
            ArtifactKey::DirElaborated { .. } => self.dir_elaborated.contains_key(version),
            ArtifactKey::MirLowered { .. } => self.mir_lowered.contains_key(version),
            ArtifactKey::MirVerified { .. } => self.mir_verified.contains_key(version),
            ArtifactKey::MirOptimized { .. } => self.mir_optimized.contains_key(version),
            ArtifactKey::ModuleQueryIndex { .. } => self.module_query_index.contains_key(version),
            ArtifactKey::WorkspaceQueryIndex { .. } => {
                self.workspace_query_index.contains_key(version)
            }
            ArtifactKey::ModuleOutput { .. } => self.module_output.contains_key(version),
            ArtifactKey::PackageOutput { .. } => self.package_output.contains_key(version),
            ArtifactKey::ModuleLinted { .. } => self.module_linted.contains_key(version),
            ArtifactKey::PackageLinted { .. } => self.package_linted.contains_key(version),
            ArtifactKey::WorkspaceLinted => self.workspace_linted.contains_key(version),
        }
    }

    /// Insert one typed artifact payload into its family map.
    fn insert_payload<T>(
        map: &ArtifactMap<T>,
        version: ArtifactVersion,
        payload: T,
        is_expected_key: bool,
        payload_name: &'static str,
    ) {
        assert!(
            is_expected_key,
            "artifact payload did not match key: key={:?} payload={payload_name}",
            version.key
        );

        map.insert(version, Arc::new(payload));
    }

    /// Publish one ready artifact.
    pub fn publish(
        &self,
        version: ArtifactVersion,
        payload: ArtifactPayload,
        dependencies: impl Into<Arc<[ArtifactDependency]>>,
        diagnostics: impl Into<Arc<DiagnosticCollection>>,
    ) {
        match payload {
            ArtifactPayload::GlobalEnvironment(payload) => Self::insert_payload(
                &self.global_environment,
                version,
                payload,
                matches!(&version.key, ArtifactKey::GlobalEnvironment { .. }),
                "GlobalEnvironment",
            ),
            ArtifactPayload::DirParsed(payload) => Self::insert_payload(
                &self.dir_parsed,
                version,
                payload,
                matches!(&version.key, ArtifactKey::DirParsed { .. }),
                "DirParsed",
            ),
            ArtifactPayload::Data(payload) => Self::insert_payload(
                &self.data,
                version,
                payload,
                matches!(&version.key, ArtifactKey::Data { .. }),
                "Data",
            ),
            ArtifactPayload::DirBound(payload) => Self::insert_payload(
                &self.dir_bound,
                version,
                payload,
                matches!(&version.key, ArtifactKey::DirBound { .. }),
                "DirBound",
            ),
            ArtifactPayload::DirImported(payload) => Self::insert_payload(
                &self.dir_imported,
                version,
                payload,
                matches!(&version.key, ArtifactKey::DirImported { .. }),
                "DirImported",
            ),
            ArtifactPayload::DirExpanded(payload) => Self::insert_payload(
                &self.dir_expanded,
                version,
                payload,
                matches!(&version.key, ArtifactKey::DirExpanded { .. }),
                "DirExpanded",
            ),
            ArtifactPayload::DirExported(payload) => Self::insert_payload(
                &self.dir_exported,
                version,
                payload,
                matches!(&version.key, ArtifactKey::DirExported { .. }),
                "DirExported",
            ),
            ArtifactPayload::DirChecked(payload) => Self::insert_payload(
                &self.dir_checked,
                version,
                payload,
                matches!(&version.key, ArtifactKey::DirChecked { .. }),
                "DirChecked",
            ),
            ArtifactPayload::DirMaterialized(payload) => Self::insert_payload(
                &self.dir_materialized,
                version,
                payload,
                matches!(&version.key, ArtifactKey::DirMaterialized { .. }),
                "DirMaterialized",
            ),
            ArtifactPayload::DirElaborated(payload) => Self::insert_payload(
                &self.dir_elaborated,
                version,
                payload,
                matches!(&version.key, ArtifactKey::DirElaborated { .. }),
                "DirElaborated",
            ),
            ArtifactPayload::MirLowered(payload) => Self::insert_payload(
                &self.mir_lowered,
                version,
                payload,
                matches!(&version.key, ArtifactKey::MirLowered { .. }),
                "MirLowered",
            ),
            ArtifactPayload::MirVerified(payload) => Self::insert_payload(
                &self.mir_verified,
                version,
                payload,
                matches!(&version.key, ArtifactKey::MirVerified { .. }),
                "MirVerified",
            ),
            ArtifactPayload::MirOptimized(payload) => Self::insert_payload(
                &self.mir_optimized,
                version,
                payload,
                matches!(&version.key, ArtifactKey::MirOptimized { .. }),
                "MirOptimized",
            ),
            ArtifactPayload::ModuleQueryIndex(payload) => Self::insert_payload(
                &self.module_query_index,
                version,
                payload,
                matches!(&version.key, ArtifactKey::ModuleQueryIndex { .. }),
                "ModuleQueryIndex",
            ),
            ArtifactPayload::WorkspaceQueryIndex(payload) => Self::insert_payload(
                &self.workspace_query_index,
                version,
                payload,
                matches!(&version.key, ArtifactKey::WorkspaceQueryIndex { .. }),
                "WorkspaceQueryIndex",
            ),
            ArtifactPayload::ModuleOutput(payload) => Self::insert_payload(
                &self.module_output,
                version,
                payload,
                matches!(&version.key, ArtifactKey::ModuleOutput { .. }),
                "ModuleOutput",
            ),
            ArtifactPayload::PackageOutput(payload) => Self::insert_payload(
                &self.package_output,
                version,
                payload,
                matches!(&version.key, ArtifactKey::PackageOutput { .. }),
                "PackageOutput",
            ),
            ArtifactPayload::ModuleLinted(payload) => Self::insert_payload(
                &self.module_linted,
                version,
                payload,
                matches!(&version.key, ArtifactKey::ModuleLinted { .. }),
                "ModuleLinted",
            ),
            ArtifactPayload::PackageLinted(payload) => Self::insert_payload(
                &self.package_linted,
                version,
                payload,
                matches!(&version.key, ArtifactKey::PackageLinted { .. }),
                "PackageLinted",
            ),
            ArtifactPayload::WorkspaceLinted(payload) => Self::insert_payload(
                &self.workspace_linted,
                version,
                payload,
                matches!(&version.key, ArtifactKey::WorkspaceLinted),
                "WorkspaceLinted",
            ),
        }

        self.entries
            .insert(version, ArtifactEntry::ok(dependencies, diagnostics));
    }

    /// Fail one artifact.
    pub fn fail(
        &self,
        version: ArtifactVersion,
        dependencies: impl Into<Arc<[ArtifactDependency]>>,
        diagnostics: impl Into<Arc<DiagnosticCollection>>,
        failure: ArtifactFailure,
    ) {
        self.entries.insert(
            version,
            ArtifactEntry::failed(dependencies, diagnostics, failure),
        );
    }
}

impl ArtifactStore {
    /// Get one global environment artifact.
    pub fn global_environment(&self, version: &ArtifactVersion) -> Option<Arc<GlobalEnvironment>> {
        self.global_environment
            .get(version)
            .map(|entry| entry.value().clone())
    }

    /// Get one DIR artifact.
    pub fn dir_parsed(&self, version: &ArtifactVersion) -> Option<Arc<DirParsed>> {
        self.dir_parsed
            .get(version)
            .map(|entry| entry.value().clone())
    }

    /// Get one data artifact.
    pub fn data(&self, version: &ArtifactVersion) -> Option<Arc<Data>> {
        self.data.get(version).map(|entry| entry.value().clone())
    }

    /// Get one bound DIR artifact.
    pub fn dir_bound(&self, version: &ArtifactVersion) -> Option<Arc<DirBound>> {
        self.dir_bound
            .get(version)
            .map(|entry| entry.value().clone())
    }

    /// Get one imported DIR artifact.
    pub fn dir_imported(&self, version: &ArtifactVersion) -> Option<Arc<DirImported>> {
        self.dir_imported
            .get(version)
            .map(|entry| entry.value().clone())
    }

    /// Get one expanded DIR artifact.
    pub fn dir_expanded(&self, version: &ArtifactVersion) -> Option<Arc<DirExpanded>> {
        self.dir_expanded
            .get(version)
            .map(|entry| entry.value().clone())
    }

    /// Get one exported DIR artifact.
    pub fn dir_exported(&self, version: &ArtifactVersion) -> Option<Arc<DirExported>> {
        self.dir_exported
            .get(version)
            .map(|entry| entry.value().clone())
    }

    /// Get one checked DIR artifact.
    pub fn dir_checked(&self, version: &ArtifactVersion) -> Option<Arc<DirChecked>> {
        self.dir_checked
            .get(version)
            .map(|entry| entry.value().clone())
    }

    /// Get one materialized DIR artifact.
    pub fn dir_materialized(&self, version: &ArtifactVersion) -> Option<Arc<DirMaterialized>> {
        self.dir_materialized
            .get(version)
            .map(|entry| entry.value().clone())
    }

    /// Get one elaborated DIR artifact.
    pub fn dir_elaborated(&self, version: &ArtifactVersion) -> Option<Arc<DirElaborated>> {
        self.dir_elaborated
            .get(version)
            .map(|entry| entry.value().clone())
    }

    /// Get one lowered MIR artifact.
    pub fn mir_lowered(&self, version: &ArtifactVersion) -> Option<Arc<MirLowered>> {
        self.mir_lowered
            .get(version)
            .map(|entry| entry.value().clone())
    }

    /// Get one verified MIR marker.
    pub fn mir_verified(&self, version: &ArtifactVersion) -> Option<Arc<MirVerified>> {
        self.mir_verified
            .get(version)
            .map(|entry| entry.value().clone())
    }

    /// Get one optimized MIR artifact.
    pub fn mir_optimized(&self, version: &ArtifactVersion) -> Option<Arc<MirOptimized>> {
        self.mir_optimized
            .get(version)
            .map(|entry| entry.value().clone())
    }

    /// Get one module query index artifact.
    pub fn module_query_index(&self, version: &ArtifactVersion) -> Option<Arc<ModuleQueryIndex>> {
        self.module_query_index
            .get(version)
            .map(|entry| entry.value().clone())
    }

    /// Get one workspace query index artifact.
    pub fn workspace_query_index(
        &self,
        version: &ArtifactVersion,
    ) -> Option<Arc<WorkspaceQueryIndex>> {
        self.workspace_query_index
            .get(version)
            .map(|entry| entry.value().clone())
    }

    /// Get one module output artifact.
    pub fn module_output(&self, version: &ArtifactVersion) -> Option<Arc<ModuleOutput>> {
        self.module_output
            .get(version)
            .map(|entry| entry.value().clone())
    }

    /// Get one package output artifact.
    pub fn package_output(&self, version: &ArtifactVersion) -> Option<Arc<PackageOutput>> {
        self.package_output
            .get(version)
            .map(|entry| entry.value().clone())
    }

    /// Get one module lint artifact.
    pub fn module_linted(&self, version: &ArtifactVersion) -> Option<Arc<ModuleLinted>> {
        self.module_linted
            .get(version)
            .map(|entry| entry.value().clone())
    }

    /// Get one package lint artifact.
    pub fn package_linted(&self, version: &ArtifactVersion) -> Option<Arc<PackageLinted>> {
        self.package_linted
            .get(version)
            .map(|entry| entry.value().clone())
    }

    /// Get one workspace lint artifact.
    pub fn workspace_linted(&self, version: &ArtifactVersion) -> Option<Arc<WorkspaceLinted>> {
        self.workspace_linted
            .get(version)
            .map(|entry| entry.value().clone())
    }
}
