use windows::Win32::Foundation::{VARIANT_FALSE, VARIANT_TRUE};
use windows::Win32::System::TaskScheduler::{
    IExecAction, ITaskDefinition, ITaskService, ITimeTrigger, TASK_ACTION_EXEC,
    TASK_COMPATIBILITY_V2_4, TASK_INSTANCES_IGNORE_NEW, TASK_RUNLEVEL_LUA, TASK_TRIGGER_TIME,
};
use windows::core::{BSTR, Interface};
use windows_sys::Win32::Foundation::SYSTEMTIME;
use windows_sys::Win32::System::SystemInformation::GetLocalTime;

use crate::diagnostic::RuntimeResult;
use crate::host::core::HostRequestContext;
use crate::host::windows::identity::{resolved_application_identifier, resolved_display_name};
use crate::platform::core::not_supported;
use crate::platform::os::abi_generated::BackgroundTaskOptionsValue;

use super::core::windows_task_error;
use crate::platform::os::background::storage::background_interval_seconds;

/// Open-ended repetition duration for repeating tasks.
const WINDOWS_TASK_REPETITION_DURATION: &str = "P9999D";

/// Maximum runtime budget for one scheduled execution.
const WINDOWS_TASK_EXECUTION_LIMIT: &str = "PT1H";

/// Build one Task Scheduler definition for one wrapper script.
pub(super) fn build_task_definition(
    context: &HostRequestContext,
    service: &ITaskService,
    identifier: &str,
    wrapper_path: &std::path::Path,
    options: &BackgroundTaskOptionsValue,
) -> RuntimeResult<ITaskDefinition> {
    let definition = unsafe { service.NewTask(0) }.map_err(|error| {
        windows_task_error(
            "destack.os.background.register",
            "ITaskService::NewTask",
            &error,
        )
    })?;

    // registration info
    let registration_info = unsafe { definition.RegistrationInfo() }.map_err(|error| {
        windows_task_error(
            "destack.os.background.register",
            "ITaskDefinition::RegistrationInfo",
            &error,
        )
    })?;
    let author = BSTR::from(resolved_application_identifier(context)?);
    let display_name = resolved_display_name(context)?;
    let description = BSTR::from(format!("{display_name} background task `{identifier}`"));

    unsafe {
        registration_info.SetAuthor(&author).map_err(|error| {
            windows_task_error(
                "destack.os.background.register",
                "IRegistrationInfo::SetAuthor",
                &error,
            )
        })?;
        registration_info
            .SetDescription(&description)
            .map_err(|error| {
                windows_task_error(
                    "destack.os.background.register",
                    "IRegistrationInfo::SetDescription",
                    &error,
                )
            })?;
    }

    // principal
    let principal = unsafe { definition.Principal() }.map_err(|error| {
        windows_task_error(
            "destack.os.background.register",
            "ITaskDefinition::Principal",
            &error,
        )
    })?;

    unsafe {
        principal
            .SetLogonType(windows::Win32::System::TaskScheduler::TASK_LOGON_INTERACTIVE_TOKEN)
            .map_err(|error| {
                windows_task_error(
                    "destack.os.background.register",
                    "IPrincipal::SetLogonType",
                    &error,
                )
            })?;
        principal.SetRunLevel(TASK_RUNLEVEL_LUA).map_err(|error| {
            windows_task_error(
                "destack.os.background.register",
                "IPrincipal::SetRunLevel",
                &error,
            )
        })?;
    }

    // settings
    let settings = unsafe { definition.Settings() }.map_err(|error| {
        windows_task_error(
            "destack.os.background.register",
            "ITaskDefinition::Settings",
            &error,
        )
    })?;

    unsafe {
        settings
            .SetMultipleInstances(TASK_INSTANCES_IGNORE_NEW)
            .map_err(|error| {
                windows_task_error(
                    "destack.os.background.register",
                    "ITaskSettings::SetMultipleInstances",
                    &error,
                )
            })?;
        settings
            .SetDisallowStartIfOnBatteries(VARIANT_FALSE)
            .map_err(|error| {
                windows_task_error(
                    "destack.os.background.register",
                    "ITaskSettings::SetDisallowStartIfOnBatteries",
                    &error,
                )
            })?;
        settings
            .SetStopIfGoingOnBatteries(VARIANT_FALSE)
            .map_err(|error| {
                windows_task_error(
                    "destack.os.background.register",
                    "ITaskSettings::SetStopIfGoingOnBatteries",
                    &error,
                )
            })?;
        settings
            .SetAllowHardTerminate(VARIANT_FALSE)
            .map_err(|error| {
                windows_task_error(
                    "destack.os.background.register",
                    "ITaskSettings::SetAllowHardTerminate",
                    &error,
                )
            })?;
        settings
            .SetStartWhenAvailable(VARIANT_TRUE)
            .map_err(|error| {
                windows_task_error(
                    "destack.os.background.register",
                    "ITaskSettings::SetStartWhenAvailable",
                    &error,
                )
            })?;
        settings
            .SetRunOnlyIfNetworkAvailable(VARIANT_FALSE)
            .map_err(|error| {
                windows_task_error(
                    "destack.os.background.register",
                    "ITaskSettings::SetRunOnlyIfNetworkAvailable",
                    &error,
                )
            })?;
        settings.SetEnabled(VARIANT_TRUE).map_err(|error| {
            windows_task_error(
                "destack.os.background.register",
                "ITaskSettings::SetEnabled",
                &error,
            )
        })?;
        settings.SetHidden(VARIANT_FALSE).map_err(|error| {
            windows_task_error(
                "destack.os.background.register",
                "ITaskSettings::SetHidden",
                &error,
            )
        })?;
        settings.SetRunOnlyIfIdle(VARIANT_FALSE).map_err(|error| {
            windows_task_error(
                "destack.os.background.register",
                "ITaskSettings::SetRunOnlyIfIdle",
                &error,
            )
        })?;
        settings.SetWakeToRun(VARIANT_FALSE).map_err(|error| {
            windows_task_error(
                "destack.os.background.register",
                "ITaskSettings::SetWakeToRun",
                &error,
            )
        })?;
        settings
            .SetExecutionTimeLimit(&BSTR::from(WINDOWS_TASK_EXECUTION_LIMIT))
            .map_err(|error| {
                windows_task_error(
                    "destack.os.background.register",
                    "ITaskSettings::SetExecutionTimeLimit",
                    &error,
                )
            })?;
        settings.SetPriority(7).map_err(|error| {
            windows_task_error(
                "destack.os.background.register",
                "ITaskSettings::SetPriority",
                &error,
            )
        })?;
        settings
            .SetCompatibility(TASK_COMPATIBILITY_V2_4)
            .map_err(|error| {
                windows_task_error(
                    "destack.os.background.register",
                    "ITaskSettings::SetCompatibility",
                    &error,
                )
            })?;
    }

    // trigger
    let triggers = unsafe { definition.Triggers() }.map_err(|error| {
        windows_task_error(
            "destack.os.background.register",
            "ITaskDefinition::Triggers",
            &error,
        )
    })?;
    let trigger = unsafe { triggers.Create(TASK_TRIGGER_TIME) }.map_err(|error| {
        windows_task_error(
            "destack.os.background.register",
            "ITriggerCollection::Create",
            &error,
        )
    })?;
    let trigger: ITimeTrigger = trigger.cast().map_err(|error| {
        windows_task_error(
            "destack.os.background.register",
            "ITrigger::cast(ITimeTrigger)",
            &error,
        )
    })?;
    let repetition = unsafe { trigger.Repetition() }.map_err(|error| {
        windows_task_error(
            "destack.os.background.register",
            "ITrigger::Repetition",
            &error,
        )
    })?;

    unsafe {
        trigger
            .SetStartBoundary(&BSTR::from(windows_task_start_boundary()))
            .map_err(|error| {
                windows_task_error(
                    "destack.os.background.register",
                    "ITrigger::SetStartBoundary",
                    &error,
                )
            })?;
        trigger.SetEnabled(VARIANT_TRUE).map_err(|error| {
            windows_task_error(
                "destack.os.background.register",
                "ITrigger::SetEnabled",
                &error,
            )
        })?;
        repetition
            .SetInterval(&BSTR::from(windows_task_interval_string(
                background_interval_seconds(options),
            )?))
            .map_err(|error| {
                windows_task_error(
                    "destack.os.background.register",
                    "IRepetitionPattern::SetInterval",
                    &error,
                )
            })?;
        repetition
            .SetDuration(&BSTR::from(WINDOWS_TASK_REPETITION_DURATION))
            .map_err(|error| {
                windows_task_error(
                    "destack.os.background.register",
                    "IRepetitionPattern::SetDuration",
                    &error,
                )
            })?;
        repetition
            .SetStopAtDurationEnd(VARIANT_FALSE)
            .map_err(|error| {
                windows_task_error(
                    "destack.os.background.register",
                    "IRepetitionPattern::SetStopAtDurationEnd",
                    &error,
                )
            })?;
    }

    // action
    let actions = unsafe { definition.Actions() }.map_err(|error| {
        windows_task_error(
            "destack.os.background.register",
            "ITaskDefinition::Actions",
            &error,
        )
    })?;
    let action = unsafe { actions.Create(TASK_ACTION_EXEC) }.map_err(|error| {
        windows_task_error(
            "destack.os.background.register",
            "IActionCollection::Create",
            &error,
        )
    })?;
    let action: IExecAction = action.cast().map_err(|error| {
        windows_task_error(
            "destack.os.background.register",
            "IAction::cast(IExecAction)",
            &error,
        )
    })?;
    let working_directory = wrapper_path
        .parent()
        .map(|path| path.to_string_lossy().into_owned())
        .unwrap_or_else(|| ".".to_string());

    unsafe {
        action
            .SetPath(&BSTR::from(wrapper_path.to_string_lossy().as_ref()))
            .map_err(|error| {
                windows_task_error(
                    "destack.os.background.register",
                    "IExecAction::SetPath",
                    &error,
                )
            })?;
        action
            .SetWorkingDirectory(&BSTR::from(working_directory))
            .map_err(|error| {
                windows_task_error(
                    "destack.os.background.register",
                    "IExecAction::SetWorkingDirectory",
                    &error,
                )
            })?;
    }

    Ok(definition)
}

/// Return one local Task Scheduler start boundary for the current registration time.
fn windows_task_start_boundary() -> String {
    let mut system_time = unsafe { std::mem::zeroed::<SYSTEMTIME>() };

    unsafe {
        GetLocalTime(&mut system_time);
    }

    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}",
        system_time.wYear,
        system_time.wMonth,
        system_time.wDay,
        system_time.wHour,
        system_time.wMinute,
        system_time.wSecond,
    )
}

/// Return one ISO-8601 repetition interval for Task Scheduler.
fn windows_task_interval_string(interval_seconds: u64) -> RuntimeResult<String> {
    if interval_seconds < 60 {
        return Err(not_supported("destack.os.background.register"));
    }

    let minutes = interval_seconds / 60;

    Ok(format!("PT{minutes}M"))
}

#[cfg(test)]
mod tests {
    use super::{windows_task_interval_string, windows_task_start_boundary};

    /// Format one local Task Scheduler start boundary in the expected time shape.
    #[test]
    fn test_windows_task_start_boundary_has_iso_local_shape() {
        let boundary = windows_task_start_boundary();
        let bytes = boundary.as_bytes();

        assert_eq!(boundary.len(), 19);
        assert_eq!(bytes[4], b'-');
        assert_eq!(bytes[7], b'-');
        assert_eq!(bytes[10], b'T');
        assert_eq!(bytes[13], b':');
        assert_eq!(bytes[16], b':');
    }

    /// Encode one whole-minute repetition interval for Task Scheduler.
    #[test]
    fn test_windows_task_interval_string_uses_whole_minutes() {
        let interval = windows_task_interval_string(300).unwrap();

        assert_eq!(interval, "PT5M");
    }
}
