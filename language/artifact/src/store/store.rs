use std::sync::Arc;

use dashmap::DashMap;
use destack_source::DiagnosticCollection;

use super::entry::{ArtifactEntry, ArtifactOutcome};
use super::pin::ArtifactPin;
use crate::{
    AmbientEnvironment, ArtifactDependency, ArtifactFailure, ArtifactKey, ArtifactPayload,
    ArtifactVersion, Ast, Data, DirChecked, DirDeclared, DirElaborated, DirExported,
    LanguageEnvironment, MirLowered, MirOptimized, ModuleLinted, ModuleOutput, PackageLinted,
    PackageOutput, WorkspaceLinted,
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

    /// Language environments by profile.
    language_environments: ArtifactMap<LanguageEnvironment>,
    /// Ambient environments by profile.
    ambient_environments: ArtifactMap<AmbientEnvironment>,

    /// AST artifacts by module.
    ast: ArtifactMap<Ast>,
    /// Parsed data artifacts by module.
    data: ArtifactMap<Data>,

    /// Declared DIR artifacts by module and profile.
    dir_declared: ArtifactMap<DirDeclared>,
    /// Exported DIR artifacts by module and profile.
    dir_exported: ArtifactMap<DirExported>,
    /// Checked DIR artifacts by module and profile.
    dir_checked: ArtifactMap<DirChecked>,
    /// Elaborated DIR artifacts by module and profile.
    dir_elaborated: ArtifactMap<DirElaborated>,

    /// Lowered MIR artifacts by module, profile, and target.
    mir_lowered: ArtifactMap<MirLowered>,
    /// Optimized MIR artifacts by module, profile, and target.
    mir_optimized: ArtifactMap<MirOptimized>,

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
            ArtifactKey::LanguageEnvironment { .. } => {
                self.language_environments.contains_key(version)
            }
            ArtifactKey::AmbientEnvironment { .. } => {
                self.ambient_environments.contains_key(version)
            }
            ArtifactKey::Ast { .. } => self.ast.contains_key(version),
            ArtifactKey::Data { .. } => self.data.contains_key(version),
            ArtifactKey::DirDeclared { .. } => self.dir_declared.contains_key(version),
            ArtifactKey::DirExported { .. } => self.dir_exported.contains_key(version),
            ArtifactKey::DirChecked { .. } => self.dir_checked.contains_key(version),
            ArtifactKey::DirElaborated { .. } => self.dir_elaborated.contains_key(version),
            ArtifactKey::MirLowered { .. } => self.mir_lowered.contains_key(version),
            ArtifactKey::MirOptimized { .. } => self.mir_optimized.contains_key(version),
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
            ArtifactPayload::LanguageEnvironment(payload) => Self::insert_payload(
                &self.language_environments,
                version,
                payload,
                matches!(&version.key, ArtifactKey::LanguageEnvironment { .. }),
                "LanguageEnvironment",
            ),
            ArtifactPayload::AmbientEnvironment(payload) => Self::insert_payload(
                &self.ambient_environments,
                version,
                payload,
                matches!(&version.key, ArtifactKey::AmbientEnvironment { .. }),
                "AmbientEnvironment",
            ),
            ArtifactPayload::Ast(payload) => Self::insert_payload(
                &self.ast,
                version,
                payload,
                matches!(&version.key, ArtifactKey::Ast { .. }),
                "Ast",
            ),
            ArtifactPayload::Data(payload) => Self::insert_payload(
                &self.data,
                version,
                payload,
                matches!(&version.key, ArtifactKey::Data { .. }),
                "Data",
            ),
            ArtifactPayload::DirDeclared(payload) => Self::insert_payload(
                &self.dir_declared,
                version,
                payload,
                matches!(&version.key, ArtifactKey::DirDeclared { .. }),
                "DirDeclared",
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
            ArtifactPayload::MirOptimized(payload) => Self::insert_payload(
                &self.mir_optimized,
                version,
                payload,
                matches!(&version.key, ArtifactKey::MirOptimized { .. }),
                "MirOptimized",
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
    /// Get one language environment artifact.
    pub fn language_environment(
        &self,
        version: &ArtifactVersion,
    ) -> Option<Arc<LanguageEnvironment>> {
        self.language_environments
            .get(version)
            .map(|entry| entry.value().clone())
    }

    /// Get one ambient environment artifact.
    pub fn ambient_environment(
        &self,
        version: &ArtifactVersion,
    ) -> Option<Arc<AmbientEnvironment>> {
        self.ambient_environments
            .get(version)
            .map(|entry| entry.value().clone())
    }

    /// Get one AST artifact.
    pub fn ast(&self, version: &ArtifactVersion) -> Option<Arc<Ast>> {
        self.ast.get(version).map(|entry| entry.value().clone())
    }

    /// Get one data artifact.
    pub fn data(&self, version: &ArtifactVersion) -> Option<Arc<Data>> {
        self.data.get(version).map(|entry| entry.value().clone())
    }

    /// Get one declared DIR artifact.
    pub fn dir_declared(&self, version: &ArtifactVersion) -> Option<Arc<DirDeclared>> {
        self.dir_declared
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

    /// Get one optimized MIR artifact.
    pub fn mir_optimized(&self, version: &ArtifactVersion) -> Option<Arc<MirOptimized>> {
        self.mir_optimized
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
