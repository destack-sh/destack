use std::path::PathBuf;

use crate::timing::tags;
use crate::{ArtifactRequirementError, Compiler, LinkError, LinkResult};

use destack_artifact::{ArtifactKey, EmitFormat};
use destack_source::{ModuleId, PackageId};
use destack_workspace::{Target, TargetDiscovery, TargetDiscoveryIssue, TargetId};

use crate::link::ScriptLinker;
use crate::link::binary::BinaryLinker;

impl Compiler {
    /// Build one package output.
    pub fn process_package_output(&self, package: PackageId, target: TargetId) -> LinkResult<()> {
        let package_stamp = self.package_stamp(package);
        if !self.package_version_matches(package_stamp.id, package_stamp.version) {
            return Ok(());
        }
        let artifact_key = ArtifactKey::package_output(package, target.clone());

        // reuse one persisted package output image when available
        if self
            .load_published_artifact(artifact_key.clone(), |compiler| {
                compiler.load_package_output_image(package, &target)
            })
            .is_some()
        {
            return Ok(());
        }

        let _timing = self.timing_scope(tags::LINK_TARGET);
        self.link_target(package_stamp.id, &target)?;
        let output = self
            .artifacts
            .package_output(package, &target)
            .ok_or_else(|| LinkError::Internal {
                package,
                message: format!("missing package output artifact for target '{target}'"),
            })?;
        self.store_artifact(&artifact_key, output.as_ref(), |compiler, output| {
            compiler.store_package_output_image(package, &target, output)
        });

        Ok(())
    }

    /// Require one package output artifact.
    pub fn require_package_output(
        &self,
        package: PackageId,
        target: &TargetId,
    ) -> Result<(), ArtifactRequirementError> {
        self.require_artifact(ArtifactKey::package_output(package, target.clone()))
    }

    /// Link all modules for one target.
    pub(crate) fn link_target(
        &self,
        package_id: PackageId,
        target_id: &TargetId,
    ) -> LinkResult<()> {
        // load package and target configuration
        let package = self.program.packages.get(package_id);
        let package = package.read();
        let package_path = package.path.clone();
        let root_dir = package
            .config
            .as_ref()
            .and_then(|config| config.options.compiler.root_dir.clone());
        let target =
            package
                .targets
                .get(target_id)
                .cloned()
                .ok_or_else(|| LinkError::MissingTarget {
                    package: package_id,
                    target: target_id.clone(),
                })?;
        drop(package);

        // discover the modules addressed by this target
        let discovered_modules =
            self.discover_target_modules(package_id, &package_path, target_id, &target)?;
        let package_dir = self.package_directory(package_path);

        // dispatch into the selected linker family
        let output = match target.emit {
            EmitFormat::Js | EmitFormat::Ts | EmitFormat::Html => ScriptLinker::new(
                self,
                &package_dir,
                root_dir.as_deref(),
                &target,
                target_id,
                package_id,
            )
            .link_target(&discovered_modules)?,
            EmitFormat::Wasm | EmitFormat::Native => BinaryLinker::new(
                self,
                &package_dir,
                root_dir.as_deref(),
                &target,
                target_id,
                package_id,
            )
            .link_target(&discovered_modules)?,
        };

        self.artifacts.publish(
            ArtifactKey::package_output(package_id, target_id.clone()),
            output,
        );

        Ok(())
    }

    /// Discover the modules addressed by one target.
    pub(crate) fn discover_target_modules(
        &self,
        package_id: PackageId,
        package_path: &Option<PathBuf>,
        target_id: &TargetId,
        target: &Target,
    ) -> LinkResult<Vec<ModuleId>> {
        let result = match target.discovery {
            TargetDiscovery::Entry => {
                self.discover_entry_modules(package_id, package_path, target, target_id)
            }
            TargetDiscovery::Include => {
                self.discover_include_modules(package_id, package_path, target)
            }
        };

        result.map_err(|issue| self.link_target_discovery_issue(issue))
    }

    /// Map one target discovery issue into a link error.
    fn link_target_discovery_issue(&self, issue: TargetDiscoveryIssue) -> LinkError {
        match issue {
            // entry discovery needs package path context
            TargetDiscoveryIssue::MissingPackagePath { package, target } => {
                LinkError::InvalidTarget {
                    anchor: package.into(),
                    package,
                    target,
                    message: "entry based discovery requires package path".to_string(),
                }
            }

            // missing entry paths are invalid target configuration
            TargetDiscoveryIssue::MissingEntry {
                package,
                target,
                path,
            } => LinkError::InvalidTarget {
                anchor: package.into(),
                package,
                target,
                message: format!("entry point not found: {}", path.display()),
            },
        }
    }

    /// Return the package directory used for linked output resolution.
    fn package_directory(&self, package_path: Option<PathBuf>) -> PathBuf {
        package_path.unwrap_or_else(|| self.program.cwd.clone())
    }
}
