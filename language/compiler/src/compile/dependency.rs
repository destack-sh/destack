use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use crate::{BuildDependency, BuildKey, Compiler};
use destack_workspace::{ArtifactDependency, ArtifactKey, OutputDependency, OutputScope};

impl Compiler {
    /// Return the dependency kind for a build key.
    pub(crate) fn build_dependency_for_key(&self, build_key: &BuildKey) -> BuildDependency {
        match build_key {
            BuildKey::Artifact(ArtifactKey::LanguageEnvironment { profile })
            | BuildKey::Artifact(ArtifactKey::IntrinsicEnvironment { profile })
            | BuildKey::Artifact(ArtifactKey::LibEnvironment { profile }) => {
                BuildDependency::Artifact(ArtifactDependency::new(
                    self.hash_build_dependency(&(build_key, self.profile_version(*profile))),
                ))
            }
            BuildKey::Artifact(ArtifactKey::Ast { module })
            | BuildKey::Artifact(ArtifactKey::DirBase { module }) => {
                BuildDependency::Artifact(ArtifactDependency::new(
                    self.hash_build_dependency(&(build_key, self.module_version(*module))),
                ))
            }
            BuildKey::Artifact(
                ArtifactKey::DirPrepared { module, profile }
                | ArtifactKey::DirResolved { module, profile }
                | ArtifactKey::DirDeclared { module, profile }
                | ArtifactKey::DirInterface { module, profile }
                | ArtifactKey::DirAnalyzed { module, profile }
                | ArtifactKey::DirElaborated { module, profile }
                | ArtifactKey::DirPatched { module, profile },
            ) => BuildDependency::Artifact(ArtifactDependency::new(self.hash_build_dependency(&(
                build_key,
                self.module_version(*module),
                self.profile_version(*profile),
            )))),
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
            ) => BuildDependency::Artifact(ArtifactDependency::new(self.hash_build_dependency(&(
                build_key,
                self.module_version(*module),
                self.profile_version(*profile),
                target,
            )))),
            BuildKey::Output(output_key) => {
                let dependency = match output_key.scope {
                    OutputScope::Module(module_id) => self.hash_build_dependency(&(
                        build_key,
                        self.module_version(module_id),
                        &output_key.target,
                    )),
                    OutputScope::Package(package_id) => self.hash_build_dependency(&(
                        build_key,
                        self.package_version(package_id),
                        &output_key.target,
                    )),
                };

                BuildDependency::Output(OutputDependency::new(dependency))
            }
        }
    }

    /// Hash one dependency tuple into one build dependency stamp.
    fn hash_build_dependency(&self, value: &impl Hash) -> u64 {
        let mut hasher = DefaultHasher::new();
        value.hash(&mut hasher);
        hasher.finish()
    }
}
