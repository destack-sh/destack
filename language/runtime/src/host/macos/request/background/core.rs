use std::fs;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::host::RequestContext;
use crate::platform::PlatformError;
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::os::abi_generated::{
    BackgroundStatusValue, BackgroundTaskOptionsValue, BackgroundTaskScheduleKindValue,
};

use super::launchd::{
    bootout_background_task_if_present, bootstrap_background_task, launchd_domain,
    launchd_is_available, launchd_trigger_background_task,
};
use super::plist::render_launchd_plist;
use crate::platform::os::background::runtime::{
    desktop_background_test_mode_enabled, enqueue_test_background_launch,
};
use crate::platform::os::background::storage::{
    background_first_run_unix_ns, background_launchd_label, background_launchd_plist_path,
    background_wrapper_script_path, remove_background_file_if_exists,
};
use crate::platform::os::background::wrapper::write_background_wrapper_script;

/// Return whether the active host exposes one desktop background scheduler.
pub(crate) fn background_status() -> RuntimeResult<BackgroundStatusValue> {
    if desktop_background_test_mode_enabled() {
        return Ok(BackgroundStatusValue::Available);
    }

    if launchd_is_available() {
        return Ok(BackgroundStatusValue::Available);
    }

    Ok(BackgroundStatusValue::Unavailable)
}

/// Register one launchd-backed desktop background task.
pub(crate) fn register_background_task(
    context: &RequestContext,
    options: &BackgroundTaskOptionsValue,
) -> RuntimeResult<()> {
    let wrapper_path =
        write_background_wrapper_script(context, &options.identifier, options.trigger)?;
    let plist_path = background_launchd_plist_path(context, &options.identifier)?;
    let plist_directory = plist_path.parent().ok_or_else(|| {
        RuntimeError::from(PlatformError::generic(
            Some(PlatformErrorCode::IoInvalidData),
            "destack.os.background.register: invalid launch worker path",
        ))
        .boxed()
    })?;
    let label = background_launchd_label(context, &options.identifier)?;
    let is_run_at_load = match options.schedule.kind {
        // immediate one-shot registrations fire on bootstrap
        BackgroundTaskScheduleKindValue::Once => options.schedule.earliest_begin_unix_ns.is_none(),

        // recurring launchd intervals start immediately when no delayed first run is requested
        BackgroundTaskScheduleKindValue::Recurring => true,
    };
    let first_run_unix_ns = if is_run_at_load {
        None
    } else {
        Some(background_first_run_unix_ns(&options.schedule)?)
    };
    let domain = launchd_domain()?;
    let plist = render_launchd_plist(
        &label,
        &wrapper_path,
        options,
        first_run_unix_ns,
        is_run_at_load,
    )?;

    let registration_result = (|| {
        // launch worker files
        fs::create_dir_all(plist_directory).map_err(|error| {
            RuntimeError::from(PlatformError::generic(
                Some(PlatformErrorCode::IoPermissionDenied),
                format!(
                    "destack.os.background.register: failed to create launch worker directory {}: {error}",
                    plist_directory.display()
                ),
            ))
            .boxed()
        })?;

        fs::write(&plist_path, plist).map_err(|error| {
            RuntimeError::from(PlatformError::generic(
                Some(PlatformErrorCode::IoPermissionDenied),
                format!(
                    "destack.os.background.register: failed to write launch worker {}: {error}",
                    plist_path.display()
                ),
            ))
            .boxed()
        })?;

        if desktop_background_test_mode_enabled() {
            return Ok(());
        }

        let plist_path = plist_path.to_string_lossy().into_owned();

        // refresh any previous launchd registration before bootstrapping the new definition
        bootout_background_task_if_present(
            "destack.os.background.register",
            &domain,
            &label,
            &plist_path,
        )?;

        bootstrap_background_task("destack.os.background.register", &domain, &plist_path)?;

        Ok(())
    })();

    // roll back local artifacts when launchd registration fails
    if let Err(error) = registration_result {
        if let Err(cleanup_error) = unregister_background_task(context, &options.identifier) {
            tracing::warn!(
                ?cleanup_error,
                identifier = options.identifier,
                "failed to roll back macOS background task registration"
            );
        }

        return Err(error);
    }

    Ok(())
}

/// Remove one launchd-backed desktop background task.
pub(crate) fn unregister_background_task(
    context: &RequestContext,
    identifier: &str,
) -> RuntimeResult<()> {
    let plist_path = background_launchd_plist_path(context, identifier)?;
    let wrapper_path = background_wrapper_script_path(context, identifier)?;
    let domain = launchd_domain()?;
    let label = background_launchd_label(context, identifier)?;
    let plist_path_string = plist_path.to_string_lossy().into_owned();

    if !desktop_background_test_mode_enabled() {
        bootout_background_task_if_present(
            "destack.os.background.unregister",
            &domain,
            &label,
            &plist_path_string,
        )?;
    }

    remove_background_file_if_exists(
        &background_launchd_plist_path(context, identifier)?,
        "destack.os.background.unregister",
    )?;
    remove_background_file_if_exists(&wrapper_path, "destack.os.background.unregister")?;

    Ok(())
}

/// Trigger one launchd-backed background task immediately.
pub(crate) fn trigger_background_task(
    context: &RequestContext,
    identifier: &str,
) -> RuntimeResult<bool> {
    if desktop_background_test_mode_enabled() {
        enqueue_test_background_launch(identifier)?;
        return Ok(true);
    }

    let label = background_launchd_label(context, identifier)?;
    let domain = launchd_domain()?;

    launchd_trigger_background_task("destack.os.background.triggerTest", &domain, &label)?;

    Ok(true)
}
