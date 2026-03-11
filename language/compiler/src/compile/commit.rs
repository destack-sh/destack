use destack_source::ModuleId;
use destack_workspace::{ArtifactKey, ModuleDirData, ModuleMirData, ProfileId, TargetId};

use crate::{BuildDependency, BuildKey, Compiler};

impl Compiler {
    /// Commit the completed build product from the current workspace state.
    pub(crate) fn commit_completed_build(&self, build_key: &BuildKey) {
        let dependency = self.build_dependency_for_key(build_key);

        match build_key {
            // FUGU #Architecture: slice 3 should make environment builders return immutable
            // artifacts through the central commit path instead of writing directly to the registry
            BuildKey::Artifact(ArtifactKey::LanguageEnvironment { .. })
            | BuildKey::Artifact(ArtifactKey::IntrinsicEnvironment { .. })
            | BuildKey::Artifact(ArtifactKey::LibEnvironment { .. }) => {}
            BuildKey::Artifact(ArtifactKey::Ast { module }) => {
                let module_id = *module;
                let module = self.program.modules.get(module_id);
                let module = module.read();
                let Some(ast) = module.ast_maybe() else {
                    return;
                };
                self.program.artifacts.set_ast(module_id, ast.to_data());
            }
            BuildKey::Artifact(ArtifactKey::DirBase { module }) => {
                let module_id = *module;
                let module = self.program.modules.get(module_id);
                let module = module.read();
                let Some(dir) = module.dir_base_maybe() else {
                    return;
                };
                self.program.artifacts.set_dir_base(module_id, dir.to_data());
            }
            BuildKey::Artifact(
                ArtifactKey::DirPrepared { module, profile }
                | ArtifactKey::DirResolved { module, profile }
                | ArtifactKey::DirDeclared { module, profile }
                | ArtifactKey::DirInterface { module, profile }
                | ArtifactKey::DirAnalyzed { module, profile }
                | ArtifactKey::DirElaborated { module, profile }
                | ArtifactKey::DirPatched { module, profile },
            ) => {
                let module_id = *module;
                let profile_id = *profile;
                let module = self.program.modules.get(module_id);
                let module = module.read();
                let Some(dir) = module.dir_maybe(profile_id) else {
                    return;
                };
                self.commit_dir_artifact(build_key, module_id, profile_id, dir.to_data());
            }
            BuildKey::Artifact(
                ArtifactKey::Mir {
                    module,
                    profile,
                    target,
                }
                | ArtifactKey::MirOptimized {
                    module,
                    profile,
                    target,
                },
            ) => {
                let module_id = *module;
                let profile_id = *profile;
                let module = self.program.modules.get(module_id);
                let module = module.read();
                let Some(mir) = module.mir_maybe(target) else {
                    return;
                };
                self.commit_mir_artifact(build_key, module_id, profile_id, target, mir.to_data());
            }
            BuildKey::Output(output_key) => {
                if let BuildDependency::Output(output_dependency) = dependency {
                    self.program
                        .outputs
                        .set_dependency(output_key.clone(), output_dependency);
                }

                return;
            }
        }

        if let (BuildKey::Artifact(artifact_key), BuildDependency::Artifact(artifact_dependency)) =
            (build_key, dependency)
        {
            self.program
                .artifacts
                .set_dependency(artifact_key.clone(), artifact_dependency);
        }
    }

    /// Commit one DIR-family artifact from the current module workspace.
    fn commit_dir_artifact(
        &self,
        build_key: &BuildKey,
        module_id: ModuleId,
        profile_id: ProfileId,
        dir_data: ModuleDirData,
    ) {
        match build_key {
            BuildKey::Artifact(ArtifactKey::DirPrepared { .. }) => {
                self.program
                    .artifacts
                    .set_dir_prepared(module_id, profile_id, dir_data);
            }
            BuildKey::Artifact(ArtifactKey::DirResolved { .. }) => {
                self.program
                    .artifacts
                    .set_dir_resolved(module_id, profile_id, dir_data);
            }
            BuildKey::Artifact(ArtifactKey::DirDeclared { .. }) => {
                self.program
                    .artifacts
                    .set_dir_declared(module_id, profile_id, dir_data);
            }
            BuildKey::Artifact(ArtifactKey::DirInterface { .. }) => {
                self.commit_dir_interface_artifact(module_id, profile_id);
            }
            BuildKey::Artifact(ArtifactKey::DirAnalyzed { .. }) => {
                self.program
                    .artifacts
                    .set_dir_analyzed(module_id, profile_id, dir_data);
            }
            BuildKey::Artifact(ArtifactKey::DirElaborated { .. }) => {
                self.program
                    .artifacts
                    .set_dir_elaborated(module_id, profile_id, dir_data);
            }
            BuildKey::Artifact(ArtifactKey::DirPatched { .. }) => {
                self.program
                    .artifacts
                    .set_dir_patched(module_id, profile_id, dir_data);
            }
            _ => {}
        }
    }

    /// Commit one converged interface artifact batch.
    fn commit_dir_interface_artifact(&self, module_id: ModuleId, profile_id: ProfileId) {
        let component_modules = self
            .interface_component_plan(module_id, profile_id)
            .map(|plan| plan.component_modules)
            .unwrap_or_else(|_| vec![module_id]);

        for component_module_id in component_modules {
            let component_module = self.program.modules.get(component_module_id);
            let component_module = component_module.read();
            let Some(component_dir) = component_module.dir_maybe(profile_id) else {
                continue;
            };
            let component_dir_data = component_dir.to_data();
            self.program
                .artifacts
                .set_dir_interface(component_module_id, profile_id, component_dir_data);
        }
    }

    /// Commit one MIR-family artifact from the current module workspace.
    fn commit_mir_artifact(
        &self,
        build_key: &BuildKey,
        module_id: ModuleId,
        profile_id: ProfileId,
        target: &TargetId,
        mir_data: ModuleMirData,
    ) {
        match build_key {
            BuildKey::Artifact(ArtifactKey::Mir { .. }) => {
                self.program
                    .artifacts
                    .set_mir(module_id, profile_id, target.clone(), mir_data);
            }
            BuildKey::Artifact(ArtifactKey::MirOptimized { .. }) => {
                self.program.artifacts.set_optimized_mir(
                    module_id,
                    profile_id,
                    target.clone(),
                    mir_data,
                );
            }
            _ => {}
        }
    }
}
