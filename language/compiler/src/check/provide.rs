use destack_core::FxIndexSet;
use std::iter;
use std::sync::Arc;

use destack_artifact::{
    ArtifactDependency, ArtifactDependencySet, ArtifactKey, ArtifactPayload, ArtifactProjectionKey,
    ArtifactSidecar, DiagnosticAnchor, DiagnosticContext, DiagnosticDisplay, DiagnosticError,
    DiagnosticLike, DiagnosticRecord,
};
use destack_repository::{
    ArtifactAttemptRecorder, ArtifactBase, ProfileId, ProviderContext, ProviderError, Revision,
};
use destack_source::{Content, DiagnosticLabel, ModuleId};

use crate::check::{AnnotatedSource, CheckState};
use crate::{Compiler, CompilerError, CompilerResult};

/// The foreign modules one module's check reads: resolution targets with
/// their full stage fan, and modules the environment digest covers.
struct ReferencedModules {
    /// Modules referenced by resolution targets.
    targets: FxIndexSet<ModuleId>,
    /// Modules covered by the environment digest.
    digested: Vec<ModuleId>,
}

/// Return the modules one module references, or None while their resolve
/// stages are still building.
fn referenced_modules(
    artifacts: &destack_repository::ArtifactReader<'_>,
    module: ModuleId,
    profile: ProfileId,
) -> CompilerResult<Option<ReferencedModules>> {
    let resolved = match artifacts.dir_resolved(module, profile) {
        Ok(resolved) => resolved,
        Err(ProviderError::Blocked { .. }) => return Ok(None),
        Err(error) => return Err(error.into()),
    };
    let digest = match artifacts.global_environment_digest(profile) {
        Ok(digest) => digest,
        Err(ProviderError::Blocked { .. }) => return Ok(None),
        Err(error) => return Err(error.into()),
    };

    let mut targets = resolved.target_modules().collect::<FxIndexSet<_>>();
    targets.shift_remove(&module);
    let digested = digest
        .modules
        .iter()
        .map(|digested| digested.module)
        .filter(|digested| *digested != module)
        .collect();

    Ok(Some(ReferencedModules { targets, digested }))
}

/// Provider context that skips observations the environment digest covers.
///
/// Reads of digested modules' bound, expanded, and resolved stages are
/// already represented by the digest's content projection: the digest
/// changes exactly when one covered stage content does.
struct DigestContext<'a> {
    /// The wrapped provider context.
    inner: &'a dyn ProviderContext,
    /// The modules the digest covers.
    covered: FxIndexSet<ModuleId>,
}

impl DigestContext<'_> {
    /// Return whether the digest stands for one observed dependency.
    fn is_covered(&self, dependency: &ArtifactDependency) -> bool {
        let key = match dependency {
            ArtifactDependency::Artifact(version) => version.key,
            ArtifactDependency::Projection(projection) => projection.projection().artifact,
            ArtifactDependency::Source(_) => return false,
        };
        let covered_stage = matches!(
            key,
            ArtifactKey::DirBound { .. }
                | ArtifactKey::DirExpanded { .. }
                | ArtifactKey::DirResolved { .. }
        );

        covered_stage
            && key
                .module_id()
                .is_some_and(|module| self.covered.contains(&module))
    }
}

impl DiagnosticContext for DigestContext<'_> {
    fn label(
        &self,
        anchor: &DiagnosticAnchor,
        message: Option<String>,
    ) -> Result<DiagnosticLabel, DiagnosticError> {
        self.inner.label(anchor, message)
    }

    fn display(&self, display: DiagnosticDisplay) -> Result<String, DiagnosticError> {
        self.inner.display(display)
    }
}

impl ProviderContext for DigestContext<'_> {
    fn revision(&self) -> Revision {
        self.inner.revision()
    }

    fn artifact_key(&self) -> ArtifactKey {
        self.inner.artifact_key()
    }

    fn artifact_base(&self) -> Option<&ArtifactBase> {
        self.inner.artifact_base()
    }

    fn artifact_dependencies(&self) -> Option<&[ArtifactDependency]> {
        self.inner.artifact_dependencies()
    }

    fn emit_events(&self) -> bool {
        self.inner.emit_events()
    }

    fn recorder(&self) -> Option<&ArtifactAttemptRecorder> {
        self.inner.recorder()
    }

    fn observe(&self, dependency: ArtifactDependency) {
        if self.is_covered(&dependency) {
            return;
        }

        self.inner.observe(dependency);
    }

    fn record_blocked(&self, artifact_key: ArtifactKey) {
        self.inner.record_blocked(artifact_key);
    }

    fn emit_diagnostics(&self, diagnostics: Vec<DiagnosticRecord>) {
        self.inner.emit_diagnostics(diagnostics);
    }

    fn emit_sidecar(&self, sidecar: ArtifactSidecar) {
        self.inner.emit_sidecar(sidecar);
    }

    fn emit(&self, diagnostic: &dyn DiagnosticLike) -> Result<(), DiagnosticError> {
        self.inner.emit(diagnostic)
    }
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

        // require the digest, which stands for every covered stage content
        dependencies.require_projection(
            ArtifactKey::global_environment_digest(profile),
            ArtifactProjectionKey::Content,
        );

        // observe package config for check options
        let repository_module = self.module(context.revision(), module)?;
        self.observe_package_config(context, repository_module.package_id, &mut dependencies)?;

        // require the stage contents of resolution targets
        let artifacts = self.artifact_reader(context);
        let Some(references) = referenced_modules(&artifacts, module, profile)? else {
            dependencies.mark_partial();

            return Ok(dependencies);
        };
        for reference in references.targets {
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
        // cover digested reads through the digest's content projection
        let plain = self.artifact_reader(context);
        let covered = plain
            .global_environment_digest(profile)
            .map_err(CompilerError::from)?
            .modules
            .iter()
            .map(|digested| digested.module)
            .collect::<FxIndexSet<_>>();
        let digest = DigestContext {
            inner: context,
            covered,
        };
        let context: &dyn ProviderContext = &digest;
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

        // require the digest, which stands for every covered stage content
        dependencies.require_projection(
            ArtifactKey::global_environment_digest(profile),
            ArtifactProjectionKey::Content,
        );

        // require declared artifacts of direct imports and implicit globals
        let artifacts = self.artifact_reader(context);
        let Some(references) = referenced_modules(&artifacts, module, profile)? else {
            dependencies.mark_partial();

            return Ok(dependencies);
        };
        for import in references.targets {
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

        // require declared judgments for each digest-covered module
        for import in references.digested {
            dependencies.require_projection(
                ArtifactKey::dir_declared(import, profile),
                ArtifactProjectionKey::Declared,
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
        // cover digested reads through the digest's content projection
        let plain = self.artifact_reader(context);
        let covered = plain
            .global_environment_digest(profile)
            .map_err(CompilerError::from)?
            .modules
            .iter()
            .map(|digested| digested.module)
            .collect::<FxIndexSet<_>>();
        let digest = DigestContext {
            inner: context,
            covered,
        };
        let context: &dyn ProviderContext = &digest;
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
