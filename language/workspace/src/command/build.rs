use std::collections::{BTreeSet, HashSet};
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tspp_artifact::{ArtifactKey, ArtifactReference};
use tspp_repository::{Revision, TraceView};
use tspp_serde::Reflect;
use tspp_source::{ModuleId, PackageId, ProductId};

use super::CommandResult;
use super::common::{
    CommandEnvVar, CommandInput, CommandOptions, CommandRevision, CommandTargetOverrides,
    ManifestOverride, impl_command_input_options,
};
use super::context::{CommandContext, SelectedTarget};
use super::outcome::CommandOutcome;

/// Request to build target artifacts.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct BuildInput {
    /// Revision selected for this build.
    pub revision: CommandRevision,
    /// Input sources for the command.
    pub inputs: Vec<CommandInput>,
    /// Whether package.json should resolve inputs when none are provided.
    pub config_inputs: bool,
    /// Optional working directory for this command.
    pub cwd: Option<PathBuf>,
    /// Optional manifest path override.
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
    #[serde(default)]
    pub trace: Option<TraceView>,
    /// Product name selected for this build.
    pub product: Option<String>,
    /// Build outputs requested by the caller.
    pub outputs: BuildOutputs,
}

impl_command_input_options!(BuildInput {
    product: None,
    outputs: BuildOutputs::default(),
});

/// Build output families requested by a caller.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct BuildOutputs {
    /// Return product artifact refs.
    pub products: bool,
    /// Return bundle artifact refs.
    pub bundles: bool,
    /// Return program artifact refs.
    pub programs: bool,
    /// Return per-module asset artifact refs.
    pub assets: bool,
}

impl Default for BuildOutputs {
    /// Return the default build output families.
    fn default() -> Self {
        Self {
            products: true,
            bundles: true,
            programs: true,
            assets: false,
        }
    }
}

/// Payload for build command output.
#[derive(Debug, Clone, Default, Serialize, Deserialize, Reflect)]
pub struct BuildPayload {
    /// Product artifacts produced by this build.
    pub products: Vec<ArtifactReference>,
    /// Bundle artifacts produced by this build.
    pub bundles: Vec<ArtifactReference>,
    /// Program artifacts produced by this build.
    pub programs: Vec<ArtifactReference>,
    /// Per-module asset artifacts produced by this build.
    pub assets: Vec<ArtifactReference>,
}

impl CommandContext<'_> {
    /// Execute a build command.
    pub(crate) async fn run_build_command(
        &mut self,
        input: &BuildInput,
    ) -> CommandResult<CommandOutcome<BuildPayload>> {
        // resolve inputs for the command
        let inputs = self.resolve_command_inputs()?;
        let modules = self.resolve_modules(&inputs)?;
        let revision = self.revision();

        // resolve the target configuration for each module
        let target_overrides = self.common.target_overrides.as_ref();
        let mut module_targets = Vec::new();
        let mut target_ids = HashSet::new();
        for module_id in &modules {
            let target = self.resolve_target_for_module(revision, *module_id, target_overrides)?;
            target_ids.insert(target.id);
            module_targets.push((*module_id, target));
        }

        // collect build roots
        let mut artifact_keys = Vec::new();
        for (module_id, target) in &module_targets {
            artifact_keys.push(target.build_root(*module_id));
        }
        let product_keys =
            self.product_roots(revision, &module_targets, input.product.as_deref())?;
        artifact_keys.extend(product_keys.iter().copied());

        // complete the requested build roots through diagnostic failures
        self.complete(revision, &artifact_keys).await?;

        // collect requested artifact refs
        let mut payload = BuildPayload::default();
        for key in &artifact_keys {
            self.push_build_artifact(revision, *key, input.outputs, &mut payload)?;
        }

        // collect diagnostics and counts
        let diagnostics = self.command_diagnostics(revision, &artifact_keys)?;
        let exit_code = diagnostics.get_status_code();
        let profile_count = module_targets
            .iter()
            .map(|(module_id, target)| self.target_profile_id(revision, *module_id, target.id))
            .collect::<Result<HashSet<_>, _>>()?
            .len();

        Ok(CommandOutcome::new(
            diagnostics,
            exit_code,
            modules.len(),
            profile_count,
            target_ids.len(),
        )
        .with_data(payload))
    }

    /// Return product roots selected by this build.
    fn product_roots(
        &self,
        revision: Revision,
        module_targets: &[(ModuleId, SelectedTarget)],
        product: Option<&str>,
    ) -> CommandResult<Vec<ArtifactKey>> {
        let mut package_ids = BTreeSet::new();
        for (_module_id, target) in module_targets {
            package_ids.insert(target.id.package_id());
        }

        let mut keys = Vec::new();
        for package_id in package_ids {
            if let Some(product) = product {
                keys.push(product_key(package_id, product));
            } else if let Some(product) = self
                .repository
                .package_default_product(revision, package_id)
                .map_err(|error| error.to_string())?
            {
                keys.push(product_key(package_id, &product));
            }
        }

        Ok(keys)
    }

    /// Push one requested build artifact ref into the build payload.
    fn push_build_artifact(
        &self,
        revision: Revision,
        key: ArtifactKey,
        outputs: BuildOutputs,
        payload: &mut BuildPayload,
    ) -> CommandResult<()> {
        let Some(version) = self
            .repository
            .artifact_version(revision, &key)
            .map_err(|error| error.to_string())?
        else {
            return Err(format!("missing build artifact: {key:?}").into());
        };
        let artifact = ArtifactReference { key, version };

        match key {
            ArtifactKey::Product { .. } if outputs.products => payload.products.push(artifact),
            ArtifactKey::Bundle { .. } if outputs.bundles => payload.bundles.push(artifact),
            ArtifactKey::Program { .. } if outputs.programs => payload.programs.push(artifact),
            ArtifactKey::Asset { .. } if outputs.assets => payload.assets.push(artifact),
            _ => {}
        }

        Ok(())
    }
}

impl SelectedTarget {
    /// Return the build root for one module.
    fn build_root(&self, module_id: ModuleId) -> ArtifactKey {
        // select the module asset root
        if self.target.emits_per_module_output() {
            return ArtifactKey::asset(module_id, self.id);
        }

        // select the linked Program root
        if self.target.output == tspp_artifact::Output::Program {
            return ArtifactKey::program(self.id.package_id(), self.id);
        }

        // select the package bundle root
        ArtifactKey::bundle(self.id.package_id(), self.id)
    }
}

/// Build one product artifact key.
fn product_key(package: PackageId, product: &str) -> ArtifactKey {
    ArtifactKey::product(package, ProductId::new(package, product))
}
