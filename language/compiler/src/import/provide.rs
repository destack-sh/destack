use std::iter;
use std::sync::Arc;

use destack_artifact::{
    ArtifactDependency, ArtifactDependencySet, ArtifactKey, ArtifactPayload, ArtifactProjectionKey,
    ArtifactSidecar, GlobalEnvironment, GlobalEnvironmentDigest, ModuleDigest, SourceDependency,
};
use destack_dir as dir;
use destack_repository::{ProfileId, ProviderContext};
use destack_source::{Content, ModuleId};

use crate::import::ImportState;
use crate::{Compiler, CompilerError, CompilerResult};

impl Compiler {
    /// Collect inputs for the global environment of one profile.
    pub(crate) fn collect_global_environment(
        &self,
        profile: ProfileId,
        context: &dyn ProviderContext,
    ) -> CompilerResult<ArtifactDependencySet> {
        let mut dependencies = ArtifactDependencySet::default();

        // language item modules back the language environment
        for module in self.repository.builtin_module_ids(context.revision())? {
            dependencies.require(ArtifactKey::dir_parsed(module));
            dependencies.require(ArtifactKey::dir_bound(module, profile));
        }

        // global module export surfaces back the global targets
        let mut globals = self.load_global_module_ids(profile, context)?;
        if let Some((module, _)) = self.tree_builder_reference(profile, context)? {
            globals.push(module);
        }
        let artifacts = self.artifact_reader(context);
        self.collect_exported_modules(globals, profile, &artifacts, &mut dependencies)?;

        Ok(dependencies)
    }

    /// Build the global environment for one profile.
    pub(crate) fn provide_global_environment(
        &self,
        profile: ProfileId,
        context: &dyn ProviderContext,
    ) -> CompilerResult<ArtifactPayload> {
        // discover environment inputs
        let globals = self.load_global_module_ids(profile, context)?;
        let language_modules = self.repository.builtin_module_ids(context.revision())?;

        // build language environment for profile
        let artifacts = self.artifact_reader(context);
        let language = self.build_language_environment(profile, &artifacts, &language_modules)?;
        let global_targets = self.build_global_targets(profile, &artifacts, &globals)?;
        let tree = self.resolve_tree_builder(profile, context, &artifacts)?;
        let environment = GlobalEnvironment {
            language,
            globals,
            global_targets_by_key: global_targets,
            tree,
        };

        Ok(ArtifactPayload::GlobalEnvironment(Arc::new(environment)))
    }

    /// Collect inputs for the environment digest of one profile.
    pub(crate) fn collect_global_environment_digest(
        &self,
        profile: ProfileId,
        context: &dyn ProviderContext,
    ) -> CompilerResult<ArtifactDependencySet> {
        let mut dependencies = ArtifactDependencySet::default();
        dependencies.require_projection(
            ArtifactKey::global_environment(profile),
            ArtifactProjectionKey::Content,
        );

        // require every implicit stage content the digest follows
        let artifacts = self.artifact_reader(context);
        let environment = match artifacts.global_environment(profile) {
            Ok(environment) => environment,
            Err(destack_repository::ProviderError::Blocked { .. }) => {
                dependencies.mark_partial();

                return Ok(dependencies);
            }
            Err(error) => return Err(error.into()),
        };
        for module in environment.implicit_modules() {
            dependencies.require_projection(
                ArtifactKey::dir_bound(module, profile),
                ArtifactProjectionKey::Content,
            );
            dependencies.require_projection(
                ArtifactKey::dir_expanded(module, profile),
                ArtifactProjectionKey::Content,
            );
            dependencies.require_projection(
                ArtifactKey::dir_resolved(module, profile),
                ArtifactProjectionKey::Content,
            );
        }

        Ok(dependencies)
    }

    /// Digest the implicit module contents for one profile.
    pub(crate) fn provide_global_environment_digest(
        &self,
        profile: ProfileId,
        context: &dyn ProviderContext,
    ) -> CompilerResult<ArtifactPayload> {
        let artifacts = self.artifact_reader(context);
        let environment = artifacts
            .global_environment(profile)
            .map_err(CompilerError::from)?;

        // digest each implicit module's stage contents into one fingerprint
        let mut modules = Vec::new();
        for module in environment.implicit_modules() {
            let stages = [
                self.content_fingerprint(context, ArtifactKey::dir_bound(module, profile))?,
                self.content_fingerprint(context, ArtifactKey::dir_expanded(module, profile))?,
                self.content_fingerprint(context, ArtifactKey::dir_resolved(module, profile))?,
            ];
            modules.push(ModuleDigest {
                module,
                content: destack_artifact::ArtifactProjectionFingerprint::new(&stages),
            });
        }

        Ok(ArtifactPayload::GlobalEnvironmentDigest(Arc::new(
            GlobalEnvironmentDigest { modules },
        )))
    }

    /// Return the content projection fingerprint of one digested artifact.
    fn content_fingerprint(
        &self,
        context: &dyn ProviderContext,
        key: ArtifactKey,
    ) -> CompilerResult<destack_artifact::ArtifactProjectionFingerprint> {
        let table = self.repository.artifact_table();
        let version = self
            .repository
            .artifact_version(context.revision(), &key)
            .map_err(|error| CompilerError::Internal {
                message: format!("failed to resolve digested artifact {key:?}: {error}"),
            })?
            .ok_or_else(|| CompilerError::Internal {
                message: format!("a digested artifact without a bound version: {key:?}"),
            })?;
        let projection =
            destack_artifact::ArtifactProjection::new(key, ArtifactProjectionKey::Content);

        table
            .projection_fingerprint(&version, &projection)
            .ok_or_else(|| CompilerError::Internal {
                message: format!("a digested artifact without a content projection: {key:?}"),
            })
    }

    /// Collect inputs for imported DIR of one module.
    pub(crate) fn collect_dir_imported(
        &self,
        module: ModuleId,
        profile: ProfileId,
    ) -> CompilerResult<ArtifactDependencySet> {
        let mut dependencies = ArtifactDependencySet::default();
        dependencies.require(ArtifactKey::dir_parsed(module));
        dependencies.require(ArtifactKey::dir_bound(module, profile));

        Ok(dependencies)
    }

    /// Build imported DIR for one module.
    pub(crate) fn provide_dir_imported(
        &self,
        module: ModuleId,
        profile: ProfileId,
        context: &dyn ProviderContext,
    ) -> CompilerResult<ArtifactPayload> {
        // load provider inputs
        let profile_state = self.profile(context.revision(), profile)?;
        let artifacts = self.artifact_reader(context);
        let parsed = artifacts.dir_parsed(module).map_err(CompilerError::from)?;
        let bound = artifacts
            .dir_bound(module, profile)
            .map_err(CompilerError::from)?;
        let module = self.module(context.revision(), module)?;
        let package = self.package(context.revision(), module.package_id)?;
        let environment = self.environment(context.revision())?;

        // observe the package set backing dependency discovery
        let packages = self
            .repository
            .package_ids(context.revision())
            .map_err(|error| CompilerError::Internal {
                message: format!("failed to load package ids: {error}"),
            })?;
        context.observe(ArtifactDependency::Source(SourceDependency::packages(
            &packages,
        )));

        // build local module table
        let view = dir::View::new(&parsed.tree);
        let mut state = ImportState::new(
            context.revision(),
            module.as_ref(),
            package.as_ref(),
            environment.as_ref(),
            profile_state.conditions(),
            context,
            &profile_state.key,
            self.strings(),
            view,
        );
        self.collect_modules(&mut state, &bound.roots)?;
        let stats = state.stats;
        let (imported, diagnostics) = state.finish();
        context.emit_sidecar(ArtifactSidecar::new(
            "metadata",
            iter::once(("phase", "import")),
            Content::Text {
                content: stats.render_metadata(),
            },
        ));
        for diagnostic in diagnostics {
            self.emit_diagnostic(context, diagnostic)?;
        }

        Ok(ArtifactPayload::DirImported(Arc::new(imported)))
    }
}
