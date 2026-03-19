use crate::diagnostic::RuntimeResult;
use crate::host::core::{HostRequest, HostRequestContext, HostRequestOutcome, HostRequestResult};
use crate::platform::os::abi_generated::{
    BackgroundStatusValue, BackgroundTaskDescriptorValue, BackgroundTaskOptionsValue,
};

use super::runtime::{complete_execution, trigger_test_execution};
use super::storage::{self, DesktopBackgroundTaskRecord, background_descriptor_from_options};
use super::{scheduler, storage as background_storage};

/// Submit one background operation when the current host provides one real backend.
pub(crate) fn submit_background_request(
    context: &HostRequestContext,
    request: &HostRequest,
) -> RuntimeResult<Option<HostRequestOutcome>> {
    match request {
        // desktop background scheduler status
        HostRequest::OsBackgroundStatus => Ok(Some(HostRequestOutcome::immediate(
            HostRequestResult::BackgroundStatus(background_status(context)?),
        ))),

        // desktop background task list
        HostRequest::OsBackgroundList => Ok(Some(HostRequestOutcome::immediate(
            HostRequestResult::BackgroundTaskDescriptors(background_list(context)?),
        ))),

        // desktop background registration
        HostRequest::OsBackgroundRegister { options } => {
            background_register(context, options)?;

            Ok(Some(HostRequestOutcome::immediate(HostRequestResult::None)))
        }

        // desktop background unregistration
        HostRequest::OsBackgroundUnregister { identifier } => {
            background_unregister(context, identifier)?;

            Ok(Some(HostRequestOutcome::immediate(HostRequestResult::None)))
        }

        // desktop trigger-test can relaunch the current process with one background marker
        HostRequest::OsBackgroundTriggerTest { identifier } => {
            Ok(Some(HostRequestOutcome::immediate(
                HostRequestResult::Bool(background_trigger_test(context, identifier)?),
            )))
        }

        // desktop completion closes one active launch-marker execution
        HostRequest::OsBackgroundComplete {
            execution_id,
            result: _,
        } => {
            complete_execution(context.host_runtime_id, execution_id)?;

            Ok(Some(HostRequestOutcome::immediate(HostRequestResult::None)))
        }
        _ => Ok(None),
    }
}

/// Return the current desktop background scheduler status.
fn background_status(context: &HostRequestContext) -> RuntimeResult<BackgroundStatusValue> {
    scheduler::background_status(context)
}

/// Return all persisted desktop background task registrations.
fn background_list(
    context: &HostRequestContext,
) -> RuntimeResult<Vec<BackgroundTaskDescriptorValue>> {
    let records = background_storage::read_background_task_records(context)?;

    Ok(records
        .into_iter()
        .map(|record| background_descriptor_from_options(record.options))
        .collect())
}

/// Register one desktop background task through the host scheduler.
fn background_register(
    context: &HostRequestContext,
    options: &BackgroundTaskOptionsValue,
) -> RuntimeResult<()> {
    storage::validate_desktop_background_options(context.platform, options)?;
    storage::ensure_background_task_directory(context)?;
    scheduler::background_register(context, options)?;

    let write_result = storage::write_background_task_record(
        context,
        &DesktopBackgroundTaskRecord {
            options: options.clone(),
        },
    );

    // keep persisted metadata and host scheduler state in sync
    if let Err(error) = write_result {
        if let Err(cleanup_error) = scheduler::background_unregister(context, &options.identifier) {
            tracing::warn!(
                ?cleanup_error,
                identifier = options.identifier,
                "failed to roll back desktop background registration after metadata write failure"
            );
        }

        return Err(error);
    }

    Ok(())
}

/// Unregister one desktop background task from the host scheduler.
fn background_unregister(context: &HostRequestContext, identifier: &str) -> RuntimeResult<()> {
    scheduler::background_unregister(context, identifier)?;
    storage::remove_background_task_record(context, identifier)?;

    Ok(())
}

/// Trigger one persisted desktop background task immediately through the host scheduler.
fn background_trigger_test(context: &HostRequestContext, identifier: &str) -> RuntimeResult<bool> {
    let record = storage::read_background_task_record(context, identifier)?;

    // when one persisted registration exists, prefer the native scheduler trigger
    if record.is_some() {
        return scheduler::trigger_background_task(context, identifier);
    }

    trigger_test_execution(identifier)
}
