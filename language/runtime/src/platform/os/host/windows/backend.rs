use windows_sys::Win32::System::SystemInformation::{
    ComputerNamePhysicalDnsHostname, GetComputerNameExW, GetNativeSystemInfo, GetVersionExW,
    OSVERSIONINFOW, PROCESSOR_ARCHITECTURE_AMD64, PROCESSOR_ARCHITECTURE_ARM,
    PROCESSOR_ARCHITECTURE_ARM64, PROCESSOR_ARCHITECTURE_INTEL, PROCESSOR_ARCHITECTURE_UNKNOWN,
    SYSTEM_INFO,
};

use crate::diagnostic::RuntimeResult;
use crate::platform::core as core_platform;
use crate::runtime::BindingCallContext;

use super::super::core::{
    HostIdentityOwned, OS_HOST_IDENTITY_OPERATION, decode_utf16_buffer, invalid_data,
};

/// Read one hostname value from the windows host.
fn hostname_value() -> RuntimeResult<String> {
    // query required buffer size first
    let mut required = 0u32;
    unsafe {
        GetComputerNameExW(
            ComputerNamePhysicalDnsHostname,
            std::ptr::null_mut(),
            &mut required,
        );
    }
    if required == 0 {
        return Err(core_platform::io_error("GetComputerNameExW"));
    }

    // read the hostname payload into one utf16 buffer
    let mut buffer = vec![0u16; required as usize];
    let mut length = required;
    let status = unsafe {
        GetComputerNameExW(
            ComputerNamePhysicalDnsHostname,
            buffer.as_mut_ptr(),
            &mut length,
        )
    };
    if status == 0 {
        return Err(core_platform::io_error("GetComputerNameExW"));
    }

    // decode hostname from reported utf16 payload length
    if length == 0 {
        return Err(invalid_data(
            OS_HOST_IDENTITY_OPERATION,
            "GetComputerNameExW returned an empty hostname payload",
        ));
    }
    let length = usize::try_from(length).map_err(|_| {
        invalid_data(
            OS_HOST_IDENTITY_OPERATION,
            "hostname length exceeded usize range",
        )
    })?;
    decode_utf16_buffer(OS_HOST_IDENTITY_OPERATION, "hostname", &buffer[..length])
}

/// Read one windows release string.
fn release_value() -> RuntimeResult<String> {
    // query version payload from Win32 APIs
    let mut version = unsafe { std::mem::zeroed::<OSVERSIONINFOW>() };
    version.dwOSVersionInfoSize = std::mem::size_of::<OSVERSIONINFOW>() as u32;
    let status = unsafe { GetVersionExW(&mut version) };
    if status == 0 {
        return Err(core_platform::io_error("GetVersionExW"));
    }

    Ok(format!(
        "{}.{}.{}",
        version.dwMajorVersion, version.dwMinorVersion, version.dwBuildNumber
    ))
}

/// Read one windows processor architecture label.
fn architecture_value() -> String {
    // query native system information for architecture mapping
    let mut info = unsafe { std::mem::zeroed::<SYSTEM_INFO>() };
    unsafe {
        GetNativeSystemInfo(&mut info);
    }

    // decode the architecture selector from the nested union payload
    let architecture = unsafe { info.Anonymous.Anonymous.wProcessorArchitecture };
    match architecture {
        PROCESSOR_ARCHITECTURE_AMD64 => "x86_64".to_string(),
        PROCESSOR_ARCHITECTURE_INTEL => "x86".to_string(),
        PROCESSOR_ARCHITECTURE_ARM => "arm".to_string(),
        PROCESSOR_ARCHITECTURE_ARM64 => "arm64".to_string(),
        PROCESSOR_ARCHITECTURE_UNKNOWN => "unknown".to_string(),
        _ => format!("arch-{architecture}"),
    }
}

/// Read one host identity payload from windows APIs.
pub(crate) fn read_host_identity(
    _context: &BindingCallContext,
) -> RuntimeResult<HostIdentityOwned> {
    // query host identity fields from windows APIs
    let hostname = hostname_value()?;
    let release = release_value()?;
    let architecture = architecture_value();

    Ok(HostIdentityOwned {
        hostname,
        kernel: "windows".to_string(),
        release,
        architecture,
    })
}
