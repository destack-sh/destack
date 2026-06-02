use destack_artifact::ArtifactKey;
use destack_source::TargetId;

use crate::tests::TestSession;

const DEFAULT_TARGET: &str = "default";

/// Check every builtin library module.
#[test]
fn test_check_library() -> Result<(), String> {
    let session = TestSession::new().build();
    let repository = session.repository();
    let revision = session.revision();
    let package = repository.builtin_package();
    let target = TargetId::new(package.package_id(), DEFAULT_TARGET);
    let profile = repository
        .profile_for_target(revision, target)
        .map_err(|error| format!("builtin library target profile should resolve:\n{error}"))?
        .id();
    let modules = repository
        .modules_for_target(revision, target)
        .map_err(|error| format!("builtin library target modules should resolve:\n{error}"))?;

    // require meaningful target discovery
    if modules.is_empty() {
        return Err("builtin library target should discover modules".to_string());
    }

    // check each module through the normal artifact path
    for module in modules {
        let key = ArtifactKey::dir_checked(module, profile);
        if let Err(error) = session.require_artifact_result(key) {
            let diagnostics = session.render_diagnostics(Some(key));
            if diagnostics.is_empty() {
                return Err(error.to_string());
            }

            return Err(diagnostics);
        }

        let diagnostics = session.render_diagnostics(Some(key));
        if !diagnostics.is_empty() {
            return Err(diagnostics);
        }
    }

    Ok(())
}
