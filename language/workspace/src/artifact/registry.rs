use std::sync::Arc;

use dashmap::DashMap;
use destack_dir::{GlobalSymbolId, WellKnownSymbol};
use destack_source::ModuleId;

use crate::{
    ArtifactDependency, ArtifactKey, ModuleAstData, ModuleDirData, ModuleMirData, ProfileId,
    TargetId,
};

use super::{IntrinsicEnvironment, LanguageEnvironment, LibEnvironment};

/// Registry of published semantic artifacts.
#[derive(Debug, Default)]
pub struct ArtifactRegistry {
    /// Dependency stamps by artifact key.
    dependencies: DashMap<ArtifactKey, ArtifactDependency>,
    /// Language environments by profile.
    language_environments: DashMap<ProfileId, Arc<LanguageEnvironment>>,
    /// Intrinsic environments by profile.
    intrinsic_environments: DashMap<ProfileId, Arc<IntrinsicEnvironment>>,
    /// Lib environments by profile.
    lib_environments: DashMap<ProfileId, Arc<LibEnvironment>>,
    /// AST snapshots by module.
    asts: DashMap<ModuleId, Arc<ModuleAstData>>,
    /// Base DIR snapshots by module.
    dir_bases: DashMap<ModuleId, Arc<ModuleDirData>>,
    /// Prepared DIR snapshots by module and profile.
    dir_prepared: DashMap<(ModuleId, ProfileId), Arc<ModuleDirData>>,
    /// Resolved DIR snapshots by module and profile.
    dir_resolved: DashMap<(ModuleId, ProfileId), Arc<ModuleDirData>>,
    /// Declared DIR snapshots by module and profile.
    dir_declared: DashMap<(ModuleId, ProfileId), Arc<ModuleDirData>>,
    /// Interface DIR snapshots by module and profile.
    dir_interface: DashMap<(ModuleId, ProfileId), Arc<ModuleDirData>>,
    /// Analyzed DIR snapshots by module and profile.
    dir_analyzed: DashMap<(ModuleId, ProfileId), Arc<ModuleDirData>>,
    /// Elaborated DIR snapshots by module and profile.
    dir_elaborated: DashMap<(ModuleId, ProfileId), Arc<ModuleDirData>>,
    /// Patched DIR snapshots by module and profile.
    dir_patched: DashMap<(ModuleId, ProfileId), Arc<ModuleDirData>>,
    /// MIR snapshots by module, profile, and target.
    mirs: DashMap<(ModuleId, ProfileId, TargetId), Arc<ModuleMirData>>,
    /// Optimized MIR snapshots by module, profile, and target.
    optimized_mirs: DashMap<(ModuleId, ProfileId, TargetId), Arc<ModuleMirData>>,
}

impl ArtifactRegistry {
    /// Create a new semantic artifact registry.
    pub fn new() -> Self {
        Self {
            dependencies: DashMap::new(),
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
            mirs: DashMap::new(),
            optimized_mirs: DashMap::new(),
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

    /// Get one language environment.
    pub fn language_environment(&self, profile: ProfileId) -> Option<Arc<LanguageEnvironment>> {
        self.language_environments
            .get(&profile)
            .map(|environment| environment.value().clone())
    }

    /// Insert one language environment.
    pub fn set_language_environment(
        &self,
        profile: ProfileId,
        environment: LanguageEnvironment,
    ) -> Arc<LanguageEnvironment> {
        let environment = Arc::new(environment);
        self.language_environments
            .insert(profile, environment.clone());
        environment
    }

    /// Remove one language environment.
    pub fn remove_language_environment(&self, profile: ProfileId) {
        self.language_environments.remove(&profile);
    }

    /// Get one intrinsic environment.
    pub fn intrinsic_environment(&self, profile: ProfileId) -> Option<Arc<IntrinsicEnvironment>> {
        self.intrinsic_environments
            .get(&profile)
            .map(|environment| environment.value().clone())
    }

    /// Insert one intrinsic environment.
    pub fn set_intrinsic_environment(
        &self,
        profile: ProfileId,
        environment: IntrinsicEnvironment,
    ) -> Arc<IntrinsicEnvironment> {
        let environment = Arc::new(environment);
        self.intrinsic_environments
            .insert(profile, environment.clone());
        environment
    }

    /// Remove one intrinsic environment.
    pub fn remove_intrinsic_environment(&self, profile: ProfileId) {
        self.intrinsic_environments.remove(&profile);
    }

    /// Get one lib environment.
    pub fn lib_environment(&self, profile: ProfileId) -> Option<Arc<LibEnvironment>> {
        self.lib_environments
            .get(&profile)
            .map(|environment| environment.value().clone())
    }

    /// Insert one lib environment.
    pub fn set_lib_environment(
        &self,
        profile: ProfileId,
        environment: LibEnvironment,
    ) -> Arc<LibEnvironment> {
        let environment = Arc::new(environment);
        self.lib_environments.insert(profile, environment.clone());
        environment
    }

    /// Remove one lib environment.
    pub fn remove_lib_environment(&self, profile: ProfileId) {
        self.lib_environments.remove(&profile);
    }

    /// Return the first available well-known type symbol from any profile environment.
    pub fn first_well_known_type_symbol(
        &self,
        well_known: WellKnownSymbol,
    ) -> Option<GlobalSymbolId> {
        for environment in self.lib_environments.iter() {
            if let Some(symbol_id) = environment
                .value()
                .well_known_symbols
                .get_type_symbol(well_known)
            {
                return Some(symbol_id);
            }
        }

        None
    }

    /// Get one AST snapshot.
    pub fn ast(&self, module: ModuleId) -> Option<Arc<ModuleAstData>> {
        self.asts.get(&module).map(|ast| ast.value().clone())
    }

    /// Insert one AST snapshot.
    pub fn set_ast(&self, module: ModuleId, ast: ModuleAstData) -> Arc<ModuleAstData> {
        let ast = Arc::new(ast);
        self.asts.insert(module, ast.clone());
        ast
    }

    /// Get one base DIR snapshot.
    pub fn dir_base(&self, module: ModuleId) -> Option<Arc<ModuleDirData>> {
        self.dir_bases.get(&module).map(|dir| dir.value().clone())
    }

    /// Insert one base DIR snapshot.
    pub fn set_dir_base(&self, module: ModuleId, dir: ModuleDirData) -> Arc<ModuleDirData> {
        let dir = Arc::new(dir);
        self.dir_bases.insert(module, dir.clone());
        dir
    }

    /// Get one prepared DIR snapshot.
    pub fn dir_prepared(&self, module: ModuleId, profile: ProfileId) -> Option<Arc<ModuleDirData>> {
        self.dir_prepared
            .get(&(module, profile))
            .map(|dir| dir.value().clone())
    }

    /// Insert one prepared DIR snapshot.
    pub fn set_dir_prepared(
        &self,
        module: ModuleId,
        profile: ProfileId,
        dir: ModuleDirData,
    ) -> Arc<ModuleDirData> {
        let dir = Arc::new(dir);
        self.dir_prepared.insert((module, profile), dir.clone());
        dir
    }

    /// Get one resolved DIR snapshot.
    pub fn dir_resolved(&self, module: ModuleId, profile: ProfileId) -> Option<Arc<ModuleDirData>> {
        self.dir_resolved
            .get(&(module, profile))
            .map(|dir| dir.value().clone())
    }

    /// Insert one resolved DIR snapshot.
    pub fn set_dir_resolved(
        &self,
        module: ModuleId,
        profile: ProfileId,
        dir: ModuleDirData,
    ) -> Arc<ModuleDirData> {
        let dir = Arc::new(dir);
        self.dir_resolved.insert((module, profile), dir.clone());
        dir
    }

    /// Get one declared DIR snapshot.
    pub fn dir_declared(&self, module: ModuleId, profile: ProfileId) -> Option<Arc<ModuleDirData>> {
        self.dir_declared
            .get(&(module, profile))
            .map(|dir| dir.value().clone())
    }

    /// Insert one declared DIR snapshot.
    pub fn set_dir_declared(
        &self,
        module: ModuleId,
        profile: ProfileId,
        dir: ModuleDirData,
    ) -> Arc<ModuleDirData> {
        let dir = Arc::new(dir);
        self.dir_declared.insert((module, profile), dir.clone());
        dir
    }

    /// Get one interface DIR snapshot.
    pub fn dir_interface(
        &self,
        module: ModuleId,
        profile: ProfileId,
    ) -> Option<Arc<ModuleDirData>> {
        self.dir_interface
            .get(&(module, profile))
            .map(|dir| dir.value().clone())
    }

    /// Insert one interface DIR snapshot.
    pub fn set_dir_interface(
        &self,
        module: ModuleId,
        profile: ProfileId,
        dir: ModuleDirData,
    ) -> Arc<ModuleDirData> {
        let dir = Arc::new(dir);
        self.dir_interface.insert((module, profile), dir.clone());
        dir
    }

    /// Get one analyzed DIR snapshot.
    pub fn dir_analyzed(&self, module: ModuleId, profile: ProfileId) -> Option<Arc<ModuleDirData>> {
        self.dir_analyzed
            .get(&(module, profile))
            .map(|dir| dir.value().clone())
    }

    /// Insert one analyzed DIR snapshot.
    pub fn set_dir_analyzed(
        &self,
        module: ModuleId,
        profile: ProfileId,
        dir: ModuleDirData,
    ) -> Arc<ModuleDirData> {
        let dir = Arc::new(dir);
        self.dir_analyzed.insert((module, profile), dir.clone());
        dir
    }

    /// Get one elaborated DIR snapshot.
    pub fn dir_elaborated(
        &self,
        module: ModuleId,
        profile: ProfileId,
    ) -> Option<Arc<ModuleDirData>> {
        self.dir_elaborated
            .get(&(module, profile))
            .map(|dir| dir.value().clone())
    }

    /// Insert one elaborated DIR snapshot.
    pub fn set_dir_elaborated(
        &self,
        module: ModuleId,
        profile: ProfileId,
        dir: ModuleDirData,
    ) -> Arc<ModuleDirData> {
        let dir = Arc::new(dir);
        self.dir_elaborated.insert((module, profile), dir.clone());
        dir
    }

    /// Get one patched DIR snapshot.
    pub fn dir_patched(&self, module: ModuleId, profile: ProfileId) -> Option<Arc<ModuleDirData>> {
        self.dir_patched
            .get(&(module, profile))
            .map(|dir| dir.value().clone())
    }

    /// Insert one patched DIR snapshot.
    pub fn set_dir_patched(
        &self,
        module: ModuleId,
        profile: ProfileId,
        dir: ModuleDirData,
    ) -> Arc<ModuleDirData> {
        let dir = Arc::new(dir);
        self.dir_patched.insert((module, profile), dir.clone());
        dir
    }

    /// Get one MIR snapshot.
    pub fn mir(
        &self,
        module: ModuleId,
        profile: ProfileId,
        target: &TargetId,
    ) -> Option<Arc<ModuleMirData>> {
        self.mirs
            .get(&(module, profile, target.clone()))
            .map(|mir| mir.value().clone())
    }

    /// Insert one MIR snapshot.
    pub fn set_mir(
        &self,
        module: ModuleId,
        profile: ProfileId,
        target: TargetId,
        mir: ModuleMirData,
    ) -> Arc<ModuleMirData> {
        let mir = Arc::new(mir);
        self.mirs.insert((module, profile, target), mir.clone());
        mir
    }

    /// Get one optimized MIR snapshot.
    pub fn optimized_mir(
        &self,
        module: ModuleId,
        profile: ProfileId,
        target: &TargetId,
    ) -> Option<Arc<ModuleMirData>> {
        self.optimized_mirs
            .get(&(module, profile, target.clone()))
            .map(|mir| mir.value().clone())
    }

    /// Insert one optimized MIR snapshot.
    pub fn set_optimized_mir(
        &self,
        module: ModuleId,
        profile: ProfileId,
        target: TargetId,
        mir: ModuleMirData,
    ) -> Arc<ModuleMirData> {
        let mir = Arc::new(mir);
        self.optimized_mirs
            .insert((module, profile, target), mir.clone());
        mir
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
        self.mirs.clear();
        self.optimized_mirs.clear();
    }
}
