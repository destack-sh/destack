use std::sync::Arc;

use dashmap::DashMap;
use destack_ast as ast;
use destack_core::StringPool;
use destack_source::{DiagnosticCollection, FileId, ModuleId};

use super::pin::ArtifactPin;
use crate::{
    AmbientEnvironment, ArtifactFingerprint, ArtifactInput, ArtifactKey, ArtifactPayload,
    ArtifactRecord, ArtifactVersion, Ast, Data, DirChecked, DirDeclared, DirElaborated,
    DirExported, LanguageEnvironment, Mir, MirOptimized, ModuleLinted, ModuleOutput,
    PackageLinted, PackageOutput, WorkspaceLinted,
};

use super::entry::{ArtifactEntry, ArtifactStatus};

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

    /// MIR artifacts by module, profile, and target.
    mir: ArtifactMap<Mir>,
    /// Optimized MIR artifacts by module, profile, and target.
    mir_optimized: ArtifactMap<MirOptimized>,

    /// Generated module artifacts by module and target.
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

    /// Publish the synthetic root AST for one program.
    pub fn publish_root_ast(&self, root_module_id: ModuleId, fallback_file_id: FileId) {
        let root_ast = ast::Tree::new();
        let mut root_module_ast = Ast::from_tree(
            root_module_id,
            root_ast,
            Vec::new(),
            StringPool::new(),
            Vec::new(),
            Vec::new(),
        );
        root_module_ast.ensure_anchor_expression(fallback_file_id);
        let version = ArtifactVersion::new(
            ArtifactKey::Ast {
                module: root_module_id,
            },
            ArtifactFingerprint::new(0),
        );
        let record = ArtifactRecord::new(
            version,
            root_module_ast,
            [],
            [],
            DiagnosticCollection::new(),
        );
        self.publish_ast(record);
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

    /// Return the exact inputs for one artifact version.
    pub fn inputs(&self, version: &ArtifactVersion) -> Option<Arc<[ArtifactInput]>> {
        self.entries
            .get(version)
            .map(|entry| Arc::clone(&entry.inputs))
    }

    /// Return the recorded diagnostics for one exact artifact version.
    pub fn diagnostics(&self, version: &ArtifactVersion) -> Option<Arc<DiagnosticCollection>> {
        self.entries
            .get(version)
            .map(|entry| Arc::clone(&entry.diagnostics))
    }

    /// Return the exact dependencies for one artifact version.
    pub fn dependencies(&self, version: &ArtifactVersion) -> Option<Arc<[ArtifactVersion]>> {
        self.entries
            .get(version)
            .map(|entry| Arc::clone(&entry.dependencies))
    }

    /// Return whether one exact artifact version entry exists.
    fn exists(&self, version: &ArtifactVersion) -> bool {
        self.entries.contains_key(version)
    }

    /// Return the exact availability status for one artifact version.
    pub fn status(&self, version: &ArtifactVersion) -> ArtifactStatus {
        if self.has(version) {
            ArtifactStatus::Ready
        } else {
            ArtifactStatus::Missing
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
            ArtifactKey::Mir { .. } => self.mir.contains_key(version),
            ArtifactKey::MirOptimized { .. } => self.mir_optimized.contains_key(version),
            ArtifactKey::ModuleOutput { .. } => self.module_output.contains_key(version),
            ArtifactKey::PackageOutput { .. } => self.package_output.contains_key(version),
            ArtifactKey::ModuleLinted { .. } => self.module_linted.contains_key(version),
            ArtifactKey::PackageLinted { .. } => self.package_linted.contains_key(version),
            ArtifactKey::WorkspaceLinted => self.workspace_linted.contains_key(version),
        }
    }

    /// Publish one complete typed artifact record into one versioned family map.
    fn publish<T>(
        &self,
        map: &ArtifactMap<T>,
        record: ArtifactRecord<T>,
        is_expected_key: bool,
        payload_name: &str,
    ) {
        assert!(
            is_expected_key,
            "artifact payload did not match key: key={:?} payload={payload_name}",
            record.version.key
        );

        map.insert(record.version, record.payload);
        self.entries.insert(
            record.version,
            ArtifactEntry::new(record.inputs, record.dependencies, record.diagnostics),
        );
    }

    /// Publish one complete artifact payload record.
    pub fn publish_payload(
        &self,
        version: ArtifactVersion,
        payload: ArtifactPayload,
        inputs: impl Into<Arc<[ArtifactInput]>>,
        dependencies: impl Into<Arc<[ArtifactVersion]>>,
        diagnostics: impl Into<Arc<DiagnosticCollection>>,
    ) {
        let inputs = inputs.into();
        let dependencies = dependencies.into();
        let diagnostics = diagnostics.into();

        match payload {
            ArtifactPayload::LanguageEnvironment(payload) => self.publish_language_environment(
                ArtifactRecord::new(version, payload, inputs, dependencies, diagnostics),
            ),
            ArtifactPayload::AmbientEnvironment(payload) => self.publish_ambient_environment(
                ArtifactRecord::new(version, payload, inputs, dependencies, diagnostics),
            ),
            ArtifactPayload::Ast(payload) => {
                self.publish_ast(ArtifactRecord::new(version, payload, inputs, dependencies, diagnostics));
            }
            ArtifactPayload::Data(payload) => {
                self.publish_data(ArtifactRecord::new(version, payload, inputs, dependencies, diagnostics));
            }
            ArtifactPayload::DirDeclared(payload) => self.publish_dir_declared(
                ArtifactRecord::new(version, payload, inputs, dependencies, diagnostics),
            ),
            ArtifactPayload::DirExported(payload) => self.publish_dir_exported(
                ArtifactRecord::new(version, payload, inputs, dependencies, diagnostics),
            ),
            ArtifactPayload::DirChecked(payload) => self.publish_dir_checked(
                ArtifactRecord::new(version, payload, inputs, dependencies, diagnostics),
            ),
            ArtifactPayload::DirElaborated(payload) => self.publish_dir_elaborated(
                ArtifactRecord::new(version, payload, inputs, dependencies, diagnostics),
            ),
            ArtifactPayload::Mir(payload) => {
                self.publish_mir(ArtifactRecord::new(version, payload, inputs, dependencies, diagnostics));
            }
            ArtifactPayload::MirOptimized(payload) => self.publish_mir_optimized(
                ArtifactRecord::new(version, payload, inputs, dependencies, diagnostics),
            ),
            ArtifactPayload::ModuleOutput(payload) => self.publish_module_output(
                ArtifactRecord::new(version, payload, inputs, dependencies, diagnostics),
            ),
            ArtifactPayload::PackageOutput(payload) => self.publish_package_output(
                ArtifactRecord::new(version, payload, inputs, dependencies, diagnostics),
            ),
            ArtifactPayload::ModuleLinted(payload) => self.publish_module_linted(
                ArtifactRecord::new(version, payload, inputs, dependencies, diagnostics),
            ),
            ArtifactPayload::PackageLinted(payload) => self.publish_package_linted(
                ArtifactRecord::new(version, payload, inputs, dependencies, diagnostics),
            ),
            ArtifactPayload::WorkspaceLinted(payload) => self.publish_workspace_linted(
                ArtifactRecord::new(version, payload, inputs, dependencies, diagnostics),
            ),
        }
    }
}

impl ArtifactStore {
    /// Publish one language environment artifact.
    pub fn publish_language_environment(&self, record: ArtifactRecord<LanguageEnvironment>) {
        let is_expected_key =
            matches!(&record.version.key, ArtifactKey::LanguageEnvironment { .. });
        self.publish(
            &self.language_environments,
            record,
            is_expected_key,
            "LanguageEnvironment",
        );
    }

    /// Get one language environment artifact.
    pub fn language_environment(
        &self,
        version: &ArtifactVersion,
    ) -> Option<Arc<LanguageEnvironment>> {
        self.language_environments
            .get(version)
            .map(|entry| entry.value().clone())
    }

    /// Publish one ambient environment artifact.
    pub fn publish_ambient_environment(&self, record: ArtifactRecord<AmbientEnvironment>) {
        let is_expected_key = matches!(&record.version.key, ArtifactKey::AmbientEnvironment { .. });
        self.publish(
            &self.ambient_environments,
            record,
            is_expected_key,
            "AmbientEnvironment",
        );
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

    /// Publish one AST artifact.
    pub fn publish_ast(&self, record: ArtifactRecord<Ast>) {
        let is_expected_key = matches!(&record.version.key, ArtifactKey::Ast { .. });
        self.publish(&self.ast, record, is_expected_key, "Ast");
    }

    /// Get one AST artifact.
    pub fn ast(&self, version: &ArtifactVersion) -> Option<Arc<Ast>> {
        self.ast.get(version).map(|entry| entry.value().clone())
    }

    /// Publish one data artifact.
    pub fn publish_data(&self, record: ArtifactRecord<Data>) {
        let is_expected_key = matches!(&record.version.key, ArtifactKey::Data { .. });
        self.publish(&self.data, record, is_expected_key, "Data");
    }

    /// Get one data artifact.
    pub fn data(&self, version: &ArtifactVersion) -> Option<Arc<Data>> {
        self.data.get(version).map(|entry| entry.value().clone())
    }

    /// Publish one declared DIR artifact.
    pub fn publish_dir_declared(&self, record: ArtifactRecord<DirDeclared>) {
        let is_expected_key = matches!(&record.version.key, ArtifactKey::DirDeclared { .. });
        self.publish(&self.dir_declared, record, is_expected_key, "DirDeclared");
    }

    /// Get one declared DIR artifact.
    pub fn dir_declared(&self, version: &ArtifactVersion) -> Option<Arc<DirDeclared>> {
        self.dir_declared
            .get(version)
            .map(|entry| entry.value().clone())
    }

    /// Publish one exported DIR artifact.
    pub fn publish_dir_exported(&self, record: ArtifactRecord<DirExported>) {
        let is_expected_key = matches!(&record.version.key, ArtifactKey::DirExported { .. });
        self.publish(&self.dir_exported, record, is_expected_key, "DirExported");
    }

    /// Get one exported DIR artifact.
    pub fn dir_exported(&self, version: &ArtifactVersion) -> Option<Arc<DirExported>> {
        self.dir_exported
            .get(version)
            .map(|entry| entry.value().clone())
    }

    /// Publish one checked DIR artifact.
    pub fn publish_dir_checked(&self, record: ArtifactRecord<DirChecked>) {
        let is_expected_key = matches!(&record.version.key, ArtifactKey::DirChecked { .. });
        self.publish(&self.dir_checked, record, is_expected_key, "DirChecked");
    }

    /// Get one checked DIR artifact.
    pub fn dir_checked(&self, version: &ArtifactVersion) -> Option<Arc<DirChecked>> {
        self.dir_checked
            .get(version)
            .map(|entry| entry.value().clone())
    }

    /// Publish one elaborated DIR artifact.
    pub fn publish_dir_elaborated(&self, record: ArtifactRecord<DirElaborated>) {
        let is_expected_key = matches!(&record.version.key, ArtifactKey::DirElaborated { .. });
        self.publish(
            &self.dir_elaborated,
            record,
            is_expected_key,
            "DirElaborated",
        );
    }

    /// Get one elaborated DIR artifact.
    pub fn dir_elaborated(&self, version: &ArtifactVersion) -> Option<Arc<DirElaborated>> {
        self.dir_elaborated
            .get(version)
            .map(|entry| entry.value().clone())
    }

    /// Publish one MIR artifact.
    pub fn publish_mir(&self, record: ArtifactRecord<Mir>) {
        let is_expected_key = matches!(&record.version.key, ArtifactKey::Mir { .. });
        self.publish(&self.mir, record, is_expected_key, "Mir");
    }

    /// Get one MIR artifact.
    pub fn mir(&self, version: &ArtifactVersion) -> Option<Arc<Mir>> {
        self.mir.get(version).map(|entry| entry.value().clone())
    }

    /// Publish one optimized MIR artifact.
    pub fn publish_mir_optimized(&self, record: ArtifactRecord<MirOptimized>) {
        let is_expected_key = matches!(&record.version.key, ArtifactKey::MirOptimized { .. });
        self.publish(&self.mir_optimized, record, is_expected_key, "MirOptimized");
    }

    /// Get one optimized MIR artifact.
    pub fn mir_optimized(&self, version: &ArtifactVersion) -> Option<Arc<MirOptimized>> {
        self.mir_optimized
            .get(version)
            .map(|entry| entry.value().clone())
    }

    /// Publish one module output artifact.
    pub fn publish_module_output(&self, record: ArtifactRecord<ModuleOutput>) {
        let is_expected_key = matches!(&record.version.key, ArtifactKey::ModuleOutput { .. });
        self.publish(&self.module_output, record, is_expected_key, "ModuleOutput");
    }

    /// Get one module output artifact.
    pub fn module_output(&self, version: &ArtifactVersion) -> Option<Arc<ModuleOutput>> {
        self.module_output
            .get(version)
            .map(|entry| entry.value().clone())
    }

    /// Publish one package output artifact.
    pub fn publish_package_output(&self, record: ArtifactRecord<PackageOutput>) {
        let is_expected_key = matches!(&record.version.key, ArtifactKey::PackageOutput { .. });
        self.publish(
            &self.package_output,
            record,
            is_expected_key,
            "PackageOutput",
        );
    }

    /// Get one package output artifact.
    pub fn package_output(&self, version: &ArtifactVersion) -> Option<Arc<PackageOutput>> {
        self.package_output
            .get(version)
            .map(|entry| entry.value().clone())
    }

    /// Publish one module lint artifact.
    pub fn publish_module_linted(&self, record: ArtifactRecord<ModuleLinted>) {
        let is_expected_key = matches!(&record.version.key, ArtifactKey::ModuleLinted { .. });
        self.publish(&self.module_linted, record, is_expected_key, "ModuleLinted");
    }

    /// Get one module lint artifact.
    pub fn module_linted(&self, version: &ArtifactVersion) -> Option<Arc<ModuleLinted>> {
        self.module_linted
            .get(version)
            .map(|entry| entry.value().clone())
    }

    /// Publish one package lint artifact.
    pub fn publish_package_linted(&self, record: ArtifactRecord<PackageLinted>) {
        let is_expected_key = matches!(&record.version.key, ArtifactKey::PackageLinted { .. });
        self.publish(
            &self.package_linted,
            record,
            is_expected_key,
            "PackageLinted",
        );
    }

    /// Get one package lint artifact.
    pub fn package_linted(&self, version: &ArtifactVersion) -> Option<Arc<PackageLinted>> {
        self.package_linted
            .get(version)
            .map(|entry| entry.value().clone())
    }

    /// Publish one workspace lint artifact.
    pub fn publish_workspace_linted(&self, record: ArtifactRecord<WorkspaceLinted>) {
        let is_expected_key = matches!(&record.version.key, ArtifactKey::WorkspaceLinted);
        self.publish(
            &self.workspace_linted,
            record,
            is_expected_key,
            "WorkspaceLinted",
        );
    }

    /// Get one workspace lint artifact.
    pub fn workspace_linted(&self, version: &ArtifactVersion) -> Option<Arc<WorkspaceLinted>> {
        self.workspace_linted
            .get(version)
            .map(|entry| entry.value().clone())
    }
}
