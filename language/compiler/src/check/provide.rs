use destack_core::FxIndexSet;
use std::iter;
use std::sync::Arc;

use destack_artifact::{
    ArtifactDependencySet, ArtifactKey, ArtifactPayload, ArtifactProjectionKey, ArtifactSidecar,
};
use destack_repository::{ProfileId, ProviderContext, ProviderError};
use destack_source::{Content, ModuleId};

use crate::check::{AnnotatedSource, CheckState};
use crate::{Compiler, CompilerError, CompilerResult};

/// Return the modules one module references, or None while their resolve
/// stages are still building.
fn referenced_modules(
    artifacts: &destack_repository::ArtifactReader<'_>,
    module: ModuleId,
    profile: ProfileId,
) -> CompilerResult<Option<FxIndexSet<ModuleId>>> {
    let resolved = match artifacts.dir_resolved(module, profile) {
        Ok(resolved) => resolved,
        Err(ProviderError::Blocked { .. }) => return Ok(None),
        Err(error) => return Err(error.into()),
    };
    let global = match artifacts.global_environment_content(profile) {
        Ok(global) => global,
        Err(ProviderError::Blocked { .. }) => return Ok(None),
        Err(error) => return Err(error.into()),
    };

    let mut references = resolved.target_modules().collect::<FxIndexSet<_>>();
    references.extend(global.implicit_modules());
    references.shift_remove(&module);

    Ok(Some(references))
}

impl Compiler {
    /// Collect inputs for one declared DIR module.
    pub(crate) fn collect_dir_declared(
        &self,
        module: ModuleId,
        profile: ProfileId,
        context: &dyn ProviderContext,
    ) -> CompilerResult<ArtifactDependencySet> {
        let mut dependencies = ArtifactDependencySet::default();
        self.require_own_stages(module, profile, &mut dependencies);
        dependencies.require_projection(
            ArtifactKey::global_environment(profile),
            ArtifactProjectionKey::Content,
        );

        // observe package config for check options
        let repository_module = self.module(context.revision(), module)?;
        self.observe_package_config(context, repository_module.package_id, &mut dependencies)?;

        // require the surface stages of referenced modules
        let artifacts = self.artifact_reader(context);
        let Some(references) = referenced_modules(&artifacts, module, profile)? else {
            dependencies.mark_partial();

            return Ok(dependencies);
        };
        for reference in references {
            dependencies.require_projection(
                ArtifactKey::dir_bound(reference, profile),
                ArtifactProjectionKey::Content,
            );
            dependencies.require_projection(
                ArtifactKey::dir_expanded(reference, profile),
                ArtifactProjectionKey::Content,
            );
            dependencies.require_projection(
                ArtifactKey::dir_resolved(reference, profile),
                ArtifactProjectionKey::Content,
            );
        }

        Ok(dependencies)
    }

    /// Provide one declared DIR module.
    pub(crate) fn provide_dir_declared(
        &self,
        module: ModuleId,
        profile: ProfileId,
        context: &dyn ProviderContext,
    ) -> CompilerResult<ArtifactPayload> {
        let artifacts = self.artifact_reader(context);
        let global = artifacts
            .global_environment_content(profile)
            .map_err(CompilerError::from)?;
        let environment = self.environment(context.revision())?;
        let repository_module = self.module(context.revision(), module)?;
        let options = self.workspace_compiler_options(context, repository_module.as_ref())?;

        // declare the module without walking callable bodies
        let mut check = CheckState::new(
            self,
            context,
            &artifacts,
            profile,
            global,
            environment,
            module,
            false,
            options.emit_events || context.emit_events(),
        )?;
        check.solve()?;

        // emit solver counters for the declaration pass
        let stats = check.stats();
        context.emit_counter("solve.variables", stats.variables as u64);
        context.emit_counter("solve.constraints", stats.constraints as u64);
        context.emit_counter("solve.types", stats.types as u64);

        // write declared DIR tables and report the pass's diagnostics
        let (declared, diagnostics) = check.write_declared(module)?;
        context.emit_diagnostics(diagnostics);

        Ok(ArtifactPayload::DirDeclared(Arc::new(declared)))
    }

    /// Collect inputs for one checked DIR module.
    pub(crate) fn collect_dir_checked(
        &self,
        module: ModuleId,
        profile: ProfileId,
        context: &dyn ProviderContext,
    ) -> CompilerResult<ArtifactDependencySet> {
        let mut dependencies = ArtifactDependencySet::default();
        self.require_own_stages(module, profile, &mut dependencies);
        dependencies.require_projection(
            ArtifactKey::global_environment(profile),
            ArtifactProjectionKey::Content,
        );

        // seed the checking pass from the module's own declared artifact
        dependencies.require(ArtifactKey::dir_declared(module, profile));

        // observe package config for check options
        let repository_module = self.module(context.revision(), module)?;
        self.observe_package_config(context, repository_module.package_id, &mut dependencies)?;

        // require declared artifacts of direct imports and implicit globals
        let artifacts = self.artifact_reader(context);
        let Some(references) = referenced_modules(&artifacts, module, profile)? else {
            dependencies.mark_partial();

            return Ok(dependencies);
        };
        for import in references {
            dependencies.require_projection(
                ArtifactKey::dir_declared(import, profile),
                ArtifactProjectionKey::Declared,
            );
            dependencies.require_projection(
                ArtifactKey::dir_bound(import, profile),
                ArtifactProjectionKey::Content,
            );
            dependencies.require_projection(
                ArtifactKey::dir_expanded(import, profile),
                ArtifactProjectionKey::Content,
            );
            dependencies.require_projection(
                ArtifactKey::dir_resolved(import, profile),
                ArtifactProjectionKey::Content,
            );
        }

        Ok(dependencies)
    }

    /// Provide one checked DIR module.
    pub(crate) fn provide_dir_checked(
        &self,
        module: ModuleId,
        profile: ProfileId,
        context: &dyn ProviderContext,
    ) -> CompilerResult<ArtifactPayload> {
        let artifacts = self.artifact_reader(context);
        let global = artifacts
            .global_environment_content(profile)
            .map_err(CompilerError::from)?;
        let environment = self.environment(context.revision())?;
        let repository_module = self.module(context.revision(), module)?;
        let options = self.workspace_compiler_options(context, repository_module.as_ref())?;

        // check the module's declarations and bodies
        let emit_events = options.emit_events || context.emit_events();
        let mut check = CheckState::new(
            self,
            context,
            &artifacts,
            profile,
            global,
            environment,
            module,
            true,
            emit_events,
        )?;
        check.solve()?;
        check.report_constant_conditions()?;

        // emit solver counters and optional trace sidecars
        let stats = check.stats();
        context.emit_counter("solve.variables", stats.variables as u64);
        context.emit_counter("solve.constraints", stats.constraints as u64);
        context.emit_counter("solve.types", stats.types as u64);
        context.emit_counter("solve.bounds", stats.bounds as u64);
        context.emit_counter("solve.decisions", stats.decisions as u64);
        if options.emit_stats {
            let content = stats.render_metadata();
            context.emit_sidecar(check_sidecar("metadata", content));
        }
        let events = emit_events.then(|| check.events());

        // close solved state, then render checked type annotations so
        //  the echo reflects write-derived values like parameter variance
        let closed = check.close_checked(module)?;
        if options.emit_checked_types {
            for source in check.render_annotated_sources()? {
                context.emit_sidecar(annotated_sidecar(source));
            }
        }
        if let Some(events) = events {
            context.emit_sidecar(check_sidecar("events", events.render()));
        }

        // write checked DIR tables and report the pass's diagnostics
        let (checked, diagnostics) = check.write_checked(closed)?;
        context.emit_diagnostics(diagnostics);

        Ok(ArtifactPayload::DirChecked(Arc::new(checked)))
    }

    /// Require one module's own DIR stage payloads.
    fn require_own_stages(
        &self,
        module: ModuleId,
        profile: ProfileId,
        dependencies: &mut ArtifactDependencySet,
    ) {
        dependencies.require(ArtifactKey::dir_parsed(module));
        dependencies.require(ArtifactKey::dir_bound(module, profile));
        dependencies.require(ArtifactKey::dir_resolved(module, profile));
        dependencies.require(ArtifactKey::dir_expanded(module, profile));
    }
}

/// Return one check-phase sidecar.
fn check_sidecar(name: &str, content: String) -> ArtifactSidecar {
    ArtifactSidecar::new(
        name,
        iter::once(("phase", "check")),
        Content::Text { content },
    )
}

/// Return one annotated source sidecar for one member module.
fn annotated_sidecar(source: AnnotatedSource) -> ArtifactSidecar {
    ArtifactSidecar::new(
        "annotated",
        [
            ("phase", "check".to_string()),
            ("module", source.module.uri.to_string()),
        ],
        Content::Text {
            content: source.content,
        },
    )
}
