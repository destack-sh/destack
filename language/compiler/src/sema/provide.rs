use destack_core::FxIndexSet;
use std::sync::Arc;

use destack_artifact::{
    ArtifactDependencySet, ArtifactKey, ArtifactPayload, ArtifactProjectionKey, DirDeclared,
    DirResolved, EnvironmentBound, EnvironmentDeclared,
};
use destack_dir as dir;
use destack_repository::{ArtifactAttemptRecorder, ProfileId, ProviderContext, ProviderError};
use destack_source::ModuleId;

use crate::sema::{CheckModuleState, CheckState, Pass};
use crate::{Compiler, CompilerError, CompilerResult};

/// The foreign modules one module's check reads through resolution targets.
struct ReferencedModules {
    /// The modules referenced by resolution targets.
    targets: FxIndexSet<ModuleId>,
}

/// Return the modules one module references, or None while their resolve stages build.
fn referenced_modules(
    artifacts: &destack_repository::ArtifactReader<'_>,
    module: ModuleId,
    profile: ProfileId,
) -> CompilerResult<Option<ReferencedModules>> {
    // wait while the module's own resolve stage is still building
    let resolved = match artifacts.read::<DirResolved>((module, profile)) {
        Ok(resolved) => resolved,
        Err(ProviderError::Blocked { .. }) => return Ok(None),
        Err(error) => return Err(error.into()),
    };

    // keep the foreign modules the resolution targets reach
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
        // require this module's own stage artifacts
        let mut dependencies = ArtifactDependencySet::default();
        dependencies.require(ArtifactKey::dir_parsed(module));
        dependencies.require(ArtifactKey::dir_bound(module, profile));
        dependencies.require(ArtifactKey::dir_resolved(module, profile));
        dependencies.require(ArtifactKey::dir_expanded(module, profile));
        dependencies.require_projection(
            ArtifactKey::environment_bound(profile),
            ArtifactProjectionKey::Content,
        );

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
        // read the profile's environment
        let artifacts = self.artifact_reader(context);
        let global = artifacts
            .read_content::<EnvironmentBound>(profile)
            .map_err(CompilerError::from)?;
        let environment = self.environment(context.revision())?;
        // declare the module without walking callable bodies
        let mut check = ArtifactAttemptRecorder::breakdown_maybe(
            context.recorder(),
            "load",
            || -> CompilerResult<_> {
                let module = CheckModuleState::load(
                    self,
                    context,
                    &artifacts,
                    profile,
                    module,
                    Pass::Declare,
                )?;

                Ok(CheckState::new(
                    self,
                    context,
                    &artifacts,
                    profile,
                    global,
                    None,
                    environment,
                    module,
                    Pass::Declare,
                    context.records_events(),
                ))
            },
        )?;

        // run the pass
        ArtifactAttemptRecorder::breakdown_maybe(context.recorder(), "run", || {
            check.run_declare()
        })?;

        // record solver counters and detailed events
        check.record_trace(context);

        // package declared DIR tables and report the pass's diagnostics
        let (declared, diagnostics) =
            ArtifactAttemptRecorder::breakdown_maybe(context.recorder(), "finish", || {
                check.finish_declare(module)
            })?;
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
        // require this module's own stage artifacts
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
        // read the profile's environment
        let artifacts = self.artifact_reader(context);
        let global = artifacts
            .read::<EnvironmentBound>(profile)
            .map_err(CompilerError::from)?;
        let declared_environment = artifacts
            .read::<EnvironmentDeclared>(profile)
            .map_err(CompilerError::from)?;
        let environment = self.environment(context.revision())?;
        // flatten the module's declared owners
        let mut check = ArtifactAttemptRecorder::breakdown_maybe(
            context.recorder(),
            "load",
            || -> CompilerResult<_> {
                let module = CheckModuleState::load(
                    self,
                    context,
                    &artifacts,
                    profile,
                    module,
                    Pass::Elaborate,
                )?;

                Ok(CheckState::new(
                    self,
                    context,
                    &artifacts,
                    profile,
                    global,
                    Some(declared_environment),
                    environment,
                    module,
                    Pass::Elaborate,
                    context.records_events(),
                ))
            },
        )?;

        // run the pass
        ArtifactAttemptRecorder::breakdown_maybe(context.recorder(), "run", || {
            check.run_elaborate()
        })?;

        // record solver counters and detailed events
        check.record_trace(context);

        // package elaborated DIR tables and report the pass's diagnostics
        let (elaborated, diagnostics) =
            ArtifactAttemptRecorder::breakdown_maybe(context.recorder(), "finish", || {
                check.finish_elaborate(module)
            })?;
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
        // require this module's own stage artifacts
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
        // read the profile's environment
        let artifacts = self.artifact_reader(context);
        let global = artifacts
            .read_content::<EnvironmentBound>(profile)
            .map_err(CompilerError::from)?;
        let declared_environment = artifacts
            .read::<EnvironmentDeclared>(profile)
            .map_err(CompilerError::from)?;
        let environment = self.environment(context.revision())?;
        // check the module's declarations and bodies
        let mut check = ArtifactAttemptRecorder::breakdown_maybe(
            context.recorder(),
            "load",
            || -> CompilerResult<_> {
                let module = CheckModuleState::load(
                    self,
                    context,
                    &artifacts,
                    profile,
                    module,
                    Pass::Check,
                )?;

                Ok(CheckState::new(
                    self,
                    context,
                    &artifacts,
                    profile,
                    global,
                    Some(declared_environment),
                    environment,
                    module,
                    Pass::Check,
                    context.records_events(),
                ))
            },
        )?;

        // run the pass
        ArtifactAttemptRecorder::breakdown_maybe(context.recorder(), "run", || check.run_check())?;

        // record solver counters and detailed events
        check.record_trace(context);

        // write checked DIR tables and report the pass's diagnostics
        let (checked, diagnostics) =
            ArtifactAttemptRecorder::breakdown_maybe(context.recorder(), "finish", || {
                check.finish_check(module)
            })?;
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
        let environment = match artifacts.read_content::<EnvironmentBound>(profile) {
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
        // read the profile's bound environment
        let artifacts = self.artifact_reader(context);
        let bound = artifacts
            .read_content::<EnvironmentBound>(profile)
            .map_err(CompilerError::from)?;

        // union each implicit module's declared indexes
        let mut environment = EnvironmentDeclared::default();
        for module in bound.implicit_modules() {
            let declared = artifacts
                .read::<DirDeclared>((module, profile))
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
                        None => environment.blanket_extensions.push(symbol),
                    }
                }

                // index implementing declarations by their interface and target root
                let root = match definition {
                    dir::Definition::Extension(extension) => extension.target.root(),
                    _ => Some(dir::TypeRoot::Declaration(symbol)),
                };
                for implementation in definition.implementations() {
                    let interface = implementation.interface;
                    let Some(dir::Type::Application(instance)) =
                        types.get_type_maybe(interface.local_id)
                    else {
                        continue;
                    };
                    environment
                        .implementations_by_interface
                        .entry(instance.symbol)
                        .or_default()
                        .push((symbol, root));
                }
            }
        }

        Ok(ArtifactPayload::EnvironmentDeclared(Arc::new(environment)))
    }
}
