use windows::Win32::Foundation::RPC_E_CHANGED_MODE;
use windows::Win32::System::Com::{
    CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED, CoCreateInstance, CoInitializeEx,
    CoUninitialize,
};
use windows::Win32::System::TaskScheduler::{
    IRegisteredTask, ITaskFolder, ITaskService, TaskScheduler,
};
use windows::Win32::System::Variant::VARIANT;
use windows::core::BSTR;

use crate::diagnostic::RuntimeResult;
use crate::host::core::HostRequestContext;
use crate::platform::core::{io_not_found, io_operation_error};
use crate::platform::diagnostic::PlatformErrorCode;

use super::core::{WINDOWS_TASK_NOT_FOUND, windows_task_error, windows_task_folder_name};
use crate::platform::os::background::storage::background_scheduler_key;

/// COM apartment guard for one Task Scheduler call.
pub(super) struct WindowsComApartment {
    /// Whether this thread should uninitialize COM on drop.
    should_uninitialize: bool,
}

impl Drop for WindowsComApartment {
    /// Tear down the COM apartment when this guard owns it.
    fn drop(&mut self) {
        if self.should_uninitialize {
            unsafe {
                CoUninitialize();
            }
        }
    }
}

/// Initialize one STA COM apartment for Task Scheduler work.
fn initialize_sta_com() -> RuntimeResult<WindowsComApartment> {
    let status = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) };

    // own the apartment when this call initialized COM
    if status.is_ok() {
        return Ok(WindowsComApartment {
            should_uninitialize: true,
        });
    }

    // otherwise reuse the existing apartment mode
    if status == RPC_E_CHANGED_MODE {
        return Ok(WindowsComApartment {
            should_uninitialize: false,
        });
    }

    Err(io_operation_error(
        "destack.os.background.status",
        Some(PlatformErrorCode::IoInvalidData),
        format!("CoInitializeEx failed with status {}", status.0),
    ))
}

/// Connect one Task Scheduler service for the current user session.
pub(super) fn connect_task_service() -> RuntimeResult<(WindowsComApartment, ITaskService)> {
    let apartment = initialize_sta_com()?;
    let empty_variant = VARIANT::default();
    let service: ITaskService =
        unsafe { CoCreateInstance(&TaskScheduler, None, CLSCTX_INPROC_SERVER) }.map_err(
            |error| {
                windows_task_error(
                    "destack.os.background.status",
                    "CoCreateInstance(TaskScheduler)",
                    &error,
                )
            },
        )?;

    unsafe {
        service.Connect(
            &empty_variant,
            &empty_variant,
            &empty_variant,
            &empty_variant,
        )
    }
    .map_err(|error| {
        windows_task_error(
            "destack.os.background.status",
            "ITaskService::Connect",
            &error,
        )
    })?;

    Ok((apartment, service))
}

/// Return the Destack task folder, creating it when requested.
pub(super) fn destack_task_folder(
    service: &ITaskService,
    context: &HostRequestContext,
    create_if_missing: bool,
) -> RuntimeResult<Option<ITaskFolder>> {
    let folder_path = BSTR::from(windows_task_folder_path(context)?);

    // reuse the existing folder when it already exists
    match unsafe { service.GetFolder(&folder_path) } {
        Ok(folder) => return Ok(Some(folder)),
        Err(error) if !create_if_missing && error.code() == WINDOWS_TASK_NOT_FOUND => {
            return Ok(None);
        }
        Err(error) if !create_if_missing => {
            return Err(windows_task_error(
                "destack.os.background.status",
                "ITaskService::GetFolder",
                &error,
            ));
        }
        Err(_) => {}
    }

    let root_folder_path = BSTR::from("\\");
    let root_folder = unsafe { service.GetFolder(&root_folder_path) }.map_err(|error| {
        windows_task_error(
            "destack.os.background.register",
            "ITaskService::GetFolder(root)",
            &error,
        )
    })?;
    let folder_name = BSTR::from(windows_task_folder_name(context)?);
    let empty_variant = VARIANT::default();
    let folder =
        unsafe { root_folder.CreateFolder(&folder_name, &empty_variant) }.map_err(|error| {
            windows_task_error(
                "destack.os.background.register",
                "ITaskFolder::CreateFolder",
                &error,
            )
        })?;

    Ok(Some(folder))
}

/// Return one registered background task from the Destack folder.
pub(super) fn registered_task(
    service: &ITaskService,
    context: &HostRequestContext,
    identifier: &str,
    operation: &'static str,
) -> RuntimeResult<IRegisteredTask> {
    let Some(folder) = destack_task_folder(service, context, false)? else {
        return Err(io_not_found(operation, "background task was not found"));
    };
    let task_name = BSTR::from(background_scheduler_key(identifier));

    unsafe { folder.GetTask(&task_name) }.map_err(|error| {
        if error.code() == WINDOWS_TASK_NOT_FOUND {
            return io_not_found(operation, "background task was not found");
        }

        windows_task_error(operation, "ITaskFolder::GetTask", &error)
    })
}

/// Return one stable Task Scheduler folder path for the active app identity.
fn windows_task_folder_path(context: &HostRequestContext) -> RuntimeResult<String> {
    let folder_name = windows_task_folder_name(context)?;

    Ok(format!(r"\{folder_name}"))
}
