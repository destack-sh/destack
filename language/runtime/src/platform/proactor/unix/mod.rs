#[cfg(not(target_os = "linux"))]
mod unix;
#[cfg(target_os = "linux")]
mod uring;

#[cfg(not(target_os = "linux"))]
pub use unix::UnixProactor;
#[cfg(target_os = "linux")]
pub use uring::IoUringProactor;
