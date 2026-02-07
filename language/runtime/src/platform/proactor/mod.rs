mod proactor;
#[cfg(unix)]
mod unix;
#[cfg(windows)]
mod windows;

pub use proactor::*;
#[cfg(target_os = "linux")]
pub use unix::IoUringProactor;
#[cfg(all(unix, not(target_os = "linux")))]
pub use unix::UnixProactor;
#[cfg(windows)]
pub use windows::IocpProactor;
