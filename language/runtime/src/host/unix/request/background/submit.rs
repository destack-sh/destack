use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::host::core::{HostRequest, HostRequestContext, HostRequestOutcome, HostRequestResult};
use crate::platform::PlatformError;
use crate::platform::os::abi_generated::{
    BackgroundStatusValue, BackgroundTaskDescriptorValue, BackgroundTaskOptionsValue,
    BackgroundTaskResultValue, BackgroundTaskScheduleKindValue,
};

use crate::host::unix::request::background::{
    background_status as linux_background_status,
    register_background_task as register_linux_background_task,
    trigger_background_task as trigger_linux_background_task,
    unregister_background_task as unregister_linux_background_task,
};
use crate::platform::os::background::runtime::complete_execution;
use crate::platform::os::background::storage::{
    DesktopBackgroundTaskRecord, background_descriptor_from_options,
    ensure_background_task_directory, read_background_task_record, read_background_task_records,
    remove_background_task_record, validate_desktop_background_options,
    write_background_task_record,
};

/// Submit one Linux background request.
pub(crate) fn submit_background_request(
    context: &HostRequestContext,
    request: &HostRequest,
) -> RuntimeResult<Option<HostRequestOutcome>> {
    match request {
        HostRequest::OsBackgroundStatus => Ok(Some(HostRequestOutcome::immediate(
            HostRequestResult::BackgroundStatus(background_status()?),
        ))),
        HostRequest::OsBackgroundList => Ok(Some(HostRequestOutcome::immediate(
            HostRequestResult::BackgroundTaskDescriptors(background_list(context)?),
        ))),
        HostRequest::OsBackgroundRegister { options } => {
            background_register(context, options)?;

            Ok(Some(HostRequestOutcome::immediate(HostRequestResult::None)))
        }
        HostRequest::OsBackgroundUnregister { identifier } => {
            background_unregister(context, identifier)?;

            Ok(Some(HostRequestOutcome::immediate(HostRequestResult::None)))
        }
        HostRequest::OsBackgroundTriggerTest { identifier } => {
            Ok(Some(HostRequestOutcome::immediate(
                HostRequestResult::Bool(background_trigger_test(context, identifier)?),
            )))
        }
        HostRequest::OsBackgroundComplete {
            execution_id,
            result,
        } => {
            let execution = complete_execution(context.host_session_id, execution_id)?;
            let record = read_background_task_record(context, &execution.identifier)?;

            // retry
            if matches!(result, BackgroundTaskResultValue::Retry) {
                trigger_linux_background_task(&execution.identifier)?;

                return Ok(Some(HostRequestOutcome::immediate(HostRequestResult::None)));
            }

            let Some(record) = record else {
                return Ok(Some(HostRequestOutcome::immediate(HostRequestResult::None)));
            };

            // one-shot cleanup
            if record.options.schedule.kind == BackgroundTaskScheduleKindValue::Once {
                background_unregister(context, &execution.identifier)?;
            }

            Ok(Some(HostRequestOutcome::immediate(HostRequestResult::None)))
        }
        _ => Ok(None),
    }
}

/// Return the current Linux background scheduler status.
fn background_status() -> RuntimeResult<BackgroundStatusValue> {
    linux_background_status()
}

/// Return every persisted Linux background task descriptor.
fn background_list(
    context: &HostRequestContext,
) -> RuntimeResult<Vec<BackgroundTaskDescriptorValue>> {
    let records = read_background_task_records(context)?;

    Ok(records
        .into_iter()
        .map(|record| background_descriptor_from_options(record.options))
        .collect())
}

/// Register one Linux background task.
fn background_register(
    context: &HostRequestContext,
    options: &BackgroundTaskOptionsValue,
) -> RuntimeResult<()> {
    validate_desktop_background_options(context.platform, options)?;
    ensure_background_task_directory(context)?;
    register_linux_background_task(context, options)?;

    let write_result = write_background_task_record(
        context,
        &DesktopBackgroundTaskRecord {
            options: options.clone(),
        },
    );

    // keep persisted metadata and host scheduler state in sync
    if let Err(error) = write_result {
        if let Err(cleanup_error) = unregister_linux_background_task(context, &options.identifier) {
            tracing::warn!(
                ?cleanup_error,
                identifier = options.identifier,
                "failed to roll back Linux background registration after metadata write failure"
            );
        }

        return Err(error);
    }

    Ok(())
}

/// Unregister one Linux background task.
fn background_unregister(context: &HostRequestContext, identifier: &str) -> RuntimeResult<()> {
    unregister_linux_background_task(context, identifier)?;
    remove_background_task_record(context, identifier)?;

    Ok(())
}

/// Trigger one persisted Linux background task.
fn background_trigger_test(context: &HostRequestContext, identifier: &str) -> RuntimeResult<bool> {
    let record = read_background_task_record(context, identifier)?;

    if record.is_some() {
        return trigger_linux_background_task(identifier);
    }

    Err(RuntimeError::from(PlatformError::generic(
        Some(crate::platform::diagnostic::PlatformErrorCode::IoNotFound),
        format!("background task `{identifier}` not found"),
    ))
    .boxed())
}
