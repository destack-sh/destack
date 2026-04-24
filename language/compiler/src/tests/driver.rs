use destack_artifact::ArtifactKey;
use destack_workspace::{Change, ProvideError, Ref, Revision};

use crate::{Compiler, CompilerContext, RequirementSet};

/// Run one requirement producing action to completion in compiler tests.
pub(crate) fn run_to_completion<T, E, F>(
    compiler: &Compiler,
    initial_revision: Revision,
    mut action: F,
) -> Result<T, E>
where
    F: FnMut(&Compiler, &CompilerContext<'_>) -> Result<T, E>,
    E: TryInto<RequirementSet, Error = E>,
{
    compiler.clear_artifact_run();
    let mut revision = initial_revision;

    loop {
        let context = compiler
            .context(revision)
            .unwrap_or_else(|error| panic!("failed to pin compiler revision {revision}: {error}"));
        let result = compiler.with_context(context, |context| action(compiler, context));

        match result {
            Ok(value) => return Ok(value),
            Err(error) => match error.try_into() {
                Ok(requirement) => {
                    revision = satisfy_compiler_requirements(compiler, revision, &requirement);
                    compiler.clear_artifact_run();
                }
                Err(error) => return Err(error),
            },
        }
    }
}

/// Provide one batch of root artifact keys to completion in compiler tests.
pub(crate) fn provide_artifacts_to_completion(
    compiler: &Compiler,
    initial_revision: Revision,
    artifact_keys: &[ArtifactKey],
) -> Revision {
    compiler.clear_artifact_run();
    let mut revision = initial_revision;
    let mut pending_artifact_keys = artifact_keys.to_vec();

    while let Some(artifact_key) = pending_artifact_keys.pop() {
        match compiler.provide(revision, artifact_key) {
            Ok(()) => {}
            Err(ProvideError::Requirements(requirements)) => {
                if requirements.has_source_requirements() {
                    revision = apply_source_requirements(compiler, revision, &requirements);
                    compiler.clear_artifact_run();
                }

                pending_artifact_keys.push(artifact_key);

                if !requirements.has_source_requirements() {
                    requirements.for_each_artifact(|requirement| {
                        pending_artifact_keys.push(requirement.version.key);
                    });
                }
            }
            Err(ProvideError::Failed(error)) => {
                panic!("failed to provide compiler artifact {artifact_key:?}: {error:?}");
            }
        }
    }

    revision
}

/// Provide one compiler requirement set inside one compiler test action.
fn satisfy_compiler_requirements(
    compiler: &Compiler,
    mut revision: Revision,
    requirement: &RequirementSet,
) -> Revision {
    let mut pending_artifact_keys = Vec::new();

    if requirement.has_file_requirements() {
        revision = apply_file_requirements(compiler, revision, requirement);
    }

    if !requirement.has_file_requirements() {
        requirement.for_each_artifact(|requirement| {
            pending_artifact_keys.push(requirement.version.key);
        });
    }

    // dependency failures are surfaced when the original action reruns
    while let Some(artifact_key) = pending_artifact_keys.pop() {
        match compiler.provide(revision, artifact_key) {
            Ok(()) => {}
            Err(ProvideError::Requirements(requirements)) => {
                if requirements.has_source_requirements() {
                    revision = apply_source_requirements(compiler, revision, &requirements);
                    compiler.clear_artifact_run();
                }

                pending_artifact_keys.push(artifact_key);

                if !requirements.has_source_requirements() {
                    requirements.for_each_artifact(|requirement| {
                        pending_artifact_keys.push(requirement.version.key);
                    });
                }
            }
            Err(ProvideError::Failed(..)) => continue,
        }
    }

    revision
}

/// Apply one batch of source requirements to the tracked workspace revision.
fn apply_file_requirements(
    compiler: &Compiler,
    revision: Revision,
    requirement: &RequirementSet,
) -> Revision {
    let mut change = Change::empty();

    // direct requested changes
    requirement.for_each_file(|requirement| {
        change.extend_from(&requirement.change);
    });

    let revision = compiler
        .repository
        .apply_to_revision(revision, change)
        .unwrap_or_else(|error| panic!("failed to apply required source edits: {error}"));
    let reference = Ref::for_workspace_root(compiler.repository.workspace_root());

    compiler
        .repository
        .point(&reference, revision)
        .unwrap_or_else(|error| panic!("failed to publish required source revision: {error}"));

    revision
}

/// Apply one batch of provider source requirements to the tracked workspace revision.
fn apply_source_requirements(
    compiler: &Compiler,
    revision: Revision,
    requirement: &destack_workspace::RequirementSet,
) -> Revision {
    let mut change = Change::empty();

    // direct requested changes
    requirement.for_each_source(|requirement| {
        change.extend_from(&requirement.change);
    });

    let revision = compiler
        .repository
        .apply_to_revision(revision, change)
        .unwrap_or_else(|error| panic!("failed to apply required source edits: {error}"));
    let reference = Ref::for_workspace_root(compiler.repository.workspace_root());

    compiler
        .repository
        .point(&reference, revision)
        .unwrap_or_else(|error| panic!("failed to publish required source revision: {error}"));

    revision
}
