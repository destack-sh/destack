#[cfg(target_os = "macos")]
mod apple;

#[cfg(target_os = "macos")]
pub(super) use apple::call_process_main_thread;
