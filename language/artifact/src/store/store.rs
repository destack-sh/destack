use std::collections::HashSet;
use std::sync::Arc;

use dashmap::DashMap;
use destack_ast as ast;
use destack_core::StringPool;
use destack_source::{DiagnosticCollection, FileId, ModuleId, ProfileId};

use super::pin::ArtifactPin;
use crate::{
    ArtifactDependency, ArtifactKey, ArtifactStamp, ArtifactVersion, Ast, Data, DirAnalyzed,
    DirBase, DirDeclared, DirElaborated, DirInterface, DirPatched, DirPrepared, DirResolved,
    IntrinsicEnvironment, LanguageEnvironment, LibraryEnvironment, MirBase, MirOptimized,
    ModuleGraph, ModuleLinted, ModuleOutput, PackageLinted, PackageOutput, WorkspaceLinted,
};

/// One versioned artifact family map.
type ArtifactMap<T> = DashMap<ArtifactVersion, Arc<T>>;

/// Exact availability status for one artifact version.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArtifactStatus {
    /// The exact payload is published.
    Ready,
    /// The exact attempt failed.
    Failed,
    /// The exact version is missing.
    Missing,
}

/// One exact artifact version entry.
#[derive(Debug, Clone)]
struct ArtifactEntry {
    /// The validated live dependencies for this exact artifact version.
    dependencies: Vec<ArtifactDependency>,
    /// The diagnostics for this exact artifact version.
    diagnostics: Arc<DiagnosticCollection>,
}

impl Default for ArtifactEntry {
    fn default() -> Self {
        Self {
            dependencies: Vec::new(),
            diagnostics: Arc::new(DiagnosticCollection::new()),
        }
    }
}

/// Store of published semantic artifacts.
#[derive(Debug, Default)]
pub struct ArtifactStore {
    /// The exact artifact version entries.
    entries: DashMap<ArtifactVersion, ArtifactEntry>,
    /// The live retain count for each exact artifact version.
    retained_versions: DashMap<ArtifactVersion, usize>,

    /// Module dependency graphs by profile.
    module_graphs: ArtifactMap<ModuleGraph>,
    /// Language environments by profile.
    language_environments: ArtifactMap<LanguageEnvironment>,
    /// Intrinsic environments by profile.
    intrinsic_environments: ArtifactMap<IntrinsicEnvironment>,
    /// Library environments by profile.
    lib_environments: ArtifactMap<LibraryEnvironment>,

    /// AST artifacts by module.
    asts: ArtifactMap<Ast>,
    /// Parsed data artifacts by module.
    datas: ArtifactMap<Data>,

    /// Base DIR artifacts by module.
    dir_bases: ArtifactMap<DirBase>,
    /// Prepared DIR artifacts by module and profile.
    dir_prepared: ArtifactMap<DirPrepared>,
    /// Resolved DIR artifacts by module and profile.
    dir_resolved: ArtifactMap<DirResolved>,
    /// Declared DIR artifacts by module and profile.
    dir_declared: ArtifactMap<DirDeclared>,
    /// Interface DIR artifacts by module and profile.
    dir_interface: ArtifactMap<DirInterface>,
    /// Analyzed DIR artifacts by module and profile.
    dir_analyzed: ArtifactMap<DirAnalyzed>,
    /// Elaborated DIR artifacts by module and profile.
    dir_elaborated: ArtifactMap<DirElaborated>,
    /// Patched DIR artifacts by module and profile.
    dir_patched: ArtifactMap<DirPatched>,

    /// Base MIR artifacts by module, profile, and target.
    mir_bases: ArtifactMap<MirBase>,
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

    /// Publish one typed payload into one versioned family map.
    fn publish<T>(
        &self,
        map: &ArtifactMap<T>,
        version: ArtifactVersion,
        payload: Arc<T>,
        is_expected_key: bool,
        payload_name: &str,
    ) {
        assert!(
            is_expected_key,
            "artifact payload did not match key: key={:?} payload={payload_name}",
            version.key
        );

        self.insert(map, version, payload);
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
        self.publish_ast(
            ArtifactVersion::new(
                ArtifactKey::Ast {
                    module: root_module_id,
                },
                ArtifactStamp::new(0),
            ),
            root_module_ast,
        );
    }

    /// Return the profile ids that currently have published artifacts for one module.
    pub fn profile_ids_for_module(&self, module: ModuleId) -> std::collections::HashSet<ProfileId> {
        let mut profiles = std::collections::HashSet::new();

        self.collect_profiles_for_module(&self.dir_prepared, module, &mut profiles);
        self.collect_profiles_for_module(&self.dir_resolved, module, &mut profiles);
        self.collect_profiles_for_module(&self.dir_declared, module, &mut profiles);
        self.collect_profiles_for_module(&self.dir_interface, module, &mut profiles);
        self.collect_profiles_for_module(&self.dir_analyzed, module, &mut profiles);
        self.collect_profiles_for_module(&self.dir_elaborated, module, &mut profiles);
        self.collect_profiles_for_module(&self.dir_patched, module, &mut profiles);
        self.collect_profiles_for_module(&self.mir_bases, module, &mut profiles);
        self.collect_profiles_for_module(&self.mir_optimized, module, &mut profiles);
        self.collect_profiles_for_module(&self.module_linted, module, &mut profiles);

        profiles
    }

    /// Evict one exact artifact version.
    pub fn evict(&self, version: &ArtifactVersion) {
        self.entries.remove(version);
        self.retained_versions.remove(version);
        match &version.key {
            ArtifactKey::ModuleGraph { .. } => {
                self.module_graphs.remove(version);
            }
            ArtifactKey::LanguageEnvironment { .. } => {
                self.language_environments.remove(version);
            }
            ArtifactKey::IntrinsicEnvironment { .. } => {
                self.intrinsic_environments.remove(version);
            }
            ArtifactKey::LibraryEnvironment { .. } => {
                self.lib_environments.remove(version);
            }
            ArtifactKey::Ast { .. } => {
                self.asts.remove(version);
            }
            ArtifactKey::Data { .. } => {
                self.datas.remove(version);
            }
            ArtifactKey::DirBase { .. } => {
                self.dir_bases.remove(version);
            }
            ArtifactKey::DirPrepared { .. } => {
                self.dir_prepared.remove(version);
            }
            ArtifactKey::DirResolved { .. } => {
                self.dir_resolved.remove(version);
            }
            ArtifactKey::DirDeclared { .. } => {
                self.dir_declared.remove(version);
            }
            ArtifactKey::DirInterface { .. } => {
                self.dir_interface.remove(version);
            }
            ArtifactKey::DirAnalyzed { .. } => {
                self.dir_analyzed.remove(version);
            }
            ArtifactKey::DirElaborated { .. } => {
                self.dir_elaborated.remove(version);
            }
            ArtifactKey::DirPatched { .. } => {
                self.dir_patched.remove(version);
            }
            ArtifactKey::MirBase { .. } => {
                self.mir_bases.remove(version);
            }
            ArtifactKey::MirOptimized { .. } => {
                self.mir_optimized.remove(version);
            }
            ArtifactKey::ModuleOutput { .. } => {
                self.module_output.remove(version);
            }
            ArtifactKey::PackageOutput { .. } => {
                self.package_output.remove(version);
            }
            ArtifactKey::ModuleLinted { .. } => {
                self.module_linted.remove(version);
            }
            ArtifactKey::PackageLinted { .. } => {
                self.package_linted.remove(version);
            }
            ArtifactKey::WorkspaceLinted => {
                self.workspace_linted.remove(version);
            }
        }
    }

    /// Evict all published versions for one semantic artifact key.
    pub fn evict_key(&self, key: &ArtifactKey) {
        match key {
            ArtifactKey::ModuleGraph { .. } => self.evict_matching(&self.module_graphs, key),
            ArtifactKey::LanguageEnvironment { .. } => {
                self.evict_matching(&self.language_environments, key)
            }
            ArtifactKey::IntrinsicEnvironment { .. } => {
                self.evict_matching(&self.intrinsic_environments, key)
            }
            ArtifactKey::LibraryEnvironment { .. } => {
                self.evict_matching(&self.lib_environments, key)
            }
            ArtifactKey::Ast { .. } => self.evict_matching(&self.asts, key),
            ArtifactKey::Data { .. } => self.evict_matching(&self.datas, key),
            ArtifactKey::DirBase { .. } => self.evict_matching(&self.dir_bases, key),
            ArtifactKey::DirPrepared { .. } => self.evict_matching(&self.dir_prepared, key),
            ArtifactKey::DirResolved { .. } => self.evict_matching(&self.dir_resolved, key),
            ArtifactKey::DirDeclared { .. } => self.evict_matching(&self.dir_declared, key),
            ArtifactKey::DirInterface { .. } => self.evict_matching(&self.dir_interface, key),
            ArtifactKey::DirAnalyzed { .. } => self.evict_matching(&self.dir_analyzed, key),
            ArtifactKey::DirElaborated { .. } => self.evict_matching(&self.dir_elaborated, key),
            ArtifactKey::DirPatched { .. } => self.evict_matching(&self.dir_patched, key),
            ArtifactKey::MirBase { .. } => self.evict_matching(&self.mir_bases, key),
            ArtifactKey::MirOptimized { .. } => self.evict_matching(&self.mir_optimized, key),
            ArtifactKey::ModuleOutput { .. } => self.evict_matching(&self.module_output, key),
            ArtifactKey::PackageOutput { .. } => self.evict_matching(&self.package_output, key),
            ArtifactKey::ModuleLinted { .. } => self.evict_matching(&self.module_linted, key),
            ArtifactKey::PackageLinted { .. } => self.evict_matching(&self.package_linted, key),
            ArtifactKey::WorkspaceLinted => self.evict_matching(&self.workspace_linted, key),
        }
    }

    /// Retain one exact live artifact version.
    pub fn retain(&self, version: &ArtifactVersion) {
        if !self.exists(version) {
            return;
        }

        let mut retain_count = self.retained_versions.entry(*version).or_insert(0);
        *retain_count += 1;
    }

    /// Retain one exact live artifact version with RAII release on drop.
    pub fn pin(self: &Arc<Self>, version: &ArtifactVersion) -> Option<ArtifactPin> {
        if !self.exists(version) {
            return None;
        }

        self.retain(version);

        Some(ArtifactPin::new(Arc::clone(self), *version))
    }

    /// Release one exact live artifact version.
    pub fn release(&self, version: &ArtifactVersion) {
        let mut dropped_last_retain = false;

        if let Some(mut retain_count) = self.retained_versions.get_mut(version) {
            if *retain_count == 1 {
                dropped_last_retain = true;
            } else {
                *retain_count -= 1;
            }
        } else {
            return;
        }

        if !dropped_last_retain {
            return;
        }

        self.retained_versions.remove(version);

        if self.has_other_published_version(version) {
            self.evict(version);
        }
    }

    /// Store one canonical dependency proof list for one exact artifact version.
    pub fn publish_dependencies(
        &self,
        version: &ArtifactVersion,
        dependencies: Vec<ArtifactDependency>,
    ) {
        if !self.exists(version) {
            return;
        }

        let mut seen = HashSet::new();
        let dependencies = dependencies
            .into_iter()
            .filter(|dependency| seen.insert(dependency.version))
            .collect();
        let mut entry = self.entry_mut(*version);
        entry.dependencies = dependencies;
    }

    /// Publish diagnostics for one exact artifact version.
    pub fn publish_diagnostics(&self, version: ArtifactVersion, diagnostics: DiagnosticCollection) {
        self.publish_entry(version);

        let mut entry = self.entry_mut(version);
        entry.diagnostics = Arc::new(diagnostics);
    }

    /// Publish one failed exact artifact attempt.
    pub fn publish_failure(&self, version: ArtifactVersion, dependencies: Vec<ArtifactDependency>) {
        self.publish_entry(version);
        self.publish_dependencies(&version, dependencies);
    }

    /// Return the recorded dependency proofs for one exact artifact version.
    pub fn dependencies(&self, version: &ArtifactVersion) -> Option<Vec<ArtifactDependency>> {
        self.entries
            .get(version)
            .map(|entry| entry.dependencies.clone())
    }

    /// Return the recorded diagnostics for one exact artifact version.
    pub fn diagnostics(&self, version: &ArtifactVersion) -> Option<Arc<DiagnosticCollection>> {
        self.entries
            .get(version)
            .map(|entry| Arc::clone(&entry.diagnostics))
    }

    /// Return whether one exact artifact version entry exists.
    pub fn exists(&self, version: &ArtifactVersion) -> bool {
        self.entries.contains_key(version)
    }

    /// Return the exact availability status for one artifact version.
    pub fn status(&self, version: &ArtifactVersion) -> ArtifactStatus {
        // published payload
        if self.contains(version) {
            return ArtifactStatus::Ready;
        }

        // failed attempt
        if self.exists(version) {
            return ArtifactStatus::Failed;
        }

        ArtifactStatus::Missing
    }

    /// Return whether one exact artifact payload is published.
    pub fn contains(&self, version: &ArtifactVersion) -> bool {
        match &version.key {
            ArtifactKey::ModuleGraph { .. } => self.module_graphs.contains_key(version),
            ArtifactKey::LanguageEnvironment { .. } => {
                self.language_environments.contains_key(version)
            }
            ArtifactKey::IntrinsicEnvironment { .. } => {
                self.intrinsic_environments.contains_key(version)
            }
            ArtifactKey::LibraryEnvironment { .. } => self.lib_environments.contains_key(version),
            ArtifactKey::Ast { .. } => self.asts.contains_key(version),
            ArtifactKey::Data { .. } => self.datas.contains_key(version),
            ArtifactKey::DirBase { .. } => self.dir_bases.contains_key(version),
            ArtifactKey::DirPrepared { .. } => self.dir_prepared.contains_key(version),
            ArtifactKey::DirResolved { .. } => self.dir_resolved.contains_key(version),
            ArtifactKey::DirDeclared { .. } => self.dir_declared.contains_key(version),
            ArtifactKey::DirInterface { .. } => self.dir_interface.contains_key(version),
            ArtifactKey::DirAnalyzed { .. } => self.dir_analyzed.contains_key(version),
            ArtifactKey::DirElaborated { .. } => self.dir_elaborated.contains_key(version),
            ArtifactKey::DirPatched { .. } => self.dir_patched.contains_key(version),
            ArtifactKey::MirBase { .. } => self.mir_bases.contains_key(version),
            ArtifactKey::MirOptimized { .. } => self.mir_optimized.contains_key(version),
            ArtifactKey::ModuleOutput { .. } => self.module_output.contains_key(version),
            ArtifactKey::PackageOutput { .. } => self.package_output.contains_key(version),
            ArtifactKey::ModuleLinted { .. } => self.module_linted.contains_key(version),
            ArtifactKey::PackageLinted { .. } => self.package_linted.contains_key(version),
            ArtifactKey::WorkspaceLinted => self.workspace_linted.contains_key(version),
        }
    }

    /// Publish one module graph at one exact version.
    pub fn publish_module_graph(
        &self,
        version: ArtifactVersion,
        payload: impl Into<Arc<ModuleGraph>>,
    ) {
        let payload = payload.into();
        let is_expected_key = matches!(&version.key, ArtifactKey::ModuleGraph { .. });

        self.publish(
            &self.module_graphs,
            version,
            payload,
            is_expected_key,
            "ModuleGraph",
        );
    }

    /// Publish one language environment at one exact version.
    pub fn publish_language_environment(
        &self,
        version: ArtifactVersion,
        payload: impl Into<Arc<LanguageEnvironment>>,
    ) {
        let payload = payload.into();
        let is_expected_key = matches!(&version.key, ArtifactKey::LanguageEnvironment { .. });

        self.publish(
            &self.language_environments,
            version,
            payload,
            is_expected_key,
            "LanguageEnvironment",
        );
    }

    /// Publish one intrinsic environment at one exact version.
    pub fn publish_intrinsic_environment(
        &self,
        version: ArtifactVersion,
        payload: impl Into<Arc<IntrinsicEnvironment>>,
    ) {
        let payload = payload.into();
        let is_expected_key = matches!(&version.key, ArtifactKey::IntrinsicEnvironment { .. });

        self.publish(
            &self.intrinsic_environments,
            version,
            payload,
            is_expected_key,
            "IntrinsicEnvironment",
        );
    }

    /// Publish one library environment at one exact version.
    pub fn publish_library_environment(
        &self,
        version: ArtifactVersion,
        payload: impl Into<Arc<LibraryEnvironment>>,
    ) {
        let payload = payload.into();
        let is_expected_key = matches!(&version.key, ArtifactKey::LibraryEnvironment { .. });

        self.publish(
            &self.lib_environments,
            version,
            payload,
            is_expected_key,
            "LibraryEnvironment",
        );
    }

    /// Publish one AST at one exact version.
    pub fn publish_ast(&self, version: ArtifactVersion, payload: impl Into<Arc<Ast>>) {
        let payload = payload.into();
        let is_expected_key = matches!(&version.key, ArtifactKey::Ast { .. });

        self.publish(&self.asts, version, payload, is_expected_key, "Ast");
    }

    /// Publish one data payload at one exact version.
    pub fn publish_data(&self, version: ArtifactVersion, payload: impl Into<Arc<Data>>) {
        let payload = payload.into();
        let is_expected_key = matches!(&version.key, ArtifactKey::Data { .. });

        self.publish(&self.datas, version, payload, is_expected_key, "Data");
    }

    /// Publish one base DIR at one exact version.
    pub fn publish_dir_base(&self, version: ArtifactVersion, payload: impl Into<Arc<DirBase>>) {
        let payload = payload.into();
        let is_expected_key = matches!(&version.key, ArtifactKey::DirBase { .. });

        self.publish(
            &self.dir_bases,
            version,
            payload,
            is_expected_key,
            "DirBase",
        );
    }

    /// Publish one prepared DIR at one exact version.
    pub fn publish_dir_prepared(
        &self,
        version: ArtifactVersion,
        payload: impl Into<Arc<DirPrepared>>,
    ) {
        let payload = payload.into();
        let is_expected_key = matches!(&version.key, ArtifactKey::DirPrepared { .. });

        self.publish(
            &self.dir_prepared,
            version,
            payload,
            is_expected_key,
            "DirPrepared",
        );
    }

    /// Publish one resolved DIR at one exact version.
    pub fn publish_dir_resolved(
        &self,
        version: ArtifactVersion,
        payload: impl Into<Arc<DirResolved>>,
    ) {
        let payload = payload.into();
        let is_expected_key = matches!(&version.key, ArtifactKey::DirResolved { .. });

        self.publish(
            &self.dir_resolved,
            version,
            payload,
            is_expected_key,
            "DirResolved",
        );
    }

    /// Publish one declared DIR at one exact version.
    pub fn publish_dir_declared(
        &self,
        version: ArtifactVersion,
        payload: impl Into<Arc<DirDeclared>>,
    ) {
        let payload = payload.into();
        let is_expected_key = matches!(&version.key, ArtifactKey::DirDeclared { .. });

        self.publish(
            &self.dir_declared,
            version,
            payload,
            is_expected_key,
            "DirDeclared",
        );
    }

    /// Publish one interface DIR at one exact version.
    pub fn publish_dir_interface(
        &self,
        version: ArtifactVersion,
        payload: impl Into<Arc<DirInterface>>,
    ) {
        let payload = payload.into();
        let is_expected_key = matches!(&version.key, ArtifactKey::DirInterface { .. });

        self.publish(
            &self.dir_interface,
            version,
            payload,
            is_expected_key,
            "DirInterface",
        );
    }

    /// Publish one analyzed DIR at one exact version.
    pub fn publish_dir_analyzed(
        &self,
        version: ArtifactVersion,
        payload: impl Into<Arc<DirAnalyzed>>,
    ) {
        let payload = payload.into();
        let is_expected_key = matches!(&version.key, ArtifactKey::DirAnalyzed { .. });

        self.publish(
            &self.dir_analyzed,
            version,
            payload,
            is_expected_key,
            "DirAnalyzed",
        );
    }

    /// Publish one elaborated DIR at one exact version.
    pub fn publish_dir_elaborated(
        &self,
        version: ArtifactVersion,
        payload: impl Into<Arc<DirElaborated>>,
    ) {
        let payload = payload.into();
        let is_expected_key = matches!(&version.key, ArtifactKey::DirElaborated { .. });

        self.publish(
            &self.dir_elaborated,
            version,
            payload,
            is_expected_key,
            "DirElaborated",
        );
    }

    /// Publish one patched DIR at one exact version.
    pub fn publish_dir_patched(
        &self,
        version: ArtifactVersion,
        payload: impl Into<Arc<DirPatched>>,
    ) {
        let payload = payload.into();
        let is_expected_key = matches!(&version.key, ArtifactKey::DirPatched { .. });

        self.publish(
            &self.dir_patched,
            version,
            payload,
            is_expected_key,
            "DirPatched",
        );
    }

    /// Publish one base MIR at one exact version.
    pub fn publish_mir_base(&self, version: ArtifactVersion, payload: impl Into<Arc<MirBase>>) {
        let payload = payload.into();
        let is_expected_key = matches!(&version.key, ArtifactKey::MirBase { .. });

        self.publish(
            &self.mir_bases,
            version,
            payload,
            is_expected_key,
            "MirBase",
        );
    }

    /// Publish one optimized MIR at one exact version.
    pub fn publish_mir_optimized(
        &self,
        version: ArtifactVersion,
        payload: impl Into<Arc<MirOptimized>>,
    ) {
        let payload = payload.into();
        let is_expected_key = matches!(&version.key, ArtifactKey::MirOptimized { .. });

        self.publish(
            &self.mir_optimized,
            version,
            payload,
            is_expected_key,
            "MirOptimized",
        );
    }

    /// Publish one module output at one exact version.
    pub fn publish_module_output(
        &self,
        version: ArtifactVersion,
        payload: impl Into<Arc<ModuleOutput>>,
    ) {
        let payload = payload.into();
        let is_expected_key = matches!(&version.key, ArtifactKey::ModuleOutput { .. });

        self.publish(
            &self.module_output,
            version,
            payload,
            is_expected_key,
            "ModuleOutput",
        );
    }

    /// Publish one package output at one exact version.
    pub fn publish_package_output(
        &self,
        version: ArtifactVersion,
        payload: impl Into<Arc<PackageOutput>>,
    ) {
        let payload = payload.into();
        let is_expected_key = matches!(&version.key, ArtifactKey::PackageOutput { .. });

        self.publish(
            &self.package_output,
            version,
            payload,
            is_expected_key,
            "PackageOutput",
        );
    }

    /// Publish one module lint surface at one exact version.
    pub fn publish_module_linted(
        &self,
        version: ArtifactVersion,
        payload: impl Into<Arc<ModuleLinted>>,
    ) {
        let payload = payload.into();
        let is_expected_key = matches!(&version.key, ArtifactKey::ModuleLinted { .. });

        self.publish(
            &self.module_linted,
            version,
            payload,
            is_expected_key,
            "ModuleLinted",
        );
    }

    /// Publish one package lint surface at one exact version.
    pub fn publish_package_linted(
        &self,
        version: ArtifactVersion,
        payload: impl Into<Arc<PackageLinted>>,
    ) {
        let payload = payload.into();
        let is_expected_key = matches!(&version.key, ArtifactKey::PackageLinted { .. });

        self.publish(
            &self.package_linted,
            version,
            payload,
            is_expected_key,
            "PackageLinted",
        );
    }

    /// Publish one workspace lint surface at one exact version.
    pub fn publish_workspace_linted(
        &self,
        version: ArtifactVersion,
        payload: impl Into<Arc<WorkspaceLinted>>,
    ) {
        let payload = payload.into();
        let is_expected_key = matches!(&version.key, ArtifactKey::WorkspaceLinted);

        self.publish(
            &self.workspace_linted,
            version,
            payload,
            is_expected_key,
            "WorkspaceLinted",
        );
    }

    /// Get one module graph.
    pub fn module_graph(&self, version: &ArtifactVersion) -> Option<Arc<ModuleGraph>> {
        self.module_graphs
            .get(version)
            .map(|entry| entry.value().clone())
    }

    /// Get one language environment.
    pub fn language_environment(
        &self,
        version: &ArtifactVersion,
    ) -> Option<Arc<LanguageEnvironment>> {
        self.language_environments
            .get(version)
            .map(|entry| entry.value().clone())
    }

    /// Get one intrinsic environment.
    pub fn intrinsic_environment(
        &self,
        version: &ArtifactVersion,
    ) -> Option<Arc<IntrinsicEnvironment>> {
        self.intrinsic_environments
            .get(version)
            .map(|entry| entry.value().clone())
    }

    /// Get one library environment.
    pub fn library_environment(
        &self,
        version: &ArtifactVersion,
    ) -> Option<Arc<LibraryEnvironment>> {
        self.lib_environments
            .get(version)
            .map(|entry| entry.value().clone())
    }

    /// Get one AST artifact.
    pub fn ast(&self, version: &ArtifactVersion) -> Option<Arc<Ast>> {
        self.asts.get(version).map(|entry| entry.value().clone())
    }

    /// Get one data artifact.
    pub fn data(&self, version: &ArtifactVersion) -> Option<Arc<Data>> {
        self.datas.get(version).map(|entry| entry.value().clone())
    }

    /// Get one base DIR artifact.
    pub fn dir_base(&self, version: &ArtifactVersion) -> Option<Arc<DirBase>> {
        self.dir_bases
            .get(version)
            .map(|entry| entry.value().clone())
    }

    /// Get one prepared DIR artifact.
    pub fn dir_prepared(&self, version: &ArtifactVersion) -> Option<Arc<DirPrepared>> {
        self.dir_prepared
            .get(version)
            .map(|entry| entry.value().clone())
    }

    /// Get one resolved DIR artifact.
    pub fn dir_resolved(&self, version: &ArtifactVersion) -> Option<Arc<DirResolved>> {
        self.dir_resolved
            .get(version)
            .map(|entry| entry.value().clone())
    }

    /// Get one declared DIR artifact.
    pub fn dir_declared(&self, version: &ArtifactVersion) -> Option<Arc<DirDeclared>> {
        self.dir_declared
            .get(version)
            .map(|entry| entry.value().clone())
    }

    /// Get one interface DIR artifact.
    pub fn dir_interface(&self, version: &ArtifactVersion) -> Option<Arc<DirInterface>> {
        self.dir_interface
            .get(version)
            .map(|entry| entry.value().clone())
    }

    /// Get one analyzed DIR artifact.
    pub fn dir_analyzed(&self, version: &ArtifactVersion) -> Option<Arc<DirAnalyzed>> {
        self.dir_analyzed
            .get(version)
            .map(|entry| entry.value().clone())
    }

    /// Get one elaborated DIR artifact.
    pub fn dir_elaborated(&self, version: &ArtifactVersion) -> Option<Arc<DirElaborated>> {
        self.dir_elaborated
            .get(version)
            .map(|entry| entry.value().clone())
    }

    /// Get one patched DIR artifact.
    pub fn dir_patched(&self, version: &ArtifactVersion) -> Option<Arc<DirPatched>> {
        self.dir_patched
            .get(version)
            .map(|entry| entry.value().clone())
    }

    /// Get one base MIR artifact.
    pub fn mir_base(&self, version: &ArtifactVersion) -> Option<Arc<MirBase>> {
        self.mir_bases
            .get(version)
            .map(|entry| entry.value().clone())
    }

    /// Get one optimized MIR artifact.
    pub fn mir_optimized(&self, version: &ArtifactVersion) -> Option<Arc<MirOptimized>> {
        self.mir_optimized
            .get(version)
            .map(|entry| entry.value().clone())
    }

    /// Get one module artifact.
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

    /// Get one module lint surface.
    pub fn module_linted(&self, version: &ArtifactVersion) -> Option<Arc<ModuleLinted>> {
        self.module_linted
            .get(version)
            .map(|entry| entry.value().clone())
    }

    /// Get one package lint surface.
    pub fn package_linted(&self, version: &ArtifactVersion) -> Option<Arc<PackageLinted>> {
        self.package_linted
            .get(version)
            .map(|entry| entry.value().clone())
    }

    /// Get one workspace lint surface.
    pub fn workspace_linted(&self, version: &ArtifactVersion) -> Option<Arc<WorkspaceLinted>> {
        self.workspace_linted
            .get(version)
            .map(|entry| entry.value().clone())
    }

    /// Clear all semantic artifacts.
    pub fn clear(&self) {
        self.entries.clear();
        self.module_graphs.clear();
        self.language_environments.clear();
        self.intrinsic_environments.clear();
        self.lib_environments.clear();
        self.asts.clear();
        self.datas.clear();
        self.dir_bases.clear();
        self.dir_prepared.clear();
        self.dir_resolved.clear();
        self.dir_declared.clear();
        self.dir_interface.clear();
        self.dir_analyzed.clear();
        self.dir_elaborated.clear();
        self.dir_patched.clear();
        self.mir_bases.clear();
        self.mir_optimized.clear();
        self.module_output.clear();
        self.package_output.clear();
        self.module_linted.clear();
        self.package_linted.clear();
        self.workspace_linted.clear();
        self.retained_versions.clear();
    }

    /// Collect profiles for one module from one versioned family map.
    fn collect_profiles_for_module<T>(
        &self,
        map: &ArtifactMap<T>,
        module: ModuleId,
        profiles: &mut std::collections::HashSet<ProfileId>,
    ) {
        for version in map.iter().map(|entry| *entry.key()) {
            if version.module_id() == Some(module)
                && let Some(profile_id) = version.profile_id()
            {
                profiles.insert(profile_id);
            }
        }
    }

    /// Publish one family entry at one exact artifact version.
    fn insert<T>(&self, map: &ArtifactMap<T>, version: ArtifactVersion, payload: Arc<T>) {
        // keep the current exact version available by default
        self.publish_entry(version);
        map.insert(version, payload);
    }

    /// Evict every published version matching one semantic artifact key.
    fn evict_matching<T>(&self, map: &ArtifactMap<T>, key: &ArtifactKey) {
        let versions: Vec<_> = map
            .iter()
            .filter_map(|entry| (entry.key().key == *key).then_some(*entry.key()))
            .collect();

        for version in versions {
            self.evict(&version);
        }
    }

    /// Publish one exact artifact record without a typed payload.
    fn publish_entry(&self, version: ArtifactVersion) {
        self.entry_mut(version);

        // release alone controls older retained versions
        let superseded_versions = self
            .entries
            .iter()
            .filter_map(|entry| {
                let candidate = entry.key();
                if *candidate == version {
                    return None;
                }

                (candidate.key == version.key && !self.retained_versions.contains_key(candidate))
                    .then_some(*candidate)
            })
            .collect::<Vec<_>>();

        for superseded_version in superseded_versions {
            self.evict(&superseded_version);
        }
    }

    /// Return whether another exact version for this semantic key is still published.
    fn has_other_published_version(&self, version: &ArtifactVersion) -> bool {
        self.entries
            .iter()
            .any(|entry| entry.key().key == version.key && *entry.key() != *version)
    }

    /// Return the mutable exact entry for one version, creating it when missing.
    fn entry_mut(
        &self,
        version: ArtifactVersion,
    ) -> dashmap::mapref::one::RefMut<'_, ArtifactVersion, ArtifactEntry> {
        self.entries.entry(version).or_default()
    }
}

#[cfg(test)]
mod tests {
    use destack_source::ModuleId;

    use crate::{ArtifactKey, ArtifactStamp, ArtifactVersion, Ast};

    use super::ArtifactStore;

    /// Release one retained version after the last live owner drops it.
    #[test]
    fn test_release_evicts_unretained_exact_version() {
        let registry = ArtifactStore::new();
        let module = ModuleId::EPHEMERAL;
        let version = ArtifactVersion::new(ArtifactKey::Ast { module }, ArtifactStamp::new(1));

        // publish and retain one exact version twice
        registry.publish_ast(version, Ast::new(module));
        registry.retain(&version);
        registry.retain(&version);

        assert!(registry.contains(&version));

        // keep the payload until the last live owner releases it
        registry.release(&version);
        assert!(registry.contains(&version));

        registry.release(&version);
        assert!(!registry.contains(&version));
    }

    /// Evict one superseded exact version when no live owner still retains it.
    #[test]
    fn test_publish_evicts_superseded_unretained_version() {
        let registry = ArtifactStore::new();
        let module = ModuleId::EPHEMERAL;
        let version_1 = ArtifactVersion::new(ArtifactKey::Ast { module }, ArtifactStamp::new(1));
        let version_2 = ArtifactVersion::new(ArtifactKey::Ast { module }, ArtifactStamp::new(2));

        // superseded latest version
        registry.publish_ast(version_1, Ast::new(module));
        registry.publish_ast(version_2, Ast::new(module));

        assert!(!registry.contains(&version_1));
        assert!(registry.contains(&version_2));
    }

    /// Keep one superseded exact version while a live owner still retains it.
    #[test]
    fn test_publish_keeps_superseded_retained_version() {
        let registry = ArtifactStore::new();
        let module = ModuleId::EPHEMERAL;
        let version_1 = ArtifactVersion::new(ArtifactKey::Ast { module }, ArtifactStamp::new(1));
        let version_2 = ArtifactVersion::new(ArtifactKey::Ast { module }, ArtifactStamp::new(2));

        // retained older version
        registry.publish_ast(version_1, Ast::new(module));
        registry.retain(&version_1);
        registry.publish_ast(version_2, Ast::new(module));

        assert!(registry.contains(&version_1));
        assert!(registry.contains(&version_2));

        // release older version
        registry.release(&version_1);

        assert!(!registry.contains(&version_1));
        assert!(registry.contains(&version_2));
    }

    /// Keep the sole published version alive after the last exact retain is released.
    #[test]
    fn test_release_keeps_sole_retained_version() {
        let registry = ArtifactStore::new();
        let module = ModuleId::EPHEMERAL;
        let version = ArtifactVersion::new(ArtifactKey::Ast { module }, ArtifactStamp::new(1));

        // the sole version stays live by default
        registry.publish_ast(version, Ast::new(module));
        registry.retain(&version);

        assert!(registry.contains(&version));

        // releasing the last retain should not evict the current live version
        registry.release(&version);

        assert!(registry.contains(&version));
    }
}
