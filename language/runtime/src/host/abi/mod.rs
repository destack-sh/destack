#[cfg_attr(feature = "generator", allow(dead_code))]
#[cfg(feature = "generator")]
pub mod background;
#[allow(dead_code)]
#[cfg(not(feature = "generator"))]
pub mod background;

#[cfg_attr(feature = "generator", allow(dead_code))]
#[cfg(feature = "generator")]
pub mod bluetooth;
#[allow(dead_code)]
#[cfg(all(not(feature = "generator"), any(test, target_os = "android")))]
pub mod bluetooth;

#[cfg_attr(feature = "generator", allow(dead_code))]
#[cfg(feature = "generator")]
pub mod calendar;
#[allow(dead_code)]
#[cfg(not(feature = "generator"))]
pub mod calendar;

#[cfg_attr(feature = "generator", allow(dead_code))]
#[cfg(feature = "generator")]
pub mod camera;
#[allow(dead_code)]
#[cfg(all(not(feature = "generator"), any(test, target_os = "android")))]
pub mod camera;

#[cfg_attr(feature = "generator", allow(dead_code))]
#[cfg(feature = "generator")]
pub mod contact;
#[allow(dead_code)]
#[cfg(not(feature = "generator"))]
pub mod contact;

#[cfg_attr(feature = "generator", allow(dead_code))]
#[cfg(feature = "generator")]
pub mod credentials;
#[allow(dead_code)]
#[cfg(all(not(feature = "generator"), any(test, target_os = "android")))]
pub mod credentials;

#[cfg_attr(feature = "generator", allow(dead_code))]
#[cfg(feature = "generator")]
pub mod crypto;
#[allow(dead_code)]
#[cfg(all(not(feature = "generator"), any(test, target_os = "android")))]
pub mod crypto;

#[cfg_attr(feature = "generator", allow(dead_code))]
#[cfg(feature = "generator")]
pub mod core;
#[allow(dead_code)]
#[cfg(not(feature = "generator"))]
pub mod core;

#[cfg_attr(feature = "generator", allow(dead_code))]
#[cfg(feature = "generator")]
pub mod describe;
#[allow(dead_code)]
#[cfg(not(feature = "generator"))]
pub mod describe;

#[cfg_attr(feature = "generator", allow(dead_code))]
#[cfg(feature = "generator")]
pub mod document;
#[allow(dead_code)]
#[cfg(not(feature = "generator"))]
pub mod document;

#[cfg_attr(feature = "generator", allow(dead_code))]
#[cfg(feature = "generator")]
pub mod intent;
#[allow(dead_code)]
#[cfg(not(feature = "generator"))]
pub mod intent;

#[cfg_attr(feature = "generator", allow(dead_code))]
#[cfg(feature = "generator")]
pub mod location;
#[allow(dead_code)]
#[cfg(not(feature = "generator"))]
pub mod location;

#[cfg_attr(feature = "generator", allow(dead_code))]
#[cfg(feature = "generator")]
pub mod media;
#[allow(dead_code)]
#[cfg(not(feature = "generator"))]
pub mod media;

#[cfg_attr(feature = "generator", allow(dead_code))]
#[cfg(feature = "generator")]
pub mod midi;
#[allow(dead_code)]
#[cfg(all(not(feature = "generator"), any(test, target_os = "android")))]
pub mod midi;

#[cfg_attr(feature = "generator", allow(dead_code))]
#[cfg(feature = "generator")]
pub mod notification;
#[allow(dead_code)]
#[cfg(not(feature = "generator"))]
pub mod notification;

#[cfg_attr(feature = "generator", allow(dead_code))]
#[cfg(feature = "generator")]
pub mod permission;
#[allow(dead_code)]
#[cfg(not(feature = "generator"))]
pub mod permission;

#[cfg_attr(feature = "generator", allow(dead_code))]
#[cfg(feature = "generator")]
pub mod text;
#[allow(dead_code)]
#[cfg(not(feature = "generator"))]
pub mod text;

#[cfg_attr(feature = "generator", allow(dead_code))]
#[cfg(feature = "generator")]
pub mod usb;
#[allow(dead_code)]
#[cfg(all(not(feature = "generator"), any(test, target_os = "android")))]
pub mod usb;

#[cfg(feature = "generator")]
use crate::host::abi::describe::HostAbiModule;

/// Return the authored generated host ABI modules.
#[cfg(feature = "generator")]
pub fn host_abi_modules() -> Vec<HostAbiModule> {
    vec![
        background::host_abi_module(),
        bluetooth::host_abi_module(),
        calendar::host_abi_module(),
        camera::host_abi_module(),
        contact::host_abi_module(),
        credentials::host_abi_module(),
        crypto::host_abi_module(),
        document::host_abi_module(),
        intent::host_abi_module(),
        location::host_abi_module(),
        media::host_abi_module(),
        midi::host_abi_module(),
        notification::host_abi_module(),
        permission::host_abi_module(),
        text::host_abi_module(),
        usb::host_abi_module(),
    ]
}
