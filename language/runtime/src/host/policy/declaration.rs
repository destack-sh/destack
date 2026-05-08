use destack_workspace::{AppOptions, AppPermission};

use super::HostRequestRequirement;
use super::core::{missing_declaration, permission_name};
use crate::diagnostic::RuntimeResult;

/// Require that one resolved requirement is satisfied by the target app declaration.
pub(super) fn require_request_requirement(
    app: &AppOptions,
    operation: &'static str,
    requirement: &HostRequestRequirement,
) -> RuntimeResult<()> {
    let is_satisfied = requirement_satisfied(app, requirement);

    // explicit declaration failure
    if !is_satisfied {
        return Err(missing_declaration(
            operation,
            requirement_message(requirement),
        ));
    }

    Ok(())
}

/// Return whether one target app declaration satisfies one request requirement.
fn requirement_satisfied(app: &AppOptions, requirement: &HostRequestRequirement) -> bool {
    match requirement {
        HostRequestRequirement::BackgroundExecution => !app.background.modes.is_empty(),
        HostRequestRequirement::BackgroundTaskIdentifier(identifier) => {
            app.background.task_identifiers.contains(identifier)
        }
        HostRequestRequirement::Permission(permission) => app.permissions.contains_key(permission),
        HostRequestRequirement::NotificationAuthorization => {
            app.notifications.enabled || app.permissions.contains_key(&AppPermission::Notifications)
        }
        HostRequestRequirement::IntentQueryScheme(scheme) => {
            app.intents.query_schemes.contains(scheme)
        }
        HostRequestRequirement::IntentShareFiles => app.intents.shares_files,
    }
}

/// Return the user-facing declaration guidance for one request requirement.
fn requirement_message(requirement: &HostRequestRequirement) -> String {
    match requirement {
        HostRequestRequirement::BackgroundExecution => {
            "declare one background.modes entry".to_string()
        }
        HostRequestRequirement::BackgroundTaskIdentifier(identifier) => {
            format!("declare background.taskIdentifiers for `{identifier}`")
        }
        HostRequestRequirement::Permission(permission) => {
            format!("declare permissions.{}", permission_name(*permission))
        }
        HostRequestRequirement::NotificationAuthorization => {
            "declare notifications.enabled or permissions.notifications".to_string()
        }
        HostRequestRequirement::IntentQueryScheme(scheme) => {
            format!("declare intents.querySchemes for scheme `{scheme}`")
        }
        HostRequestRequirement::IntentShareFiles => "declare intents.sharesFiles".to_string(),
    }
}
