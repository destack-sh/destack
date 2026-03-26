use windows::Win32::System::TaskScheduler::{TASK_CREATE_OR_UPDATE, TASK_LOGON_INTERACTIVE_TOKEN};
use windows::Win32::System::Variant::VARIANT;
use windows::core::{BSTR, Error as WindowsError};
use windows_core::HRESULT;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::host::core::HostRequestContext;
use crate::host::windows::identity::resolved_application_identifier;
use crate::platform::core::io_operation_error;
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::os::abi_generated::{BackgroundStatusValue, BackgroundTaskOptionsValue};

use super::definition::build_task_definition;
use super::scheduler::{connect_task_service, destack_task_folder, registered_task};
use crate::platform::os::background::runtime::{
    desktop_background_test_mode_enabled, enqueue_test_background_launch,
};
use crate::platform::os::background::storage::{
    background_scheduler_key, background_wrapper_script_path, remove_background_file_if_exists,
};
use crate::platform::os::background::wrapper::write_background_wrapper_script;

/// Windows cancellation status code for missing scheduled tasks.
pub(super) const WINDOWS_TASK_NOT_FOUND: HRESULT = HRESULT(0x8004130Fu32 as i32);

/// Return whether the active host exposes one Windows background scheduler.
pub(crate) fn background_status() -> RuntimeResult<BackgroundStatusValue> {
    if desktop_background_test_mode_enabled() {
        return Ok(BackgroundStatusValue::Available);
    }

    if connect_task_service().is_ok() {
        return Ok(BackgroundStatusValue::Available);
    }

    Ok(BackgroundStatusValue::Unavailable)
}

/// Register one Task Scheduler backed desktop background task.
pub(crate) fn register_background_task(
    context: &HostRequestContext,
    options: &BackgroundTaskOptionsValue,
) -> RuntimeResult<()> {
    let wrapper_path =
        write_background_wrapper_script(context, &options.identifier, options.trigger)?;

    if desktop_background_test_mode_enabled() {
        return Ok(());
    }

    let registration_result = (|| {
        // task scheduler
        let (_apartment, service) = connect_task_service()?;
        let folder = destack_task_folder(&service, context, true)?.ok_or_else(|| {
            io_operation_error(
                "destack.os.background.register",
                Some(PlatformErrorCode::IoNotFound),
                "windows task folder was not created",
            )
        })?;
        let task_definition = build_task_definition(
            context,
            &service,
            &options.identifier,
            &wrapper_path,
            options,
        )?;
        let task_name = BSTR::from(background_scheduler_key(&options.identifier));
        let empty_variant = VARIANT::default();

        unsafe {
            folder.RegisterTaskDefinition(
                &task_name,
                &task_definition,
                TASK_CREATE_OR_UPDATE.0,
                &empty_variant,
                &empty_variant,
                TASK_LOGON_INTERACTIVE_TOKEN,
                &empty_variant,
            )
        }
        .map_err(|error| {
            windows_task_error(
                "destack.os.background.register",
                "ITaskFolder::RegisterTaskDefinition",
                &error,
            )
        })?;

        Ok(())
    })();

    // remove the wrapper script when task registration fails
    if let Err(error) = registration_result {
        if let Err(cleanup_error) =
            remove_background_file_if_exists(&wrapper_path, "destack.os.background.register")
        {
            tracing::warn!(
                ?cleanup_error,
                identifier = options.identifier,
                "failed to roll back Windows background task registration"
            );
        }

        return Err(error);
    }

    Ok(())
}

/// Remove one Task Scheduler backed desktop background task.
pub(crate) fn unregister_background_task(
    context: &HostRequestContext,
    identifier: &str,
) -> RuntimeResult<()> {
    let wrapper_path = background_wrapper_script_path(context, identifier)?;

    if !desktop_background_test_mode_enabled()
        && let Ok((_apartment, service)) = connect_task_service()
        && let Ok(Some(folder)) = destack_task_folder(&service, context, false)
    {
        let task_name = BSTR::from(background_scheduler_key(identifier));
        let delete_result = unsafe { folder.DeleteTask(&task_name, 0) };

        // a missing task is already unregistered
        if let Err(error) = delete_result
            && error.code() != WINDOWS_TASK_NOT_FOUND
        {
            return Err(windows_task_error(
                "destack.os.background.unregister",
                "ITaskFolder::DeleteTask",
                &error,
            ));
        }
    }

    remove_background_file_if_exists(&wrapper_path, "destack.os.background.unregister")?;

    Ok(())
}

/// Trigger one Task Scheduler backed background task immediately.
pub(crate) fn trigger_background_task(
    context: &HostRequestContext,
    identifier: &str,
) -> RuntimeResult<bool> {
    if desktop_background_test_mode_enabled() {
        enqueue_test_background_launch(identifier)?;
        return Ok(true);
    }

    let (_apartment, service) = connect_task_service()?;
    let task = registered_task(
        &service,
        context,
        identifier,
        "destack.os.background.triggerTest",
    )?;
    let empty_variant = VARIANT::default();

    unsafe { task.Run(&empty_variant) }.map_err(|error| {
        windows_task_error(
            "destack.os.background.triggerTest",
            "IRegisteredTask::Run",
            &error,
        )
    })?;

    Ok(true)
}

/// Return one stable Task Scheduler folder name for the active app identity.
pub(super) fn windows_task_folder_name(context: &HostRequestContext) -> RuntimeResult<String> {
    let app_identifier = resolved_application_identifier(context)?;

    Ok(sanitized_windows_task_folder_component(&app_identifier))
}

/// Return one folder-safe Task Scheduler component for one app identifier.
fn sanitized_windows_task_folder_component(value: &str) -> String {
    let mut output = String::with_capacity(value.len());

    for character in value.chars() {
        if character.is_ascii_alphanumeric() || matches!(character, '.' | '_' | '-') {
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
        return "destack".to_string();
    }

    output
}

/// Map one Windows Task Scheduler error into one runtime error.
pub(super) fn windows_task_error(
    operation: &'static str,
    detail: &str,
    error: &WindowsError,
) -> Box<RuntimeError> {
    io_operation_error(
        operation,
        Some(PlatformErrorCode::IoInvalidData),
        format!("{detail} failed with status {}", error.code().0),
    )
}
