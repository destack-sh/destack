#[cfg(any(test, target_os = "android"))]
mod android;
#[cfg(any(target_os = "ios", target_os = "macos"))]
pub(crate) mod apple;
pub(crate) mod core;
#[cfg(any(test, target_os = "dragonfly"))]
mod dragonfly;
#[cfg(any(test, target_os = "freebsd"))]
mod freebsd;
#[cfg(any(test, target_os = "haiku"))]
mod haiku;
#[cfg(any(test, target_os = "illumos"))]
mod illumos;
#[cfg(any(test, target_os = "ios"))]
mod ios;
#[cfg(any(test, target_os = "linux"))]
mod linux;
#[cfg(any(test, target_os = "macos"))]
mod macos;
#[cfg(any(test, target_os = "netbsd"))]
mod netbsd;
#[cfg(any(test, target_os = "openbsd"))]
mod openbsd;
#[cfg(any(test, target_os = "solaris"))]
mod solaris;
#[cfg(any(
    test,
    target_os = "dragonfly",
    target_os = "freebsd",
    target_os = "haiku",
    target_os = "illumos",
    target_os = "linux",
    target_os = "netbsd",
    target_os = "openbsd",
    target_os = "solaris"
))]
pub mod unix;
#[cfg(not(any(
    target_os = "android",
    target_os = "dragonfly",
    target_os = "freebsd",
    target_os = "haiku",
    target_os = "illumos",
    target_os = "ios",
    target_os = "linux",
    target_os = "macos",
    target_os = "netbsd",
    target_os = "openbsd",
    target_os = "solaris",
    windows,
)))]
mod unsupported;
#[cfg(any(test, windows))]
mod windows;

pub use core::{
    Host, HostAdapter, HostEvent, HostEventKind, HostInterruptionEvent, HostLifecycleEvent,
    HostLifecycleState, HostMemoryPressureEvent, HostMemoryPressureLevel, HostPermissionEvent,
    HostPlatform, HostPollOutcome, HostPowerMode, HostPowerModeEvent, HostState, HostThermalEvent,
    HostThermalState, HostWallClockEvent, HostWindowEvent, HostWindowFocusEvent, default_host,
};
#[cfg(windows)]
pub(crate) use windows::process_ingress_loop;

#[cfg(any(test, target_os = "android"))]
pub use android::{
    AndroidActivityLifecycle, AndroidHostBindings, AndroidHostComputeHardwareMacCallback,
    AndroidHostCredentialsAuthenticateCallback, AndroidHostCredentialsCallbacks,
    AndroidHostCredentialsContainsCallback, AndroidHostCredentialsDeleteCallback,
    AndroidHostCredentialsReadCallback, AndroidHostCredentialsWriteCallback,
    AndroidHostCryptoCallbacks, AndroidHostDecryptHardwareKeyCallback,
    AndroidHostDecryptHardwareSecretKeyCallback, AndroidHostDeleteCertificateCallback,
    AndroidHostDeleteHardwareKeyCallback, AndroidHostDeriveHardwareSharedSecretCallback,
    AndroidHostEncryptHardwareSecretKeyCallback, AndroidHostExportHardwarePublicKeyCallback,
    AndroidHostGenerateHardwareKeyPairCallback, AndroidHostGenerateHardwareSecretKeyCallback,
    AndroidHostImportCertificateCallback, AndroidHostSignHardwareKeyCallback,
    AndroidHostSupportsHardwareKeyCallback, HOST_STATUS_BUFFER_TOO_SMALL, HOST_STATUS_FAILED,
    HOST_STATUS_INVALID_ARGUMENT, HOST_STATUS_NOT_FOUND, HOST_STATUS_NOT_SUPPORTED, HOST_STATUS_OK,
    HOST_STATUS_PERMISSION_DENIED, android_notify_activity_lifecycle,
    android_notify_interruption_changed, android_notify_memory_pressure_changed,
    android_notify_permission_request_in_flight, android_notify_permission_result,
    android_notify_power_mode_changed, android_notify_thermal_state_changed, android_notify_wake,
    android_notify_wall_clock_changed, android_notify_window_available,
    android_notify_window_focus_changed, android_notify_window_resized,
    android_notify_window_terminated, destack_host_android_credentials_authenticate,
    destack_host_android_credentials_contains, destack_host_android_credentials_delete,
    destack_host_android_credentials_read, destack_host_android_credentials_write,
    destack_host_android_crypto_compute_hardware_mac,
    destack_host_android_crypto_decrypt_hardware_key,
    destack_host_android_crypto_decrypt_hardware_secret_key,
    destack_host_android_crypto_delete_certificate,
    destack_host_android_crypto_delete_hardware_key,
    destack_host_android_crypto_derive_hardware_shared_secret,
    destack_host_android_crypto_encrypt_hardware_secret_key,
    destack_host_android_crypto_export_hardware_public_key,
    destack_host_android_crypto_generate_hardware_key_pair,
    destack_host_android_crypto_generate_hardware_secret_key,
    destack_host_android_crypto_import_certificate, destack_host_android_crypto_sign_hardware_key,
    destack_host_android_crypto_supports_certificate_write,
    destack_host_android_crypto_supports_hardware_key,
    destack_host_android_crypto_supports_hardware_key_pair,
    destack_host_android_crypto_supports_hardware_secret_key,
    destack_host_android_notify_activity_lifecycle,
    destack_host_android_notify_interruption_changed,
    destack_host_android_notify_memory_pressure_changed,
    destack_host_android_notify_permission_request_in_flight,
    destack_host_android_notify_permission_result, destack_host_android_notify_power_mode_changed,
    destack_host_android_notify_thermal_state_changed, destack_host_android_notify_wake,
    destack_host_android_notify_wall_clock_changed, destack_host_android_notify_window_available,
    destack_host_android_notify_window_focus_changed, destack_host_android_notify_window_resized,
    destack_host_android_notify_window_terminated, destack_host_android_register_bindings,
};

#[cfg(any(test, target_os = "macos"))]
pub use macos::{
    MacosApplicationLifecycle, destack_host_macos_notify_application_lifecycle,
    destack_host_macos_notify_interruption_changed,
    destack_host_macos_notify_memory_pressure_changed, destack_host_macos_notify_permission_result,
    destack_host_macos_notify_power_mode_changed, destack_host_macos_notify_thermal_state_changed,
    destack_host_macos_notify_wake, destack_host_macos_notify_wall_clock_changed,
    destack_host_macos_notify_window_available, destack_host_macos_notify_window_focus_changed,
    destack_host_macos_notify_window_resized, destack_host_macos_notify_window_terminated,
    macos_notify_application_lifecycle, macos_notify_interruption_changed,
    macos_notify_memory_pressure_changed, macos_notify_permission_result,
    macos_notify_power_mode_changed, macos_notify_thermal_state_changed, macos_notify_wake,
    macos_notify_wall_clock_changed, macos_notify_window_available,
    macos_notify_window_focus_changed, macos_notify_window_resized, macos_notify_window_terminated,
};

#[cfg(any(test, target_os = "ios"))]
pub use ios::{
    IosApplicationLifecycle, destack_host_ios_notify_application_lifecycle,
    destack_host_ios_notify_interruption_changed, destack_host_ios_notify_memory_pressure_changed,
    destack_host_ios_notify_permission_result, destack_host_ios_notify_power_mode_changed,
    destack_host_ios_notify_thermal_state_changed, destack_host_ios_notify_wake,
    destack_host_ios_notify_wall_clock_changed, destack_host_ios_notify_window_available,
    destack_host_ios_notify_window_focus_changed, destack_host_ios_notify_window_resized,
    destack_host_ios_notify_window_terminated, ios_notify_application_lifecycle,
    ios_notify_interruption_changed, ios_notify_memory_pressure_changed,
    ios_notify_permission_result, ios_notify_power_mode_changed, ios_notify_thermal_state_changed,
    ios_notify_wake, ios_notify_wall_clock_changed, ios_notify_window_available,
    ios_notify_window_focus_changed, ios_notify_window_resized, ios_notify_window_terminated,
};

#[cfg(any(test, windows))]
pub use windows::{
    WindowsApplicationLifecycle, destack_host_windows_notify_application_lifecycle,
    destack_host_windows_notify_interruption_changed,
    destack_host_windows_notify_memory_pressure_changed,
    destack_host_windows_notify_permission_result, destack_host_windows_notify_power_mode_changed,
    destack_host_windows_notify_thermal_state_changed, destack_host_windows_notify_wake,
    destack_host_windows_notify_wall_clock_changed, destack_host_windows_notify_window_available,
    destack_host_windows_notify_window_focus_changed, destack_host_windows_notify_window_resized,
    destack_host_windows_notify_window_terminated, windows_notify_application_lifecycle,
    windows_notify_interruption_changed, windows_notify_memory_pressure_changed,
    windows_notify_permission_result, windows_notify_power_mode_changed,
    windows_notify_thermal_state_changed, windows_notify_wake, windows_notify_wall_clock_changed,
    windows_notify_window_available, windows_notify_window_focus_changed,
    windows_notify_window_resized, windows_notify_window_terminated,
};

#[cfg(any(test, target_os = "linux"))]
pub use linux::{
    LinuxApplicationLifecycle, destack_host_linux_notify_application_lifecycle,
    destack_host_linux_notify_interruption_changed,
    destack_host_linux_notify_memory_pressure_changed, destack_host_linux_notify_permission_result,
    destack_host_linux_notify_power_mode_changed, destack_host_linux_notify_thermal_state_changed,
    destack_host_linux_notify_wake, destack_host_linux_notify_wall_clock_changed,
    destack_host_linux_notify_window_available, destack_host_linux_notify_window_focus_changed,
    destack_host_linux_notify_window_resized, destack_host_linux_notify_window_terminated,
    linux_notify_application_lifecycle, linux_notify_interruption_changed,
    linux_notify_memory_pressure_changed, linux_notify_permission_result,
    linux_notify_power_mode_changed, linux_notify_thermal_state_changed, linux_notify_wake,
    linux_notify_wall_clock_changed, linux_notify_window_available,
    linux_notify_window_focus_changed, linux_notify_window_resized, linux_notify_window_terminated,
};

#[cfg(any(test, target_os = "freebsd"))]
pub use freebsd::{
    FreeBsdApplicationLifecycle, destack_host_freebsd_notify_application_lifecycle,
    destack_host_freebsd_notify_interruption_changed,
    destack_host_freebsd_notify_memory_pressure_changed,
    destack_host_freebsd_notify_permission_result, destack_host_freebsd_notify_power_mode_changed,
    destack_host_freebsd_notify_thermal_state_changed, destack_host_freebsd_notify_wake,
    destack_host_freebsd_notify_wall_clock_changed, destack_host_freebsd_notify_window_available,
    destack_host_freebsd_notify_window_focus_changed, destack_host_freebsd_notify_window_resized,
    destack_host_freebsd_notify_window_terminated, freebsd_notify_application_lifecycle,
    freebsd_notify_interruption_changed, freebsd_notify_memory_pressure_changed,
    freebsd_notify_permission_result, freebsd_notify_power_mode_changed,
    freebsd_notify_thermal_state_changed, freebsd_notify_wake, freebsd_notify_wall_clock_changed,
    freebsd_notify_window_available, freebsd_notify_window_focus_changed,
    freebsd_notify_window_resized, freebsd_notify_window_terminated,
};

#[cfg(any(test, target_os = "dragonfly"))]
pub use dragonfly::{
    DragonflyApplicationLifecycle, destack_host_dragonfly_notify_application_lifecycle,
    destack_host_dragonfly_notify_interruption_changed,
    destack_host_dragonfly_notify_memory_pressure_changed,
    destack_host_dragonfly_notify_permission_result,
    destack_host_dragonfly_notify_power_mode_changed,
    destack_host_dragonfly_notify_thermal_state_changed, destack_host_dragonfly_notify_wake,
    destack_host_dragonfly_notify_wall_clock_changed,
    destack_host_dragonfly_notify_window_available,
    destack_host_dragonfly_notify_window_focus_changed,
    destack_host_dragonfly_notify_window_resized, destack_host_dragonfly_notify_window_terminated,
    dragonfly_notify_application_lifecycle, dragonfly_notify_interruption_changed,
    dragonfly_notify_memory_pressure_changed, dragonfly_notify_permission_result,
    dragonfly_notify_power_mode_changed, dragonfly_notify_thermal_state_changed,
    dragonfly_notify_wake, dragonfly_notify_wall_clock_changed, dragonfly_notify_window_available,
    dragonfly_notify_window_focus_changed, dragonfly_notify_window_resized,
    dragonfly_notify_window_terminated,
};

#[cfg(any(test, target_os = "netbsd"))]
pub use netbsd::{
    NetBsdApplicationLifecycle, destack_host_netbsd_notify_application_lifecycle,
    destack_host_netbsd_notify_interruption_changed,
    destack_host_netbsd_notify_memory_pressure_changed,
    destack_host_netbsd_notify_permission_result, destack_host_netbsd_notify_power_mode_changed,
    destack_host_netbsd_notify_thermal_state_changed, destack_host_netbsd_notify_wake,
    destack_host_netbsd_notify_wall_clock_changed, destack_host_netbsd_notify_window_available,
    destack_host_netbsd_notify_window_focus_changed, destack_host_netbsd_notify_window_resized,
    destack_host_netbsd_notify_window_terminated, netbsd_notify_application_lifecycle,
    netbsd_notify_interruption_changed, netbsd_notify_memory_pressure_changed,
    netbsd_notify_permission_result, netbsd_notify_power_mode_changed,
    netbsd_notify_thermal_state_changed, netbsd_notify_wake, netbsd_notify_wall_clock_changed,
    netbsd_notify_window_available, netbsd_notify_window_focus_changed,
    netbsd_notify_window_resized, netbsd_notify_window_terminated,
};

#[cfg(any(test, target_os = "openbsd"))]
pub use openbsd::{
    OpenBsdApplicationLifecycle, destack_host_openbsd_notify_application_lifecycle,
    destack_host_openbsd_notify_interruption_changed,
    destack_host_openbsd_notify_memory_pressure_changed,
    destack_host_openbsd_notify_permission_result, destack_host_openbsd_notify_power_mode_changed,
    destack_host_openbsd_notify_thermal_state_changed, destack_host_openbsd_notify_wake,
    destack_host_openbsd_notify_wall_clock_changed, destack_host_openbsd_notify_window_available,
    destack_host_openbsd_notify_window_focus_changed, destack_host_openbsd_notify_window_resized,
    destack_host_openbsd_notify_window_terminated, openbsd_notify_application_lifecycle,
    openbsd_notify_interruption_changed, openbsd_notify_memory_pressure_changed,
    openbsd_notify_permission_result, openbsd_notify_power_mode_changed,
    openbsd_notify_thermal_state_changed, openbsd_notify_wake, openbsd_notify_wall_clock_changed,
    openbsd_notify_window_available, openbsd_notify_window_focus_changed,
    openbsd_notify_window_resized, openbsd_notify_window_terminated,
};

#[cfg(any(test, target_os = "illumos"))]
pub use illumos::{
    IllumosApplicationLifecycle, destack_host_illumos_notify_application_lifecycle,
    destack_host_illumos_notify_interruption_changed,
    destack_host_illumos_notify_memory_pressure_changed,
    destack_host_illumos_notify_permission_result, destack_host_illumos_notify_power_mode_changed,
    destack_host_illumos_notify_thermal_state_changed, destack_host_illumos_notify_wake,
    destack_host_illumos_notify_wall_clock_changed, destack_host_illumos_notify_window_available,
    destack_host_illumos_notify_window_focus_changed, destack_host_illumos_notify_window_resized,
    destack_host_illumos_notify_window_terminated, illumos_notify_application_lifecycle,
    illumos_notify_interruption_changed, illumos_notify_memory_pressure_changed,
    illumos_notify_permission_result, illumos_notify_power_mode_changed,
    illumos_notify_thermal_state_changed, illumos_notify_wake, illumos_notify_wall_clock_changed,
    illumos_notify_window_available, illumos_notify_window_focus_changed,
    illumos_notify_window_resized, illumos_notify_window_terminated,
};

#[cfg(any(test, target_os = "solaris"))]
pub use solaris::{
    SolarisApplicationLifecycle, destack_host_solaris_notify_application_lifecycle,
    destack_host_solaris_notify_interruption_changed,
    destack_host_solaris_notify_memory_pressure_changed,
    destack_host_solaris_notify_permission_result, destack_host_solaris_notify_power_mode_changed,
    destack_host_solaris_notify_thermal_state_changed, destack_host_solaris_notify_wake,
    destack_host_solaris_notify_wall_clock_changed, destack_host_solaris_notify_window_available,
    destack_host_solaris_notify_window_focus_changed, destack_host_solaris_notify_window_resized,
    destack_host_solaris_notify_window_terminated, solaris_notify_application_lifecycle,
    solaris_notify_interruption_changed, solaris_notify_memory_pressure_changed,
    solaris_notify_permission_result, solaris_notify_power_mode_changed,
    solaris_notify_thermal_state_changed, solaris_notify_wake, solaris_notify_wall_clock_changed,
    solaris_notify_window_available, solaris_notify_window_focus_changed,
    solaris_notify_window_resized, solaris_notify_window_terminated,
};

#[cfg(any(test, target_os = "haiku"))]
pub use haiku::{
    HaikuApplicationLifecycle, destack_host_haiku_notify_application_lifecycle,
    destack_host_haiku_notify_interruption_changed,
    destack_host_haiku_notify_memory_pressure_changed, destack_host_haiku_notify_permission_result,
    destack_host_haiku_notify_power_mode_changed, destack_host_haiku_notify_thermal_state_changed,
    destack_host_haiku_notify_wake, destack_host_haiku_notify_wall_clock_changed,
    destack_host_haiku_notify_window_available, destack_host_haiku_notify_window_focus_changed,
    destack_host_haiku_notify_window_resized, destack_host_haiku_notify_window_terminated,
    haiku_notify_application_lifecycle, haiku_notify_interruption_changed,
    haiku_notify_memory_pressure_changed, haiku_notify_permission_result,
    haiku_notify_power_mode_changed, haiku_notify_thermal_state_changed, haiku_notify_wake,
    haiku_notify_wall_clock_changed, haiku_notify_window_available,
    haiku_notify_window_focus_changed, haiku_notify_window_resized, haiku_notify_window_terminated,
};
