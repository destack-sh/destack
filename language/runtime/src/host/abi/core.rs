use crate::diagnostic::RuntimeResult;
use crate::platform::NativeAbiCodec;
use crate::runtime::{BindingCallContext, NativeStringRef};

/// Process-global host-session handle passed through the host ABI.
#[cfg_attr(not(any(target_os = "android", target_os = "ios")), allow(dead_code))]
pub(crate) type HostSessionHandle = u64;

/// One optional host string reference.
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub(crate) struct HostOptionalStringRef {
    /// Whether the optional field is present.
    pub has_value: bool,
    /// The wrapped string reference.
    pub value: NativeStringRef,
}

impl HostOptionalStringRef {
    /// Return one absent optional string reference.
    pub(crate) fn none() -> Self {
        Self {
            has_value: false,
            value: NativeStringRef::from(""),
        }
    }
}

impl NativeAbiCodec for HostOptionalStringRef {
    type Value = Option<String>;

    unsafe fn into_value(self) -> RuntimeResult<Self::Value> {
        if !self.has_value {
            return Ok(None);
        }

        Ok(Some(unsafe {
            <NativeStringRef as NativeAbiCodec>::into_value(self.value)?
        }))
    }

    fn from_value(binding: &BindingCallContext, value: Self::Value) -> Self {
        match value {
            Some(value) => Self {
                has_value: true,
                value: <NativeStringRef as NativeAbiCodec>::from_value(binding, value),
            },
            None => Self::none(),
        }
    }
}

/// One optional host `u32`.
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub(crate) struct HostOptionalU32 {
    /// Whether the optional field is present.
    pub has_value: bool,
    /// The wrapped integer value.
    pub value: u32,
}

impl HostOptionalU32 {
    /// Return one absent optional `u32`.
    pub(crate) const fn none() -> Self {
        Self {
            has_value: false,
            value: 0,
        }
    }
}

impl NativeAbiCodec for HostOptionalU32 {
    type Value = Option<u32>;

    unsafe fn into_value(self) -> RuntimeResult<Self::Value> {
        if self.has_value {
            Ok(Some(self.value))
        } else {
            Ok(None)
        }
    }

    fn from_value(_binding: &BindingCallContext, value: Self::Value) -> Self {
        match value {
            Some(value) => Self {
                has_value: true,
                value,
            },
            None => Self::none(),
        }
    }
}

/// One optional host `u64`.
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub(crate) struct HostOptionalU64 {
    /// Whether the optional field is present.
    pub has_value: bool,
    /// The wrapped integer value.
    pub value: u64,
}

impl HostOptionalU64 {
    /// Return one absent optional `u64`.
    pub(crate) const fn none() -> Self {
        Self {
            has_value: false,
            value: 0,
        }
    }
}

impl NativeAbiCodec for HostOptionalU64 {
    type Value = Option<u64>;

    unsafe fn into_value(self) -> RuntimeResult<Self::Value> {
        if self.has_value {
            Ok(Some(self.value))
        } else {
            Ok(None)
        }
    }

    fn from_value(_binding: &BindingCallContext, value: Self::Value) -> Self {
        match value {
            Some(value) => Self {
                has_value: true,
                value,
            },
            None => Self::none(),
        }
    }
}

/// One optional host `i8`.
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub(crate) struct HostOptionalI8 {
    /// Whether the optional field is present.
    pub has_value: bool,
    /// The wrapped integer value.
    pub value: i8,
}

impl HostOptionalI8 {
    /// Return one absent optional `i8`.
    pub(crate) const fn none() -> Self {
        Self {
            has_value: false,
            value: 0,
        }
    }
}

impl NativeAbiCodec for HostOptionalI8 {
    type Value = Option<i8>;

    unsafe fn into_value(self) -> RuntimeResult<Self::Value> {
        if self.has_value {
            Ok(Some(self.value))
        } else {
            Ok(None)
        }
    }

    fn from_value(_binding: &BindingCallContext, value: Self::Value) -> Self {
        match value {
            Some(value) => Self {
                has_value: true,
                value,
            },
            None => Self::none(),
        }
    }
}

/// One host callback ABI status code.
#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u32)]
pub(crate) enum HostStatus {
    /// The host callback succeeded.
    Ok = 0,
    /// The host callback is not supported.
    NotSupported = 1,
    /// The host callback rejected one invalid argument.
    InvalidArgument = 2,
    /// The host callback could not resolve one object.
    NotFound = 3,
    /// The host callback was denied permission.
    PermissionDenied = 4,
    /// The host callback output buffer was too small.
    BufferTooSmall = 5,
    /// The host callback failed generically.
    Failed = 6,
    /// The host callback would block in nonblocking mode.
    WouldBlock = 7,
}

#[allow(dead_code)]
impl HostStatus {
    /// Return the raw ABI status code.
    pub(crate) const fn code(self) -> u32 {
        self as u32
    }

    /// Decode one raw ABI status code when it is known.
    pub(crate) const fn from_code(code: u32) -> Option<Self> {
        match code {
            0 => Some(Self::Ok),
            1 => Some(Self::NotSupported),
            2 => Some(Self::InvalidArgument),
            3 => Some(Self::NotFound),
            4 => Some(Self::PermissionDenied),
            5 => Some(Self::BufferTooSmall),
            6 => Some(Self::Failed),
            7 => Some(Self::WouldBlock),
            _ => None,
        }
    }
}
