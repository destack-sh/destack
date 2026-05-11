use std::path::{Path, PathBuf};
use std::sync::Arc;

use windows::Win32::Foundation::{
    APPMODEL_ERROR_NO_PACKAGE, ERROR_INSUFFICIENT_BUFFER, ERROR_SUCCESS, WIN32_ERROR,
};
use windows::Win32::Storage::EnhancedStorage::{
    PKEY_AppUserModel_ID, PKEY_AppUserModel_ToastActivatorCLSID,
};
use windows::Win32::Storage::Packaging::Appx::GetCurrentPackageFullName;
use windows::Win32::System::Com::StructuredStorage::{
    InitPropVariantFromCLSID, PROPVARIANT, PROPVARIANT_0, PROPVARIANT_0_0, PROPVARIANT_0_0_0,
};
use windows::Win32::System::Com::{
    CLSCTX_INPROC_SERVER, CLSCTX_LOCAL_SERVER, CO_MTA_USAGE_COOKIE, CoCreateInstance,
    CoIncrementMTAUsage, CoRegisterClassObject, CoResumeClassObjects, CoTaskMemAlloc,
    IClassFactory, IPersistFile, REGCLS_MULTIPLEUSE, REGCLS_SUSPENDED,
};
use windows::Win32::System::Registry::{
    HKEY, HKEY_CURRENT_USER, KEY_SET_VALUE, REG_OPTION_NON_VOLATILE, REG_SZ, RegCreateKeyExW,
    RegSetValueExW,
};
use windows::Win32::UI::Shell::PropertiesSystem::IPropertyStore;
use windows::Win32::UI::Shell::{
    FOLDERID_Programs, SetCurrentProcessExplicitAppUserModelID, ShellLink,
};
use windows::core::{HSTRING, PCWSTR};
use windows_core::{IUnknownImpl, Interface};

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::host::RequestContext;
use crate::host::os::windows::identity::{
    current_executable_path, resolved_application_identifier, resolved_display_name,
    resolved_icon_path,
};
use crate::platform::PlatformError;
use crate::platform::core::windows_known_folder_path;
use crate::platform::diagnostic::PlatformErrorCode;
use crate::runtime::service::executor::thread::ServiceThreadExecutor;
use crate::runtime::service::{Service, spawn_service_thread};
use crate::runtime::{ExecutionAffinity, ExecutionMode, ExecutionPolicy};

use super::core::{
    WINDOWS_NOTIFICATION_ACTIVATOR_FACTORY, WINDOWS_NOTIFICATION_APP_ID_MAX_LENGTH,
    WINDOWS_NOTIFICATION_APP_LINK_EXTENSION, WINDOWS_NOTIFICATION_ICON_BACKGROUND_COLOR,
    WINDOWS_NOTIFICATION_LOCAL_SERVER_REGISTRY_PREFIX, WINDOWS_NOTIFICATION_REGISTRY_PREFIX,
    WINDOWS_TOAST_IDENTITY, fnv1a64, sanitized_windows_notification_component,
    truncated_windows_notification_app_id, windows_notification_error,
};

/// Desktop toast identity metadata for the current Windows process.
#[derive(Debug, Clone)]
pub(super) struct WindowsToastIdentity {
    /// The toast application id when the process is unpackaged.
    pub(super) app_id: HSTRING,
    /// The explicit unpackaged application id when present.
    pub(super) explicit_app_id: Option<String>,
    /// Whether this process must use the explicit app id path.
    pub(super) uses_explicit_app_id: bool,
    /// Stable activation CLSID for unpackaged desktop activation.
    pub(super) activator_clsid: windows::core::GUID,
}

/// Owned registry key handle for one temporary write scope.
pub(super) struct WindowsRegistryKey {
    /// The raw registry key handle.
    pub(super) key: HKEY,
}

/// Owned PROPVARIANT cleared automatically after one property-store write.
pub(super) struct WindowsPropVariant {
    /// The inner PROPVARIANT payload.
    pub(super) value: PROPVARIANT,
}

/// Process-global Windows COM notification service.
pub(super) struct WindowsNotificationComService {
    /// Dedicated COM service thread that owns the local-server class registration.
    pub(super) _executor: ServiceThreadExecutor<WindowsNotificationComServerState>,
}

/// Live COM server state held for the lifetime of the dedicated server thread.
pub(super) struct WindowsNotificationComServerState {
    /// MTA usage cookie that keeps the COM multithreaded apartment alive.
    pub(super) _mta_usage_cookie: CO_MTA_USAGE_COOKIE,
    /// Class factory registration cookie for the notification activator.
    pub(super) _registration_cookie: u32,
    /// Strong class factory reference kept alive for the registered COM server.
    pub(super) _class_factory: IClassFactory,
}

/// The execution policy for the Windows notification COM service.
const WINDOWS_NOTIFICATION_COM_POLICY: ExecutionPolicy =
    ExecutionPolicy::process(ExecutionMode::Thread).with_affinity(ExecutionAffinity::WindowsMta);

impl Service for WindowsNotificationComService {
    const POLICY: ExecutionPolicy = WINDOWS_NOTIFICATION_COM_POLICY;
}

/// Return one Windows toast notifier configured for packaged or unpackaged hosts.
pub(super) fn windows_toast_notifier(
    context: &RequestContext,
) -> RuntimeResult<windows::UI::Notifications::ToastNotifier> {
    use windows::UI::Notifications::ToastNotificationManager;

    let identity = windows_toast_identity(context)?;

    if !identity.uses_explicit_app_id {
        return ToastNotificationManager::CreateToastNotifier().map_err(windows_notification_error);
    }

    ToastNotificationManager::CreateToastNotifierWithId(&identity.app_id)
        .map_err(windows_notification_error)
}

/// Return one Windows toast history object after ensuring desktop identity setup.
pub(super) fn windows_toast_history(
    context: &RequestContext,
) -> RuntimeResult<windows::UI::Notifications::ToastNotificationHistory> {
    use windows::UI::Notifications::ToastNotificationManager;

    windows_toast_identity(context)?;

    ToastNotificationManager::History().map_err(windows_notification_error)
}

/// Return the configured toast identity for the current Windows process.
pub(super) fn windows_toast_identity(
    context: &RequestContext,
) -> RuntimeResult<&'static WindowsToastIdentity> {
    let activator_clsid = windows_notification_activator_clsid(context)?;
    let explicit_app_id = if windows_process_has_package_identity()? {
        None
    } else {
        Some(windows_notification_app_id(context)?)
    };
    let identity = WINDOWS_TOAST_IDENTITY.get_or_init(|| {
        if let Some(app_id) = explicit_app_id.clone() {
            ensure_windows_desktop_notification_registration(context, &app_id, &activator_clsid)?;

            unsafe { SetCurrentProcessExplicitAppUserModelID(&HSTRING::from(app_id.as_str())) }
                .map_err(windows_notification_error)?;

            return Ok(WindowsToastIdentity {
                app_id: HSTRING::from(app_id.as_str()),
                explicit_app_id: Some(app_id),
                uses_explicit_app_id: true,
                activator_clsid,
            });
        }

        Ok(WindowsToastIdentity {
            app_id: HSTRING::new(),
            explicit_app_id: None,
            uses_explicit_app_id: false,
            activator_clsid,
        })
    });

    match identity {
        Ok(identity) => {
            if identity.explicit_app_id != explicit_app_id {
                return Err(RuntimeError::from(PlatformError::generic(
                    Some(PlatformErrorCode::Generic),
                    "destack.os.notification detected conflicting Windows desktop app identities in one process",
                ))
                .boxed());
            }

            Ok(identity)
        }
        Err(error) => Err(error.clone()),
    }
}

/// Return whether the current Windows process has package identity.
pub(super) fn windows_process_has_package_identity() -> RuntimeResult<bool> {
    let mut length = 0u32;
    let status = unsafe { GetCurrentPackageFullName(&mut length, None) };

    if status == ERROR_INSUFFICIENT_BUFFER {
        return Ok(true);
    }

    if status == APPMODEL_ERROR_NO_PACKAGE {
        return Ok(false);
    }

    if status == ERROR_SUCCESS {
        return Ok(true);
    }

    Err(windows_notification_registry_status_error(
        "GetCurrentPackageFullName",
        status,
    ))
}

/// Register the current unpackaged process under one stable desktop toast app id.
fn ensure_windows_desktop_notification_registration(
    context: &RequestContext,
    app_id: &str,
    activator_clsid: &windows::core::GUID,
) -> RuntimeResult<()> {
    let key =
        windows_registry_create_key(&format!(r"{WINDOWS_NOTIFICATION_REGISTRY_PREFIX}\{app_id}"))?;
    let display_name = windows_notification_display_name(context)?;
    let icon_path = windows_notification_icon_path(context)?;

    windows_registry_set_string_value(&key, "DisplayName", &display_name)?;
    windows_registry_set_wide_value(&key, "IconUri", &icon_path)?;
    windows_registry_set_string_value(
        &key,
        "IconBackgroundColor",
        WINDOWS_NOTIFICATION_ICON_BACKGROUND_COLOR,
    )?;
    ensure_windows_notification_local_server_registration(context, activator_clsid)?;
    ensure_windows_notification_shortcut(context, app_id, activator_clsid)?;

    Ok(())
}

/// Register the unpackaged local-server COM activator for this process.
fn ensure_windows_notification_local_server_registration(
    context: &RequestContext,
    activator_clsid: &windows::core::GUID,
) -> RuntimeResult<()> {
    let clsid = windows_guid_string(activator_clsid);
    let class_key = windows_registry_create_key(&format!(
        r"{WINDOWS_NOTIFICATION_LOCAL_SERVER_REGISTRY_PREFIX}\{clsid}"
    ))?;
    let local_server_key = windows_registry_create_key(&format!(
        r"{WINDOWS_NOTIFICATION_LOCAL_SERVER_REGISTRY_PREFIX}\{clsid}\LocalServer32"
    ))?;
    let display_name = windows_notification_display_name(context)?;
    let executable_path = current_executable_path()?;
    let executable_command = windows_local_server_command(&executable_path);

    windows_notification_registry_set_default_string_value(&class_key, &display_name)?;
    windows_notification_registry_set_default_string_value(&local_server_key, &executable_command)?;

    Ok(())
}

/// Create or update the Start Menu shortcut required for unpackaged desktop toasts.
fn ensure_windows_notification_shortcut(
    context: &RequestContext,
    app_id: &str,
    activator_clsid: &windows::core::GUID,
) -> RuntimeResult<()> {
    let shortcut_path = windows_notification_shortcut_path(app_id)?;
    let display_name = windows_notification_display_name(context)?;
    let executable_path = current_executable_path()?;
    let executable_path_wide = path_to_wide(&executable_path)?;
    let shortcut_path_wide = path_to_wide(&shortcut_path)?;
    let description = HSTRING::from(display_name);
    let app_id = HSTRING::from(app_id);

    let shell_link = unsafe {
        CoCreateInstance::<_, windows::Win32::UI::Shell::IShellLinkW>(
            &ShellLink,
            None,
            CLSCTX_INPROC_SERVER,
        )
    }
    .map_err(windows_notification_error)?;
    let property_store = shell_link
        .cast::<IPropertyStore>()
        .map_err(windows_notification_error)?;
    let persist_file = shell_link
        .cast::<IPersistFile>()
        .map_err(windows_notification_error)?;

    unsafe {
        shell_link
            .SetPath(PCWSTR(executable_path_wide.as_ptr()))
            .map_err(windows_notification_error)?;
        shell_link
            .SetIconLocation(PCWSTR(executable_path_wide.as_ptr()), 0)
            .map_err(windows_notification_error)?;
        shell_link
            .SetDescription(&description)
            .map_err(windows_notification_error)?;
    }

    let app_id_wide = wide_string_bytes(&app_id.to_string_lossy());
    let app_id_value = windows_propvariant_from_string(&app_id_wide)?;
    let activator_clsid_value = windows_propvariant_from_clsid(activator_clsid)?;

    unsafe {
        property_store
            .SetValue(&PKEY_AppUserModel_ID, &app_id_value.value)
            .map_err(windows_notification_error)?;
        property_store
            .SetValue(
                &PKEY_AppUserModel_ToastActivatorCLSID,
                &activator_clsid_value.value,
            )
            .map_err(windows_notification_error)?;
        property_store
            .Commit()
            .map_err(windows_notification_error)?;
        persist_file
            .Save(PCWSTR(shortcut_path_wide.as_ptr()), true)
            .map_err(windows_notification_error)?;
    }

    Ok(())
}

/// Return one shared COM registration for the Windows notification activator.
pub(super) fn ensure_windows_notification_com_registration(
    context: &RequestContext,
) -> RuntimeResult<()> {
    let identity = windows_toast_identity(context)?;

    if !identity.uses_explicit_app_id {
        return Ok(());
    }

    let _service = windows_notification_com_service(identity.activator_clsid)?;

    Ok(())
}

/// Return one stable unpackaged toast app id for the current executable.
pub(super) fn windows_notification_app_id(context: &RequestContext) -> RuntimeResult<String> {
    let executable_path = current_executable_path()?;
    let path_hash = fnv1a64(executable_path.as_os_str().to_string_lossy().as_bytes());
    let declared_identifier = context.app_identity.identifier.as_deref().map(str::trim);
    let app_identifier = resolved_application_identifier(context)?;
    let app_id = sanitized_windows_notification_component(&app_identifier);

    if declared_identifier.is_some() {
        return Ok(truncated_windows_notification_app_id(app_id));
    }

    let hash_suffix = format!(".{path_hash:016x}");
    let max_identifier_length = WINDOWS_NOTIFICATION_APP_ID_MAX_LENGTH
        .saturating_sub(hash_suffix.len())
        .max(1usize);
    let app_id = if app_id.len() > max_identifier_length {
        app_id[..max_identifier_length].to_string()
    } else {
        app_id
    };

    Ok(format!("{app_id}{hash_suffix}"))
}

/// Return one display name for the current Windows notification host identity.
pub(super) fn windows_notification_display_name(context: &RequestContext) -> RuntimeResult<String> {
    resolved_display_name(context)
}

/// Return one UTF-16 icon path payload for the current executable.
pub(super) fn windows_notification_icon_path(context: &RequestContext) -> RuntimeResult<Vec<u16>> {
    use std::os::windows::ffi::OsStrExt;

    let icon_path = resolved_icon_path(context)?;
    let mut icon_path = icon_path.as_os_str().encode_wide().collect::<Vec<_>>();
    icon_path.push(0);

    Ok(icon_path)
}

/// Write one string registry value for the unpackaged toast app id entry.
pub(super) fn windows_registry_set_string_value(
    key: &WindowsRegistryKey,
    name: &str,
    value: &str,
) -> RuntimeResult<()> {
    let value = wide_string_bytes(value);

    windows_registry_set_wide_value(key, name, &value)
}

/// Write one UTF-16 registry value for the unpackaged toast app id entry.
pub(super) fn windows_registry_set_wide_value(
    key: &WindowsRegistryKey,
    name: &str,
    value: &[u16],
) -> RuntimeResult<()> {
    let name = wide_string_bytes(name);
    let data = wide_bytes(value);
    let status = unsafe {
        RegSetValueExW(
            key.key,
            PCWSTR(name.as_ptr().cast()),
            None,
            REG_SZ,
            Some(data),
        )
    };

    if status == ERROR_SUCCESS {
        return Ok(());
    }

    Err(windows_notification_registry_status_error(
        "RegSetValueExW",
        status,
    ))
}

/// Return one registry key handle for the provided key path.
pub(super) fn windows_registry_create_key(path: &str) -> RuntimeResult<WindowsRegistryKey> {
    let path = wide_string_bytes(path);
    let mut key = HKEY::default();
    let status = unsafe {
        RegCreateKeyExW(
            HKEY_CURRENT_USER,
            PCWSTR(path.as_ptr()),
            None,
            None,
            REG_OPTION_NON_VOLATILE,
            KEY_SET_VALUE,
            None,
            &mut key,
            None,
        )
    };

    if status == ERROR_SUCCESS {
        return Ok(WindowsRegistryKey { key });
    }

    Err(windows_notification_registry_status_error(
        "RegCreateKeyExW",
        status,
    ))
}

/// Write one default unnamed string registry value.
pub(super) fn windows_notification_registry_set_default_string_value(
    key: &WindowsRegistryKey,
    value: &str,
) -> RuntimeResult<()> {
    let value = wide_string_bytes(value);
    let data = wide_bytes(&value);
    let status = unsafe { RegSetValueExW(key.key, PCWSTR::null(), None, REG_SZ, Some(data)) };

    if status == ERROR_SUCCESS {
        return Ok(());
    }

    Err(windows_notification_registry_status_error(
        "RegSetValueExW",
        status,
    ))
}

/// Return one UTF-16 path buffer suitable for Win32 path APIs.
fn path_to_wide(path: &Path) -> RuntimeResult<Vec<u16>> {
    let path = path.to_str().ok_or_else(|| {
        RuntimeError::from(PlatformError::generic(
            Some(PlatformErrorCode::Generic),
            "destack.os.notification failed to encode one Windows path as UTF-16",
        ))
        .boxed()
    })?;

    Ok(wide_string_bytes(path))
}

/// Return one PROPVARIANT containing one string value.
pub(super) fn windows_propvariant_from_string(value: &[u16]) -> RuntimeResult<WindowsPropVariant> {
    use windows::Win32::System::Variant::VT_LPWSTR;
    use windows::core::PWSTR;

    let byte_length = std::mem::size_of_val(value);
    let buffer = unsafe { CoTaskMemAlloc(byte_length) } as *mut u16;

    if buffer.is_null() {
        return Err(RuntimeError::from(PlatformError::generic(
            Some(PlatformErrorCode::Generic),
            "destack.os.notification failed to allocate one Windows property string",
        ))
        .boxed());
    }

    unsafe {
        std::ptr::copy_nonoverlapping(value.as_ptr(), buffer, value.len());
    }

    let value = PROPVARIANT {
        Anonymous: PROPVARIANT_0 {
            Anonymous: std::mem::ManuallyDrop::new(PROPVARIANT_0_0 {
                vt: VT_LPWSTR,
                wReserved1: 0,
                wReserved2: 0,
                wReserved3: 0,
                Anonymous: PROPVARIANT_0_0_0 {
                    pwszVal: PWSTR(buffer),
                },
            }),
        },
    };

    Ok(WindowsPropVariant { value })
}

/// Return one PROPVARIANT containing one CLSID value.
pub(super) fn windows_propvariant_from_clsid(
    value: &windows::core::GUID,
) -> RuntimeResult<WindowsPropVariant> {
    let value = unsafe { InitPropVariantFromCLSID(value) }.map_err(windows_notification_error)?;

    Ok(WindowsPropVariant { value })
}

/// Return the LocalServer32 command string for this executable.
pub(super) fn windows_local_server_command(executable_path: &Path) -> String {
    format!("\"{}\"", executable_path.to_string_lossy())
}

/// Return the Start Menu shortcut path for one unpackaged notification app id.
pub(super) fn windows_notification_shortcut_path(app_id: &str) -> RuntimeResult<PathBuf> {
    let programs_directory =
        windows_known_folder_path(&FOLDERID_Programs, "destack.os.notification")?;
    let file_name = format!(
        "{}.{WINDOWS_NOTIFICATION_APP_LINK_EXTENSION}",
        sanitized_windows_notification_component(app_id)
    );

    Ok(programs_directory.join(file_name))
}

/// Start the dedicated COM server thread for unpackaged Windows notification activation.
fn start_windows_notification_com_server(
    activator_clsid: windows::core::GUID,
) -> RuntimeResult<WindowsNotificationComService> {
    let executor = spawn_service_thread(
        "destack-notification-com",
        WindowsNotificationComService::POLICY,
        move || run_windows_notification_com_server_state(activator_clsid),
    )?;

    Ok(WindowsNotificationComService {
        _executor: executor,
    })
}

/// Build the live COM server state for the dedicated server thread.
fn run_windows_notification_com_server_state(
    activator_clsid: windows::core::GUID,
) -> RuntimeResult<WindowsNotificationComServerState> {
    let _mta_usage_cookie = unsafe { CoIncrementMTAUsage() }.map_err(windows_notification_error)?;
    let class_factory = WINDOWS_NOTIFICATION_ACTIVATOR_FACTORY.to_interface::<IClassFactory>();
    let registration_cookie = unsafe {
        CoRegisterClassObject(
            &activator_clsid,
            &class_factory,
            CLSCTX_LOCAL_SERVER,
            REGCLS_MULTIPLEUSE | REGCLS_SUSPENDED,
        )
    }
    .map_err(windows_notification_error)?;

    unsafe { CoResumeClassObjects() }.map_err(windows_notification_error)?;

    Ok(WindowsNotificationComServerState {
        _mta_usage_cookie,
        _registration_cookie: registration_cookie,
        _class_factory: class_factory,
    })
}

/// Return the shared Windows COM notification service.
fn windows_notification_com_service(
    activator_clsid: windows::core::GUID,
) -> RuntimeResult<Arc<WindowsNotificationComService>> {
    WindowsNotificationComService::global(|| start_windows_notification_com_server(activator_clsid))
}

/// Return one deterministic activation CLSID for the current Windows app identity.
pub(super) fn windows_notification_activator_clsid(
    context: &RequestContext,
) -> RuntimeResult<windows::core::GUID> {
    use openssl::sha::sha1;

    let application_id = resolved_application_identifier(context)?;
    let mut name = Vec::new();
    name.extend_from_slice(
        &super::core::WINDOWS_NOTIFICATION_ACTIVATOR_NAMESPACE
            .to_u128()
            .to_be_bytes(),
    );
    name.extend_from_slice(application_id.as_bytes());

    let digest = sha1(&name);
    let mut uuid = [0u8; 16];
    uuid.copy_from_slice(&digest[..16]);

    uuid[6] = (uuid[6] & 0x0f) | 0x50;
    uuid[8] = (uuid[8] & 0x3f) | 0x80;

    Ok(windows::core::GUID::from_values(
        u32::from_be_bytes([uuid[0], uuid[1], uuid[2], uuid[3]]),
        u16::from_be_bytes([uuid[4], uuid[5]]),
        u16::from_be_bytes([uuid[6], uuid[7]]),
        [
            uuid[8], uuid[9], uuid[10], uuid[11], uuid[12], uuid[13], uuid[14], uuid[15],
        ],
    ))
}

/// Return one canonical Windows GUID string with surrounding braces.
pub(super) fn windows_guid_string(value: &windows::core::GUID) -> String {
    format!(
        "{{{:08x}-{:04x}-{:04x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}}}",
        value.data1,
        value.data2,
        value.data3,
        value.data4[0],
        value.data4[1],
        value.data4[2],
        value.data4[3],
        value.data4[4],
        value.data4[5],
        value.data4[6],
        value.data4[7]
    )
}

/// Return one nul-terminated UTF-16 buffer for one Windows string payload.
fn wide_string_bytes(value: &str) -> Vec<u16> {
    let mut wide = value.encode_utf16().collect::<Vec<_>>();
    wide.push(0);
    wide
}

/// Return one byte view over one UTF-16 buffer.
fn wide_bytes(value: &[u16]) -> &[u8] {
    unsafe { std::slice::from_raw_parts(value.as_ptr().cast::<u8>(), std::mem::size_of_val(value)) }
}

/// Map one Windows registry status into one runtime error.
fn windows_notification_registry_status_error(
    call: &str,
    status: WIN32_ERROR,
) -> Box<RuntimeError> {
    RuntimeError::from(PlatformError::generic(
        Some(PlatformErrorCode::Generic),
        format!(
            "destack.os.notification Windows desktop identity call {call} failed with status {}",
            status.0
        ),
    ))
    .boxed()
}
