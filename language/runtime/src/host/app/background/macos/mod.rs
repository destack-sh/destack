mod core;
mod launchd;
mod plist;

pub(in crate::host::app::background) use self::core::{
    background_status, register_background_task, trigger_background_task,
    unregister_background_task,
};
