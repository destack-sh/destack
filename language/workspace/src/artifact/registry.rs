use std::collections::HashSet;
use std::sync::Arc;

use dashmap::DashMap;
use destack_dir::{GlobalSymbolId, WellKnownSymbol};
use destack_source::ModuleId;

use crate::{
    ArtifactDependency, ArtifactKey, ModuleAst, ModuleDir, ModuleMir, ProfileId, TargetId,
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
    /// AST artifacts by module.
    asts: DashMap<ModuleId, Arc<ModuleAst>>,
    /// Base DIR artifacts by module.
    dir_bases: DashMap<ModuleId, Arc<ModuleDir>>,
    /// Prepared DIR artifacts by module and profile.
    dir_prepared: DashMap<(ModuleId, ProfileId), Arc<ModuleDir>>,
    /// Resolved DIR artifacts by module and profile.
    dir_resolved: DashMap<(ModuleId, ProfileId), Arc<ModuleDir>>,
    /// Declared DIR artifacts by module and profile.
    dir_declared: DashMap<(ModuleId, ProfileId), Arc<ModuleDir>>,
    /// Interface DIR artifacts by module and profile.
    dir_interface: DashMap<(ModuleId, ProfileId), Arc<ModuleDir>>,
    /// Analyzed DIR artifacts by module and profile.
    dir_analyzed: DashMap<(ModuleId, ProfileId), Arc<ModuleDir>>,
    /// Elaborated DIR artifacts by module and profile.
    dir_elaborated: DashMap<(ModuleId, ProfileId), Arc<ModuleDir>>,
    /// Patched DIR artifacts by module and profile.
    dir_patched: DashMap<(ModuleId, ProfileId), Arc<ModuleDir>>,
    /// Base MIR artifacts by module, profile, and target.
    mir_bases: DashMap<(ModuleId, ProfileId, TargetId), Arc<ModuleMir>>,
    /// Optimized MIR artifacts by module, profile, and target.
    mir_optimized: DashMap<(ModuleId, ProfileId, TargetId), Arc<ModuleMir>>,
}

impl ArtifactRegistry {
    /// FUGU #Cleanup: typed exact reads are fine, but publication and invalidation should collapse away from this set/remove helper forest.

    /// Get one shared artifact payload from a map.
    fn get_shared<K, V>(&self, entries: &DashMap<K, Arc<V>>, key: &K) -> Option<Arc<V>>
    where
        K: Eq + std::hash::Hash,
    {
        entries.get(key).map(|entry| entry.value().clone())
    }

    /// Insert one shared artifact payload into a map.
    fn set_shared<K, V>(
        &self,
        entries: &DashMap<K, Arc<V>>,
        key: K,
        value: impl Into<Arc<V>>,
    ) -> Arc<V>
    where
        K: Eq + std::hash::Hash,
    {
        let value = value.into();
        entries.insert(key, value.clone());
        value
    }

    /// Remove one shared artifact payload from a map.
    fn remove_shared<K, V>(&self, entries: &DashMap<K, Arc<V>>, key: &K)
    where
        K: Eq + std::hash::Hash,
    {
        entries.remove(key);
    }

    /// Remove one shared artifact payload and its dependency stamp.
    fn remove_shared_with_dependency<K, V>(
        &self,
        entries: &DashMap<K, Arc<V>>,
        key: &K,
        artifact_key: ArtifactKey,
    ) where
        K: Eq + std::hash::Hash,
    {
        self.remove_shared(entries, key);
        self.remove_dependency(&artifact_key);
    }

    /// Collect profile ids from one module/profile map.
    fn collect_profile_ids(
        &self,
        profiles: &mut HashSet<ProfileId>,
        entries: &DashMap<(ModuleId, ProfileId), Arc<ModuleDir>>,
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
    fn collect_module_ids(
        &self,
        modules: &mut HashSet<ModuleId>,
        entries: &DashMap<(ModuleId, ProfileId), Arc<ModuleDir>>,
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

    /// Remove one dependency stamp for one artifact key.
    pub fn remove_dependency(&self, key: &ArtifactKey) {
        self.dependencies.remove(key);
    }

    /// Get one language environment.
    pub fn language_environment(&self, profile: ProfileId) -> Option<Arc<LanguageEnvironment>> {
        self.get_shared(&self.language_environments, &profile)
    }

    /// Insert one language environment.
    pub fn set_language_environment(
        &self,
        profile: ProfileId,
        environment: LanguageEnvironment,
    ) -> Arc<LanguageEnvironment> {
        self.set_shared(&self.language_environments, profile, environment)
    }

    /// Remove one language environment.
    pub fn remove_language_environment(&self, profile: ProfileId) {
        self.remove_shared_with_dependency(
            &self.language_environments,
            &profile,
            ArtifactKey::LanguageEnvironment { profile },
        );
    }

    /// Get one intrinsic environment.
    pub fn intrinsic_environment(&self, profile: ProfileId) -> Option<Arc<IntrinsicEnvironment>> {
        self.get_shared(&self.intrinsic_environments, &profile)
    }

    /// Insert one intrinsic environment.
    pub fn set_intrinsic_environment(
        &self,
        profile: ProfileId,
        environment: IntrinsicEnvironment,
    ) -> Arc<IntrinsicEnvironment> {
        self.set_shared(&self.intrinsic_environments, profile, environment)
    }

    /// Remove one intrinsic environment.
    pub fn remove_intrinsic_environment(&self, profile: ProfileId) {
        self.remove_shared_with_dependency(
            &self.intrinsic_environments,
            &profile,
            ArtifactKey::IntrinsicEnvironment { profile },
        );
    }

    /// Get one lib environment.
    pub fn lib_environment(&self, profile: ProfileId) -> Option<Arc<LibEnvironment>> {
        self.get_shared(&self.lib_environments, &profile)
    }

    /// Insert one lib environment.
    pub fn set_lib_environment(
        &self,
        profile: ProfileId,
        environment: LibEnvironment,
    ) -> Arc<LibEnvironment> {
        self.set_shared(&self.lib_environments, profile, environment)
    }

    /// Remove one lib environment.
    pub fn remove_lib_environment(&self, profile: ProfileId) {
        self.remove_shared_with_dependency(
            &self.lib_environments,
            &profile,
            ArtifactKey::LibEnvironment { profile },
        );
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

    /// Get one AST artifact.
    pub fn ast(&self, module: ModuleId) -> Option<Arc<ModuleAst>> {
        self.get_shared(&self.asts, &module)
    }

    /// Insert one AST artifact.
    pub fn set_ast(&self, module: ModuleId, ast: impl Into<Arc<ModuleAst>>) -> Arc<ModuleAst> {
        self.set_shared(&self.asts, module, ast)
    }

    /// Remove one AST artifact.
    pub fn remove_ast(&self, module: ModuleId) {
        self.remove_shared_with_dependency(&self.asts, &module, ArtifactKey::Ast { module });
    }

    /// Get one base DIR artifact.
    pub fn dir_base(&self, module: ModuleId) -> Option<Arc<ModuleDir>> {
        self.get_shared(&self.dir_bases, &module)
    }

    /// Insert one base DIR artifact.
    pub fn set_dir_base(&self, module: ModuleId, dir: impl Into<Arc<ModuleDir>>) -> Arc<ModuleDir> {
        self.set_shared(&self.dir_bases, module, dir)
    }

    /// Get one prepared DIR artifact.
    pub fn dir_prepared(&self, module: ModuleId, profile: ProfileId) -> Option<Arc<ModuleDir>> {
        self.get_shared(&self.dir_prepared, &(module, profile))
    }

    /// Insert one prepared DIR artifact.
    pub fn set_dir_prepared(
        &self,
        module: ModuleId,
        profile: ProfileId,
        dir: impl Into<Arc<ModuleDir>>,
    ) -> Arc<ModuleDir> {
        self.set_shared(&self.dir_prepared, (module, profile), dir)
    }

    /// Remove one prepared DIR artifact.
    pub fn remove_dir_prepared(&self, module: ModuleId, profile: ProfileId) {
        self.remove_shared_with_dependency(
            &self.dir_prepared,
            &(module, profile),
            ArtifactKey::DirPrepared { module, profile },
        );
    }

    /// Get one resolved DIR artifact.
    pub fn dir_resolved(&self, module: ModuleId, profile: ProfileId) -> Option<Arc<ModuleDir>> {
        self.get_shared(&self.dir_resolved, &(module, profile))
    }

    /// Insert one resolved DIR artifact.
    pub fn set_dir_resolved(
        &self,
        module: ModuleId,
        profile: ProfileId,
        dir: impl Into<Arc<ModuleDir>>,
    ) -> Arc<ModuleDir> {
        self.set_shared(&self.dir_resolved, (module, profile), dir)
    }

    /// Remove one resolved DIR artifact.
    pub fn remove_dir_resolved(&self, module: ModuleId, profile: ProfileId) {
        self.remove_shared_with_dependency(
            &self.dir_resolved,
            &(module, profile),
            ArtifactKey::DirResolved { module, profile },
        );
    }

    /// Get one declared DIR artifact.
    pub fn dir_declared(&self, module: ModuleId, profile: ProfileId) -> Option<Arc<ModuleDir>> {
        self.get_shared(&self.dir_declared, &(module, profile))
    }

    /// Insert one declared DIR artifact.
    pub fn set_dir_declared(
        &self,
        module: ModuleId,
        profile: ProfileId,
        dir: impl Into<Arc<ModuleDir>>,
    ) -> Arc<ModuleDir> {
        self.set_shared(&self.dir_declared, (module, profile), dir)
    }

    /// Remove one declared DIR artifact.
    pub fn remove_dir_declared(&self, module: ModuleId, profile: ProfileId) {
        self.remove_shared_with_dependency(
            &self.dir_declared,
            &(module, profile),
            ArtifactKey::DirDeclared { module, profile },
        );
    }

    /// Get one interface DIR artifact.
    pub fn dir_interface(&self, module: ModuleId, profile: ProfileId) -> Option<Arc<ModuleDir>> {
        self.get_shared(&self.dir_interface, &(module, profile))
    }

    /// Insert one interface DIR artifact.
    pub fn set_dir_interface(
        &self,
        module: ModuleId,
        profile: ProfileId,
        dir: impl Into<Arc<ModuleDir>>,
    ) -> Arc<ModuleDir> {
        self.set_shared(&self.dir_interface, (module, profile), dir)
    }

    /// Remove one interface DIR artifact.
    pub fn remove_dir_interface(&self, module: ModuleId, profile: ProfileId) {
        self.remove_shared_with_dependency(
            &self.dir_interface,
            &(module, profile),
            ArtifactKey::DirInterface { module, profile },
        );
    }

    /// Get one analyzed DIR artifact.
    pub fn dir_analyzed(&self, module: ModuleId, profile: ProfileId) -> Option<Arc<ModuleDir>> {
        self.get_shared(&self.dir_analyzed, &(module, profile))
    }

    /// Insert one analyzed DIR artifact.
    pub fn set_dir_analyzed(
        &self,
        module: ModuleId,
        profile: ProfileId,
        dir: impl Into<Arc<ModuleDir>>,
    ) -> Arc<ModuleDir> {
        self.set_shared(&self.dir_analyzed, (module, profile), dir)
    }

    /// Remove one analyzed DIR artifact.
    pub fn remove_dir_analyzed(&self, module: ModuleId, profile: ProfileId) {
        self.remove_shared_with_dependency(
            &self.dir_analyzed,
            &(module, profile),
            ArtifactKey::DirAnalyzed { module, profile },
        );
    }

    /// Get one elaborated DIR artifact.
    pub fn dir_elaborated(&self, module: ModuleId, profile: ProfileId) -> Option<Arc<ModuleDir>> {
        self.get_shared(&self.dir_elaborated, &(module, profile))
    }

    /// Insert one elaborated DIR artifact.
    pub fn set_dir_elaborated(
        &self,
        module: ModuleId,
        profile: ProfileId,
        dir: impl Into<Arc<ModuleDir>>,
    ) -> Arc<ModuleDir> {
        self.set_shared(&self.dir_elaborated, (module, profile), dir)
    }

    /// Remove one elaborated DIR artifact.
    pub fn remove_dir_elaborated(&self, module: ModuleId, profile: ProfileId) {
        self.remove_shared_with_dependency(
            &self.dir_elaborated,
            &(module, profile),
            ArtifactKey::DirElaborated { module, profile },
        );
    }

    /// Get one patched DIR artifact.
    pub fn dir_patched(&self, module: ModuleId, profile: ProfileId) -> Option<Arc<ModuleDir>> {
        self.get_shared(&self.dir_patched, &(module, profile))
    }

    /// Insert one patched DIR artifact.
    pub fn set_dir_patched(
        &self,
        module: ModuleId,
        profile: ProfileId,
        dir: impl Into<Arc<ModuleDir>>,
    ) -> Arc<ModuleDir> {
        self.set_shared(&self.dir_patched, (module, profile), dir)
    }

    /// Remove one patched DIR artifact.
    pub fn remove_dir_patched(&self, module: ModuleId, profile: ProfileId) {
        self.remove_shared_with_dependency(
            &self.dir_patched,
            &(module, profile),
            ArtifactKey::DirPatched { module, profile },
        );
    }

    /// Get one base MIR artifact.
    pub fn mir_base(
        &self,
        module: ModuleId,
        profile: ProfileId,
        target: &TargetId,
    ) -> Option<Arc<ModuleMir>> {
        self.get_shared(&self.mir_bases, &(module, profile, target.clone()))
    }

    /// Insert one base MIR artifact.
    pub fn set_mir_base(
        &self,
        module: ModuleId,
        profile: ProfileId,
        target: TargetId,
        mir: impl Into<Arc<ModuleMir>>,
    ) -> Arc<ModuleMir> {
        self.set_shared(&self.mir_bases, (module, profile, target), mir)
    }

    /// Remove one base MIR artifact.
    pub fn remove_mir_base(&self, module: ModuleId, profile: ProfileId, target: &TargetId) {
        self.remove_shared_with_dependency(
            &self.mir_bases,
            &(module, profile, target.clone()),
            ArtifactKey::MirBase {
                module,
                profile,
                target: target.clone(),
            },
        );
    }

    /// Get one optimized MIR artifact.
    pub fn mir_optimized(
        &self,
        module: ModuleId,
        profile: ProfileId,
        target: &TargetId,
    ) -> Option<Arc<ModuleMir>> {
        self.get_shared(&self.mir_optimized, &(module, profile, target.clone()))
    }

    /// Insert one optimized MIR artifact.
    pub fn set_mir_optimized(
        &self,
        module: ModuleId,
        profile: ProfileId,
        target: TargetId,
        mir: impl Into<Arc<ModuleMir>>,
    ) -> Arc<ModuleMir> {
        self.set_shared(&self.mir_optimized, (module, profile, target), mir)
    }

    /// Remove one optimized MIR artifact.
    pub fn remove_mir_optimized(&self, module: ModuleId, profile: ProfileId, target: &TargetId) {
        self.remove_shared_with_dependency(
            &self.mir_optimized,
            &(module, profile, target.clone()),
            ArtifactKey::MirOptimized {
                module,
                profile,
                target: target.clone(),
            },
        );
    }

    /// Return all target ids with MIR products for one module and profile.
    pub fn target_ids_for_mir(&self, module: ModuleId, profile: ProfileId) -> Vec<TargetId> {
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

    /// Remove all published module artifacts for the provided profiles.
    pub fn remove_module_profiles(&self, module: ModuleId, profiles: &HashSet<ProfileId>) {
        for profile in profiles {
            self.remove_dir_prepared(module, *profile);
            self.remove_dir_resolved(module, *profile);
            self.remove_dir_declared(module, *profile);
            self.remove_dir_interface(module, *profile);
            self.remove_dir_analyzed(module, *profile);
            self.remove_dir_elaborated(module, *profile);
            self.remove_dir_patched(module, *profile);

            for target in self.target_ids_for_mir(module, *profile) {
                self.remove_mir_base(module, *profile, &target);
                self.remove_mir_optimized(module, *profile, &target);
            }
        }
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
