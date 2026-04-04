use std::path::PathBuf;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::host::RequestContext;
use crate::platform::PlatformError;
use crate::platform::diagnostic::PlatformErrorCode;

const FALLBACK_APPLICATION_ID_PREFIX: &str = "dev.destack";
const FALLBACK_DISPLAY_NAME: &str = "Destack";

/// Resolve one stable application identifier for Unix host integrations.
pub(crate) fn resolved_application_identifier(context: &RequestContext) -> RuntimeResult<String> {
    // configured identity
    if let Some(identifier) = context.app_identity.identifier.as_deref() {
        let identifier = identifier.trim();

        if identifier.is_empty() {
            return Err(invalid_identity_error(
                "identifier",
                "destack target app identity identifier must not be empty",
            ));
        }

        return Ok(identifier.to_string());
    }

    // development fallback
    let executable_name = current_executable_display_name()?;
    let executable_name = sanitized_identifier_component(&executable_name);

    Ok(format!(
        "{FALLBACK_APPLICATION_ID_PREFIX}.{executable_name}"
    ))
}

/// Resolve one human-facing display name for Unix host integrations.
pub(crate) fn resolved_display_name(context: &RequestContext) -> RuntimeResult<String> {
    // configured display name
    if let Some(display_name) = context.app_identity.display_name.as_deref() {
        let display_name = display_name.trim();

        if display_name.is_empty() {
            return Err(invalid_identity_error(
                "displayName",
                "destack target app identity displayName must not be empty",
            ));
        }

        return Ok(display_name.to_string());
    }

    // executable fallback
    let executable_name = current_executable_display_name()?;

    if executable_name.is_empty() {
        return Ok(FALLBACK_DISPLAY_NAME.to_string());
    }

    Ok(executable_name)
}

/// Return the current executable path for Unix host identity fallback logic.
pub(crate) fn current_executable_path() -> RuntimeResult<PathBuf> {
    std::env::current_exe().map_err(|error| {
        RuntimeError::from(PlatformError::generic(
            Some(PlatformErrorCode::Generic),
            format!("failed to read the current executable path for host identity: {error}"),
        ))
        .boxed()
    })
}

/// Return one best-effort executable display name for Unix host identity fallbacks.
fn current_executable_display_name() -> RuntimeResult<String> {
    let executable_path = current_executable_path()?;
    let executable_name = executable_path
        .file_stem()
        .and_then(|value| value.to_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(FALLBACK_DISPLAY_NAME);

    Ok(executable_name.to_string())
}

/// Return one identifier-safe component for one free-form executable name.
fn sanitized_identifier_component(value: &str) -> String {
    let mut output = String::with_capacity(value.len());

    for character in value.chars() {
        if character.is_ascii_alphanumeric() {
            output.push(character.to_ascii_lowercase());
            continue;
        }

        if matches!(character, '.' | '_' | '-') {
            output.push(character);
            continue;
        }

        if output.ends_with('_') {
            continue;
        }

        output.push('_');
    }

    let output = output.trim_matches(['.', '_', '-']).to_string();

    if output.is_empty() {
        return "runtime".to_string();
    }

    if output
        .chars()
        .next()
        .is_some_and(|character| character.is_ascii_digit())
    {
        return format!("app_{output}");
    }

    output
}

/// Return one invalid app identity error.
fn invalid_identity_error(argument: &str, message: &str) -> Box<RuntimeError> {
    RuntimeError::from(PlatformError::invalid_argument_value(argument, message)).boxed()
}
