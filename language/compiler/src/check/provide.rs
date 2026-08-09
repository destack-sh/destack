use destack_core::FxIndexSet;
use std::iter;
use std::sync::Arc;

use destack_artifact::{
    ArtifactDependencySet, ArtifactKey, ArtifactPayload, ArtifactProjectionKey, ArtifactSidecar,
    EnvironmentDeclared,
};
use destack_dir as dir;
use destack_repository::{ProfileId, ProviderContext, ProviderError};
use destack_source::{Content, ModuleId};

use crate::check::{AnnotatedSource, CheckState, Pass};
use crate::{Compiler, CompilerError, CompilerResult};

/// The foreign modules one module's check reads through resolution targets.
struct ReferencedModules {
    /// Modules referenced by resolution targets.
    targets: FxIndexSet<ModuleId>,
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
    let mut targets = resolved.target_modules().collect::<FxIndexSet<_>>();
    targets.shift_remove(&module);

    Ok(Some(ReferencedModules { targets }))
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
        dependencies.require(ArtifactKey::dir_parsed(module));
        dependencies.require(ArtifactKey::dir_bound(module, profile));
        dependencies.require(ArtifactKey::dir_resolved(module, profile));
        dependencies.require(ArtifactKey::dir_expanded(module, profile));
        dependencies.require_projection(
            ArtifactKey::environment_bound(profile),
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
        let artifacts = self.artifact_reader(context);
        let global = artifacts
            .environment_bound_content(profile)
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
            None,
            environment,
            module,
            Pass::Declare,
            options.emit_events || context.emit_events(),
        )?;
        check.run_declare()?;

        // emit solver counters for the declaration pass
        let stats = check.stats();
        context.emit_counter("solve.variables", stats.variables as u64);
        context.emit_counter("solve.constraints", stats.constraints as u64);

        // package declared DIR tables and report the pass's diagnostics
        let (declared, diagnostics) = check.finish_declare(module)?;
        context.emit_diagnostics(diagnostics);

        Ok(ArtifactPayload::DirDeclared(Arc::new(declared)))
    }

    /// Collect inputs for one elaborated DIR module.
    pub(crate) fn collect_dir_elaborated(
        &self,
        module: ModuleId,
        profile: ProfileId,
        context: &dyn ProviderContext,
    ) -> CompilerResult<ArtifactDependencySet> {
        let mut dependencies = ArtifactDependencySet::default();
        dependencies.require(ArtifactKey::dir_parsed(module));
        dependencies.require(ArtifactKey::dir_bound(module, profile));
        dependencies.require(ArtifactKey::dir_resolved(module, profile));
        dependencies.require(ArtifactKey::dir_expanded(module, profile));
        dependencies.require(ArtifactKey::dir_declared(module, profile));
        dependencies.require_projection(
            ArtifactKey::environment_bound(profile),
            ArtifactProjectionKey::Content,
        );
        dependencies.require_projection(
            ArtifactKey::environment_declared(profile),
            ArtifactProjectionKey::Content,
        );

        // observe package config for check options
        let repository_module = self.module(context.revision(), module)?;
        self.observe_package_config(context, repository_module.package_id, &mut dependencies)?;

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

        Ok(dependencies)
    }

    /// Provide one elaborated DIR module.
    pub(crate) fn provide_dir_elaborated(
        &self,
        module: ModuleId,
        profile: ProfileId,
        context: &dyn ProviderContext,
    ) -> CompilerResult<ArtifactPayload> {
        let artifacts = self.artifact_reader(context);
        let global = artifacts
            .environment_bound_content(profile)
            .map_err(CompilerError::from)?;
        let declared_environment = artifacts
            .environment_declared(profile)
            .map_err(CompilerError::from)?;
        let environment = self.environment(context.revision())?;
        let repository_module = self.module(context.revision(), module)?;
        let options = self.workspace_compiler_options(context, repository_module.as_ref())?;

        // flatten the module's declared owners
        let mut check = CheckState::new(
            self,
            context,
            &artifacts,
            profile,
            global,
            Some(declared_environment),
            environment,
            module,
            Pass::Elaborate,
            options.emit_events || context.emit_events(),
        )?;
        check.run_elaborate()?;

        // package elaborated DIR tables and report the pass's diagnostics
        let (elaborated, diagnostics) = check.finish_elaborate(module)?;
        context.emit_diagnostics(diagnostics);

        Ok(ArtifactPayload::DirElaborated(Arc::new(elaborated)))
    }

    /// Collect inputs for one checked DIR module.
    pub(crate) fn collect_dir_checked(
        &self,
        module: ModuleId,
        profile: ProfileId,
        context: &dyn ProviderContext,
    ) -> CompilerResult<ArtifactDependencySet> {
        let mut dependencies = ArtifactDependencySet::default();
        dependencies.require(ArtifactKey::dir_parsed(module));
        dependencies.require(ArtifactKey::dir_bound(module, profile));
        dependencies.require(ArtifactKey::dir_resolved(module, profile));
        dependencies.require(ArtifactKey::dir_expanded(module, profile));
        dependencies.require_projection(
            ArtifactKey::environment_bound(profile),
            ArtifactProjectionKey::Content,
        );

        // require the aggregate implicit declarations
        dependencies.require_projection(
            ArtifactKey::environment_declared(profile),
            ArtifactProjectionKey::Content,
        );

        // seed the checking pass from the module's own committed artifacts
        dependencies.require(ArtifactKey::dir_declared(module, profile));
        dependencies.require(ArtifactKey::dir_elaborated(module, profile));

        // observe package config for check options
        let repository_module = self.module(context.revision(), module)?;
        self.observe_package_config(context, repository_module.package_id, &mut dependencies)?;

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
                ArtifactKey::dir_elaborated(import, profile),
                ArtifactProjectionKey::Elaborated,
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
            .environment_bound_content(profile)
            .map_err(CompilerError::from)?;
        let declared_environment = artifacts
            .environment_declared(profile)
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
            Some(declared_environment),
            environment,
            module,
            Pass::Check,
            emit_events,
        )?;
        check.run_check()?;

        // emit solver counters and optional trace sidecars
        let stats = check.stats();
        context.emit_counter("solve.variables", stats.variables as u64);
        context.emit_counter("solve.constraints", stats.constraints as u64);
        context.emit_counter("solve.bounds", stats.bounds as u64);
        context.emit_counter("solve.decisions", stats.decisions as u64);
        if options.emit_stats {
            let content = stats.render_metadata();
            context.emit_sidecar(check_sidecar("metadata", content));
        }
        let events = emit_events.then(|| check.events());

        if let Some(events) = events {
            context.emit_sidecar(check_sidecar("events", events.render()));
        }

        // write checked DIR tables and report the pass's diagnostics;
        //  annotations render inside, after the write settles every type
        let (checked, diagnostics, annotated) =
            check.finish_check(module, options.emit_checked_types)?;
        for source in annotated {
            context.emit_sidecar(annotated_sidecar(source));
        }
        context.emit_diagnostics(diagnostics);

        Ok(ArtifactPayload::DirChecked(Arc::new(checked)))
    }

    /// Collect inputs for the declared environment of one profile.
    pub(crate) fn collect_environment_declared(
        &self,
        profile: ProfileId,
        context: &dyn ProviderContext,
    ) -> CompilerResult<ArtifactDependencySet> {
        let mut dependencies = ArtifactDependencySet::default();
        dependencies.require_projection(
            ArtifactKey::environment_bound(profile),
            ArtifactProjectionKey::Content,
        );

        // implicit module declarations back every index
        let artifacts = self.artifact_reader(context);
        let environment = match artifacts.environment_bound_content(profile) {
            Ok(environment) => environment,
            Err(destack_repository::ProviderError::Blocked { .. }) => return Ok(dependencies),
            Err(error) => return Err(error.into()),
        };
        for module in environment.implicit_modules() {
            dependencies.require_projection(
                ArtifactKey::dir_declared(module, profile),
                ArtifactProjectionKey::Declared,
            );
        }

        Ok(dependencies)
    }

    /// Build the declared environment for one profile.
    pub(crate) fn provide_environment_declared(
        &self,
        profile: ProfileId,
        context: &dyn ProviderContext,
    ) -> CompilerResult<ArtifactPayload> {
        let artifacts = self.artifact_reader(context);
        let bound = artifacts
            .environment_bound_content(profile)
            .map_err(CompilerError::from)?;

        // union each implicit module's declared indexes
        let mut environment = EnvironmentDeclared::default();
        for module in bound.implicit_modules() {
            let declared = artifacts
                .dir_declared(module, profile)
                .map_err(CompilerError::from)?;
            let types = declared.types.clone();
            for (symbol, definition) in declared.definitions.iter_definitions() {
                // index extension declarations by their resolved target
                if let dir::Definition::Extension(extension) = definition {
                    match extension.target.root() {
                        Some(root) => {
                            environment
                                .extensions_by_root
                                .entry(root)
                                .or_default()
                                .push(symbol);
                        }
                        None => {
                            let target = extension.target.r#type();
                            match types.get_type_maybe(target.local_id) {
                                Some(dir::Type::Primitive(primitive)) => environment
                                    .extensions_by_primitive
                                    .entry(primitive)
                                    .or_default()
                                    .push(symbol),
                                _ => environment.blanket_extensions.push(symbol),
                            }
                        }
                    }
                }

                // index implementing declarations by their interface
                for implementation in definition.implementations() {
                    let interface = implementation.ty;
                    let Some(dir::Type::Application(instance)) =
                        types.get_type_maybe(interface.local_id)
                    else {
                        continue;
                    };
                    environment
                        .implementations_by_interface
                        .entry(instance.symbol)
                        .or_default()
                        .push(symbol);
                }
            }
        }

        Ok(ArtifactPayload::EnvironmentDeclared(Arc::new(environment)))
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
