use destack_source::ModuleId;
use destack_workspace::{ArtifactKey, ModuleDirData, ProfileId};

use crate::{BuildDependency, BuildKey, BuildProduct, Compiler};

impl Compiler {
    /// Commit the completed build product from the current workspace state.
    pub(crate) fn commit_completed_build(
        &self,
        build_key: &BuildKey,
        product: &Option<BuildProduct>,
    ) {
        let dependency = self.build_dependency_for_key(build_key);

        if let Some(product) = product {
            self.commit_completed_product(build_key, product);
        }

        self.set_build_dependency(build_key, dependency);
    }

    /// Record one dependency stamp for one completed build key.
    fn set_build_dependency(&self, build_key: &BuildKey, dependency: BuildDependency) {
        match (build_key, dependency) {
            (BuildKey::Artifact(artifact_key), BuildDependency::Artifact(artifact_dependency)) => {
                self.program
                    .artifacts
                    .set_dependency(artifact_key.clone(), artifact_dependency);
            }
            (BuildKey::Output(output_key), BuildDependency::Output(output_dependency)) => {
                self.program
                    .outputs
                    .set_dependency(output_key.clone(), output_dependency);
            }
            _ => {}
        }
    }

    /// Commit one direct build product payload.
    fn commit_completed_product(&self, build_key: &BuildKey, product: &BuildProduct) {
        match (build_key, product) {
            (
                BuildKey::Artifact(ArtifactKey::LanguageEnvironment { profile }),
                BuildProduct::LanguageEnvironment(environment),
            ) => {
                self.program
                    .artifacts
                    .set_language_environment(*profile, environment.clone());
            }
            (
                BuildKey::Artifact(ArtifactKey::IntrinsicEnvironment { profile }),
                BuildProduct::IntrinsicEnvironment(environment),
            ) => {
                self.program
                    .artifacts
                    .set_intrinsic_environment(*profile, environment.clone());
            }
            (
                BuildKey::Artifact(ArtifactKey::LibEnvironment { profile }),
                BuildProduct::LibEnvironment(environment),
            ) => {
                self.program
                    .artifacts
                    .set_lib_environment(*profile, environment.clone());
            }
            (BuildKey::Artifact(ArtifactKey::Ast { module }), BuildProduct::Ast(ast)) => {
                self.program.artifacts.set_ast(*module, ast.clone());
            }
            (BuildKey::Artifact(ArtifactKey::DirBase { module }), BuildProduct::Dir(dir_data)) => {
                self.program
                    .artifacts
                    .set_dir_base(*module, dir_data.clone());
            }
            (
                BuildKey::Artifact(ArtifactKey::DirPrepared { module, profile }),
                BuildProduct::Dir(dir_data),
            ) => {
                self.program
                    .artifacts
                    .set_dir_prepared(*module, *profile, dir_data.clone());
            }
            (
                BuildKey::Artifact(ArtifactKey::DirResolved { module, profile }),
                BuildProduct::Dir(dir_data),
            ) => {
                self.program
                    .artifacts
                    .set_dir_resolved(*module, *profile, dir_data.clone());
            }
            (
                BuildKey::Artifact(ArtifactKey::DirDeclared { module, profile }),
                BuildProduct::Dir(dir_data),
            ) => {
                self.program
                    .artifacts
                    .set_dir_declared(*module, *profile, dir_data.clone());
            }
            (
                BuildKey::Artifact(ArtifactKey::DirInterface { .. }),
                BuildProduct::DirInterfaceBatch(entries),
            ) => {
                self.commit_dir_interface_batch(entries);
            }
            (
                BuildKey::Artifact(ArtifactKey::DirAnalyzed { module, profile }),
                BuildProduct::Dir(dir_data),
            ) => {
                self.program
                    .artifacts
                    .set_dir_analyzed(*module, *profile, dir_data.clone());
            }
            (
                BuildKey::Artifact(ArtifactKey::DirElaborated { module, profile }),
                BuildProduct::Dir(dir_data),
            ) => {
                self.program
                    .artifacts
                    .set_dir_elaborated(*module, *profile, dir_data.clone());
            }
            (
                BuildKey::Artifact(ArtifactKey::DirPatched { module, profile }),
                BuildProduct::Dir(dir_data),
            ) => {
                self.program
                    .artifacts
                    .set_dir_patched(*module, *profile, dir_data.clone());
            }
            (
                BuildKey::Artifact(ArtifactKey::Mir {
                    module,
                    profile,
                    target,
                }),
                BuildProduct::Mir(mir_data),
            ) => {
                self.program
                    .artifacts
                    .set_mir(*module, *profile, target.clone(), mir_data.clone());
            }
            (
                BuildKey::Artifact(ArtifactKey::MirOptimized {
                    module,
                    profile,
                    target,
                }),
                BuildProduct::Mir(mir_data),
            ) => {
                self.program.artifacts.set_optimized_mir(
                    *module,
                    *profile,
                    target.clone(),
                    mir_data.clone(),
                );
            }
            _ => {}
        }
    }

    /// Commit one converged interface artifact batch from direct build outputs.
    fn commit_dir_interface_batch(&self, entries: &[(ModuleId, ProfileId, ModuleDirData)]) {
        for (module_id, profile_id, dir_data) in entries {
            self.program
                .artifacts
                .set_dir_interface(*module_id, *profile_id, dir_data.clone());
        }
    }
}
