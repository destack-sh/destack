use std::collections::HashSet;
use std::sync::Arc;

use dashmap::DashMap;
use destack_source::ModuleId;

use crate::{
    ArtifactDependency, ArtifactKey, Ast, DirAnalyzed, DirBase, DirDeclared, DirElaborated,
    DirInterface, DirPatched, DirPrepared, DirResolved, MirBase, MirOptimized, ModuleGraph,
    ProfileId, TargetId,
};

use super::{IntrinsicEnvironment, LanguageEnvironment, LibEnvironment};

/// One published artifact payload.
#[derive(Debug, Clone)]
pub enum ArtifactPayload {
    /// One module graph payload.
    ModuleGraph(Arc<ModuleGraph>),
    /// One language environment payload.
    LanguageEnvironment(Arc<LanguageEnvironment>),
    /// One intrinsic environment payload.
    IntrinsicEnvironment(Arc<IntrinsicEnvironment>),
    /// One lib environment payload.
    LibEnvironment(Arc<LibEnvironment>),
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

impl From<LibEnvironment> for ArtifactPayload {
    fn from(value: LibEnvironment) -> Self {
        Self::LibEnvironment(Arc::new(value))
    }
}

impl From<Arc<LibEnvironment>> for ArtifactPayload {
    fn from(value: Arc<LibEnvironment>) -> Self {
        Self::LibEnvironment(value)
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

/// Registry of published semantic artifacts.
#[derive(Debug, Default)]
pub struct ArtifactRegistry {
    /// Dependency stamps by artifact key.
    dependencies: DashMap<ArtifactKey, ArtifactDependency>,
    /// Module dependency graphs by profile.
    module_graphs: DashMap<ProfileId, Arc<ModuleGraph>>,
    /// Language environments by profile.
    language_environments: DashMap<ProfileId, Arc<LanguageEnvironment>>,
    /// Intrinsic environments by profile.
    intrinsic_environments: DashMap<ProfileId, Arc<IntrinsicEnvironment>>,
    /// Lib environments by profile.
    lib_environments: DashMap<ProfileId, Arc<LibEnvironment>>,
    /// AST artifacts by module.
    asts: DashMap<ModuleId, Arc<Ast>>,
    /// Base DIR artifacts by module.
    dir_bases: DashMap<ModuleId, Arc<DirBase>>,
    /// Prepared DIR artifacts by module and profile.
    dir_prepared: DashMap<(ModuleId, ProfileId), Arc<DirPrepared>>,
    /// Resolved DIR artifacts by module and profile.
    dir_resolved: DashMap<(ModuleId, ProfileId), Arc<DirResolved>>,
    /// Declared DIR artifacts by module and profile.
    dir_declared: DashMap<(ModuleId, ProfileId), Arc<DirDeclared>>,
    /// Interface DIR artifacts by module and profile.
    dir_interface: DashMap<(ModuleId, ProfileId), Arc<DirInterface>>,
    /// Analyzed DIR artifacts by module and profile.
    dir_analyzed: DashMap<(ModuleId, ProfileId), Arc<DirAnalyzed>>,
    /// Elaborated DIR artifacts by module and profile.
    dir_elaborated: DashMap<(ModuleId, ProfileId), Arc<DirElaborated>>,
    /// Patched DIR artifacts by module and profile.
    dir_patched: DashMap<(ModuleId, ProfileId), Arc<DirPatched>>,
    /// Base MIR artifacts by module, profile, and target.
    mir_bases: DashMap<(ModuleId, ProfileId, TargetId), Arc<MirBase>>,
    /// Optimized MIR artifacts by module, profile, and target.
    mir_optimized: DashMap<(ModuleId, ProfileId, TargetId), Arc<MirOptimized>>,
}

impl ArtifactRegistry {
    /// Collect profile ids from one module/profile map.
    fn collect_profile_ids<T>(
        &self,
        profiles: &mut HashSet<ProfileId>,
        entries: &DashMap<(ModuleId, ProfileId), Arc<T>>,
        module: ModuleId,
    ) {
        for entry in entries.iter() {
            let (entry_module, profile) = *entry.key();
            if entry_module == module {
                profiles.insert(profile);
            }
        }
    }

    /// Collect module ids from one module/profile map.
    fn collect_module_ids<T>(
        &self,
        modules: &mut HashSet<ModuleId>,
        entries: &DashMap<(ModuleId, ProfileId), Arc<T>>,
        profiles: &HashSet<ProfileId>,
    ) {
        for entry in entries.iter() {
            let (module, profile) = *entry.key();
            if profiles.contains(&profile) {
                modules.insert(module);
            }
        }
    }

    /// Create a new semantic artifact registry.
    pub fn new() -> Self {
        Self {
            dependencies: DashMap::new(),
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
        }
    }

    /// Get the dependency stamp for one artifact key.
    pub fn dependency(&self, key: &ArtifactKey) -> Option<ArtifactDependency> {
        self.dependencies
            .get(key)
            .map(|dependency| *dependency.value())
    }

    /// Insert one dependency stamp for one artifact key.
    pub fn set_dependency(&self, key: ArtifactKey, dependency: ArtifactDependency) {
        self.dependencies.insert(key, dependency);
    }

    /// Invalidate one published artifact and its dependency stamp.
    pub fn invalidate(&self, key: &ArtifactKey) {
        match key {
            ArtifactKey::ModuleGraph { profile } => {
                self.module_graphs.remove(profile);
            }
            ArtifactKey::LanguageEnvironment { profile } => {
                self.language_environments.remove(profile);
            }
            ArtifactKey::IntrinsicEnvironment { profile } => {
                self.intrinsic_environments.remove(profile);
            }
            ArtifactKey::LibEnvironment { profile } => {
                self.lib_environments.remove(profile);
            }
            ArtifactKey::Ast { module } => {
                self.asts.remove(module);
            }
            ArtifactKey::DirBase { module } => {
                self.dir_bases.remove(module);
            }
            ArtifactKey::DirPrepared { module, profile } => {
                self.dir_prepared.remove(&(*module, *profile));
            }
            ArtifactKey::DirResolved { module, profile } => {
                self.dir_resolved.remove(&(*module, *profile));
            }
            ArtifactKey::DirDeclared { module, profile } => {
                self.dir_declared.remove(&(*module, *profile));
            }
            ArtifactKey::DirInterface { module, profile } => {
                self.dir_interface.remove(&(*module, *profile));
            }
            ArtifactKey::DirAnalyzed { module, profile } => {
                self.dir_analyzed.remove(&(*module, *profile));
            }
            ArtifactKey::DirElaborated { module, profile } => {
                self.dir_elaborated.remove(&(*module, *profile));
            }
            ArtifactKey::DirPatched { module, profile } => {
                self.dir_patched.remove(&(*module, *profile));
            }
            ArtifactKey::MirBase {
                module,
                profile,
                target,
            } => {
                self.mir_bases.remove(&(*module, *profile, target.clone()));
            }
            ArtifactKey::MirOptimized {
                module,
                profile,
                target,
            } => {
                self.mir_optimized
                    .remove(&(*module, *profile, target.clone()));
            }
        }

        self.dependencies.remove(key);
    }

    /// Publish one semantic artifact at one exact key.
    pub fn publish(&self, key: ArtifactKey, artifact: impl Into<ArtifactPayload>) {
        let artifact = artifact.into();

        match (key, artifact) {
            (ArtifactKey::ModuleGraph { profile }, ArtifactPayload::ModuleGraph(module_graph)) => {
                self.module_graphs.insert(profile, module_graph);
            }
            (
                ArtifactKey::LanguageEnvironment { profile },
                ArtifactPayload::LanguageEnvironment(environment),
            ) => {
                self.language_environments.insert(profile, environment);
            }
            (
                ArtifactKey::IntrinsicEnvironment { profile },
                ArtifactPayload::IntrinsicEnvironment(environment),
            ) => {
                self.intrinsic_environments.insert(profile, environment);
            }
            (
                ArtifactKey::LibEnvironment { profile },
                ArtifactPayload::LibEnvironment(environment),
            ) => {
                self.lib_environments.insert(profile, environment);
            }
            (ArtifactKey::Ast { module }, ArtifactPayload::Ast(ast)) => {
                self.asts.insert(module, ast);
            }
            (ArtifactKey::DirBase { module }, ArtifactPayload::DirBase(dir)) => {
                self.dir_bases.insert(module, dir);
            }
            (ArtifactKey::DirPrepared { module, profile }, ArtifactPayload::DirPrepared(dir)) => {
                self.dir_prepared.insert((module, profile), dir);
            }
            (ArtifactKey::DirResolved { module, profile }, ArtifactPayload::DirResolved(dir)) => {
                self.dir_resolved.insert((module, profile), dir);
            }
            (ArtifactKey::DirDeclared { module, profile }, ArtifactPayload::DirDeclared(dir)) => {
                self.dir_declared.insert((module, profile), dir);
            }
            (ArtifactKey::DirInterface { module, profile }, ArtifactPayload::DirInterface(dir)) => {
                self.dir_interface.insert((module, profile), dir);
            }
            (ArtifactKey::DirAnalyzed { module, profile }, ArtifactPayload::DirAnalyzed(dir)) => {
                self.dir_analyzed.insert((module, profile), dir);
            }
            (
                ArtifactKey::DirElaborated { module, profile },
                ArtifactPayload::DirElaborated(dir),
            ) => {
                self.dir_elaborated.insert((module, profile), dir);
            }
            (ArtifactKey::DirPatched { module, profile }, ArtifactPayload::DirPatched(dir)) => {
                self.dir_patched.insert((module, profile), dir);
            }
            (
                ArtifactKey::MirBase {
                    module,
                    profile,
                    target,
                },
                ArtifactPayload::MirBase(mir),
            ) => {
                self.mir_bases.insert((module, profile, target), mir);
            }
            (
                ArtifactKey::MirOptimized {
                    module,
                    profile,
                    target,
                },
                ArtifactPayload::MirOptimized(mir),
            ) => {
                self.mir_optimized.insert((module, profile, target), mir);
            }
            (key, artifact) => {
                panic!("artifact payload did not match key: key={key:?} artifact={artifact:?}");
            }
        }
    }

    /// Get one language environment.
    pub fn module_graph(&self, profile: ProfileId) -> Option<Arc<ModuleGraph>> {
        self.module_graphs
            .get(&profile)
            .map(|entry| entry.value().clone())
    }

    /// Get one language environment.
    pub fn language_environment(&self, profile: ProfileId) -> Option<Arc<LanguageEnvironment>> {
        self.language_environments
            .get(&profile)
            .map(|entry| entry.value().clone())
    }

    /// Get one intrinsic environment.
    pub fn intrinsic_environment(&self, profile: ProfileId) -> Option<Arc<IntrinsicEnvironment>> {
        self.intrinsic_environments
            .get(&profile)
            .map(|entry| entry.value().clone())
    }

    /// Get one lib environment.
    pub fn lib_environment(&self, profile: ProfileId) -> Option<Arc<LibEnvironment>> {
        self.lib_environments
            .get(&profile)
            .map(|entry| entry.value().clone())
    }

    /// Get one AST artifact.
    pub fn ast(&self, module: ModuleId) -> Option<Arc<Ast>> {
        self.asts.get(&module).map(|entry| entry.value().clone())
    }

    /// Get one base DIR artifact.
    pub fn dir_base(&self, module: ModuleId) -> Option<Arc<DirBase>> {
        self.dir_bases
            .get(&module)
            .map(|entry| entry.value().clone())
    }

    /// Get one prepared DIR artifact.
    pub fn dir_prepared(&self, module: ModuleId, profile: ProfileId) -> Option<Arc<DirPrepared>> {
        self.dir_prepared
            .get(&(module, profile))
            .map(|entry| entry.value().clone())
    }

    /// Get one resolved DIR artifact.
    pub fn dir_resolved(&self, module: ModuleId, profile: ProfileId) -> Option<Arc<DirResolved>> {
        self.dir_resolved
            .get(&(module, profile))
            .map(|entry| entry.value().clone())
    }

    /// Get one declared DIR artifact.
    pub fn dir_declared(&self, module: ModuleId, profile: ProfileId) -> Option<Arc<DirDeclared>> {
        self.dir_declared
            .get(&(module, profile))
            .map(|entry| entry.value().clone())
    }

    /// Get one interface DIR artifact.
    pub fn dir_interface(&self, module: ModuleId, profile: ProfileId) -> Option<Arc<DirInterface>> {
        self.dir_interface
            .get(&(module, profile))
            .map(|entry| entry.value().clone())
    }

    /// Get one analyzed DIR artifact.
    pub fn dir_analyzed(&self, module: ModuleId, profile: ProfileId) -> Option<Arc<DirAnalyzed>> {
        self.dir_analyzed
            .get(&(module, profile))
            .map(|entry| entry.value().clone())
    }

    /// Get one elaborated DIR artifact.
    pub fn dir_elaborated(
        &self,
        module: ModuleId,
        profile: ProfileId,
    ) -> Option<Arc<DirElaborated>> {
        self.dir_elaborated
            .get(&(module, profile))
            .map(|entry| entry.value().clone())
    }

    /// Get one patched DIR artifact.
    pub fn dir_patched(&self, module: ModuleId, profile: ProfileId) -> Option<Arc<DirPatched>> {
        self.dir_patched
            .get(&(module, profile))
            .map(|entry| entry.value().clone())
    }

    /// Get one base MIR artifact.
    pub fn mir_base(
        &self,
        module: ModuleId,
        profile: ProfileId,
        target: &TargetId,
    ) -> Option<Arc<MirBase>> {
        self.mir_bases
            .get(&(module, profile, target.clone()))
            .map(|entry| entry.value().clone())
    }

    /// Get one optimized MIR artifact.
    pub fn mir_optimized(
        &self,
        module: ModuleId,
        profile: ProfileId,
        target: &TargetId,
    ) -> Option<Arc<MirOptimized>> {
        self.mir_optimized
            .get(&(module, profile, target.clone()))
            .map(|entry| entry.value().clone())
    }

    /// Return all target ids with MIR products for one module and profile.
    pub(crate) fn target_ids_for_mir(&self, module: ModuleId, profile: ProfileId) -> Vec<TargetId> {
        let mut targets = Vec::new();

        for entry in self.mir_bases.iter() {
            let (entry_module, entry_profile, target) = entry.key();
            if *entry_module == module && *entry_profile == profile {
                targets.push(target.clone());
            }
        }

        for entry in self.mir_optimized.iter() {
            let (entry_module, entry_profile, target) = entry.key();
            if *entry_module == module
                && *entry_profile == profile
                && !targets.iter().any(|existing| existing == target)
            {
                targets.push(target.clone());
            }
        }

        targets
    }

    /// Return the profile ids that currently have published artifacts for one module.
    pub fn profile_ids_for_module(&self, module: ModuleId) -> HashSet<ProfileId> {
        let mut profiles = HashSet::new();
        self.collect_profile_ids(&mut profiles, &self.dir_prepared, module);
        self.collect_profile_ids(&mut profiles, &self.dir_resolved, module);
        self.collect_profile_ids(&mut profiles, &self.dir_declared, module);
        self.collect_profile_ids(&mut profiles, &self.dir_interface, module);
        self.collect_profile_ids(&mut profiles, &self.dir_analyzed, module);
        self.collect_profile_ids(&mut profiles, &self.dir_elaborated, module);
        self.collect_profile_ids(&mut profiles, &self.dir_patched, module);

        for entry in self.mir_bases.iter() {
            let (entry_module, profile, _) = entry.key();
            if *entry_module == module {
                profiles.insert(*profile);
            }
        }

        for entry in self.mir_optimized.iter() {
            let (entry_module, profile, _) = entry.key();
            if *entry_module == module {
                profiles.insert(*profile);
            }
        }

        profiles
    }

    /// Return the module ids that currently have published artifacts for any profile in the set.
    pub fn module_ids_for_profiles(&self, profiles: &HashSet<ProfileId>) -> HashSet<ModuleId> {
        let mut modules = HashSet::new();

        if profiles.is_empty() {
            return modules;
        }

        self.collect_module_ids(&mut modules, &self.dir_prepared, profiles);
        self.collect_module_ids(&mut modules, &self.dir_resolved, profiles);
        self.collect_module_ids(&mut modules, &self.dir_declared, profiles);
        self.collect_module_ids(&mut modules, &self.dir_interface, profiles);
        self.collect_module_ids(&mut modules, &self.dir_analyzed, profiles);
        self.collect_module_ids(&mut modules, &self.dir_elaborated, profiles);
        self.collect_module_ids(&mut modules, &self.dir_patched, profiles);

        for entry in self.mir_bases.iter() {
            let (module, profile, _) = entry.key();
            if profiles.contains(profile) {
                modules.insert(*module);
            }
        }

        for entry in self.mir_optimized.iter() {
            let (module, profile, _) = entry.key();
            if profiles.contains(profile) {
                modules.insert(*module);
            }
        }

        modules
    }

    /// Clear all semantic artifacts.
    pub fn clear(&self) {
        self.dependencies.clear();
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
    }
}
