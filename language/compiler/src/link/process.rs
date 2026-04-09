use std::path::PathBuf;

use crate::timing::tags;
use crate::{Compiler, CompilerContext, LinkError, LinkResult, RequirementError};

use destack_artifact::{ArtifactKey, EmitFormat};
use destack_source::{ModuleId, PackageId, TargetId};
use destack_workspace::{Target, TargetDiscovery, TargetDiscoveryError};

use crate::link::ScriptLinker;
use crate::link::binary::BinaryLinker;

impl Compiler {
    /// Build one package output.
    pub fn process_package_output(
        &self,
        package: PackageId,
        target: TargetId,
        context: &CompilerContext<'_>,
    ) -> LinkResult<()> {
        let revision = context.revision();
        let artifact_key = ArtifactKey::package_output(package, target);
        // reuse one persisted package output image when available
        if context
            .restore_cached_artifact(
                artifact_key,
                |compiler| compiler.load_package_output_image(revision, package, &target),
                |store, version, payload| store.publish_package_output(version, payload),
            )
            .is_some()
        {
            return Ok(());
        }

        let _timing = self.timing_scope(tags::LINK_TARGET);
        self.link_target(package, &target, context)?;
        let output = self
            .package_output(package, &target)
            .ok_or_else(|| LinkError::Internal {
                package,
                message: format!("missing package output artifact for target '{target}'"),
            })?;
        context.store_artifact(
            &artifact_key,
            output.as_ref(),
            |compiler, _artifact_stamp, output| {
                compiler.store_package_output_image(revision, package, &target, output)
            },
        );

        Ok(())
    }

    /// Require one package output artifact.
    pub fn require_package_output(
        &self,
        revision: destack_workspace::Revision,
        package: PackageId,
        target: &TargetId,
    ) -> Result<(), RequirementError> {
        self.require_artifact(revision, ArtifactKey::package_output(package, *target))
    }

    /// Link all modules for one target.
    pub(crate) fn link_target(
        &self,
        package_id: PackageId,
        target_id: &TargetId,
        context: &CompilerContext<'_>,
    ) -> LinkResult<()> {
        // load package and target configuration
        let package = context.package(package_id);
        let package_options = context.package_options(package_id);
        let package_path = package.path.clone();
        let root_dir = package_options
            .as_ref()
            .and_then(|config| config.compiler.root_dir.clone());
        let target = package
            .targets
            .get(target_id)
            .cloned()
            .ok_or(LinkError::MissingTarget {
                package: package_id,
                target: *target_id,
            })?;
        // discover the modules addressed by this target
        let discovered_modules = self.discover_target_modules(
            context.revision(),
            package_id,
            &package_path,
            target_id,
            &target,
        )?;
        let package_dir = self.package_directory(package_path);

        // dispatch into the selected linker family
        let output = match target.emit {
            EmitFormat::Js | EmitFormat::Ts | EmitFormat::Html => ScriptLinker::new(
                self,
                context,
                &package_dir,
                root_dir.as_deref(),
                &target,
                target_id,
                package_id,
            )
            .link_target(&discovered_modules)?,
            EmitFormat::Wasm | EmitFormat::Native => BinaryLinker::new(
                self,
                context,
                &package_dir,
                root_dir.as_deref(),
                &target,
                target_id,
                package_id,
            )
            .link_target(&discovered_modules)?,
        };

        self.publish_artifact(
            ArtifactKey::package_output(package_id, *target_id),
            output,
            |store, version, payload| store.publish_package_output(version, payload),
        );

        Ok(())
    }

    /// Discover the modules addressed by one target.
    pub(crate) fn discover_target_modules(
        &self,
        revision: destack_workspace::Revision,
        package_id: PackageId,
        package_path: &Option<PathBuf>,
        target_id: &TargetId,
        target: &Target,
    ) -> LinkResult<Vec<ModuleId>> {
        let result = match target.discovery {
            TargetDiscovery::Entry => {
                self.discover_entry_modules(revision, package_id, package_path, target, target_id)
            }
            TargetDiscovery::Include => {
                self.discover_include_modules(revision, package_id, package_path, target, target_id)
            }
        };

        result.map_err(|issue| self.link_target_discovery_issue(issue))
    }

    /// Map one target discovery issue into a link error.
    fn link_target_discovery_issue(&self, issue: TargetDiscoveryError) -> LinkError {
        match issue {
            TargetDiscoveryError::Repository {
                package,
                target,
                message,
            } => LinkError::InvalidTarget {
                anchor: package.into(),
                package,
                target,
                message,
            },
            // entry discovery needs package path context
            TargetDiscoveryError::MissingPackagePath { package, target } => {
                LinkError::InvalidTarget {
                    anchor: package.into(),
                    package,
                    target,
                    message: "entry based discovery requires package path".to_string(),
                }
            }

            // missing entry paths are invalid target configuration
            TargetDiscoveryError::MissingEntry {
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
        package_path.unwrap_or_else(|| self.repository.workspace_root().to_path_buf())
    }
}
