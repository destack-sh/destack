use std::sync::Arc;

use destack_dir::InferTable;
use destack_source::ModuleId;
use destack_workspace::{
    ArtifactKey, ModuleAst, ModuleDir, ModuleDirData, ModuleMir, ModuleMirData, ProfileId,
    Program, TargetId,
};

use crate::{BuildDependency, BuildKey, Compiler};

impl Compiler {
    /// Publish one infer table for follow-up solve and commit tasks.
    pub(crate) fn publish_infer_table_for_module(
        &self,
        module_id: ModuleId,
        profile: ProfileId,
        infer: InferTable,
    ) {
        // FUGU #Architecture: slice 3 must delete this bridge once infer state
        // is rebuilt from artifact-backed transient builders instead of module-owned mutable DIR
        let module = self.program.modules.get(module_id);
        let module = module.read();
        let Some(dir) = module.dir_maybe(profile) else {
            return;
        };
        dir.publish_analyze_infer_table(infer);
    }

    /// Mutate one published infer table for a module and profile.
    pub(crate) fn with_infer_table_for_module_mut<R>(
        &self,
        module_id: ModuleId,
        profile: ProfileId,
        handle: impl FnOnce(&mut InferTable) -> R,
    ) -> Option<R> {
        // FUGU #Architecture: slice 3 must delete this bridge once infer state
        // is rebuilt from artifact-backed transient builders instead of module-owned mutable DIR
        let module = self.program.modules.get(module_id);
        let module = module.read();
        let dir = module.dir_maybe(profile)?;
        dir.with_analyze_infer_table_mut(handle)
    }

    /// Clear one published infer table for a module and profile.
    pub(crate) fn clear_infer_table_for_module(&self, module_id: ModuleId, profile: ProfileId) {
        // FUGU #Architecture: slice 3 must delete this bridge once infer state
        // is rebuilt from artifact-backed transient builders instead of module-owned mutable DIR
        let module = self.program.modules.get(module_id);
        let module = module.read();
        let Some(dir) = module.dir_maybe(profile) else {
            return;
        };
        dir.clear_analyze_infer_table();
    }

    // FUGU #Architecture: slice 3 must delete this bridge once builders start from artifact data directly instead of module-owned mutable workspace
    /// Ensure one build key has the transient module workspace it still expects.
    pub(crate) fn ensure_workspace_for_build_key(&self, build_key: &BuildKey) {
        match build_key {
            BuildKey::Artifact(ArtifactKey::LanguageEnvironment { .. })
            | BuildKey::Artifact(ArtifactKey::IntrinsicEnvironment { .. })
            | BuildKey::Artifact(ArtifactKey::LibEnvironment { .. })
            | BuildKey::Output(_) => {}
            BuildKey::Artifact(ArtifactKey::Ast { module }) => {
                self.ensure_ast_workspace(*module);
            }
            BuildKey::Artifact(ArtifactKey::DirBase { module }) => {
                self.ensure_ast_workspace(*module);
                self.ensure_dir_base_workspace(*module);
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
                self.ensure_ast_workspace(*module);
                self.ensure_dir_base_workspace(*module);
                self.ensure_dir_workspace_for_artifact(*module, *profile, build_key);
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
                self.ensure_ast_workspace(*module);
                self.ensure_dir_base_workspace(*module);
                self.ensure_dir_workspace_for_artifact(
                    *module,
                    *profile,
                    &BuildKey::Artifact(ArtifactKey::DirPatched {
                        module: *module,
                        profile: *profile,
                    }),
                );
                self.ensure_mir_workspace_for_artifact(*module, *profile, target, build_key);
            }
        }
    }

    /// Seed one module AST workspace from committed artifact truth.
    fn ensure_ast_workspace(&self, module_id: ModuleId) {
        let key = ArtifactKey::Ast { module: module_id };
        let Some(ast_data) =
            self.artifact_snapshot_if_satisfied(key, |program| program.artifacts.ast(module_id))
        else {
            return;
        };

        let module = self.program.modules.get(module_id);
        let mut module = module.write();

        // keep existing mutable workspace if it already exists
        if module.ast_maybe().is_some() {
            return;
        }

        module.set_ast(ModuleAst::from_data(ast_data.as_ref().clone()));
    }

    /// Seed one module base DIR workspace from committed artifact truth.
    fn ensure_dir_base_workspace(&self, module_id: ModuleId) {
        let key = ArtifactKey::DirBase { module: module_id };
        let Some(dir_data) = self
            .artifact_snapshot_if_satisfied(key, |program| program.artifacts.dir_base(module_id))
        else {
            return;
        };

        let module = self.program.modules.get(module_id);
        let mut module = module.write();

        // keep existing mutable workspace if it already exists
        if module.dir_base_maybe().is_some() {
            return;
        }

        module.set_dir_base(ModuleDir::from_data(dir_data.as_ref().clone()));
    }

    /// Seed one module profile DIR workspace from the nearest committed artifact snapshot.
    fn ensure_dir_workspace(
        &self,
        module_id: ModuleId,
        profile: ProfileId,
        snapshot: Option<Arc<ModuleDirData>>,
    ) {
        // prefer committed artifact truth when available
        let Some(dir_data) = snapshot else {
            return;
        };

        let module = self.program.modules.get(module_id);
        let mut module = module.write();

        module.set_dir(profile, ModuleDir::from_data(dir_data.as_ref().clone()));
    }

    /// Seed one module profile DIR workspace from one artifact snapshot.
    fn ensure_dir_workspace_for_artifact(
        &self,
        module_id: ModuleId,
        profile: ProfileId,
        build_key: &BuildKey,
    ) {
        let BuildKey::Artifact(artifact_key) = build_key else {
            return;
        };

        let snapshot = self.artifact_dir_snapshot(artifact_key);
        self.ensure_dir_workspace(module_id, profile, snapshot);
    }

    /// Seed one module MIR workspace from the nearest committed artifact snapshot.
    fn ensure_mir_workspace(
        &self,
        module_id: ModuleId,
        _profile: ProfileId,
        _target: &TargetId,
        snapshot: Option<Arc<ModuleMirData>>,
    ) {
        // prefer committed artifact truth when available
        let Some(mir_data) = snapshot else {
            return;
        };

        let module = self.program.modules.get(module_id);
        let mut module = module.write();

        module.set_mir(ModuleMir::from_data(mir_data.as_ref().clone()));
    }

    /// Seed one module MIR workspace from one artifact snapshot.
    fn ensure_mir_workspace_for_artifact(
        &self,
        module_id: ModuleId,
        profile: ProfileId,
        target: &TargetId,
        build_key: &BuildKey,
    ) {
        let BuildKey::Artifact(artifact_key) = build_key else {
            return;
        };

        let snapshot = self.artifact_mir_snapshot(artifact_key);
        self.ensure_mir_workspace(module_id, profile, target, snapshot);
    }

    /// Read one committed DIR snapshot only when it satisfies the current dependency.
    fn artifact_dir_snapshot(&self, key: &ArtifactKey) -> Option<Arc<ModuleDirData>> {
        match key {
            ArtifactKey::DirPrepared { module, profile } => self
                .artifact_snapshot_if_satisfied(
                    ArtifactKey::DirPrepared {
                        module: *module,
                        profile: *profile,
                    },
                    |program| program.artifacts.dir_prepared(*module, *profile),
                )
                .or_else(|| {
                    self.artifact_snapshot_if_satisfied(
                        ArtifactKey::DirBase { module: *module },
                        |program| program.artifacts.dir_base(*module),
                    )
                }),
            ArtifactKey::DirResolved { module, profile } => self
                .artifact_snapshot_if_satisfied(
                    ArtifactKey::DirResolved {
                        module: *module,
                        profile: *profile,
                    },
                    |program| program.artifacts.dir_resolved(*module, *profile),
                )
                .or_else(|| {
                    self.artifact_snapshot_if_satisfied(
                        ArtifactKey::DirPrepared {
                            module: *module,
                            profile: *profile,
                        },
                        |program| program.artifacts.dir_prepared(*module, *profile),
                    )
                }),
            ArtifactKey::DirDeclared { module, profile } => self
                .artifact_snapshot_if_satisfied(
                    ArtifactKey::DirDeclared {
                        module: *module,
                        profile: *profile,
                    },
                    |program| program.artifacts.dir_declared(*module, *profile),
                )
                .or_else(|| {
                    self.artifact_snapshot_if_satisfied(
                        ArtifactKey::DirResolved {
                            module: *module,
                            profile: *profile,
                        },
                        |program| program.artifacts.dir_resolved(*module, *profile),
                    )
                }),
            ArtifactKey::DirInterface { module, profile } => self
                .artifact_snapshot_if_satisfied(
                    ArtifactKey::DirInterface {
                        module: *module,
                        profile: *profile,
                    },
                    |program| program.artifacts.dir_interface(*module, *profile),
                )
                .or_else(|| {
                    self.artifact_snapshot_if_satisfied(
                        ArtifactKey::DirDeclared {
                            module: *module,
                            profile: *profile,
                        },
                        |program| program.artifacts.dir_declared(*module, *profile),
                    )
                }),
            ArtifactKey::DirAnalyzed { module, profile } => self
                .artifact_snapshot_if_satisfied(
                    ArtifactKey::DirAnalyzed {
                        module: *module,
                        profile: *profile,
                    },
                    |program| program.artifacts.dir_analyzed(*module, *profile),
                )
                .or_else(|| {
                    self.artifact_snapshot_if_satisfied(
                        ArtifactKey::DirDeclared {
                            module: *module,
                            profile: *profile,
                        },
                        |program| program.artifacts.dir_declared(*module, *profile),
                    )
                }),
            ArtifactKey::DirElaborated { module, profile } => self
                .artifact_snapshot_if_satisfied(
                    ArtifactKey::DirElaborated {
                        module: *module,
                        profile: *profile,
                    },
                    |program| program.artifacts.dir_elaborated(*module, *profile),
                )
                .or_else(|| {
                    self.artifact_snapshot_if_satisfied(
                        ArtifactKey::DirAnalyzed {
                            module: *module,
                            profile: *profile,
                        },
                        |program| program.artifacts.dir_analyzed(*module, *profile),
                    )
                }),
            ArtifactKey::DirPatched { module, profile } => self
                .artifact_snapshot_if_satisfied(
                    ArtifactKey::DirPatched {
                        module: *module,
                        profile: *profile,
                    },
                    |program| program.artifacts.dir_patched(*module, *profile),
                )
                .or_else(|| {
                    self.artifact_snapshot_if_satisfied(
                        ArtifactKey::DirElaborated {
                            module: *module,
                            profile: *profile,
                        },
                        |program| program.artifacts.dir_elaborated(*module, *profile),
                    )
                }),
            _ => None,
        }
    }

    /// Read one committed MIR snapshot only when it satisfies the current dependency.
    fn artifact_mir_snapshot(&self, key: &ArtifactKey) -> Option<Arc<ModuleMirData>> {
        match key {
            ArtifactKey::Mir {
                module,
                profile,
                target,
            } => self.artifact_snapshot_if_satisfied(
                ArtifactKey::Mir {
                    module: *module,
                    profile: *profile,
                    target: target.clone(),
                },
                |program| program.artifacts.mir(*module, *profile, target),
            ),
            ArtifactKey::MirOptimized {
                module,
                profile,
                target,
            } => self
                .artifact_snapshot_if_satisfied(
                    ArtifactKey::MirOptimized {
                        module: *module,
                        profile: *profile,
                        target: target.clone(),
                    },
                    |program| program.artifacts.optimized_mir(*module, *profile, target),
                )
                .or_else(|| {
                    self.artifact_snapshot_if_satisfied(
                        ArtifactKey::Mir {
                            module: *module,
                            profile: *profile,
                            target: target.clone(),
                        },
                        |program| program.artifacts.mir(*module, *profile, target),
                    )
                }),
            _ => None,
        }
    }

    /// Read one committed artifact snapshot only when it satisfies the current dependency.
    fn artifact_snapshot_if_satisfied<T>(
        &self,
        key: ArtifactKey,
        snapshot: impl FnOnce(&Program) -> Option<Arc<T>>,
    ) -> Option<Arc<T>> {
        let BuildDependency::Artifact(expected_dependency) =
            self.build_dependency_for_key(&BuildKey::Artifact(key.clone()))
        else {
            return None;
        };

        if self.program.artifacts.dependency(&key) != Some(expected_dependency) {
            return None;
        }

        snapshot(&self.program)
    }
}
