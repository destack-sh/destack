use std::sync::Arc;

use dashmap::DashMap;
use destack_ast as ast;
use destack_core::StringPool;
use destack_source::{FileId, ModuleId, ProfileId};

use crate::{
    ArtifactDependency, ArtifactKey, ArtifactVersion, Ast, DirAnalyzed, DirBase, DirDeclared,
    DirElaborated, DirInterface, DirPatched, DirPrepared, DirResolved, IntrinsicEnvironment,
    LanguageEnvironment, LibraryEnvironment, MirBase, MirOptimized, ModuleArtifact, ModuleGraph,
    PackageOutput,
};

/// One published artifact payload.
#[derive(Debug, Clone)]
pub enum ArtifactPayload {
    /// One module graph payload.
    ModuleGraph(Arc<ModuleGraph>),
    /// One language environment payload.
    LanguageEnvironment(Arc<LanguageEnvironment>),
    /// One intrinsic environment payload.
    IntrinsicEnvironment(Arc<IntrinsicEnvironment>),
    /// One library environment payload.
    LibraryEnvironment(Arc<LibraryEnvironment>),
    /// One AST payload.
    Ast(Arc<Ast>),
    /// One base DIR payload.
    DirBase(Arc<DirBase>),
    /// One prepared DIR payload.
    DirPrepared(Arc<DirPrepared>),
    /// One resolved DIR payload.
    DirResolved(Arc<DirResolved>),
    /// One declared DIR payload.
    DirDeclared(Arc<DirDeclared>),
    /// One interface DIR payload.
    DirInterface(Arc<DirInterface>),
    /// One analyzed DIR payload.
    DirAnalyzed(Arc<DirAnalyzed>),
    /// One elaborated DIR payload.
    DirElaborated(Arc<DirElaborated>),
    /// One patched DIR payload.
    DirPatched(Arc<DirPatched>),
    /// One base MIR payload.
    MirBase(Arc<MirBase>),
    /// One optimized MIR payload.
    MirOptimized(Arc<MirOptimized>),
    /// One module artifact payload.
    ModuleArtifact(Arc<ModuleArtifact>),
    /// One package output payload.
    PackageOutput(Arc<PackageOutput>),
}

impl From<LanguageEnvironment> for ArtifactPayload {
    fn from(value: LanguageEnvironment) -> Self {
        Self::LanguageEnvironment(Arc::new(value))
    }
}

impl From<ModuleGraph> for ArtifactPayload {
    fn from(value: ModuleGraph) -> Self {
        Self::ModuleGraph(Arc::new(value))
    }
}

impl From<Arc<ModuleGraph>> for ArtifactPayload {
    fn from(value: Arc<ModuleGraph>) -> Self {
        Self::ModuleGraph(value)
    }
}

impl From<Arc<LanguageEnvironment>> for ArtifactPayload {
    fn from(value: Arc<LanguageEnvironment>) -> Self {
        Self::LanguageEnvironment(value)
    }
}

impl From<IntrinsicEnvironment> for ArtifactPayload {
    fn from(value: IntrinsicEnvironment) -> Self {
        Self::IntrinsicEnvironment(Arc::new(value))
    }
}

impl From<Arc<IntrinsicEnvironment>> for ArtifactPayload {
    fn from(value: Arc<IntrinsicEnvironment>) -> Self {
        Self::IntrinsicEnvironment(value)
    }
}

impl From<LibraryEnvironment> for ArtifactPayload {
    fn from(value: LibraryEnvironment) -> Self {
        Self::LibraryEnvironment(Arc::new(value))
    }
}

impl From<Arc<LibraryEnvironment>> for ArtifactPayload {
    fn from(value: Arc<LibraryEnvironment>) -> Self {
        Self::LibraryEnvironment(value)
    }
}

impl From<Ast> for ArtifactPayload {
    fn from(value: Ast) -> Self {
        Self::Ast(Arc::new(value))
    }
}

impl From<Arc<Ast>> for ArtifactPayload {
    fn from(value: Arc<Ast>) -> Self {
        Self::Ast(value)
    }
}

impl From<DirBase> for ArtifactPayload {
    fn from(value: DirBase) -> Self {
        Self::DirBase(Arc::new(value))
    }
}

impl From<Arc<DirBase>> for ArtifactPayload {
    fn from(value: Arc<DirBase>) -> Self {
        Self::DirBase(value)
    }
}

impl From<DirPrepared> for ArtifactPayload {
    fn from(value: DirPrepared) -> Self {
        Self::DirPrepared(Arc::new(value))
    }
}

impl From<Arc<DirPrepared>> for ArtifactPayload {
    fn from(value: Arc<DirPrepared>) -> Self {
        Self::DirPrepared(value)
    }
}

impl From<DirResolved> for ArtifactPayload {
    fn from(value: DirResolved) -> Self {
        Self::DirResolved(Arc::new(value))
    }
}

impl From<Arc<DirResolved>> for ArtifactPayload {
    fn from(value: Arc<DirResolved>) -> Self {
        Self::DirResolved(value)
    }
}

impl From<DirDeclared> for ArtifactPayload {
    fn from(value: DirDeclared) -> Self {
        Self::DirDeclared(Arc::new(value))
    }
}

impl From<Arc<DirDeclared>> for ArtifactPayload {
    fn from(value: Arc<DirDeclared>) -> Self {
        Self::DirDeclared(value)
    }
}

impl From<DirInterface> for ArtifactPayload {
    fn from(value: DirInterface) -> Self {
        Self::DirInterface(Arc::new(value))
    }
}

impl From<Arc<DirInterface>> for ArtifactPayload {
    fn from(value: Arc<DirInterface>) -> Self {
        Self::DirInterface(value)
    }
}

impl From<DirAnalyzed> for ArtifactPayload {
    fn from(value: DirAnalyzed) -> Self {
        Self::DirAnalyzed(Arc::new(value))
    }
}

impl From<Arc<DirAnalyzed>> for ArtifactPayload {
    fn from(value: Arc<DirAnalyzed>) -> Self {
        Self::DirAnalyzed(value)
    }
}

impl From<DirElaborated> for ArtifactPayload {
    fn from(value: DirElaborated) -> Self {
        Self::DirElaborated(Arc::new(value))
    }
}

impl From<Arc<DirElaborated>> for ArtifactPayload {
    fn from(value: Arc<DirElaborated>) -> Self {
        Self::DirElaborated(value)
    }
}

impl From<DirPatched> for ArtifactPayload {
    fn from(value: DirPatched) -> Self {
        Self::DirPatched(Arc::new(value))
    }
}

impl From<Arc<DirPatched>> for ArtifactPayload {
    fn from(value: Arc<DirPatched>) -> Self {
        Self::DirPatched(value)
    }
}

impl From<MirBase> for ArtifactPayload {
    fn from(value: MirBase) -> Self {
        Self::MirBase(Arc::new(value))
    }
}

impl From<Arc<MirBase>> for ArtifactPayload {
    fn from(value: Arc<MirBase>) -> Self {
        Self::MirBase(value)
    }
}

impl From<MirOptimized> for ArtifactPayload {
    fn from(value: MirOptimized) -> Self {
        Self::MirOptimized(Arc::new(value))
    }
}

impl From<Arc<MirOptimized>> for ArtifactPayload {
    fn from(value: Arc<MirOptimized>) -> Self {
        Self::MirOptimized(value)
    }
}

impl From<ModuleArtifact> for ArtifactPayload {
    fn from(value: ModuleArtifact) -> Self {
        Self::ModuleArtifact(Arc::new(value))
    }
}

impl From<Arc<ModuleArtifact>> for ArtifactPayload {
    fn from(value: Arc<ModuleArtifact>) -> Self {
        Self::ModuleArtifact(value)
    }
}

impl From<PackageOutput> for ArtifactPayload {
    fn from(value: PackageOutput) -> Self {
        Self::PackageOutput(Arc::new(value))
    }
}

impl From<Arc<PackageOutput>> for ArtifactPayload {
    fn from(value: Arc<PackageOutput>) -> Self {
        Self::PackageOutput(value)
    }
}

/// Store of published semantic artifacts.
#[derive(Debug, Default)]
pub struct ArtifactStore {
    /// Module dependency graphs by profile.
    module_graphs: DashMap<ArtifactVersion, Arc<ModuleGraph>>,
    /// Language environments by profile.
    language_environments: DashMap<ArtifactVersion, Arc<LanguageEnvironment>>,
    /// Intrinsic environments by profile.
    intrinsic_environments: DashMap<ArtifactVersion, Arc<IntrinsicEnvironment>>,
    /// Library environments by profile.
    lib_environments: DashMap<ArtifactVersion, Arc<LibraryEnvironment>>,

    /// AST artifacts by module.
    asts: DashMap<ArtifactVersion, Arc<Ast>>,

    /// Base DIR artifacts by module.
    dir_bases: DashMap<ArtifactVersion, Arc<DirBase>>,
    /// Prepared DIR artifacts by module and profile.
    dir_prepared: DashMap<ArtifactVersion, Arc<DirPrepared>>,
    /// Resolved DIR artifacts by module and profile.
    dir_resolved: DashMap<ArtifactVersion, Arc<DirResolved>>,
    /// Declared DIR artifacts by module and profile.
    dir_declared: DashMap<ArtifactVersion, Arc<DirDeclared>>,
    /// Interface DIR artifacts by module and profile.
    dir_interface: DashMap<ArtifactVersion, Arc<DirInterface>>,
    /// Analyzed DIR artifacts by module and profile.
    dir_analyzed: DashMap<ArtifactVersion, Arc<DirAnalyzed>>,
    /// Elaborated DIR artifacts by module and profile.
    dir_elaborated: DashMap<ArtifactVersion, Arc<DirElaborated>>,
    /// Patched DIR artifacts by module and profile.
    dir_patched: DashMap<ArtifactVersion, Arc<DirPatched>>,

    /// Base MIR artifacts by module, profile, and target.
    mir_bases: DashMap<ArtifactVersion, Arc<MirBase>>,
    /// Optimized MIR artifacts by module, profile, and target.
    mir_optimized: DashMap<ArtifactVersion, Arc<MirOptimized>>,

    /// Generated module artifacts by module and target.
    module_artifact: DashMap<ArtifactVersion, Arc<ModuleArtifact>>,
    /// Output entries by package and target.
    package_output: DashMap<ArtifactVersion, Arc<PackageOutput>>,
}

impl ArtifactStore {
    /// Create a new semantic artifact store.
    pub fn new() -> Self {
        Self {
            module_graphs: DashMap::new(),
            language_environments: DashMap::new(),
            intrinsic_environments: DashMap::new(),
            lib_environments: DashMap::new(),
            asts: DashMap::new(),
            dir_bases: DashMap::new(),
            dir_prepared: DashMap::new(),
            dir_resolved: DashMap::new(),
            dir_declared: DashMap::new(),
            dir_interface: DashMap::new(),
            dir_analyzed: DashMap::new(),
            dir_elaborated: DashMap::new(),
            dir_patched: DashMap::new(),
            mir_bases: DashMap::new(),
            mir_optimized: DashMap::new(),
            module_artifact: DashMap::new(),
            package_output: DashMap::new(),
        }
    }

    /// Publish the synthetic root AST for one program.
    pub fn publish_root_ast(&self, root_module_id: ModuleId, fallback_file_id: FileId) {
        let root_ast = ast::NodeTree::new();
        let mut root_module_ast = Ast::from_tree(
            root_module_id,
            root_ast,
            Vec::new(),
            StringPool::new(),
            Vec::new(),
            Vec::new(),
        );
        root_module_ast.ensure_anchor_expression(fallback_file_id);
        self.publish(
            ArtifactVersion::new(
                ArtifactKey::Ast {
                    module: root_module_id,
                },
                ArtifactDependency::new(0),
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

        profiles
    }

    /// Evict one published artifact version.
    pub fn evict(&self, version: &ArtifactVersion) {
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
            ArtifactKey::ModuleArtifact { .. } => {
                self.module_artifact.remove(version);
            }
            ArtifactKey::PackageOutput { .. } => {
                self.package_output.remove(version);
            }
        }
    }

    /// Evict all published module graph versions for one profile.
    pub fn evict_module_graphs(&self, profile: ProfileId) {
        let versions: Vec<_> = self
            .module_graphs
            .iter()
            .filter_map(|entry| {
                let version = entry.key();
                let ArtifactKey::ModuleGraph {
                    profile: entry_profile,
                } = &version.key
                else {
                    return None;
                };

                (*entry_profile == profile).then_some(version.clone())
            })
            .collect();

        for version in versions {
            self.module_graphs.remove(&version);
        }
    }

    /// Return whether one exact artifact version is published.
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
            ArtifactKey::ModuleArtifact { .. } => self.module_artifact.contains_key(version),
            ArtifactKey::PackageOutput { .. } => self.package_output.contains_key(version),
        }
    }

    /// Publish one semantic artifact at one exact version.
    pub fn publish(&self, version: ArtifactVersion, artifact: impl Into<ArtifactPayload>) {
        let artifact = artifact.into();

        match (&version.key, artifact) {
            (ArtifactKey::ModuleGraph { .. }, ArtifactPayload::ModuleGraph(module_graph)) => {
                self.module_graphs.insert(version, module_graph);
            }
            (
                ArtifactKey::LanguageEnvironment { .. },
                ArtifactPayload::LanguageEnvironment(environment),
            ) => {
                self.language_environments.insert(version, environment);
            }
            (
                ArtifactKey::IntrinsicEnvironment { .. },
                ArtifactPayload::IntrinsicEnvironment(environment),
            ) => {
                self.intrinsic_environments.insert(version, environment);
            }
            (
                ArtifactKey::LibraryEnvironment { .. },
                ArtifactPayload::LibraryEnvironment(environment),
            ) => {
                self.lib_environments.insert(version, environment);
            }
            (ArtifactKey::Ast { .. }, ArtifactPayload::Ast(ast)) => {
                self.asts.insert(version, ast);
            }
            (ArtifactKey::DirBase { .. }, ArtifactPayload::DirBase(dir)) => {
                self.dir_bases.insert(version, dir);
            }
            (ArtifactKey::DirPrepared { .. }, ArtifactPayload::DirPrepared(dir)) => {
                self.dir_prepared.insert(version, dir);
            }
            (ArtifactKey::DirResolved { .. }, ArtifactPayload::DirResolved(dir)) => {
                self.dir_resolved.insert(version, dir);
            }
            (ArtifactKey::DirDeclared { .. }, ArtifactPayload::DirDeclared(dir)) => {
                self.dir_declared.insert(version, dir);
            }
            (ArtifactKey::DirInterface { .. }, ArtifactPayload::DirInterface(dir)) => {
                self.dir_interface.insert(version, dir);
            }
            (ArtifactKey::DirAnalyzed { .. }, ArtifactPayload::DirAnalyzed(dir)) => {
                self.dir_analyzed.insert(version, dir);
            }
            (ArtifactKey::DirElaborated { .. }, ArtifactPayload::DirElaborated(dir)) => {
                self.dir_elaborated.insert(version, dir);
            }
            (ArtifactKey::DirPatched { .. }, ArtifactPayload::DirPatched(dir)) => {
                self.dir_patched.insert(version, dir);
            }
            (ArtifactKey::MirBase { .. }, ArtifactPayload::MirBase(mir)) => {
                self.mir_bases.insert(version, mir);
            }
            (ArtifactKey::MirOptimized { .. }, ArtifactPayload::MirOptimized(mir)) => {
                self.mir_optimized.insert(version, mir);
            }
            (ArtifactKey::ModuleArtifact { .. }, ArtifactPayload::ModuleArtifact(artifact)) => {
                self.module_artifact.insert(version, artifact);
            }
            (ArtifactKey::PackageOutput { .. }, ArtifactPayload::PackageOutput(emit)) => {
                self.package_output.insert(version, emit);
            }
            (key, artifact) => {
                panic!("artifact payload did not match key: key={key:?} artifact={artifact:?}");
            }
        }
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
    pub fn module_artifact(&self, version: &ArtifactVersion) -> Option<Arc<ModuleArtifact>> {
        self.module_artifact
            .get(version)
            .map(|entry| entry.value().clone())
    }

    /// Get one package output artifact.
    pub fn package_output(&self, version: &ArtifactVersion) -> Option<Arc<PackageOutput>> {
        self.package_output
            .get(version)
            .map(|entry| entry.value().clone())
    }

    /// Clear all semantic artifacts.
    pub fn clear(&self) {
        self.module_graphs.clear();
        self.language_environments.clear();
        self.intrinsic_environments.clear();
        self.lib_environments.clear();
        self.asts.clear();
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
        self.module_artifact.clear();
        self.package_output.clear();
    }

    /// Collect profiles for one module from one versioned family map.
    fn collect_profiles_for_module<T>(
        &self,
        map: &DashMap<ArtifactVersion, Arc<T>>,
        module: ModuleId,
        profiles: &mut std::collections::HashSet<ProfileId>,
    ) {
        for version in map.iter().map(|entry| entry.key().clone()) {
            if version.module_id() == Some(module)
                && let Some(profile_id) = version.profile_id()
            {
                profiles.insert(profile_id);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use destack_source::{ModuleId, ProfileId};

    use crate::{
        ArtifactDependency, ArtifactKey, ArtifactVersion, BinaryArtifact, ModuleArtifact,
        ModuleGraph, ObjectArtifact, PackageOutput,
    };

    use super::ArtifactStore;

    /// Clear every published artifact family from the registry.
    #[test]
    fn test_clear_removes_all_published_artifact_families() {
        let registry = ArtifactStore::new();
        let profile = ProfileId::new(1);
        let module = ModuleId::EPHEMERAL;
        let package = destack_source::PackageId::EPHEMERAL;
        let target = destack_source::TargetId::new(package, "test");

        // seed a representative sample of graph and output artifacts
        registry.publish(
            ArtifactVersion::new(
                ArtifactKey::module_graph(profile),
                ArtifactDependency::new(1),
            ),
            ModuleGraph::new(profile),
        );
        registry.publish(
            ArtifactVersion::new(
                ArtifactKey::ModuleArtifact { module, target },
                ArtifactDependency::new(2),
            ),
            ModuleArtifact::Binary(Box::new(BinaryArtifact::Object(Box::new(ObjectArtifact {
                bytes: Vec::new().into(),
                debug: Vec::new(),
            })))),
        );
        registry.publish(
            ArtifactVersion::new(
                ArtifactKey::PackageOutput { package, target },
                ArtifactDependency::new(3),
            ),
            PackageOutput::default(),
        );

        // clear the full registry state
        registry.clear();

        // the registry should not retain stale live payloads after clear
        let graph_version = ArtifactVersion::new(
            ArtifactKey::module_graph(profile),
            ArtifactDependency::new(1),
        );
        let module_version = ArtifactVersion::new(
            ArtifactKey::ModuleArtifact { module, target },
            ArtifactDependency::new(2),
        );
        let package_version = ArtifactVersion::new(
            ArtifactKey::PackageOutput { package, target },
            ArtifactDependency::new(3),
        );

        assert!(registry.module_graph(&graph_version).is_none());
        assert!(registry.module_artifact(&module_version).is_none());
        assert!(registry.package_output(&package_version).is_none());
    }
}
