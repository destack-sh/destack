mod core;
mod path;
mod record;

pub(super) use self::core::{
    DesktopBackgroundTaskRecord, background_descriptor_from_options, background_interval_seconds,
    remove_background_file_if_exists, validate_desktop_background_options,
};
#[cfg(windows)]
pub(super) use self::path::background_scheduler_key;
pub(super) use self::path::{
    background_launchd_label, background_launchd_plist_path, background_wrapper_script_path,
    ensure_background_scheduler_directory,
};
#[cfg(target_os = "linux")]
pub(super) use self::path::{background_systemd_service_path, background_systemd_timer_path};
pub(super) use self::record::{
    ensure_background_task_directory, read_background_task_record, read_background_task_records,
    remove_background_task_record, write_background_task_record,
};
