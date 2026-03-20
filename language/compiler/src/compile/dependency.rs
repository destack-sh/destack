use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use destack_workspace::{ArtifactDependency, ArtifactKey};

use crate::Compiler;

impl Compiler {
    /// Return the dependency stamp for an artifact key.
    pub(crate) fn artifact_dependency_for_key(
        &self,
        artifact_key: &ArtifactKey,
    ) -> ArtifactDependency {
        match artifact_key {
            ArtifactKey::ModuleGraph { profile } => ArtifactDependency::new(
                self.hash_build_dependency(&(artifact_key, self.profile_version(*profile))),
            ),
            ArtifactKey::LanguageEnvironment { profile }
            | ArtifactKey::IntrinsicEnvironment { profile }
            | ArtifactKey::LibraryEnvironment { profile } => ArtifactDependency::new(
                self.hash_build_dependency(&(artifact_key, self.profile_version(*profile))),
            ),
            ArtifactKey::Ast { module } | ArtifactKey::DirBase { module } => {
                ArtifactDependency::new(
                    self.hash_build_dependency(&(
                        artifact_key,
                        self.module_source_version(*module),
                    )),
                )
            }
            ArtifactKey::DirPrepared { module, profile }
            | ArtifactKey::DirResolved { module, profile }
            | ArtifactKey::DirDeclared { module, profile }
            | ArtifactKey::DirInterface { module, profile }
            | ArtifactKey::DirAnalyzed { module, profile }
            | ArtifactKey::DirElaborated { module, profile }
            | ArtifactKey::DirPatched { module, profile } => {
                ArtifactDependency::new(self.hash_build_dependency(&(
                    artifact_key,
                    self.module_source_version(*module),
                    self.profile_version(*profile),
                )))
            }
            ArtifactKey::MirBase {
                module,
                profile,
                target,
            }
            | ArtifactKey::MirOptimized {
                module,
                profile,
                target,
            } => ArtifactDependency::new(self.hash_build_dependency(&(
                artifact_key,
                self.module_source_version(*module),
                self.profile_version(*profile),
                target,
            ))),
            ArtifactKey::ModuleOutput { module, target } => {
                ArtifactDependency::new(self.hash_build_dependency(&(
                    artifact_key,
                    self.module_source_version(*module),
                    target,
                )))
            }
            ArtifactKey::PackageOutput { package, target } => ArtifactDependency::new(
                self.hash_build_dependency(&(artifact_key, self.package_version(*package), target)),
            ),
        }
    }

    /// Hash one dependency tuple into one artifact dependency stamp.
    fn hash_build_dependency(&self, value: &impl Hash) -> u64 {
        let mut hasher = DefaultHasher::new();
        value.hash(&mut hasher);
        hasher.finish()
    }
}
