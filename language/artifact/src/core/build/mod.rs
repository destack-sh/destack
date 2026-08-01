mod id;

#[cfg(any(target_os = "android", target_os = "linux"))]
mod elf;
#[cfg(target_os = "macos")]
mod macho;
#[cfg(any(
    target_os = "wasi",
    all(
        not(all(target_arch = "wasm32", target_os = "unknown")),
        not(target_os = "android"),
        not(target_os = "linux"),
        not(target_os = "macos"),
        not(windows)
    )
))]
mod unsupported;
#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
mod wasm;
#[cfg(windows)]
mod windows;

pub use id::*;
