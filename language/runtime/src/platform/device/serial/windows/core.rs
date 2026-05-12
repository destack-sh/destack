pub(super) use std::sync::Arc;
pub(super) use std::time::Duration;

pub(super) use windows_sys::Win32::Devices::Communication::{
    CE_BREAK, CE_FRAME, CE_OVERRUN, CE_RXOVER, CE_RXPARITY, CLEAR_COMM_ERROR_FLAGS, CLRBREAK,
    CLRDTR, CLRRTS, COMM_EVENT_MASK, COMMTIMEOUTS, COMSTAT, ClearCommError, DCB,
    ESCAPE_COMM_FUNCTION, EV_BREAK, EV_CTS, EV_DSR, EV_ERR, EV_RING, EV_RLSD, EV_RXCHAR,
    EVENPARITY, EscapeCommFunction, GetCommModemStatus, GetCommState, MARKPARITY, MS_CTS_ON,
    MS_DSR_ON, MS_RING_ON, MS_RLSD_ON, NOPARITY, ODDPARITY, ONE5STOPBITS, ONESTOPBIT,
    PURGE_RXABORT, PURGE_RXCLEAR, PURGE_TXABORT, PURGE_TXCLEAR, PurgeComm, SETBREAK, SETDTR,
    SETRTS, SPACEPARITY, SetCommMask, SetCommState, SetCommTimeouts, SetupComm, TWOSTOPBITS,
    WaitCommEvent,
};
pub(super) use windows_sys::Win32::Devices::DeviceAndDriverInstallation::{
    CM_NOTIFY_ACTION, CM_NOTIFY_ACTION_DEVICEINTERFACEARRIVAL,
    CM_NOTIFY_ACTION_DEVICEINTERFACEREMOVAL, CM_NOTIFY_EVENT_DATA, CM_NOTIFY_FILTER,
    CM_NOTIFY_FILTER_0, CM_NOTIFY_FILTER_0_2, CM_NOTIFY_FILTER_TYPE_DEVICEINTERFACE,
    CM_Register_Notification, CM_Unregister_Notification, CR_SUCCESS, DICS_FLAG_GLOBAL,
    DIGCF_DEVICEINTERFACE, DIGCF_PRESENT, DIREG_DEV, HCMNOTIFICATION, HDEVINFO,
    SP_DEVICE_INTERFACE_DATA, SP_DEVICE_INTERFACE_DETAIL_DATA_W, SP_DEVINFO_DATA, SPDRP_DEVICEDESC,
    SPDRP_FRIENDLYNAME, SPDRP_MFG, SetupDiDestroyDeviceInfoList, SetupDiEnumDeviceInterfaces,
    SetupDiGetClassDevsW, SetupDiGetDeviceInstanceIdW, SetupDiGetDeviceInterfaceDetailW,
    SetupDiGetDevicePropertyW, SetupDiGetDeviceRegistryPropertyW, SetupDiOpenDevRegKey,
};
pub(super) use windows_sys::Win32::Devices::Properties::{
    DEVPKEY_Device_BusReportedDeviceDesc, DEVPKEY_Device_FriendlyName, DEVPKEY_Device_Manufacturer,
    DEVPROPTYPE,
};
pub(super) use windows_sys::Win32::Foundation::{
    ERROR_FILE_NOT_FOUND, ERROR_INSUFFICIENT_BUFFER, ERROR_INVALID_DATA, ERROR_IO_PENDING,
    ERROR_MORE_DATA, ERROR_NO_MORE_ITEMS, ERROR_NOT_FOUND, ERROR_OPERATION_ABORTED,
    ERROR_PATH_NOT_FOUND, ERROR_SEM_TIMEOUT, ERROR_SUCCESS, HANDLE, HANDLE_FLAG_INHERIT,
    INVALID_HANDLE_VALUE, SetHandleInformation, WAIT_TIMEOUT,
};
pub(super) use windows_sys::Win32::Storage::FileSystem::{
    CreateFileW, FILE_ATTRIBUTE_NORMAL, FILE_FLAG_OVERLAPPED, FILE_GENERIC_READ,
    FILE_GENERIC_WRITE, FlushFileBuffers, OPEN_EXISTING, ReadFile, WriteFile,
};
pub(super) use windows_sys::Win32::System::IO::{
    CancelIoEx, GetOverlappedResult, GetOverlappedResultEx, OVERLAPPED,
};
pub(super) use windows_sys::Win32::System::Ioctl::GUID_DEVINTERFACE_COMPORT;
pub(super) use windows_sys::Win32::System::Registry::{
    HKEY, KEY_READ, REG_SZ, RegCloseKey, RegQueryValueExW,
};
pub(super) use windows_sys::Win32::System::Threading::{CreateEventW, INFINITE, ResetEvent};
pub(super) use windows_sys::Win32::System::WindowsProgramming::{
    DTR_CONTROL_ENABLE, DTR_CONTROL_HANDSHAKE, RTS_CONTROL_ENABLE, RTS_CONTROL_HANDSHAKE,
};

pub(super) use crate::diagnostic::{RuntimeError, RuntimeResult};
pub(super) use crate::platform::abi::{NativeSlice, NativeStringRef};
pub(super) use crate::platform::core::BoundedQueue;
pub(super) use crate::platform::device::{
    SerialDataBits, SerialErrorKind, SerialEvent, SerialFlowControl, SerialInputSignals,
    SerialOutputSignals, SerialParity, SerialPortConfig, SerialPortDescriptor,
    SerialPortOpenOptions, SerialPortTransport, SerialStopBits, SerialWatchEvent,
};
pub(super) use crate::platform::diagnostic::PlatformErrorCode;
pub(super) use crate::platform::resource::{ResourceEntry, ResourceKind};
pub(super) use crate::platform::{PlatformError, core as core_platform, resource};
pub(super) use crate::runtime::BindingCallContext;

/// Default serial queue size hint used when the caller supplies no override.
pub(super) const DEFAULT_SERIAL_BUFFER_SIZE: u32 = 4096;

/// Maximum queued serial events per open windows port.
pub(super) const SERIAL_EVENT_QUEUE_CAPACITY: usize = 128;

/// Event mask used by the windows serial event loop.
pub(super) const SERIAL_COMM_EVENT_MASK: COMM_EVENT_MASK =
    EV_RXCHAR | EV_ERR | EV_BREAK | EV_CTS | EV_DSR | EV_RING | EV_RLSD;

/// Initial buffer length for one SetupAPI property read.
pub(super) const INITIAL_SETUP_PROPERTY_CAPACITY: usize = 256;

/// Initial buffer length for one SetupAPI instance-id read.
pub(super) const INITIAL_INSTANCE_ID_CAPACITY: usize = 256;

/// DCB bit for binary mode.
pub(super) const DCB_BINARY_BIT: u32 = 1 << 0;

/// DCB bit for parity enable.
pub(super) const DCB_PARITY_ENABLE_BIT: u32 = 1 << 1;

/// DCB bit for RTS/CTS outbound flow control.
pub(super) const DCB_OUTX_CTS_FLOW_BIT: u32 = 1 << 2;

/// DCB bit for DSR outbound flow control.
pub(super) const DCB_OUTX_DSR_FLOW_BIT: u32 = 1 << 3;

/// DCB field shift for DTR control.
pub(super) const DCB_DTR_CONTROL_SHIFT: u32 = 4;

/// DCB field mask for DTR control.
pub(super) const DCB_DTR_CONTROL_MASK: u32 = 0b11 << DCB_DTR_CONTROL_SHIFT;

/// DCB bit for DSR sensitivity.
pub(super) const DCB_DSR_SENSITIVITY_BIT: u32 = 1 << 6;

/// DCB bit for continuing TX while remote XOFF is active.
pub(super) const DCB_TX_CONTINUE_ON_XOFF_BIT: u32 = 1 << 7;

/// DCB bit for outbound XON/XOFF.
pub(super) const DCB_OUT_X_BIT: u32 = 1 << 8;

/// DCB bit for inbound XON/XOFF.
pub(super) const DCB_IN_X_BIT: u32 = 1 << 9;

/// DCB bit for error-character substitution.
pub(super) const DCB_ERROR_CHAR_BIT: u32 = 1 << 10;

/// DCB bit for null stripping.
pub(super) const DCB_NULL_BIT: u32 = 1 << 11;

/// DCB field shift for RTS control.
pub(super) const DCB_RTS_CONTROL_SHIFT: u32 = 12;

/// DCB field mask for RTS control.
pub(super) const DCB_RTS_CONTROL_MASK: u32 = 0b11 << DCB_RTS_CONTROL_SHIFT;

/// DCB bit for abort-on-error mode.
pub(super) const DCB_ABORT_ON_ERROR_BIT: u32 = 1 << 14;
