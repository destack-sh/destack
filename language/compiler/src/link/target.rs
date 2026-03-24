use crate::{Compiler, LinkError, LinkResult, TargetDiscoveryIssue};

use destack_artifact::ArtifactKey;
use destack_source::{ModuleId, PackageId, Span};
use destack_workspace::{TargetDiscovery, TargetId};

use super::assembly::TargetAssembly;

impl Compiler {
    /// Return one file-start span for one module.
    pub(crate) fn module_anchor_span(&self, module_id: ModuleId) -> Span {
        let module = self.program.modules.get(module_id);

        Span::empty(module.file_id)
    }

    /// Return one file-start span for one package config or manifest.
    pub(crate) fn package_anchor_span(&self, package_id: PackageId) -> Span {
        let package = self.program.packages.get(package_id);
        let package = package.read();
        if let Some(config) = package.config.as_ref() {
            return Span::empty(config.file_id);
        }
        if let Some(manifest) = package.manifest.as_ref() {
            return Span::empty(manifest.file_id);
        }
        Span::empty(self.program.fallback_file_id)
    }

    /// Discover the entry module set for one target.
    fn discover_target_modules(
        &self,
        package_id: PackageId,
        package_path: &Option<std::path::PathBuf>,
        target_id: &TargetId,
        target: &destack_workspace::Target,
    ) -> LinkResult<Vec<ModuleId>> {
        match target.discovery {
            TargetDiscovery::Entry => self
                .discover_entry_modules(package_id, package_path, target, target_id)
                .map_err(|issue| match issue {
                    TargetDiscoveryIssue::MissingPackagePath { package, target } => {
                        LinkError::InvalidTarget {
                            span: self.package_anchor_span(package),
                            package,
                            target,
                            message: "entry-based discovery requires package path".to_string(),
                        }
                    }
                    TargetDiscoveryIssue::MissingEntry { package, path, .. } => {
                        LinkError::Internal {
                            package,
                            message: format!("entry point not found: {}", path.display()),
                        }
                    }
                }),
            TargetDiscovery::Include => self
                .discover_include_modules(package_id, package_path, target)
                .map_err(|issue| match issue {
                    TargetDiscoveryIssue::MissingPackagePath { package, target } => {
                        LinkError::InvalidTarget {
                            span: self.package_anchor_span(package),
                            package,
                            target,
                            message: "entry-based discovery requires package path".to_string(),
                        }
                    }
                    TargetDiscoveryIssue::MissingEntry { package, path, .. } => {
                        LinkError::Internal {
                            package,
                            message: format!("entry point not found: {}", path.display()),
                        }
                    }
                }),
        }
    }

    /// Require the directly discovered generated module artifacts for one target.
    fn require_target_module_artifacts(
        &self,
        discovered_modules: &[ModuleId],
        target_id: &TargetId,
        package_id: PackageId,
    ) -> LinkResult<Vec<ModuleId>> {
        let mut collector = crate::ArtifactRequirementCollector::new();
        let mut module_artifact_keys = Vec::new();

        // require exactly the directly discovered generated artifacts
        for module_id in discovered_modules.iter().copied() {
            let profile_id = self
                .program
                .profile_id_for_target(module_id, target_id)
                .ok_or_else(|| LinkError::Internal {
                    package: package_id,
                    message: format!("profile not found for target '{}'", target_id.name),
                })?;
            let result = self.require_module_artifact(module_id, profile_id, target_id);
            collector.try_collect(result);
            module_artifact_keys.push(module_id);
        }

        // yield while required generated artifacts are still pending
        if let Some(requirement) = collector.try_into_requirement() {
            return Err(LinkError::Yield { requirement });
        }

        module_artifact_keys.sort_unstable();
        module_artifact_keys.dedup();

        Ok(module_artifact_keys)
    }

    /// Link all modules for a target.
    pub(super) fn link_target(
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

        // discover the modules addressed by this target
        let discovered_modules =
            self.discover_target_modules(package_id, &package_path, target_id, &target)?;

        // require the generated artifacts needed for this target family
        let module_artifact_keys = if target.uses_js_generate_pipeline() {
            self.require_script_target_artifacts(
                &discovered_modules,
                &target,
                target_id,
                package_id,
            )?
        } else {
            self.require_target_module_artifacts(&discovered_modules, target_id, package_id)?
        };

        let package_dir = package_path
            .clone()
            .unwrap_or_else(|| self.program.cwd.clone());
        let assembly = self.assemble_target_assembly(
            &discovered_modules,
            &module_artifact_keys,
            &package_dir,
            root_dir.as_deref(),
            &target,
            target_id,
            package_id,
        )?;
        let output = self.package_output_from_target_assembly(
            &package_dir,
            root_dir.as_deref(),
            target_id,
            package_id,
            &target,
            assembly,
        )?;

        self.artifacts.publish(
            ArtifactKey::package_output(package_id, target_id.clone()),
            output,
        );

        Ok(())
    }

    /// Assemble one target from the required generated module artifacts.
    fn assemble_target_assembly(
        &self,
        entry_modules: &[ModuleId],
        module_ids: &[ModuleId],
        package_dir: &std::path::Path,
        root_dir: Option<&std::path::Path>,
        target: &destack_workspace::Target,
        target_id: &TargetId,
        package_id: PackageId,
    ) -> LinkResult<TargetAssembly> {
        // script targets assemble through the script linker
        if target.uses_js_generate_pipeline() {
            let assembly = self.assemble_script_target_assembly(
                entry_modules,
                module_ids,
                package_dir,
                root_dir,
                target,
                target_id,
                package_id,
            )?;

            return Ok(TargetAssembly::Script(assembly));
        }

        #[cfg(feature = "native-codegen")]
        {
            // binary targets assemble through the binary linker
            if target.uses_native_generate_pipeline() {
                use destack_artifact::EmitFormat;
                let assembly = match target.emit {
                    EmitFormat::Native => self.assemble_native_target_assembly(
                        entry_modules,
                        module_ids,
                        package_dir,
                        root_dir,
                        target,
                        target_id,
                        package_id,
                    )?,
                    EmitFormat::Wasm => self.assemble_wasm_target_assembly(
                        entry_modules,
                        module_ids,
                        package_dir,
                        root_dir,
                        target,
                        target_id,
                        package_id,
                    )?,
                    other => {
                        return Err(LinkError::Internal {
                            package: package_id,
                            message: format!(
                                "unsupported binary target assembly for '{other:?}' target '{}'",
                                target.name
                            ),
                        });
                    }
                };

                return Ok(TargetAssembly::Binary(assembly));
            }
        }

        // unsupported link target family
        Err(LinkError::Internal {
            package: package_id,
            message: format!(
                "unsupported target assembly for '{:?}' target '{}'",
                target.emit, target.name
            ),
        })
    }
}
