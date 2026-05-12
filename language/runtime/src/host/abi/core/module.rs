use std::mem::MaybeUninit;

use crate::diagnostic::RuntimeResult;
use crate::platform::NativeAbiCodec;
use crate::runtime::BindingCallContext;

/// Process-global host-session handle passed through the host ABI.
#[cfg_attr(not(any(target_os = "android", target_os = "ios")), allow(dead_code))]
pub(crate) type HostSessionHandle = u64;

/// One optional host ABI payload.
#[derive(Copy, Debug)]
#[repr(C)]
pub struct HostAbiOptional<T: Copy> {
    /// Whether the optional field is present.
    pub has_value: bool,
    /// The wrapped value when present.
    pub value: MaybeUninit<T>,
}

impl<T: Copy> Clone for HostAbiOptional<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: Copy> HostAbiOptional<T> {
    /// Return one absent optional payload.
    pub(crate) fn none() -> Self {
        Self {
            has_value: false,
            value: MaybeUninit::uninit(),
        }
    }
}

impl<T> NativeAbiCodec for HostAbiOptional<T>
where
    T: NativeAbiCodec + Copy,
{
    type Value = Option<T::Value>;

    unsafe fn into_value(self) -> RuntimeResult<Self::Value> {
        if !self.has_value {
            return Ok(None);
        }

        Ok(Some(unsafe { T::into_value(self.value.assume_init())? }))
    }

    fn from_value(binding: &BindingCallContext, value: Option<T::Value>) -> Self {
        match value {
            Some(value) => Self {
                has_value: true,
                value: MaybeUninit::new(T::from_value(binding, value)),
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
