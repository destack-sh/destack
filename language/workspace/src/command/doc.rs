use std::path::PathBuf;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tspp_artifact::{ArtifactKey, IndexKind};
use tspp_doc::{Generator, PackageReference};
use tspp_repository::{ConditionSet, ExportKind, Package, Revision, TraceView};
use tspp_serde::Reflect;
use tspp_source::{ModuleId, ProfileId};

use super::common::{
    CommandEnvVar, CommandInput, CommandOptions, CommandRevision, CommandTargetOverrides,
    ManifestOverride, impl_command_input_options,
};
use super::context::CommandContext;
use super::outcome::CommandOutcome;
use super::{CommandError, CommandResult};

/// Options for the doc command.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect, Default)]
pub struct DocOptions {}

/// Request to generate documentation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct DocInput {
    /// Revision selected for documentation generation.
    pub revision: CommandRevision,
    /// Input sources for the command.
    pub inputs: Vec<CommandInput>,
    /// Whether destack.json should resolve inputs when none are provided.
    pub config_inputs: bool,
    /// Optional working directory for this command.
    pub cwd: Option<PathBuf>,
    /// Optional Destack manifest path override.
    pub manifest: Option<PathBuf>,
    /// Optional target name override.
    pub target: Option<String>,
    /// Optional target overrides.
    pub target_overrides: Option<CommandTargetOverrides>,
    /// Optional profile name override.
    pub profile: Option<String>,
    /// Optional environment overrides.
    pub env: Vec<CommandEnvVar>,
    /// Optional manifest overrides.
    pub overrides: Vec<ManifestOverride>,
    /// Whether the command should watch for changes.
    pub watch: bool,
    /// Whether the command should skip writes.
    pub dry_run: bool,
    /// Trace detail returned for this command.
    pub trace: Option<TraceView>,
}

impl_command_input_options!(DocInput {});

/// Payload for generated package documentation.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, Reflect)]
pub struct DocPayload {
    /// The generated checked package reference.
    pub reference: Option<PackageReference>,
}

impl CommandContext<'_> {
    /// Execute a doc command.
    pub(crate) async fn run_doc_command(
        &mut self,
        _options: &DocOptions,
    ) -> CommandResult<CommandOutcome<DocPayload>> {
        let inputs = self.resolve_command_inputs()?;
        let modules = self.resolve_modules(&inputs)?;
        let first_module = modules
            .first()
            .copied()
            .ok_or_else(|| CommandError::config("doc requires one package target"))?;
        let revision = self.revision();
        let package = self.documented_package(revision, &modules)?;
        let profile = self.selected_profile_id(revision, first_module)?;
        let selected_profile = self
            .repository
            .profile(revision, profile)
            .map_err(|error| error.to_string())?
            .ok_or_else(|| CommandError::config(format!("missing profile {profile:?}")))?;
        let public_modules =
            self.documented_modules(revision, &package, selected_profile.conditions())?;

        // check the complete package through the normal compiler graph
        let package_modules = self
            .repository
            .package_module_ids(revision, package.id)
            .map_err(|error| CommandError::internal(error.to_string()))?;
        let checked_keys = package_modules
            .iter()
            .map(|module| ArtifactKey::dir_checked(*module, profile))
            .collect::<Vec<_>>();
        self.complete(revision, &checked_keys).await?;

        // complete the public exports and checked state read by the generator
        let export_keys = public_modules
            .iter()
            .map(|(_, module)| ArtifactKey::module_index(*module, profile, IndexKind::Exports))
            .collect::<Vec<_>>();
        self.complete(revision, &export_keys).await?;
        let mut artifact_keys = checked_keys;
        artifact_keys.extend(export_keys);
        let (reference_modules, reference_keys) = self
            .complete_reference_artifacts(revision, profile, &package_modules)
            .await?;
        artifact_keys.extend(reference_keys);
        let diagnostics = self.command_diagnostics(revision, &artifact_keys)?;
        let exit_code = diagnostics.get_status_code();
        if exit_code != 0 {
            return Ok(
                CommandOutcome::new(diagnostics, exit_code, modules.len(), 1, 0)
                    .with_data(DocPayload::default()),
            );
        }

        // generate the stable package artifact from completed compiler state
        let reference = Generator::read(
            self.repository.as_ref(),
            revision,
            profile,
            reference_modules,
        )
        .map_err(|error| CommandError::internal(error.to_string()))?
        .generate(&package, public_modules)
        .map_err(|error| CommandError::internal(error.to_string()))?;
        let payload = DocPayload {
            reference: Some(reference),
        };

        Ok(CommandOutcome::new(diagnostics, exit_code, modules.len(), 1, 0).with_data(payload))
    }

    /// Complete the checked import closure read while rendering package declarations.
    async fn complete_reference_artifacts(
        &self,
        revision: Revision,
        profile: ProfileId,
        roots: &[ModuleId],
    ) -> CommandResult<(Vec<ModuleId>, Vec<ArtifactKey>)> {
        // walk the imports of the package roots
        let (modules, graph_keys) = self.import_closure(revision, profile, roots).await?;

        // retain every compiler phase composed while printing declarations
        let mut keys = modules
            .iter()
            .copied()
            .flat_map(|module| {
                [
                    ArtifactKey::dir_parsed(module),
                    ArtifactKey::dir_bound(module, profile),
                    ArtifactKey::dir_expanded(module, profile),
                    ArtifactKey::dir_declared(module, profile),
                    ArtifactKey::dir_elaborated(module, profile),
                    ArtifactKey::dir_checked(module, profile),
                ]
            })
            .collect::<Vec<_>>();
        self.provide(revision, &keys).await?;

        // complete member and export indexes for referenced types and namespaces
        let index_keys = modules
            .iter()
            .flat_map(|module| {
                [
                    ArtifactKey::module_index(*module, profile, IndexKind::Members),
                    ArtifactKey::module_index(*module, profile, IndexKind::Exports),
                ]
            })
            .collect::<Vec<_>>();
        self.complete(revision, &index_keys).await?;
        keys.extend(index_keys);
        keys.extend(graph_keys);

        Ok((modules, keys))
    }

    /// Return the single package selected for documentation.
    fn documented_package(
        &self,
        revision: Revision,
        modules: &[ModuleId],
    ) -> CommandResult<Arc<Package>> {
        let first_module = modules
            .first()
            .copied()
            .ok_or_else(|| CommandError::config("doc requires one package target"))?;
        let first = self
            .repository
            .module(revision, first_module)
            .map_err(|error| error.to_string())?
            .ok_or_else(|| CommandError::source(format!("missing module {first_module:?}")))?;

        // require every selected root to belong to one package
        for module_id in &modules[1..] {
            let module = self
                .repository
                .module(revision, *module_id)
                .map_err(|error| error.to_string())?
                .ok_or_else(|| CommandError::source(format!("missing module {module_id:?}")))?;
            if module.package_id != first.package_id {
                return Err(CommandError::config(
                    "doc accepts source roots from exactly one package",
                ));
            }
        }

        self.repository
            .package(revision, first.package_id)
            .map_err(|error| CommandError::internal(error.to_string()))?
            .ok_or_else(|| CommandError::config(format!("missing package {:?}", first.package_id)))
    }

    /// Resolve active public module exports in manifest order.
    fn documented_modules(
        &self,
        revision: Revision,
        package: &Package,
        conditions: &ConditionSet,
    ) -> CommandResult<Vec<(String, ModuleId)>> {
        let root = package.path.as_deref().ok_or_else(|| {
            CommandError::config("documented package must have a filesystem root")
        })?;
        let name = package
            .name
            .as_deref()
            .ok_or_else(|| CommandError::config("documented package must have a name"))?;
        let mut modules = Vec::new();

        // retain active source module exports only
        for (key, export) in &package.exports {
            if export.kind != ExportKind::Module || !export.matches(conditions) {
                continue;
            }
            let path = root.join(export.path.trim_start_matches("./"));
            let module = self
                .repository
                .module_id_for_path(revision, &path)
                .map_err(|error| error.to_string())?
                .ok_or_else(|| {
                    CommandError::config(format!(
                        "package export '{key}' does not resolve to {}",
                        path.display()
                    ))
                })?;
            let specifier = package_specifier(name, package.is_builtin, key)?;
            modules.push((specifier, module));
        }

        if modules.is_empty() {
            return Err(CommandError::config(
                "documented package has no active module exports",
            ));
        }

        Ok(modules)
    }
}

/// Return one public import specifier from its package export key.
fn package_specifier(name: &str, is_builtin: bool, key: &str) -> CommandResult<String> {
    let suffix = match key {
        "." => "",
        key if key.starts_with("./") => &key[2..],
        _ => {
            return Err(CommandError::config(format!(
                "invalid package export key '{key}'"
            )));
        }
    };
    let separator = if is_builtin || name == "tspp" {
        ":"
    } else {
        "/"
    };

    if suffix.is_empty() {
        Ok(name.to_string())
    } else {
        Ok(format!("{name}{separator}{suffix}"))
    }
}
