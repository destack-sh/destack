// generated bridge target, do not edit

use std::ffi::c_char;
use std::{ptr, slice};

use crate::core::{
    DestackError, DestackStatus, c_string, destroy_array, destroy_string, owned_array, read_bytes,
    read_string, return_status, write_out,
};

/// C ABI owned byte array.
#[repr(C)]
#[derive(Debug)]
pub struct DestackByteArray {
    /// Owned byte pointer.
    pub(crate) ptr: *mut u8,
    /// Byte count.
    pub(crate) len: usize,
}

/// C ABI unsigned 128 bit value.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct DestackU128 {
    /// Most significant 64 bits.
    pub(crate) high: u64,
    /// Least significant 64 bits.
    pub(crate) low: u64,
}

/// C ABI owned string array.
#[repr(C)]
#[derive(Debug)]
pub struct DestackStringArray {
    /// Owned string pointer.
    pub(crate) ptr: *mut *mut c_char,
    /// String count.
    pub(crate) len: usize,
}

/// C ABI optional owned string.
#[repr(C)]
#[derive(Debug)]
pub struct DestackOptionalString {
    /// Whether the value is present.
    pub(crate) is_some: bool,
    /// Owned string value when present.
    pub(crate) value: *mut c_char,
}

impl DestackByteArray {
    /// Convert Rust bytes into one C ABI byte array.
    pub(crate) fn from_vec(values: Vec<u8>) -> Self {
        let (ptr, len) = owned_array(values);
        Self { ptr, len }
    }

    /// Convert this C ABI byte array into Rust bytes.
    pub(crate) fn into_vec(self) -> Result<Vec<u8>, String> {
        read_bytes(self.ptr.cast_const(), self.len)
    }

    /// Destroy this C ABI byte array.
    pub(crate) fn destroy(&mut self) {
        if self.ptr.is_null() {
            return;
        }
        unsafe {
            destroy_array(self.ptr, self.len, |_| {});
        }
        self.ptr = ptr::null_mut();
        self.len = 0;
    }
}

impl DestackU128 {
    /// Convert one Rust `u128` into one C ABI value.
    pub(crate) fn from_bridge(value: u128) -> Self {
        Self {
            high: (value >> 64) as u64,
            low: value as u64,
        }
    }

    /// Convert this C ABI value into one Rust `u128`.
    pub(crate) fn to_bridge(self) -> u128 {
        ((self.high as u128) << 64) | self.low as u128
    }
}

impl DestackStringArray {
    /// Convert Rust strings into one C ABI string array.
    pub(crate) fn from_bridge(values: Vec<String>) -> Result<Self, String> {
        let mut converted = Vec::with_capacity(values.len());
        for value in values {
            converted.push(c_string(value)?);
        }
        let (ptr, len) = owned_array(converted);
        Ok(Self { ptr, len })
    }

    /// Convert this C ABI string array into Rust strings.
    pub(crate) fn to_bridge(&self) -> Result<Vec<String>, String> {
        if self.len == 0 {
            return Ok(Vec::new());
        }
        if self.ptr.is_null() {
            return Err("string array pointer is null".to_string());
        }
        let values = unsafe { slice::from_raw_parts(self.ptr, self.len) };
        let mut converted = Vec::with_capacity(values.len());
        for value in values {
            converted.push(read_string(*value)?);
        }
        Ok(converted)
    }

    /// Destroy this C ABI string array.
    pub(crate) fn destroy(&mut self) {
        if self.ptr.is_null() {
            return;
        }
        unsafe {
            destroy_array(self.ptr, self.len, |value| {
                destroy_string(*value);
                *value = ptr::null_mut();
            });
        }
        self.ptr = ptr::null_mut();
        self.len = 0;
    }
}

impl DestackOptionalString {
    /// Convert one optional string into one C ABI optional string.
    pub(crate) fn from_bridge(value: Option<String>) -> Result<Self, String> {
        let Some(value) = value else {
            return Ok(Self::empty());
        };
        Ok(Self {
            is_some: true,
            value: c_string(value)?,
        })
    }

    /// Convert this C ABI optional string into one optional string.
    pub(crate) fn to_bridge(&self) -> Result<Option<String>, String> {
        if self.is_some {
            Ok(Some(read_string(self.value)?))
        } else {
            Ok(None)
        }
    }

    /// Destroy this C ABI optional string.
    pub(crate) fn destroy(&mut self) {
        if self.is_some {
            destroy_string(self.value);
        }
        *self = Self::empty();
    }

    /// Return one empty C ABI optional string.
    pub(crate) fn empty() -> Self {
        Self {
            is_some: false,
            value: ptr::null_mut(),
        }
    }
}

/// C ABI artifact key handle.
#[repr(C)]
#[derive(Debug)]
pub struct DestackArtifactKey {
    /// Rust bridge value.
    pub(crate) value: destack::language::ArtifactKey,
}

/// C ABI bridge enum.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DestackApplicability {
    /// Machine-applicable suggestion.
    Automatic = 0,
    /// Machine-applicable suggestion that may change behavior.
    Unsafe = 1,
    /// Maybe incorrect suggestion.
    Dangerous = 2,
}

/// C ABI bridge enum array.
#[repr(C)]
#[derive(Debug)]
pub struct DestackApplicabilityArray {
    /// Owned value pointer.
    pub(crate) ptr: *mut DestackApplicability,
    /// Value count.
    pub(crate) len: usize,
}

/// C ABI optional bridge enum.
#[repr(C)]
#[derive(Debug)]
pub struct DestackOptionalApplicability {
    /// Whether the value is present.
    pub(crate) is_some: bool,
    /// Value when present.
    pub(crate) value: DestackApplicability,
}

impl DestackApplicability {
    /// Convert one bridge enum into one C ABI enum.
    pub(crate) fn from_bridge(value: destack::language::Applicability) -> Result<Self, String> {
        Ok(match value {
            destack::language::Applicability::Automatic => Self::Automatic,
            destack::language::Applicability::Unsafe => Self::Unsafe,
            destack::language::Applicability::Dangerous => Self::Dangerous,
        })
    }

    /// Convert this C ABI enum into one bridge enum.
    pub(crate) fn to_bridge(self) -> Result<destack::language::Applicability, String> {
        Ok(match self {
            Self::Automatic => destack::language::Applicability::Automatic,
            Self::Unsafe => destack::language::Applicability::Unsafe,
            Self::Dangerous => destack::language::Applicability::Dangerous,
        })
    }

    /// Destroy this C ABI enum.
    pub(crate) fn destroy(&mut self) {}
    /// Return one empty C ABI enum.
    pub(crate) fn empty() -> Self {
        Self::Automatic
    }
}

impl DestackApplicabilityArray {
    /// Convert bridge values into one C ABI array.
    pub(crate) fn from_bridge(
        values: Vec<destack::language::Applicability>,
    ) -> Result<Self, String> {
        let mut converted = Vec::with_capacity(values.len());
        for value in values {
            converted.push(DestackApplicability::from_bridge(value)?);
        }
        let (ptr, len) = owned_array(converted);
        Ok(Self { ptr, len })
    }

    /// Convert this C ABI array into bridge values.
    pub(crate) fn to_bridge(&self) -> Result<Vec<destack::language::Applicability>, String> {
        if self.len == 0 {
            return Ok(Vec::new());
        }
        if self.ptr.is_null() {
            return Err("array pointer is null".to_string());
        }
        let values = unsafe { slice::from_raw_parts(self.ptr, self.len) };
        let mut converted = Vec::with_capacity(values.len());
        for value in values {
            converted.push(value.to_bridge()?);
        }
        Ok(converted)
    }

    /// Destroy this C ABI array.
    pub(crate) fn destroy(&mut self) {
        if self.ptr.is_null() {
            return;
        }
        unsafe {
            destroy_array(self.ptr, self.len, |value| value.destroy());
        }
        self.ptr = ptr::null_mut();
        self.len = 0;
    }
}

impl DestackOptionalApplicability {
    /// Convert one optional bridge value into one C ABI optional value.
    pub(crate) fn from_bridge(
        value: Option<destack::language::Applicability>,
    ) -> Result<Self, String> {
        let Some(value) = value else {
            return Ok(Self {
                is_some: false,
                value: DestackApplicability::empty(),
            });
        };
        Ok(Self {
            is_some: true,
            value: DestackApplicability::from_bridge(value)?,
        })
    }

    /// Convert this C ABI optional value into one bridge optional value.
    pub(crate) fn to_bridge(&self) -> Result<Option<destack::language::Applicability>, String> {
        if self.is_some {
            Ok(Some(self.value.to_bridge()?))
        } else {
            Ok(None)
        }
    }

    /// Destroy this C ABI optional value.
    pub(crate) fn destroy(&mut self) {
        if self.is_some {
            self.value.destroy();
        }
        self.is_some = false;
        self.value = DestackApplicability::empty();
    }
}

/// C ABI bridge value.
#[repr(C)]
#[derive(Debug)]
pub struct DestackArtifactVersion {
    /// Semantic artifact slot.
    pub(crate) key: *mut DestackArtifactKey,
    /// Exact semantic fingerprint.
    pub(crate) fingerprint: *mut c_char,
}

/// C ABI bridge value array.
#[repr(C)]
#[derive(Debug)]
pub struct DestackArtifactVersionArray {
    /// Owned value pointer.
    pub(crate) ptr: *mut DestackArtifactVersion,
    /// Value count.
    pub(crate) len: usize,
}

/// C ABI optional bridge value.
#[repr(C)]
#[derive(Debug)]
pub struct DestackOptionalArtifactVersion {
    /// Whether the value is present.
    pub(crate) is_some: bool,
    /// Value when present.
    pub(crate) value: DestackArtifactVersion,
}

impl DestackArtifactVersion {
    /// Convert one bridge value into one C ABI value.
    pub(crate) fn from_bridge(value: destack::language::ArtifactVersion) -> Result<Self, String> {
        Ok(Self {
            key: Box::into_raw(Box::new(DestackArtifactKey { value: value.key })),
            fingerprint: c_string(value.fingerprint)?,
        })
    }

    /// Convert this C ABI value into one bridge value.
    pub(crate) fn to_bridge(&self) -> Result<destack::language::ArtifactVersion, String> {
        Ok(destack::language::ArtifactVersion {
            key: {
                let value = unsafe { self.key.as_ref() }.ok_or("artifact key is null")?;
                value.value.clone()
            },
            fingerprint: read_string(self.fingerprint)?,
        })
    }

    /// Destroy this C ABI value.
    pub(crate) fn destroy(&mut self) {
        if !self.key.is_null() {
            unsafe {
                destack_artifact_key_destroy(self.key);
            }
            self.key = ptr::null_mut();
        }
        destroy_string(self.fingerprint);
        self.fingerprint = ptr::null_mut();
    }

    /// Return one empty C ABI value.
    pub(crate) fn empty() -> Self {
        Self {
            key: ptr::null_mut(),
            fingerprint: ptr::null_mut(),
        }
    }
}

impl DestackArtifactVersionArray {
    /// Convert bridge values into one C ABI array.
    pub(crate) fn from_bridge(
        values: Vec<destack::language::ArtifactVersion>,
    ) -> Result<Self, String> {
        let mut converted = Vec::with_capacity(values.len());
        for value in values {
            converted.push(DestackArtifactVersion::from_bridge(value)?);
        }
        let (ptr, len) = owned_array(converted);
        Ok(Self { ptr, len })
    }

    /// Convert this C ABI array into bridge values.
    pub(crate) fn to_bridge(&self) -> Result<Vec<destack::language::ArtifactVersion>, String> {
        if self.len == 0 {
            return Ok(Vec::new());
        }
        if self.ptr.is_null() {
            return Err("array pointer is null".to_string());
        }
        let values = unsafe { slice::from_raw_parts(self.ptr, self.len) };
        let mut converted = Vec::with_capacity(values.len());
        for value in values {
            converted.push(value.to_bridge()?);
        }
        Ok(converted)
    }

    /// Destroy this C ABI array.
    pub(crate) fn destroy(&mut self) {
        if self.ptr.is_null() {
            return;
        }
        unsafe {
            destroy_array(self.ptr, self.len, |value| value.destroy());
        }
        self.ptr = ptr::null_mut();
        self.len = 0;
    }
}

impl DestackOptionalArtifactVersion {
    /// Convert one optional bridge value into one C ABI optional value.
    pub(crate) fn from_bridge(
        value: Option<destack::language::ArtifactVersion>,
    ) -> Result<Self, String> {
        let Some(value) = value else {
            return Ok(Self {
                is_some: false,
                value: DestackArtifactVersion::empty(),
            });
        };
        Ok(Self {
            is_some: true,
            value: DestackArtifactVersion::from_bridge(value)?,
        })
    }

    /// Convert this C ABI optional value into one bridge optional value.
    pub(crate) fn to_bridge(&self) -> Result<Option<destack::language::ArtifactVersion>, String> {
        if self.is_some {
            Ok(Some(self.value.to_bridge()?))
        } else {
            Ok(None)
        }
    }

    /// Destroy this C ABI optional value.
    pub(crate) fn destroy(&mut self) {
        if self.is_some {
            self.value.destroy();
        }
        self.is_some = false;
        self.value = DestackArtifactVersion::empty();
    }
}

/// C ABI bridge value.
#[repr(C)]
#[derive(Debug)]
pub struct DestackContentId {
    /// Canonical lowercase hex content id.
    pub(crate) id: *mut c_char,
}

/// C ABI bridge value array.
#[repr(C)]
#[derive(Debug)]
pub struct DestackContentIdArray {
    /// Owned value pointer.
    pub(crate) ptr: *mut DestackContentId,
    /// Value count.
    pub(crate) len: usize,
}

/// C ABI optional bridge value.
#[repr(C)]
#[derive(Debug)]
pub struct DestackOptionalContentId {
    /// Whether the value is present.
    pub(crate) is_some: bool,
    /// Value when present.
    pub(crate) value: DestackContentId,
}

impl DestackContentId {
    /// Convert one bridge value into one C ABI value.
    pub(crate) fn from_bridge(value: destack::language::ContentId) -> Result<Self, String> {
        Ok(Self {
            id: c_string(value.id)?,
        })
    }

    /// Convert this C ABI value into one bridge value.
    pub(crate) fn to_bridge(&self) -> Result<destack::language::ContentId, String> {
        Ok(destack::language::ContentId {
            id: read_string(self.id)?,
        })
    }

    /// Destroy this C ABI value.
    pub(crate) fn destroy(&mut self) {
        destroy_string(self.id);
        self.id = ptr::null_mut();
    }

    /// Return one empty C ABI value.
    pub(crate) fn empty() -> Self {
        Self {
            id: ptr::null_mut(),
        }
    }
}

impl DestackContentIdArray {
    /// Convert bridge values into one C ABI array.
    pub(crate) fn from_bridge(values: Vec<destack::language::ContentId>) -> Result<Self, String> {
        let mut converted = Vec::with_capacity(values.len());
        for value in values {
            converted.push(DestackContentId::from_bridge(value)?);
        }
        let (ptr, len) = owned_array(converted);
        Ok(Self { ptr, len })
    }

    /// Convert this C ABI array into bridge values.
    pub(crate) fn to_bridge(&self) -> Result<Vec<destack::language::ContentId>, String> {
        if self.len == 0 {
            return Ok(Vec::new());
        }
        if self.ptr.is_null() {
            return Err("array pointer is null".to_string());
        }
        let values = unsafe { slice::from_raw_parts(self.ptr, self.len) };
        let mut converted = Vec::with_capacity(values.len());
        for value in values {
            converted.push(value.to_bridge()?);
        }
        Ok(converted)
    }

    /// Destroy this C ABI array.
    pub(crate) fn destroy(&mut self) {
        if self.ptr.is_null() {
            return;
        }
        unsafe {
            destroy_array(self.ptr, self.len, |value| value.destroy());
        }
        self.ptr = ptr::null_mut();
        self.len = 0;
    }
}

impl DestackOptionalContentId {
    /// Convert one optional bridge value into one C ABI optional value.
    pub(crate) fn from_bridge(value: Option<destack::language::ContentId>) -> Result<Self, String> {
        let Some(value) = value else {
            return Ok(Self {
                is_some: false,
                value: DestackContentId::empty(),
            });
        };
        Ok(Self {
            is_some: true,
            value: DestackContentId::from_bridge(value)?,
        })
    }

    /// Convert this C ABI optional value into one bridge optional value.
    pub(crate) fn to_bridge(&self) -> Result<Option<destack::language::ContentId>, String> {
        if self.is_some {
            Ok(Some(self.value.to_bridge()?))
        } else {
            Ok(None)
        }
    }

    /// Destroy this C ABI optional value.
    pub(crate) fn destroy(&mut self) {
        if self.is_some {
            self.value.destroy();
        }
        self.is_some = false;
        self.value = DestackContentId::empty();
    }
}

/// C ABI bridge value.
#[repr(C)]
#[derive(Debug)]
pub struct DestackFileId {
    /// Canonical lowercase hex file id.
    pub(crate) id: *mut c_char,
}

/// C ABI bridge value array.
#[repr(C)]
#[derive(Debug)]
pub struct DestackFileIdArray {
    /// Owned value pointer.
    pub(crate) ptr: *mut DestackFileId,
    /// Value count.
    pub(crate) len: usize,
}

/// C ABI optional bridge value.
#[repr(C)]
#[derive(Debug)]
pub struct DestackOptionalFileId {
    /// Whether the value is present.
    pub(crate) is_some: bool,
    /// Value when present.
    pub(crate) value: DestackFileId,
}

impl DestackFileId {
    /// Convert one bridge value into one C ABI value.
    pub(crate) fn from_bridge(value: destack::language::FileId) -> Result<Self, String> {
        Ok(Self {
            id: c_string(value.id)?,
        })
    }

    /// Convert this C ABI value into one bridge value.
    pub(crate) fn to_bridge(&self) -> Result<destack::language::FileId, String> {
        Ok(destack::language::FileId {
            id: read_string(self.id)?,
        })
    }

    /// Destroy this C ABI value.
    pub(crate) fn destroy(&mut self) {
        destroy_string(self.id);
        self.id = ptr::null_mut();
    }

    /// Return one empty C ABI value.
    pub(crate) fn empty() -> Self {
        Self {
            id: ptr::null_mut(),
        }
    }
}

impl DestackFileIdArray {
    /// Convert bridge values into one C ABI array.
    pub(crate) fn from_bridge(values: Vec<destack::language::FileId>) -> Result<Self, String> {
        let mut converted = Vec::with_capacity(values.len());
        for value in values {
            converted.push(DestackFileId::from_bridge(value)?);
        }
        let (ptr, len) = owned_array(converted);
        Ok(Self { ptr, len })
    }

    /// Convert this C ABI array into bridge values.
    pub(crate) fn to_bridge(&self) -> Result<Vec<destack::language::FileId>, String> {
        if self.len == 0 {
            return Ok(Vec::new());
        }
        if self.ptr.is_null() {
            return Err("array pointer is null".to_string());
        }
        let values = unsafe { slice::from_raw_parts(self.ptr, self.len) };
        let mut converted = Vec::with_capacity(values.len());
        for value in values {
            converted.push(value.to_bridge()?);
        }
        Ok(converted)
    }

    /// Destroy this C ABI array.
    pub(crate) fn destroy(&mut self) {
        if self.ptr.is_null() {
            return;
        }
        unsafe {
            destroy_array(self.ptr, self.len, |value| value.destroy());
        }
        self.ptr = ptr::null_mut();
        self.len = 0;
    }
}

impl DestackOptionalFileId {
    /// Convert one optional bridge value into one C ABI optional value.
    pub(crate) fn from_bridge(value: Option<destack::language::FileId>) -> Result<Self, String> {
        let Some(value) = value else {
            return Ok(Self {
                is_some: false,
                value: DestackFileId::empty(),
            });
        };
        Ok(Self {
            is_some: true,
            value: DestackFileId::from_bridge(value)?,
        })
    }

    /// Convert this C ABI optional value into one bridge optional value.
    pub(crate) fn to_bridge(&self) -> Result<Option<destack::language::FileId>, String> {
        if self.is_some {
            Ok(Some(self.value.to_bridge()?))
        } else {
            Ok(None)
        }
    }

    /// Destroy this C ABI optional value.
    pub(crate) fn destroy(&mut self) {
        if self.is_some {
            self.value.destroy();
        }
        self.is_some = false;
        self.value = DestackFileId::empty();
    }
}

/// C ABI bridge enum kind.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DestackArtifactDependencyKind {
    /// Another exact artifact version.
    Artifact = 0,
    /// One exact primitive source observation.
    Source = 1,
}

/// C ABI bridge enum.
#[repr(C)]
#[derive(Debug)]
pub struct DestackArtifactDependency {
    /// Active enum variant.
    pub(crate) kind: DestackArtifactDependencyKind,
    pub(crate) version: DestackArtifactVersion,
    pub(crate) file: DestackFileId,
    pub(crate) content: DestackContentId,
}

/// C ABI bridge enum array.
#[repr(C)]
#[derive(Debug)]
pub struct DestackArtifactDependencyArray {
    /// Owned value pointer.
    pub(crate) ptr: *mut DestackArtifactDependency,
    /// Value count.
    pub(crate) len: usize,
}

/// C ABI optional bridge enum.
#[repr(C)]
#[derive(Debug)]
pub struct DestackOptionalArtifactDependency {
    /// Whether the value is present.
    pub(crate) is_some: bool,
    /// Value when present.
    pub(crate) value: DestackArtifactDependency,
}

impl DestackArtifactDependency {
    /// Convert one bridge enum into one C ABI enum.
    pub(crate) fn from_bridge(
        value: destack::language::ArtifactDependency,
    ) -> Result<Self, String> {
        Ok(match value {
            destack::language::ArtifactDependency::Artifact { version } => Self {
                kind: DestackArtifactDependencyKind::Artifact,
                version: DestackArtifactVersion::from_bridge(version)?,
                file: DestackFileId::empty(),
                content: DestackContentId::empty(),
            },
            destack::language::ArtifactDependency::Source { file, content } => Self {
                kind: DestackArtifactDependencyKind::Source,
                version: DestackArtifactVersion::empty(),
                file: DestackFileId::from_bridge(file)?,
                content: DestackContentId::from_bridge(content)?,
            },
        })
    }

    /// Convert this C ABI enum into one bridge enum.
    pub(crate) fn to_bridge(&self) -> Result<destack::language::ArtifactDependency, String> {
        match self.kind {
            DestackArtifactDependencyKind::Artifact => {
                let version = self.version.to_bridge()?;
                Ok(destack::language::ArtifactDependency::Artifact { version })
            }
            DestackArtifactDependencyKind::Source => {
                let file = self.file.to_bridge()?;
                let content = self.content.to_bridge()?;
                Ok(destack::language::ArtifactDependency::Source { file, content })
            }
        }
    }

    /// Destroy this C ABI enum.
    pub(crate) fn destroy(&mut self) {
        self.version.destroy();
        self.file.destroy();
        self.content.destroy();
    }

    /// Return one empty C ABI enum.
    pub(crate) fn empty() -> Self {
        Self {
            kind: DestackArtifactDependencyKind::Artifact,
            version: DestackArtifactVersion::empty(),
            file: DestackFileId::empty(),
            content: DestackContentId::empty(),
        }
    }
}

impl DestackArtifactDependencyArray {
    /// Convert bridge values into one C ABI array.
    pub(crate) fn from_bridge(
        values: Vec<destack::language::ArtifactDependency>,
    ) -> Result<Self, String> {
        let mut converted = Vec::with_capacity(values.len());
        for value in values {
            converted.push(DestackArtifactDependency::from_bridge(value)?);
        }
        let (ptr, len) = owned_array(converted);
        Ok(Self { ptr, len })
    }

    /// Convert this C ABI array into bridge values.
    pub(crate) fn to_bridge(&self) -> Result<Vec<destack::language::ArtifactDependency>, String> {
        if self.len == 0 {
            return Ok(Vec::new());
        }
        if self.ptr.is_null() {
            return Err("array pointer is null".to_string());
        }
        let values = unsafe { slice::from_raw_parts(self.ptr, self.len) };
        let mut converted = Vec::with_capacity(values.len());
        for value in values {
            converted.push(value.to_bridge()?);
        }
        Ok(converted)
    }

    /// Destroy this C ABI array.
    pub(crate) fn destroy(&mut self) {
        if self.ptr.is_null() {
            return;
        }
        unsafe {
            destroy_array(self.ptr, self.len, |value| value.destroy());
        }
        self.ptr = ptr::null_mut();
        self.len = 0;
    }
}

impl DestackOptionalArtifactDependency {
    /// Convert one optional bridge value into one C ABI optional value.
    pub(crate) fn from_bridge(
        value: Option<destack::language::ArtifactDependency>,
    ) -> Result<Self, String> {
        let Some(value) = value else {
            return Ok(Self {
                is_some: false,
                value: DestackArtifactDependency::empty(),
            });
        };
        Ok(Self {
            is_some: true,
            value: DestackArtifactDependency::from_bridge(value)?,
        })
    }

    /// Convert this C ABI optional value into one bridge optional value.
    pub(crate) fn to_bridge(
        &self,
    ) -> Result<Option<destack::language::ArtifactDependency>, String> {
        if self.is_some {
            Ok(Some(self.value.to_bridge()?))
        } else {
            Ok(None)
        }
    }

    /// Destroy this C ABI optional value.
    pub(crate) fn destroy(&mut self) {
        if self.is_some {
            self.value.destroy();
        }
        self.is_some = false;
        self.value = DestackArtifactDependency::empty();
    }
}

/// C ABI bridge value.
#[repr(C)]
#[derive(Debug)]
pub struct DestackArtifactSidecarLabel {
    /// Label key.
    pub(crate) key: *mut c_char,
    /// Label value.
    pub(crate) value: *mut c_char,
}

/// C ABI bridge value array.
#[repr(C)]
#[derive(Debug)]
pub struct DestackArtifactSidecarLabelArray {
    /// Owned value pointer.
    pub(crate) ptr: *mut DestackArtifactSidecarLabel,
    /// Value count.
    pub(crate) len: usize,
}

/// C ABI optional bridge value.
#[repr(C)]
#[derive(Debug)]
pub struct DestackOptionalArtifactSidecarLabel {
    /// Whether the value is present.
    pub(crate) is_some: bool,
    /// Value when present.
    pub(crate) value: DestackArtifactSidecarLabel,
}

impl DestackArtifactSidecarLabel {
    /// Convert one bridge value into one C ABI value.
    pub(crate) fn from_bridge(
        value: destack::language::ArtifactSidecarLabel,
    ) -> Result<Self, String> {
        Ok(Self {
            key: c_string(value.key)?,
            value: c_string(value.value)?,
        })
    }

    /// Convert this C ABI value into one bridge value.
    pub(crate) fn to_bridge(&self) -> Result<destack::language::ArtifactSidecarLabel, String> {
        Ok(destack::language::ArtifactSidecarLabel {
            key: read_string(self.key)?,
            value: read_string(self.value)?,
        })
    }

    /// Destroy this C ABI value.
    pub(crate) fn destroy(&mut self) {
        destroy_string(self.key);
        self.key = ptr::null_mut();
        destroy_string(self.value);
        self.value = ptr::null_mut();
    }

    /// Return one empty C ABI value.
    pub(crate) fn empty() -> Self {
        Self {
            key: ptr::null_mut(),
            value: ptr::null_mut(),
        }
    }
}

impl DestackArtifactSidecarLabelArray {
    /// Convert bridge values into one C ABI array.
    pub(crate) fn from_bridge(
        values: Vec<destack::language::ArtifactSidecarLabel>,
    ) -> Result<Self, String> {
        let mut converted = Vec::with_capacity(values.len());
        for value in values {
            converted.push(DestackArtifactSidecarLabel::from_bridge(value)?);
        }
        let (ptr, len) = owned_array(converted);
        Ok(Self { ptr, len })
    }

    /// Convert this C ABI array into bridge values.
    pub(crate) fn to_bridge(&self) -> Result<Vec<destack::language::ArtifactSidecarLabel>, String> {
        if self.len == 0 {
            return Ok(Vec::new());
        }
        if self.ptr.is_null() {
            return Err("array pointer is null".to_string());
        }
        let values = unsafe { slice::from_raw_parts(self.ptr, self.len) };
        let mut converted = Vec::with_capacity(values.len());
        for value in values {
            converted.push(value.to_bridge()?);
        }
        Ok(converted)
    }

    /// Destroy this C ABI array.
    pub(crate) fn destroy(&mut self) {
        if self.ptr.is_null() {
            return;
        }
        unsafe {
            destroy_array(self.ptr, self.len, |value| value.destroy());
        }
        self.ptr = ptr::null_mut();
        self.len = 0;
    }
}

impl DestackOptionalArtifactSidecarLabel {
    /// Convert one optional bridge value into one C ABI optional value.
    pub(crate) fn from_bridge(
        value: Option<destack::language::ArtifactSidecarLabel>,
    ) -> Result<Self, String> {
        let Some(value) = value else {
            return Ok(Self {
                is_some: false,
                value: DestackArtifactSidecarLabel::empty(),
            });
        };
        Ok(Self {
            is_some: true,
            value: DestackArtifactSidecarLabel::from_bridge(value)?,
        })
    }

    /// Convert this C ABI optional value into one bridge optional value.
    pub(crate) fn to_bridge(
        &self,
    ) -> Result<Option<destack::language::ArtifactSidecarLabel>, String> {
        if self.is_some {
            Ok(Some(self.value.to_bridge()?))
        } else {
            Ok(None)
        }
    }

    /// Destroy this C ABI optional value.
    pub(crate) fn destroy(&mut self) {
        if self.is_some {
            self.value.destroy();
        }
        self.is_some = false;
        self.value = DestackArtifactSidecarLabel::empty();
    }
}

/// C ABI bridge enum kind.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DestackContentKind {
    /// Text content.
    Text = 0,
    /// Binary content.
    Binary = 1,
}

/// C ABI bridge enum.
#[repr(C)]
#[derive(Debug)]
pub struct DestackContent {
    /// Active enum variant.
    pub(crate) kind: DestackContentKind,
    pub(crate) text_content: *mut c_char,
    pub(crate) binary_content: DestackByteArray,
}

/// C ABI bridge enum array.
#[repr(C)]
#[derive(Debug)]
pub struct DestackContentArray {
    /// Owned value pointer.
    pub(crate) ptr: *mut DestackContent,
    /// Value count.
    pub(crate) len: usize,
}

/// C ABI optional bridge enum.
#[repr(C)]
#[derive(Debug)]
pub struct DestackOptionalContent {
    /// Whether the value is present.
    pub(crate) is_some: bool,
    /// Value when present.
    pub(crate) value: DestackContent,
}

impl DestackContent {
    /// Convert one bridge enum into one C ABI enum.
    pub(crate) fn from_bridge(value: destack::language::Content) -> Result<Self, String> {
        Ok(match value {
            destack::language::Content::Text { content } => Self {
                kind: DestackContentKind::Text,
                text_content: c_string(content)?,
                binary_content: DestackByteArray {
                    ptr: ptr::null_mut(),
                    len: 0,
                },
            },
            destack::language::Content::Binary { content } => Self {
                kind: DestackContentKind::Binary,
                text_content: ptr::null_mut(),
                binary_content: DestackByteArray::from_vec(content),
            },
        })
    }

    /// Convert this C ABI enum into one bridge enum.
    pub(crate) fn to_bridge(&self) -> Result<destack::language::Content, String> {
        match self.kind {
            DestackContentKind::Text => {
                let text_content = read_string(self.text_content)?;
                Ok(destack::language::Content::Text {
                    content: text_content,
                })
            }
            DestackContentKind::Binary => {
                let binary_content = read_bytes(
                    self.binary_content.ptr.cast_const(),
                    self.binary_content.len,
                )?;
                Ok(destack::language::Content::Binary {
                    content: binary_content,
                })
            }
        }
    }

    /// Destroy this C ABI enum.
    pub(crate) fn destroy(&mut self) {
        destroy_string(self.text_content);
        self.text_content = ptr::null_mut();
        self.binary_content.destroy();
    }

    /// Return one empty C ABI enum.
    pub(crate) fn empty() -> Self {
        Self {
            kind: DestackContentKind::Text,
            text_content: ptr::null_mut(),
            binary_content: DestackByteArray {
                ptr: ptr::null_mut(),
                len: 0,
            },
        }
    }
}

impl DestackContentArray {
    /// Convert bridge values into one C ABI array.
    pub(crate) fn from_bridge(values: Vec<destack::language::Content>) -> Result<Self, String> {
        let mut converted = Vec::with_capacity(values.len());
        for value in values {
            converted.push(DestackContent::from_bridge(value)?);
        }
        let (ptr, len) = owned_array(converted);
        Ok(Self { ptr, len })
    }

    /// Convert this C ABI array into bridge values.
    pub(crate) fn to_bridge(&self) -> Result<Vec<destack::language::Content>, String> {
        if self.len == 0 {
            return Ok(Vec::new());
        }
        if self.ptr.is_null() {
            return Err("array pointer is null".to_string());
        }
        let values = unsafe { slice::from_raw_parts(self.ptr, self.len) };
        let mut converted = Vec::with_capacity(values.len());
        for value in values {
            converted.push(value.to_bridge()?);
        }
        Ok(converted)
    }

    /// Destroy this C ABI array.
    pub(crate) fn destroy(&mut self) {
        if self.ptr.is_null() {
            return;
        }
        unsafe {
            destroy_array(self.ptr, self.len, |value| value.destroy());
        }
        self.ptr = ptr::null_mut();
        self.len = 0;
    }
}

impl DestackOptionalContent {
    /// Convert one optional bridge value into one C ABI optional value.
    pub(crate) fn from_bridge(value: Option<destack::language::Content>) -> Result<Self, String> {
        let Some(value) = value else {
            return Ok(Self {
                is_some: false,
                value: DestackContent::empty(),
            });
        };
        Ok(Self {
            is_some: true,
            value: DestackContent::from_bridge(value)?,
        })
    }

    /// Convert this C ABI optional value into one bridge optional value.
    pub(crate) fn to_bridge(&self) -> Result<Option<destack::language::Content>, String> {
        if self.is_some {
            Ok(Some(self.value.to_bridge()?))
        } else {
            Ok(None)
        }
    }

    /// Destroy this C ABI optional value.
    pub(crate) fn destroy(&mut self) {
        if self.is_some {
            self.value.destroy();
        }
        self.is_some = false;
        self.value = DestackContent::empty();
    }
}

/// C ABI bridge value.
#[repr(C)]
#[derive(Debug)]
pub struct DestackArtifactSidecar {
    /// Sidecar name.
    pub(crate) name: *mut c_char,
    /// Stable labels describing this sidecar.
    pub(crate) labels: DestackArtifactSidecarLabelArray,
    /// Sidecar content.
    pub(crate) content: DestackContent,
}

/// C ABI bridge value array.
#[repr(C)]
#[derive(Debug)]
pub struct DestackArtifactSidecarArray {
    /// Owned value pointer.
    pub(crate) ptr: *mut DestackArtifactSidecar,
    /// Value count.
    pub(crate) len: usize,
}

/// C ABI optional bridge value.
#[repr(C)]
#[derive(Debug)]
pub struct DestackOptionalArtifactSidecar {
    /// Whether the value is present.
    pub(crate) is_some: bool,
    /// Value when present.
    pub(crate) value: DestackArtifactSidecar,
}

impl DestackArtifactSidecar {
    /// Convert one bridge value into one C ABI value.
    pub(crate) fn from_bridge(value: destack::language::ArtifactSidecar) -> Result<Self, String> {
        Ok(Self {
            name: c_string(value.name)?,
            labels: DestackArtifactSidecarLabelArray::from_bridge(value.labels)?,
            content: DestackContent::from_bridge(value.content)?,
        })
    }

    /// Convert this C ABI value into one bridge value.
    pub(crate) fn to_bridge(&self) -> Result<destack::language::ArtifactSidecar, String> {
        Ok(destack::language::ArtifactSidecar {
            name: read_string(self.name)?,
            labels: self.labels.to_bridge()?,
            content: self.content.to_bridge()?,
        })
    }

    /// Destroy this C ABI value.
    pub(crate) fn destroy(&mut self) {
        destroy_string(self.name);
        self.name = ptr::null_mut();
        self.labels.destroy();
        self.content.destroy();
    }

    /// Return one empty C ABI value.
    pub(crate) fn empty() -> Self {
        Self {
            name: ptr::null_mut(),
            labels: DestackArtifactSidecarLabelArray {
                ptr: ptr::null_mut(),
                len: 0,
            },
            content: DestackContent::empty(),
        }
    }
}

impl DestackArtifactSidecarArray {
    /// Convert bridge values into one C ABI array.
    pub(crate) fn from_bridge(
        values: Vec<destack::language::ArtifactSidecar>,
    ) -> Result<Self, String> {
        let mut converted = Vec::with_capacity(values.len());
        for value in values {
            converted.push(DestackArtifactSidecar::from_bridge(value)?);
        }
        let (ptr, len) = owned_array(converted);
        Ok(Self { ptr, len })
    }

    /// Convert this C ABI array into bridge values.
    pub(crate) fn to_bridge(&self) -> Result<Vec<destack::language::ArtifactSidecar>, String> {
        if self.len == 0 {
            return Ok(Vec::new());
        }
        if self.ptr.is_null() {
            return Err("array pointer is null".to_string());
        }
        let values = unsafe { slice::from_raw_parts(self.ptr, self.len) };
        let mut converted = Vec::with_capacity(values.len());
        for value in values {
            converted.push(value.to_bridge()?);
        }
        Ok(converted)
    }

    /// Destroy this C ABI array.
    pub(crate) fn destroy(&mut self) {
        if self.ptr.is_null() {
            return;
        }
        unsafe {
            destroy_array(self.ptr, self.len, |value| value.destroy());
        }
        self.ptr = ptr::null_mut();
        self.len = 0;
    }
}

impl DestackOptionalArtifactSidecar {
    /// Convert one optional bridge value into one C ABI optional value.
    pub(crate) fn from_bridge(
        value: Option<destack::language::ArtifactSidecar>,
    ) -> Result<Self, String> {
        let Some(value) = value else {
            return Ok(Self {
                is_some: false,
                value: DestackArtifactSidecar::empty(),
            });
        };
        Ok(Self {
            is_some: true,
            value: DestackArtifactSidecar::from_bridge(value)?,
        })
    }

    /// Convert this C ABI optional value into one bridge optional value.
    pub(crate) fn to_bridge(&self) -> Result<Option<destack::language::ArtifactSidecar>, String> {
        if self.is_some {
            Ok(Some(self.value.to_bridge()?))
        } else {
            Ok(None)
        }
    }

    /// Destroy this C ABI optional value.
    pub(crate) fn destroy(&mut self) {
        if self.is_some {
            self.value.destroy();
        }
        self.is_some = false;
        self.value = DestackArtifactSidecar::empty();
    }
}

/// C ABI bridge value.
#[repr(C)]
#[derive(Debug)]
pub struct DestackArtifactString {
    /// Canonical lowercase hex string id.
    pub(crate) id: *mut c_char,
    /// Interned string text.
    pub(crate) text: *mut c_char,
}

/// C ABI bridge value array.
#[repr(C)]
#[derive(Debug)]
pub struct DestackArtifactStringArray {
    /// Owned value pointer.
    pub(crate) ptr: *mut DestackArtifactString,
    /// Value count.
    pub(crate) len: usize,
}

/// C ABI optional bridge value.
#[repr(C)]
#[derive(Debug)]
pub struct DestackOptionalArtifactString {
    /// Whether the value is present.
    pub(crate) is_some: bool,
    /// Value when present.
    pub(crate) value: DestackArtifactString,
}

impl DestackArtifactString {
    /// Convert one bridge value into one C ABI value.
    pub(crate) fn from_bridge(value: destack::language::ArtifactString) -> Result<Self, String> {
        Ok(Self {
            id: c_string(value.id)?,
            text: c_string(value.text)?,
        })
    }

    /// Convert this C ABI value into one bridge value.
    pub(crate) fn to_bridge(&self) -> Result<destack::language::ArtifactString, String> {
        Ok(destack::language::ArtifactString {
            id: read_string(self.id)?,
            text: read_string(self.text)?,
        })
    }

    /// Destroy this C ABI value.
    pub(crate) fn destroy(&mut self) {
        destroy_string(self.id);
        self.id = ptr::null_mut();
        destroy_string(self.text);
        self.text = ptr::null_mut();
    }

    /// Return one empty C ABI value.
    pub(crate) fn empty() -> Self {
        Self {
            id: ptr::null_mut(),
            text: ptr::null_mut(),
        }
    }
}

impl DestackArtifactStringArray {
    /// Convert bridge values into one C ABI array.
    pub(crate) fn from_bridge(
        values: Vec<destack::language::ArtifactString>,
    ) -> Result<Self, String> {
        let mut converted = Vec::with_capacity(values.len());
        for value in values {
            converted.push(DestackArtifactString::from_bridge(value)?);
        }
        let (ptr, len) = owned_array(converted);
        Ok(Self { ptr, len })
    }

    /// Convert this C ABI array into bridge values.
    pub(crate) fn to_bridge(&self) -> Result<Vec<destack::language::ArtifactString>, String> {
        if self.len == 0 {
            return Ok(Vec::new());
        }
        if self.ptr.is_null() {
            return Err("array pointer is null".to_string());
        }
        let values = unsafe { slice::from_raw_parts(self.ptr, self.len) };
        let mut converted = Vec::with_capacity(values.len());
        for value in values {
            converted.push(value.to_bridge()?);
        }
        Ok(converted)
    }

    /// Destroy this C ABI array.
    pub(crate) fn destroy(&mut self) {
        if self.ptr.is_null() {
            return;
        }
        unsafe {
            destroy_array(self.ptr, self.len, |value| value.destroy());
        }
        self.ptr = ptr::null_mut();
        self.len = 0;
    }
}

impl DestackOptionalArtifactString {
    /// Convert one optional bridge value into one C ABI optional value.
    pub(crate) fn from_bridge(
        value: Option<destack::language::ArtifactString>,
    ) -> Result<Self, String> {
        let Some(value) = value else {
            return Ok(Self {
                is_some: false,
                value: DestackArtifactString::empty(),
            });
        };
        Ok(Self {
            is_some: true,
            value: DestackArtifactString::from_bridge(value)?,
        })
    }

    /// Convert this C ABI optional value into one bridge optional value.
    pub(crate) fn to_bridge(&self) -> Result<Option<destack::language::ArtifactString>, String> {
        if self.is_some {
            Ok(Some(self.value.to_bridge()?))
        } else {
            Ok(None)
        }
    }

    /// Destroy this C ABI optional value.
    pub(crate) fn destroy(&mut self) {
        if self.is_some {
            self.value.destroy();
        }
        self.is_some = false;
        self.value = DestackArtifactString::empty();
    }
}

/// C ABI bridge value.
#[repr(C)]
#[derive(Debug)]
pub struct DestackDiagnosticHelp {
    /// Help message.
    pub(crate) message: *mut c_char,
}

/// C ABI bridge value array.
#[repr(C)]
#[derive(Debug)]
pub struct DestackDiagnosticHelpArray {
    /// Owned value pointer.
    pub(crate) ptr: *mut DestackDiagnosticHelp,
    /// Value count.
    pub(crate) len: usize,
}

/// C ABI optional bridge value.
#[repr(C)]
#[derive(Debug)]
pub struct DestackOptionalDiagnosticHelp {
    /// Whether the value is present.
    pub(crate) is_some: bool,
    /// Value when present.
    pub(crate) value: DestackDiagnosticHelp,
}

impl DestackDiagnosticHelp {
    /// Convert one bridge value into one C ABI value.
    pub(crate) fn from_bridge(value: destack::language::DiagnosticHelp) -> Result<Self, String> {
        Ok(Self {
            message: c_string(value.message)?,
        })
    }

    /// Convert this C ABI value into one bridge value.
    pub(crate) fn to_bridge(&self) -> Result<destack::language::DiagnosticHelp, String> {
        Ok(destack::language::DiagnosticHelp {
            message: read_string(self.message)?,
        })
    }

    /// Destroy this C ABI value.
    pub(crate) fn destroy(&mut self) {
        destroy_string(self.message);
        self.message = ptr::null_mut();
    }

    /// Return one empty C ABI value.
    pub(crate) fn empty() -> Self {
        Self {
            message: ptr::null_mut(),
        }
    }
}

impl DestackDiagnosticHelpArray {
    /// Convert bridge values into one C ABI array.
    pub(crate) fn from_bridge(
        values: Vec<destack::language::DiagnosticHelp>,
    ) -> Result<Self, String> {
        let mut converted = Vec::with_capacity(values.len());
        for value in values {
            converted.push(DestackDiagnosticHelp::from_bridge(value)?);
        }
        let (ptr, len) = owned_array(converted);
        Ok(Self { ptr, len })
    }

    /// Convert this C ABI array into bridge values.
    pub(crate) fn to_bridge(&self) -> Result<Vec<destack::language::DiagnosticHelp>, String> {
        if self.len == 0 {
            return Ok(Vec::new());
        }
        if self.ptr.is_null() {
            return Err("array pointer is null".to_string());
        }
        let values = unsafe { slice::from_raw_parts(self.ptr, self.len) };
        let mut converted = Vec::with_capacity(values.len());
        for value in values {
            converted.push(value.to_bridge()?);
        }
        Ok(converted)
    }

    /// Destroy this C ABI array.
    pub(crate) fn destroy(&mut self) {
        if self.ptr.is_null() {
            return;
        }
        unsafe {
            destroy_array(self.ptr, self.len, |value| value.destroy());
        }
        self.ptr = ptr::null_mut();
        self.len = 0;
    }
}

impl DestackOptionalDiagnosticHelp {
    /// Convert one optional bridge value into one C ABI optional value.
    pub(crate) fn from_bridge(
        value: Option<destack::language::DiagnosticHelp>,
    ) -> Result<Self, String> {
        let Some(value) = value else {
            return Ok(Self {
                is_some: false,
                value: DestackDiagnosticHelp::empty(),
            });
        };
        Ok(Self {
            is_some: true,
            value: DestackDiagnosticHelp::from_bridge(value)?,
        })
    }

    /// Convert this C ABI optional value into one bridge optional value.
    pub(crate) fn to_bridge(&self) -> Result<Option<destack::language::DiagnosticHelp>, String> {
        if self.is_some {
            Ok(Some(self.value.to_bridge()?))
        } else {
            Ok(None)
        }
    }

    /// Destroy this C ABI optional value.
    pub(crate) fn destroy(&mut self) {
        if self.is_some {
            self.value.destroy();
        }
        self.is_some = false;
        self.value = DestackDiagnosticHelp::empty();
    }
}

/// C ABI bridge value.
#[repr(C)]
#[derive(Debug)]
pub struct DestackSpan {
    /// File containing this span.
    pub(crate) file: DestackFileId,
    /// Inclusive start byte offset.
    pub(crate) start: u32,
    /// Exclusive end byte offset.
    pub(crate) end: u32,
}

/// C ABI bridge value array.
#[repr(C)]
#[derive(Debug)]
pub struct DestackSpanArray {
    /// Owned value pointer.
    pub(crate) ptr: *mut DestackSpan,
    /// Value count.
    pub(crate) len: usize,
}

/// C ABI optional bridge value.
#[repr(C)]
#[derive(Debug)]
pub struct DestackOptionalSpan {
    /// Whether the value is present.
    pub(crate) is_some: bool,
    /// Value when present.
    pub(crate) value: DestackSpan,
}

impl DestackSpan {
    /// Convert one bridge value into one C ABI value.
    pub(crate) fn from_bridge(value: destack::language::Span) -> Result<Self, String> {
        Ok(Self {
            file: DestackFileId::from_bridge(value.file)?,
            start: value.start,
            end: value.end,
        })
    }

    /// Convert this C ABI value into one bridge value.
    pub(crate) fn to_bridge(&self) -> Result<destack::language::Span, String> {
        Ok(destack::language::Span {
            file: self.file.to_bridge()?,
            start: self.start,
            end: self.end,
        })
    }

    /// Destroy this C ABI value.
    pub(crate) fn destroy(&mut self) {
        self.file.destroy();
    }

    /// Return one empty C ABI value.
    pub(crate) fn empty() -> Self {
        Self {
            file: DestackFileId::empty(),
            start: 0,
            end: 0,
        }
    }
}

impl DestackSpanArray {
    /// Convert bridge values into one C ABI array.
    pub(crate) fn from_bridge(values: Vec<destack::language::Span>) -> Result<Self, String> {
        let mut converted = Vec::with_capacity(values.len());
        for value in values {
            converted.push(DestackSpan::from_bridge(value)?);
        }
        let (ptr, len) = owned_array(converted);
        Ok(Self { ptr, len })
    }

    /// Convert this C ABI array into bridge values.
    pub(crate) fn to_bridge(&self) -> Result<Vec<destack::language::Span>, String> {
        if self.len == 0 {
            return Ok(Vec::new());
        }
        if self.ptr.is_null() {
            return Err("array pointer is null".to_string());
        }
        let values = unsafe { slice::from_raw_parts(self.ptr, self.len) };
        let mut converted = Vec::with_capacity(values.len());
        for value in values {
            converted.push(value.to_bridge()?);
        }
        Ok(converted)
    }

    /// Destroy this C ABI array.
    pub(crate) fn destroy(&mut self) {
        if self.ptr.is_null() {
            return;
        }
        unsafe {
            destroy_array(self.ptr, self.len, |value| value.destroy());
        }
        self.ptr = ptr::null_mut();
        self.len = 0;
    }
}

impl DestackOptionalSpan {
    /// Convert one optional bridge value into one C ABI optional value.
    pub(crate) fn from_bridge(value: Option<destack::language::Span>) -> Result<Self, String> {
        let Some(value) = value else {
            return Ok(Self {
                is_some: false,
                value: DestackSpan::empty(),
            });
        };
        Ok(Self {
            is_some: true,
            value: DestackSpan::from_bridge(value)?,
        })
    }

    /// Convert this C ABI optional value into one bridge optional value.
    pub(crate) fn to_bridge(&self) -> Result<Option<destack::language::Span>, String> {
        if self.is_some {
            Ok(Some(self.value.to_bridge()?))
        } else {
            Ok(None)
        }
    }

    /// Destroy this C ABI optional value.
    pub(crate) fn destroy(&mut self) {
        if self.is_some {
            self.value.destroy();
        }
        self.is_some = false;
        self.value = DestackSpan::empty();
    }
}

/// C ABI bridge value.
#[repr(C)]
#[derive(Debug)]
pub struct DestackDiagnosticLabel {
    /// Exact content containing the span.
    pub(crate) content: DestackContentId,
    /// Concrete source span.
    pub(crate) span: DestackSpan,
    /// Optional label shown on the span.
    pub(crate) message: DestackOptionalString,
}

/// C ABI bridge value array.
#[repr(C)]
#[derive(Debug)]
pub struct DestackDiagnosticLabelArray {
    /// Owned value pointer.
    pub(crate) ptr: *mut DestackDiagnosticLabel,
    /// Value count.
    pub(crate) len: usize,
}

/// C ABI optional bridge value.
#[repr(C)]
#[derive(Debug)]
pub struct DestackOptionalDiagnosticLabel {
    /// Whether the value is present.
    pub(crate) is_some: bool,
    /// Value when present.
    pub(crate) value: DestackDiagnosticLabel,
}

impl DestackDiagnosticLabel {
    /// Convert one bridge value into one C ABI value.
    pub(crate) fn from_bridge(value: destack::language::DiagnosticLabel) -> Result<Self, String> {
        Ok(Self {
            content: DestackContentId::from_bridge(value.content)?,
            span: DestackSpan::from_bridge(value.span)?,
            message: DestackOptionalString::from_bridge(value.message)?,
        })
    }

    /// Convert this C ABI value into one bridge value.
    pub(crate) fn to_bridge(&self) -> Result<destack::language::DiagnosticLabel, String> {
        Ok(destack::language::DiagnosticLabel {
            content: self.content.to_bridge()?,
            span: self.span.to_bridge()?,
            message: self.message.to_bridge()?,
        })
    }

    /// Destroy this C ABI value.
    pub(crate) fn destroy(&mut self) {
        self.content.destroy();
        self.span.destroy();
        self.message.destroy();
    }

    /// Return one empty C ABI value.
    pub(crate) fn empty() -> Self {
        Self {
            content: DestackContentId::empty(),
            span: DestackSpan::empty(),
            message: DestackOptionalString::empty(),
        }
    }
}

impl DestackDiagnosticLabelArray {
    /// Convert bridge values into one C ABI array.
    pub(crate) fn from_bridge(
        values: Vec<destack::language::DiagnosticLabel>,
    ) -> Result<Self, String> {
        let mut converted = Vec::with_capacity(values.len());
        for value in values {
            converted.push(DestackDiagnosticLabel::from_bridge(value)?);
        }
        let (ptr, len) = owned_array(converted);
        Ok(Self { ptr, len })
    }

    /// Convert this C ABI array into bridge values.
    pub(crate) fn to_bridge(&self) -> Result<Vec<destack::language::DiagnosticLabel>, String> {
        if self.len == 0 {
            return Ok(Vec::new());
        }
        if self.ptr.is_null() {
            return Err("array pointer is null".to_string());
        }
        let values = unsafe { slice::from_raw_parts(self.ptr, self.len) };
        let mut converted = Vec::with_capacity(values.len());
        for value in values {
            converted.push(value.to_bridge()?);
        }
        Ok(converted)
    }

    /// Destroy this C ABI array.
    pub(crate) fn destroy(&mut self) {
        if self.ptr.is_null() {
            return;
        }
        unsafe {
            destroy_array(self.ptr, self.len, |value| value.destroy());
        }
        self.ptr = ptr::null_mut();
        self.len = 0;
    }
}

impl DestackOptionalDiagnosticLabel {
    /// Convert one optional bridge value into one C ABI optional value.
    pub(crate) fn from_bridge(
        value: Option<destack::language::DiagnosticLabel>,
    ) -> Result<Self, String> {
        let Some(value) = value else {
            return Ok(Self {
                is_some: false,
                value: DestackDiagnosticLabel::empty(),
            });
        };
        Ok(Self {
            is_some: true,
            value: DestackDiagnosticLabel::from_bridge(value)?,
        })
    }

    /// Convert this C ABI optional value into one bridge optional value.
    pub(crate) fn to_bridge(&self) -> Result<Option<destack::language::DiagnosticLabel>, String> {
        if self.is_some {
            Ok(Some(self.value.to_bridge()?))
        } else {
            Ok(None)
        }
    }

    /// Destroy this C ABI optional value.
    pub(crate) fn destroy(&mut self) {
        if self.is_some {
            self.value.destroy();
        }
        self.is_some = false;
        self.value = DestackDiagnosticLabel::empty();
    }
}

/// C ABI bridge value.
#[repr(C)]
#[derive(Debug)]
pub struct DestackDiagnosticNote {
    /// Note message.
    pub(crate) message: *mut c_char,
}

/// C ABI bridge value array.
#[repr(C)]
#[derive(Debug)]
pub struct DestackDiagnosticNoteArray {
    /// Owned value pointer.
    pub(crate) ptr: *mut DestackDiagnosticNote,
    /// Value count.
    pub(crate) len: usize,
}

/// C ABI optional bridge value.
#[repr(C)]
#[derive(Debug)]
pub struct DestackOptionalDiagnosticNote {
    /// Whether the value is present.
    pub(crate) is_some: bool,
    /// Value when present.
    pub(crate) value: DestackDiagnosticNote,
}

impl DestackDiagnosticNote {
    /// Convert one bridge value into one C ABI value.
    pub(crate) fn from_bridge(value: destack::language::DiagnosticNote) -> Result<Self, String> {
        Ok(Self {
            message: c_string(value.message)?,
        })
    }

    /// Convert this C ABI value into one bridge value.
    pub(crate) fn to_bridge(&self) -> Result<destack::language::DiagnosticNote, String> {
        Ok(destack::language::DiagnosticNote {
            message: read_string(self.message)?,
        })
    }

    /// Destroy this C ABI value.
    pub(crate) fn destroy(&mut self) {
        destroy_string(self.message);
        self.message = ptr::null_mut();
    }

    /// Return one empty C ABI value.
    pub(crate) fn empty() -> Self {
        Self {
            message: ptr::null_mut(),
        }
    }
}

impl DestackDiagnosticNoteArray {
    /// Convert bridge values into one C ABI array.
    pub(crate) fn from_bridge(
        values: Vec<destack::language::DiagnosticNote>,
    ) -> Result<Self, String> {
        let mut converted = Vec::with_capacity(values.len());
        for value in values {
            converted.push(DestackDiagnosticNote::from_bridge(value)?);
        }
        let (ptr, len) = owned_array(converted);
        Ok(Self { ptr, len })
    }

    /// Convert this C ABI array into bridge values.
    pub(crate) fn to_bridge(&self) -> Result<Vec<destack::language::DiagnosticNote>, String> {
        if self.len == 0 {
            return Ok(Vec::new());
        }
        if self.ptr.is_null() {
            return Err("array pointer is null".to_string());
        }
        let values = unsafe { slice::from_raw_parts(self.ptr, self.len) };
        let mut converted = Vec::with_capacity(values.len());
        for value in values {
            converted.push(value.to_bridge()?);
        }
        Ok(converted)
    }

    /// Destroy this C ABI array.
    pub(crate) fn destroy(&mut self) {
        if self.ptr.is_null() {
            return;
        }
        unsafe {
            destroy_array(self.ptr, self.len, |value| value.destroy());
        }
        self.ptr = ptr::null_mut();
        self.len = 0;
    }
}

impl DestackOptionalDiagnosticNote {
    /// Convert one optional bridge value into one C ABI optional value.
    pub(crate) fn from_bridge(
        value: Option<destack::language::DiagnosticNote>,
    ) -> Result<Self, String> {
        let Some(value) = value else {
            return Ok(Self {
                is_some: false,
                value: DestackDiagnosticNote::empty(),
            });
        };
        Ok(Self {
            is_some: true,
            value: DestackDiagnosticNote::from_bridge(value)?,
        })
    }

    /// Convert this C ABI optional value into one bridge optional value.
    pub(crate) fn to_bridge(&self) -> Result<Option<destack::language::DiagnosticNote>, String> {
        if self.is_some {
            Ok(Some(self.value.to_bridge()?))
        } else {
            Ok(None)
        }
    }

    /// Destroy this C ABI optional value.
    pub(crate) fn destroy(&mut self) {
        if self.is_some {
            self.value.destroy();
        }
        self.is_some = false;
        self.value = DestackDiagnosticNote::empty();
    }
}

/// C ABI bridge enum.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DestackDiagnosticSeverity {
    /// Informative message.
    Note = 0,
    /// Non-critical issue.
    Warning = 1,
    /// Critical issue.
    Error = 2,
}

/// C ABI bridge enum array.
#[repr(C)]
#[derive(Debug)]
pub struct DestackDiagnosticSeverityArray {
    /// Owned value pointer.
    pub(crate) ptr: *mut DestackDiagnosticSeverity,
    /// Value count.
    pub(crate) len: usize,
}

/// C ABI optional bridge enum.
#[repr(C)]
#[derive(Debug)]
pub struct DestackOptionalDiagnosticSeverity {
    /// Whether the value is present.
    pub(crate) is_some: bool,
    /// Value when present.
    pub(crate) value: DestackDiagnosticSeverity,
}

impl DestackDiagnosticSeverity {
    /// Convert one bridge enum into one C ABI enum.
    pub(crate) fn from_bridge(
        value: destack::language::DiagnosticSeverity,
    ) -> Result<Self, String> {
        Ok(match value {
            destack::language::DiagnosticSeverity::Note => Self::Note,
            destack::language::DiagnosticSeverity::Warning => Self::Warning,
            destack::language::DiagnosticSeverity::Error => Self::Error,
        })
    }

    /// Convert this C ABI enum into one bridge enum.
    pub(crate) fn to_bridge(self) -> Result<destack::language::DiagnosticSeverity, String> {
        Ok(match self {
            Self::Note => destack::language::DiagnosticSeverity::Note,
            Self::Warning => destack::language::DiagnosticSeverity::Warning,
            Self::Error => destack::language::DiagnosticSeverity::Error,
        })
    }

    /// Destroy this C ABI enum.
    pub(crate) fn destroy(&mut self) {}
    /// Return one empty C ABI enum.
    pub(crate) fn empty() -> Self {
        Self::Note
    }
}

impl DestackDiagnosticSeverityArray {
    /// Convert bridge values into one C ABI array.
    pub(crate) fn from_bridge(
        values: Vec<destack::language::DiagnosticSeverity>,
    ) -> Result<Self, String> {
        let mut converted = Vec::with_capacity(values.len());
        for value in values {
            converted.push(DestackDiagnosticSeverity::from_bridge(value)?);
        }
        let (ptr, len) = owned_array(converted);
        Ok(Self { ptr, len })
    }

    /// Convert this C ABI array into bridge values.
    pub(crate) fn to_bridge(&self) -> Result<Vec<destack::language::DiagnosticSeverity>, String> {
        if self.len == 0 {
            return Ok(Vec::new());
        }
        if self.ptr.is_null() {
            return Err("array pointer is null".to_string());
        }
        let values = unsafe { slice::from_raw_parts(self.ptr, self.len) };
        let mut converted = Vec::with_capacity(values.len());
        for value in values {
            converted.push(value.to_bridge()?);
        }
        Ok(converted)
    }

    /// Destroy this C ABI array.
    pub(crate) fn destroy(&mut self) {
        if self.ptr.is_null() {
            return;
        }
        unsafe {
            destroy_array(self.ptr, self.len, |value| value.destroy());
        }
        self.ptr = ptr::null_mut();
        self.len = 0;
    }
}

impl DestackOptionalDiagnosticSeverity {
    /// Convert one optional bridge value into one C ABI optional value.
    pub(crate) fn from_bridge(
        value: Option<destack::language::DiagnosticSeverity>,
    ) -> Result<Self, String> {
        let Some(value) = value else {
            return Ok(Self {
                is_some: false,
                value: DestackDiagnosticSeverity::empty(),
            });
        };
        Ok(Self {
            is_some: true,
            value: DestackDiagnosticSeverity::from_bridge(value)?,
        })
    }

    /// Convert this C ABI optional value into one bridge optional value.
    pub(crate) fn to_bridge(
        &self,
    ) -> Result<Option<destack::language::DiagnosticSeverity>, String> {
        if self.is_some {
            Ok(Some(self.value.to_bridge()?))
        } else {
            Ok(None)
        }
    }

    /// Destroy this C ABI optional value.
    pub(crate) fn destroy(&mut self) {
        if self.is_some {
            self.value.destroy();
        }
        self.is_some = false;
        self.value = DestackDiagnosticSeverity::empty();
    }
}

/// C ABI bridge value.
#[repr(C)]
#[derive(Debug)]
pub struct DestackPatch {
    /// Source span to replace.
    pub(crate) span: DestackSpan,
    /// Patch text.
    pub(crate) new_text: *mut c_char,
}

/// C ABI bridge value array.
#[repr(C)]
#[derive(Debug)]
pub struct DestackPatchArray {
    /// Owned value pointer.
    pub(crate) ptr: *mut DestackPatch,
    /// Value count.
    pub(crate) len: usize,
}

/// C ABI optional bridge value.
#[repr(C)]
#[derive(Debug)]
pub struct DestackOptionalPatch {
    /// Whether the value is present.
    pub(crate) is_some: bool,
    /// Value when present.
    pub(crate) value: DestackPatch,
}

impl DestackPatch {
    /// Convert one bridge value into one C ABI value.
    pub(crate) fn from_bridge(value: destack::language::Patch) -> Result<Self, String> {
        Ok(Self {
            span: DestackSpan::from_bridge(value.span)?,
            new_text: c_string(value.new_text)?,
        })
    }

    /// Convert this C ABI value into one bridge value.
    pub(crate) fn to_bridge(&self) -> Result<destack::language::Patch, String> {
        Ok(destack::language::Patch {
            span: self.span.to_bridge()?,
            new_text: read_string(self.new_text)?,
        })
    }

    /// Destroy this C ABI value.
    pub(crate) fn destroy(&mut self) {
        self.span.destroy();
        destroy_string(self.new_text);
        self.new_text = ptr::null_mut();
    }

    /// Return one empty C ABI value.
    pub(crate) fn empty() -> Self {
        Self {
            span: DestackSpan::empty(),
            new_text: ptr::null_mut(),
        }
    }
}

impl DestackPatchArray {
    /// Convert bridge values into one C ABI array.
    pub(crate) fn from_bridge(values: Vec<destack::language::Patch>) -> Result<Self, String> {
        let mut converted = Vec::with_capacity(values.len());
        for value in values {
            converted.push(DestackPatch::from_bridge(value)?);
        }
        let (ptr, len) = owned_array(converted);
        Ok(Self { ptr, len })
    }

    /// Convert this C ABI array into bridge values.
    pub(crate) fn to_bridge(&self) -> Result<Vec<destack::language::Patch>, String> {
        if self.len == 0 {
            return Ok(Vec::new());
        }
        if self.ptr.is_null() {
            return Err("array pointer is null".to_string());
        }
        let values = unsafe { slice::from_raw_parts(self.ptr, self.len) };
        let mut converted = Vec::with_capacity(values.len());
        for value in values {
            converted.push(value.to_bridge()?);
        }
        Ok(converted)
    }

    /// Destroy this C ABI array.
    pub(crate) fn destroy(&mut self) {
        if self.ptr.is_null() {
            return;
        }
        unsafe {
            destroy_array(self.ptr, self.len, |value| value.destroy());
        }
        self.ptr = ptr::null_mut();
        self.len = 0;
    }
}

impl DestackOptionalPatch {
    /// Convert one optional bridge value into one C ABI optional value.
    pub(crate) fn from_bridge(value: Option<destack::language::Patch>) -> Result<Self, String> {
        let Some(value) = value else {
            return Ok(Self {
                is_some: false,
                value: DestackPatch::empty(),
            });
        };
        Ok(Self {
            is_some: true,
            value: DestackPatch::from_bridge(value)?,
        })
    }

    /// Convert this C ABI optional value into one bridge optional value.
    pub(crate) fn to_bridge(&self) -> Result<Option<destack::language::Patch>, String> {
        if self.is_some {
            Ok(Some(self.value.to_bridge()?))
        } else {
            Ok(None)
        }
    }

    /// Destroy this C ABI optional value.
    pub(crate) fn destroy(&mut self) {
        if self.is_some {
            self.value.destroy();
        }
        self.is_some = false;
        self.value = DestackPatch::empty();
    }
}

/// C ABI bridge value.
#[repr(C)]
#[derive(Debug)]
pub struct DestackFilePatch {
    /// Edited file.
    pub(crate) file: DestackFileId,
    /// Source patches.
    pub(crate) patches: DestackPatchArray,
}

/// C ABI bridge value array.
#[repr(C)]
#[derive(Debug)]
pub struct DestackFilePatchArray {
    /// Owned value pointer.
    pub(crate) ptr: *mut DestackFilePatch,
    /// Value count.
    pub(crate) len: usize,
}

/// C ABI optional bridge value.
#[repr(C)]
#[derive(Debug)]
pub struct DestackOptionalFilePatch {
    /// Whether the value is present.
    pub(crate) is_some: bool,
    /// Value when present.
    pub(crate) value: DestackFilePatch,
}

impl DestackFilePatch {
    /// Convert one bridge value into one C ABI value.
    pub(crate) fn from_bridge(value: destack::language::FilePatch) -> Result<Self, String> {
        Ok(Self {
            file: DestackFileId::from_bridge(value.file)?,
            patches: DestackPatchArray::from_bridge(value.patches)?,
        })
    }

    /// Convert this C ABI value into one bridge value.
    pub(crate) fn to_bridge(&self) -> Result<destack::language::FilePatch, String> {
        Ok(destack::language::FilePatch {
            file: self.file.to_bridge()?,
            patches: self.patches.to_bridge()?,
        })
    }

    /// Destroy this C ABI value.
    pub(crate) fn destroy(&mut self) {
        self.file.destroy();
        self.patches.destroy();
    }

    /// Return one empty C ABI value.
    pub(crate) fn empty() -> Self {
        Self {
            file: DestackFileId::empty(),
            patches: DestackPatchArray {
                ptr: ptr::null_mut(),
                len: 0,
            },
        }
    }
}

impl DestackFilePatchArray {
    /// Convert bridge values into one C ABI array.
    pub(crate) fn from_bridge(values: Vec<destack::language::FilePatch>) -> Result<Self, String> {
        let mut converted = Vec::with_capacity(values.len());
        for value in values {
            converted.push(DestackFilePatch::from_bridge(value)?);
        }
        let (ptr, len) = owned_array(converted);
        Ok(Self { ptr, len })
    }

    /// Convert this C ABI array into bridge values.
    pub(crate) fn to_bridge(&self) -> Result<Vec<destack::language::FilePatch>, String> {
        if self.len == 0 {
            return Ok(Vec::new());
        }
        if self.ptr.is_null() {
            return Err("array pointer is null".to_string());
        }
        let values = unsafe { slice::from_raw_parts(self.ptr, self.len) };
        let mut converted = Vec::with_capacity(values.len());
        for value in values {
            converted.push(value.to_bridge()?);
        }
        Ok(converted)
    }

    /// Destroy this C ABI array.
    pub(crate) fn destroy(&mut self) {
        if self.ptr.is_null() {
            return;
        }
        unsafe {
            destroy_array(self.ptr, self.len, |value| value.destroy());
        }
        self.ptr = ptr::null_mut();
        self.len = 0;
    }
}

impl DestackOptionalFilePatch {
    /// Convert one optional bridge value into one C ABI optional value.
    pub(crate) fn from_bridge(value: Option<destack::language::FilePatch>) -> Result<Self, String> {
        let Some(value) = value else {
            return Ok(Self {
                is_some: false,
                value: DestackFilePatch::empty(),
            });
        };
        Ok(Self {
            is_some: true,
            value: DestackFilePatch::from_bridge(value)?,
        })
    }

    /// Convert this C ABI optional value into one bridge optional value.
    pub(crate) fn to_bridge(&self) -> Result<Option<destack::language::FilePatch>, String> {
        if self.is_some {
            Ok(Some(self.value.to_bridge()?))
        } else {
            Ok(None)
        }
    }

    /// Destroy this C ABI optional value.
    pub(crate) fn destroy(&mut self) {
        if self.is_some {
            self.value.destroy();
        }
        self.is_some = false;
        self.value = DestackFilePatch::empty();
    }
}

/// C ABI bridge value.
#[repr(C)]
#[derive(Debug)]
pub struct DestackPatchSet {
    /// Per-file patches.
    pub(crate) files: DestackFilePatchArray,
}

/// C ABI bridge value array.
#[repr(C)]
#[derive(Debug)]
pub struct DestackPatchSetArray {
    /// Owned value pointer.
    pub(crate) ptr: *mut DestackPatchSet,
    /// Value count.
    pub(crate) len: usize,
}

/// C ABI optional bridge value.
#[repr(C)]
#[derive(Debug)]
pub struct DestackOptionalPatchSet {
    /// Whether the value is present.
    pub(crate) is_some: bool,
    /// Value when present.
    pub(crate) value: DestackPatchSet,
}

impl DestackPatchSet {
    /// Convert one bridge value into one C ABI value.
    pub(crate) fn from_bridge(value: destack::language::PatchSet) -> Result<Self, String> {
        Ok(Self {
            files: DestackFilePatchArray::from_bridge(value.files)?,
        })
    }

    /// Convert this C ABI value into one bridge value.
    pub(crate) fn to_bridge(&self) -> Result<destack::language::PatchSet, String> {
        Ok(destack::language::PatchSet {
            files: self.files.to_bridge()?,
        })
    }

    /// Destroy this C ABI value.
    pub(crate) fn destroy(&mut self) {
        self.files.destroy();
    }

    /// Return one empty C ABI value.
    pub(crate) fn empty() -> Self {
        Self {
            files: DestackFilePatchArray {
                ptr: ptr::null_mut(),
                len: 0,
            },
        }
    }
}

impl DestackPatchSetArray {
    /// Convert bridge values into one C ABI array.
    pub(crate) fn from_bridge(values: Vec<destack::language::PatchSet>) -> Result<Self, String> {
        let mut converted = Vec::with_capacity(values.len());
        for value in values {
            converted.push(DestackPatchSet::from_bridge(value)?);
        }
        let (ptr, len) = owned_array(converted);
        Ok(Self { ptr, len })
    }

    /// Convert this C ABI array into bridge values.
    pub(crate) fn to_bridge(&self) -> Result<Vec<destack::language::PatchSet>, String> {
        if self.len == 0 {
            return Ok(Vec::new());
        }
        if self.ptr.is_null() {
            return Err("array pointer is null".to_string());
        }
        let values = unsafe { slice::from_raw_parts(self.ptr, self.len) };
        let mut converted = Vec::with_capacity(values.len());
        for value in values {
            converted.push(value.to_bridge()?);
        }
        Ok(converted)
    }

    /// Destroy this C ABI array.
    pub(crate) fn destroy(&mut self) {
        if self.ptr.is_null() {
            return;
        }
        unsafe {
            destroy_array(self.ptr, self.len, |value| value.destroy());
        }
        self.ptr = ptr::null_mut();
        self.len = 0;
    }
}

impl DestackOptionalPatchSet {
    /// Convert one optional bridge value into one C ABI optional value.
    pub(crate) fn from_bridge(value: Option<destack::language::PatchSet>) -> Result<Self, String> {
        let Some(value) = value else {
            return Ok(Self {
                is_some: false,
                value: DestackPatchSet::empty(),
            });
        };
        Ok(Self {
            is_some: true,
            value: DestackPatchSet::from_bridge(value)?,
        })
    }

    /// Convert this C ABI optional value into one bridge optional value.
    pub(crate) fn to_bridge(&self) -> Result<Option<destack::language::PatchSet>, String> {
        if self.is_some {
            Ok(Some(self.value.to_bridge()?))
        } else {
            Ok(None)
        }
    }

    /// Destroy this C ABI optional value.
    pub(crate) fn destroy(&mut self) {
        if self.is_some {
            self.value.destroy();
        }
        self.is_some = false;
        self.value = DestackPatchSet::empty();
    }
}

/// C ABI bridge value.
#[repr(C)]
#[derive(Debug)]
pub struct DestackDiagnosticSuggestion {
    /// Exact source patches for machine application.
    pub(crate) patches: DestackPatchSet,
    /// Source labels to show with the suggestion.
    pub(crate) labels: DestackDiagnosticLabelArray,
    /// Suggestion message.
    pub(crate) message: *mut c_char,
    /// Suggestion applicability.
    pub(crate) applicability: DestackApplicability,
}

/// C ABI bridge value array.
#[repr(C)]
#[derive(Debug)]
pub struct DestackDiagnosticSuggestionArray {
    /// Owned value pointer.
    pub(crate) ptr: *mut DestackDiagnosticSuggestion,
    /// Value count.
    pub(crate) len: usize,
}

/// C ABI optional bridge value.
#[repr(C)]
#[derive(Debug)]
pub struct DestackOptionalDiagnosticSuggestion {
    /// Whether the value is present.
    pub(crate) is_some: bool,
    /// Value when present.
    pub(crate) value: DestackDiagnosticSuggestion,
}

impl DestackDiagnosticSuggestion {
    /// Convert one bridge value into one C ABI value.
    pub(crate) fn from_bridge(
        value: destack::language::DiagnosticSuggestion,
    ) -> Result<Self, String> {
        Ok(Self {
            patches: DestackPatchSet::from_bridge(value.patches)?,
            labels: DestackDiagnosticLabelArray::from_bridge(value.labels)?,
            message: c_string(value.message)?,
            applicability: DestackApplicability::from_bridge(value.applicability)?,
        })
    }

    /// Convert this C ABI value into one bridge value.
    pub(crate) fn to_bridge(&self) -> Result<destack::language::DiagnosticSuggestion, String> {
        Ok(destack::language::DiagnosticSuggestion {
            patches: self.patches.to_bridge()?,
            labels: self.labels.to_bridge()?,
            message: read_string(self.message)?,
            applicability: self.applicability.to_bridge()?,
        })
    }

    /// Destroy this C ABI value.
    pub(crate) fn destroy(&mut self) {
        self.patches.destroy();
        self.labels.destroy();
        destroy_string(self.message);
        self.message = ptr::null_mut();
        self.applicability.destroy();
    }

    /// Return one empty C ABI value.
    pub(crate) fn empty() -> Self {
        Self {
            patches: DestackPatchSet::empty(),
            labels: DestackDiagnosticLabelArray {
                ptr: ptr::null_mut(),
                len: 0,
            },
            message: ptr::null_mut(),
            applicability: DestackApplicability::empty(),
        }
    }
}

impl DestackDiagnosticSuggestionArray {
    /// Convert bridge values into one C ABI array.
    pub(crate) fn from_bridge(
        values: Vec<destack::language::DiagnosticSuggestion>,
    ) -> Result<Self, String> {
        let mut converted = Vec::with_capacity(values.len());
        for value in values {
            converted.push(DestackDiagnosticSuggestion::from_bridge(value)?);
        }
        let (ptr, len) = owned_array(converted);
        Ok(Self { ptr, len })
    }

    /// Convert this C ABI array into bridge values.
    pub(crate) fn to_bridge(&self) -> Result<Vec<destack::language::DiagnosticSuggestion>, String> {
        if self.len == 0 {
            return Ok(Vec::new());
        }
        if self.ptr.is_null() {
            return Err("array pointer is null".to_string());
        }
        let values = unsafe { slice::from_raw_parts(self.ptr, self.len) };
        let mut converted = Vec::with_capacity(values.len());
        for value in values {
            converted.push(value.to_bridge()?);
        }
        Ok(converted)
    }

    /// Destroy this C ABI array.
    pub(crate) fn destroy(&mut self) {
        if self.ptr.is_null() {
            return;
        }
        unsafe {
            destroy_array(self.ptr, self.len, |value| value.destroy());
        }
        self.ptr = ptr::null_mut();
        self.len = 0;
    }
}

impl DestackOptionalDiagnosticSuggestion {
    /// Convert one optional bridge value into one C ABI optional value.
    pub(crate) fn from_bridge(
        value: Option<destack::language::DiagnosticSuggestion>,
    ) -> Result<Self, String> {
        let Some(value) = value else {
            return Ok(Self {
                is_some: false,
                value: DestackDiagnosticSuggestion::empty(),
            });
        };
        Ok(Self {
            is_some: true,
            value: DestackDiagnosticSuggestion::from_bridge(value)?,
        })
    }

    /// Convert this C ABI optional value into one bridge optional value.
    pub(crate) fn to_bridge(
        &self,
    ) -> Result<Option<destack::language::DiagnosticSuggestion>, String> {
        if self.is_some {
            Ok(Some(self.value.to_bridge()?))
        } else {
            Ok(None)
        }
    }

    /// Destroy this C ABI optional value.
    pub(crate) fn destroy(&mut self) {
        if self.is_some {
            self.value.destroy();
        }
        self.is_some = false;
        self.value = DestackDiagnosticSuggestion::empty();
    }
}

/// C ABI bridge enum.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DestackDiagnosticTag {
    /// Unused or unnecessary source.
    Unnecessary = 0,
    /// Deprecated source.
    Deprecated = 1,
}

/// C ABI bridge enum array.
#[repr(C)]
#[derive(Debug)]
pub struct DestackDiagnosticTagArray {
    /// Owned value pointer.
    pub(crate) ptr: *mut DestackDiagnosticTag,
    /// Value count.
    pub(crate) len: usize,
}

/// C ABI optional bridge enum.
#[repr(C)]
#[derive(Debug)]
pub struct DestackOptionalDiagnosticTag {
    /// Whether the value is present.
    pub(crate) is_some: bool,
    /// Value when present.
    pub(crate) value: DestackDiagnosticTag,
}

impl DestackDiagnosticTag {
    /// Convert one bridge enum into one C ABI enum.
    pub(crate) fn from_bridge(value: destack::language::DiagnosticTag) -> Result<Self, String> {
        Ok(match value {
            destack::language::DiagnosticTag::Unnecessary => Self::Unnecessary,
            destack::language::DiagnosticTag::Deprecated => Self::Deprecated,
        })
    }

    /// Convert this C ABI enum into one bridge enum.
    pub(crate) fn to_bridge(self) -> Result<destack::language::DiagnosticTag, String> {
        Ok(match self {
            Self::Unnecessary => destack::language::DiagnosticTag::Unnecessary,
            Self::Deprecated => destack::language::DiagnosticTag::Deprecated,
        })
    }

    /// Destroy this C ABI enum.
    pub(crate) fn destroy(&mut self) {}
    /// Return one empty C ABI enum.
    pub(crate) fn empty() -> Self {
        Self::Unnecessary
    }
}

impl DestackDiagnosticTagArray {
    /// Convert bridge values into one C ABI array.
    pub(crate) fn from_bridge(
        values: Vec<destack::language::DiagnosticTag>,
    ) -> Result<Self, String> {
        let mut converted = Vec::with_capacity(values.len());
        for value in values {
            converted.push(DestackDiagnosticTag::from_bridge(value)?);
        }
        let (ptr, len) = owned_array(converted);
        Ok(Self { ptr, len })
    }

    /// Convert this C ABI array into bridge values.
    pub(crate) fn to_bridge(&self) -> Result<Vec<destack::language::DiagnosticTag>, String> {
        if self.len == 0 {
            return Ok(Vec::new());
        }
        if self.ptr.is_null() {
            return Err("array pointer is null".to_string());
        }
        let values = unsafe { slice::from_raw_parts(self.ptr, self.len) };
        let mut converted = Vec::with_capacity(values.len());
        for value in values {
            converted.push(value.to_bridge()?);
        }
        Ok(converted)
    }

    /// Destroy this C ABI array.
    pub(crate) fn destroy(&mut self) {
        if self.ptr.is_null() {
            return;
        }
        unsafe {
            destroy_array(self.ptr, self.len, |value| value.destroy());
        }
        self.ptr = ptr::null_mut();
        self.len = 0;
    }
}

impl DestackOptionalDiagnosticTag {
    /// Convert one optional bridge value into one C ABI optional value.
    pub(crate) fn from_bridge(
        value: Option<destack::language::DiagnosticTag>,
    ) -> Result<Self, String> {
        let Some(value) = value else {
            return Ok(Self {
                is_some: false,
                value: DestackDiagnosticTag::empty(),
            });
        };
        Ok(Self {
            is_some: true,
            value: DestackDiagnosticTag::from_bridge(value)?,
        })
    }

    /// Convert this C ABI optional value into one bridge optional value.
    pub(crate) fn to_bridge(&self) -> Result<Option<destack::language::DiagnosticTag>, String> {
        if self.is_some {
            Ok(Some(self.value.to_bridge()?))
        } else {
            Ok(None)
        }
    }

    /// Destroy this C ABI optional value.
    pub(crate) fn destroy(&mut self) {
        if self.is_some {
            self.value.destroy();
        }
        self.is_some = false;
        self.value = DestackDiagnosticTag::empty();
    }
}

/// C ABI bridge value.
#[repr(C)]
#[derive(Debug)]
pub struct DestackDiagnostic {
    /// Stable diagnostic code.
    pub(crate) code: *mut c_char,
    /// Diagnostic severity.
    pub(crate) severity: DestackDiagnosticSeverity,
    /// Diagnostic message.
    pub(crate) message: *mut c_char,
    /// Main source label.
    pub(crate) primary: DestackDiagnosticLabel,
    /// Additional source labels.
    pub(crate) labels: DestackDiagnosticLabelArray,
    /// Extra context.
    pub(crate) notes: DestackDiagnosticNoteArray,
    /// Fixing or avoidance guidance.
    pub(crate) helps: DestackDiagnosticHelpArray,
    /// Suggested source changes.
    pub(crate) suggestions: DestackDiagnosticSuggestionArray,
    /// Extra semantic tags.
    pub(crate) tags: DestackDiagnosticTagArray,
}

/// C ABI bridge value array.
#[repr(C)]
#[derive(Debug)]
pub struct DestackDiagnosticArray {
    /// Owned value pointer.
    pub(crate) ptr: *mut DestackDiagnostic,
    /// Value count.
    pub(crate) len: usize,
}

/// C ABI optional bridge value.
#[repr(C)]
#[derive(Debug)]
pub struct DestackOptionalDiagnostic {
    /// Whether the value is present.
    pub(crate) is_some: bool,
    /// Value when present.
    pub(crate) value: DestackDiagnostic,
}

impl DestackDiagnostic {
    /// Convert one bridge value into one C ABI value.
    pub(crate) fn from_bridge(value: destack::language::Diagnostic) -> Result<Self, String> {
        Ok(Self {
            code: c_string(value.code)?,
            severity: DestackDiagnosticSeverity::from_bridge(value.severity)?,
            message: c_string(value.message)?,
            primary: DestackDiagnosticLabel::from_bridge(value.primary)?,
            labels: DestackDiagnosticLabelArray::from_bridge(value.labels)?,
            notes: DestackDiagnosticNoteArray::from_bridge(value.notes)?,
            helps: DestackDiagnosticHelpArray::from_bridge(value.helps)?,
            suggestions: DestackDiagnosticSuggestionArray::from_bridge(value.suggestions)?,
            tags: DestackDiagnosticTagArray::from_bridge(value.tags)?,
        })
    }

    /// Convert this C ABI value into one bridge value.
    pub(crate) fn to_bridge(&self) -> Result<destack::language::Diagnostic, String> {
        Ok(destack::language::Diagnostic {
            code: read_string(self.code)?,
            severity: self.severity.to_bridge()?,
            message: read_string(self.message)?,
            primary: self.primary.to_bridge()?,
            labels: self.labels.to_bridge()?,
            notes: self.notes.to_bridge()?,
            helps: self.helps.to_bridge()?,
            suggestions: self.suggestions.to_bridge()?,
            tags: self.tags.to_bridge()?,
        })
    }

    /// Destroy this C ABI value.
    pub(crate) fn destroy(&mut self) {
        destroy_string(self.code);
        self.code = ptr::null_mut();
        self.severity.destroy();
        destroy_string(self.message);
        self.message = ptr::null_mut();
        self.primary.destroy();
        self.labels.destroy();
        self.notes.destroy();
        self.helps.destroy();
        self.suggestions.destroy();
        self.tags.destroy();
    }

    /// Return one empty C ABI value.
    pub(crate) fn empty() -> Self {
        Self {
            code: ptr::null_mut(),
            severity: DestackDiagnosticSeverity::empty(),
            message: ptr::null_mut(),
            primary: DestackDiagnosticLabel::empty(),
            labels: DestackDiagnosticLabelArray {
                ptr: ptr::null_mut(),
                len: 0,
            },
            notes: DestackDiagnosticNoteArray {
                ptr: ptr::null_mut(),
                len: 0,
            },
            helps: DestackDiagnosticHelpArray {
                ptr: ptr::null_mut(),
                len: 0,
            },
            suggestions: DestackDiagnosticSuggestionArray {
                ptr: ptr::null_mut(),
                len: 0,
            },
            tags: DestackDiagnosticTagArray {
                ptr: ptr::null_mut(),
                len: 0,
            },
        }
    }
}

impl DestackDiagnosticArray {
    /// Convert bridge values into one C ABI array.
    pub(crate) fn from_bridge(values: Vec<destack::language::Diagnostic>) -> Result<Self, String> {
        let mut converted = Vec::with_capacity(values.len());
        for value in values {
            converted.push(DestackDiagnostic::from_bridge(value)?);
        }
        let (ptr, len) = owned_array(converted);
        Ok(Self { ptr, len })
    }

    /// Convert this C ABI array into bridge values.
    pub(crate) fn to_bridge(&self) -> Result<Vec<destack::language::Diagnostic>, String> {
        if self.len == 0 {
            return Ok(Vec::new());
        }
        if self.ptr.is_null() {
            return Err("array pointer is null".to_string());
        }
        let values = unsafe { slice::from_raw_parts(self.ptr, self.len) };
        let mut converted = Vec::with_capacity(values.len());
        for value in values {
            converted.push(value.to_bridge()?);
        }
        Ok(converted)
    }

    /// Destroy this C ABI array.
    pub(crate) fn destroy(&mut self) {
        if self.ptr.is_null() {
            return;
        }
        unsafe {
            destroy_array(self.ptr, self.len, |value| value.destroy());
        }
        self.ptr = ptr::null_mut();
        self.len = 0;
    }
}

impl DestackOptionalDiagnostic {
    /// Convert one optional bridge value into one C ABI optional value.
    pub(crate) fn from_bridge(
        value: Option<destack::language::Diagnostic>,
    ) -> Result<Self, String> {
        let Some(value) = value else {
            return Ok(Self {
                is_some: false,
                value: DestackDiagnostic::empty(),
            });
        };
        Ok(Self {
            is_some: true,
            value: DestackDiagnostic::from_bridge(value)?,
        })
    }

    /// Convert this C ABI optional value into one bridge optional value.
    pub(crate) fn to_bridge(&self) -> Result<Option<destack::language::Diagnostic>, String> {
        if self.is_some {
            Ok(Some(self.value.to_bridge()?))
        } else {
            Ok(None)
        }
    }

    /// Destroy this C ABI optional value.
    pub(crate) fn destroy(&mut self) {
        if self.is_some {
            self.value.destroy();
        }
        self.is_some = false;
        self.value = DestackDiagnostic::empty();
    }
}

/// C ABI bridge value.
#[repr(C)]
#[derive(Debug)]
pub struct DestackArtifactRecord {
    /// The exact artifact version.
    pub(crate) version: DestackArtifactVersion,
    /// The predecessor artifact this record was incrementally built from.
    pub(crate) base: DestackOptionalArtifactVersion,
    /// Serialized artifact payload bytes.
    pub(crate) payload: DestackByteArray,
    /// String pool needed to interpret interned ids in the payload.
    pub(crate) strings: DestackArtifactStringArray,
    /// Exact artifact dependencies.
    pub(crate) dependencies: DestackArtifactDependencyArray,
    /// Diagnostics recorded for this artifact version.
    pub(crate) diagnostics: DestackDiagnosticArray,
    /// Artifact sidecars recorded for this artifact version.
    pub(crate) sidecars: DestackArtifactSidecarArray,
}

/// C ABI bridge value array.
#[repr(C)]
#[derive(Debug)]
pub struct DestackArtifactRecordArray {
    /// Owned value pointer.
    pub(crate) ptr: *mut DestackArtifactRecord,
    /// Value count.
    pub(crate) len: usize,
}

/// C ABI optional bridge value.
#[repr(C)]
#[derive(Debug)]
pub struct DestackOptionalArtifactRecord {
    /// Whether the value is present.
    pub(crate) is_some: bool,
    /// Value when present.
    pub(crate) value: DestackArtifactRecord,
}

impl DestackArtifactRecord {
    /// Convert one bridge value into one C ABI value.
    pub(crate) fn from_bridge(value: destack::language::ArtifactRecord) -> Result<Self, String> {
        Ok(Self {
            version: DestackArtifactVersion::from_bridge(value.version)?,
            base: DestackOptionalArtifactVersion::from_bridge(value.base)?,
            payload: DestackByteArray::from_vec(value.payload),
            strings: DestackArtifactStringArray::from_bridge(value.strings)?,
            dependencies: DestackArtifactDependencyArray::from_bridge(value.dependencies)?,
            diagnostics: DestackDiagnosticArray::from_bridge(value.diagnostics)?,
            sidecars: DestackArtifactSidecarArray::from_bridge(value.sidecars)?,
        })
    }

    /// Convert this C ABI value into one bridge value.
    pub(crate) fn to_bridge(&self) -> Result<destack::language::ArtifactRecord, String> {
        Ok(destack::language::ArtifactRecord {
            version: self.version.to_bridge()?,
            base: self.base.to_bridge()?,
            payload: read_bytes(self.payload.ptr.cast_const(), self.payload.len)?,
            strings: self.strings.to_bridge()?,
            dependencies: self.dependencies.to_bridge()?,
            diagnostics: self.diagnostics.to_bridge()?,
            sidecars: self.sidecars.to_bridge()?,
        })
    }

    /// Destroy this C ABI value.
    pub(crate) fn destroy(&mut self) {
        self.version.destroy();
        self.base.destroy();
        self.payload.destroy();
        self.strings.destroy();
        self.dependencies.destroy();
        self.diagnostics.destroy();
        self.sidecars.destroy();
    }

    /// Return one empty C ABI value.
    pub(crate) fn empty() -> Self {
        Self {
            version: DestackArtifactVersion::empty(),
            base: DestackOptionalArtifactVersion {
                is_some: false,
                value: DestackArtifactVersion::empty(),
            },
            payload: DestackByteArray {
                ptr: ptr::null_mut(),
                len: 0,
            },
            strings: DestackArtifactStringArray {
                ptr: ptr::null_mut(),
                len: 0,
            },
            dependencies: DestackArtifactDependencyArray {
                ptr: ptr::null_mut(),
                len: 0,
            },
            diagnostics: DestackDiagnosticArray {
                ptr: ptr::null_mut(),
                len: 0,
            },
            sidecars: DestackArtifactSidecarArray {
                ptr: ptr::null_mut(),
                len: 0,
            },
        }
    }
}

impl DestackArtifactRecordArray {
    /// Convert bridge values into one C ABI array.
    pub(crate) fn from_bridge(
        values: Vec<destack::language::ArtifactRecord>,
    ) -> Result<Self, String> {
        let mut converted = Vec::with_capacity(values.len());
        for value in values {
            converted.push(DestackArtifactRecord::from_bridge(value)?);
        }
        let (ptr, len) = owned_array(converted);
        Ok(Self { ptr, len })
    }

    /// Convert this C ABI array into bridge values.
    pub(crate) fn to_bridge(&self) -> Result<Vec<destack::language::ArtifactRecord>, String> {
        if self.len == 0 {
            return Ok(Vec::new());
        }
        if self.ptr.is_null() {
            return Err("array pointer is null".to_string());
        }
        let values = unsafe { slice::from_raw_parts(self.ptr, self.len) };
        let mut converted = Vec::with_capacity(values.len());
        for value in values {
            converted.push(value.to_bridge()?);
        }
        Ok(converted)
    }

    /// Destroy this C ABI array.
    pub(crate) fn destroy(&mut self) {
        if self.ptr.is_null() {
            return;
        }
        unsafe {
            destroy_array(self.ptr, self.len, |value| value.destroy());
        }
        self.ptr = ptr::null_mut();
        self.len = 0;
    }
}

impl DestackOptionalArtifactRecord {
    /// Convert one optional bridge value into one C ABI optional value.
    pub(crate) fn from_bridge(
        value: Option<destack::language::ArtifactRecord>,
    ) -> Result<Self, String> {
        let Some(value) = value else {
            return Ok(Self {
                is_some: false,
                value: DestackArtifactRecord::empty(),
            });
        };
        Ok(Self {
            is_some: true,
            value: DestackArtifactRecord::from_bridge(value)?,
        })
    }

    /// Convert this C ABI optional value into one bridge optional value.
    pub(crate) fn to_bridge(&self) -> Result<Option<destack::language::ArtifactRecord>, String> {
        if self.is_some {
            Ok(Some(self.value.to_bridge()?))
        } else {
            Ok(None)
        }
    }

    /// Destroy this C ABI optional value.
    pub(crate) fn destroy(&mut self) {
        if self.is_some {
            self.value.destroy();
        }
        self.is_some = false;
        self.value = DestackArtifactRecord::empty();
    }
}

/// C ABI bridge value.
#[repr(C)]
#[derive(Debug)]
pub struct DestackComponentId {
    /// Canonical lowercase hex component id.
    pub(crate) id: *mut c_char,
}

/// C ABI bridge value array.
#[repr(C)]
#[derive(Debug)]
pub struct DestackComponentIdArray {
    /// Owned value pointer.
    pub(crate) ptr: *mut DestackComponentId,
    /// Value count.
    pub(crate) len: usize,
}

/// C ABI optional bridge value.
#[repr(C)]
#[derive(Debug)]
pub struct DestackOptionalComponentId {
    /// Whether the value is present.
    pub(crate) is_some: bool,
    /// Value when present.
    pub(crate) value: DestackComponentId,
}

impl DestackComponentId {
    /// Convert one bridge value into one C ABI value.
    pub(crate) fn from_bridge(value: destack::language::ComponentId) -> Result<Self, String> {
        Ok(Self {
            id: c_string(value.id)?,
        })
    }

    /// Convert this C ABI value into one bridge value.
    pub(crate) fn to_bridge(&self) -> Result<destack::language::ComponentId, String> {
        Ok(destack::language::ComponentId {
            id: read_string(self.id)?,
        })
    }

    /// Destroy this C ABI value.
    pub(crate) fn destroy(&mut self) {
        destroy_string(self.id);
        self.id = ptr::null_mut();
    }

    /// Return one empty C ABI value.
    pub(crate) fn empty() -> Self {
        Self {
            id: ptr::null_mut(),
        }
    }
}

impl DestackComponentIdArray {
    /// Convert bridge values into one C ABI array.
    pub(crate) fn from_bridge(values: Vec<destack::language::ComponentId>) -> Result<Self, String> {
        let mut converted = Vec::with_capacity(values.len());
        for value in values {
            converted.push(DestackComponentId::from_bridge(value)?);
        }
        let (ptr, len) = owned_array(converted);
        Ok(Self { ptr, len })
    }

    /// Convert this C ABI array into bridge values.
    pub(crate) fn to_bridge(&self) -> Result<Vec<destack::language::ComponentId>, String> {
        if self.len == 0 {
            return Ok(Vec::new());
        }
        if self.ptr.is_null() {
            return Err("array pointer is null".to_string());
        }
        let values = unsafe { slice::from_raw_parts(self.ptr, self.len) };
        let mut converted = Vec::with_capacity(values.len());
        for value in values {
            converted.push(value.to_bridge()?);
        }
        Ok(converted)
    }

    /// Destroy this C ABI array.
    pub(crate) fn destroy(&mut self) {
        if self.ptr.is_null() {
            return;
        }
        unsafe {
            destroy_array(self.ptr, self.len, |value| value.destroy());
        }
        self.ptr = ptr::null_mut();
        self.len = 0;
    }
}

impl DestackOptionalComponentId {
    /// Convert one optional bridge value into one C ABI optional value.
    pub(crate) fn from_bridge(
        value: Option<destack::language::ComponentId>,
    ) -> Result<Self, String> {
        let Some(value) = value else {
            return Ok(Self {
                is_some: false,
                value: DestackComponentId::empty(),
            });
        };
        Ok(Self {
            is_some: true,
            value: DestackComponentId::from_bridge(value)?,
        })
    }

    /// Convert this C ABI optional value into one bridge optional value.
    pub(crate) fn to_bridge(&self) -> Result<Option<destack::language::ComponentId>, String> {
        if self.is_some {
            Ok(Some(self.value.to_bridge()?))
        } else {
            Ok(None)
        }
    }

    /// Destroy this C ABI optional value.
    pub(crate) fn destroy(&mut self) {
        if self.is_some {
            self.value.destroy();
        }
        self.is_some = false;
        self.value = DestackComponentId::empty();
    }
}

/// C ABI bridge value.
#[repr(C)]
#[derive(Debug)]
pub struct DestackPackageId {
    /// Canonical lowercase hex package id.
    pub(crate) id: *mut c_char,
}

/// C ABI bridge value array.
#[repr(C)]
#[derive(Debug)]
pub struct DestackPackageIdArray {
    /// Owned value pointer.
    pub(crate) ptr: *mut DestackPackageId,
    /// Value count.
    pub(crate) len: usize,
}

/// C ABI optional bridge value.
#[repr(C)]
#[derive(Debug)]
pub struct DestackOptionalPackageId {
    /// Whether the value is present.
    pub(crate) is_some: bool,
    /// Value when present.
    pub(crate) value: DestackPackageId,
}

impl DestackPackageId {
    /// Convert one bridge value into one C ABI value.
    pub(crate) fn from_bridge(value: destack::language::PackageId) -> Result<Self, String> {
        Ok(Self {
            id: c_string(value.id)?,
        })
    }

    /// Convert this C ABI value into one bridge value.
    pub(crate) fn to_bridge(&self) -> Result<destack::language::PackageId, String> {
        Ok(destack::language::PackageId {
            id: read_string(self.id)?,
        })
    }

    /// Destroy this C ABI value.
    pub(crate) fn destroy(&mut self) {
        destroy_string(self.id);
        self.id = ptr::null_mut();
    }

    /// Return one empty C ABI value.
    pub(crate) fn empty() -> Self {
        Self {
            id: ptr::null_mut(),
        }
    }
}

impl DestackPackageIdArray {
    /// Convert bridge values into one C ABI array.
    pub(crate) fn from_bridge(values: Vec<destack::language::PackageId>) -> Result<Self, String> {
        let mut converted = Vec::with_capacity(values.len());
        for value in values {
            converted.push(DestackPackageId::from_bridge(value)?);
        }
        let (ptr, len) = owned_array(converted);
        Ok(Self { ptr, len })
    }

    /// Convert this C ABI array into bridge values.
    pub(crate) fn to_bridge(&self) -> Result<Vec<destack::language::PackageId>, String> {
        if self.len == 0 {
            return Ok(Vec::new());
        }
        if self.ptr.is_null() {
            return Err("array pointer is null".to_string());
        }
        let values = unsafe { slice::from_raw_parts(self.ptr, self.len) };
        let mut converted = Vec::with_capacity(values.len());
        for value in values {
            converted.push(value.to_bridge()?);
        }
        Ok(converted)
    }

    /// Destroy this C ABI array.
    pub(crate) fn destroy(&mut self) {
        if self.ptr.is_null() {
            return;
        }
        unsafe {
            destroy_array(self.ptr, self.len, |value| value.destroy());
        }
        self.ptr = ptr::null_mut();
        self.len = 0;
    }
}

impl DestackOptionalPackageId {
    /// Convert one optional bridge value into one C ABI optional value.
    pub(crate) fn from_bridge(value: Option<destack::language::PackageId>) -> Result<Self, String> {
        let Some(value) = value else {
            return Ok(Self {
                is_some: false,
                value: DestackPackageId::empty(),
            });
        };
        Ok(Self {
            is_some: true,
            value: DestackPackageId::from_bridge(value)?,
        })
    }

    /// Convert this C ABI optional value into one bridge optional value.
    pub(crate) fn to_bridge(&self) -> Result<Option<destack::language::PackageId>, String> {
        if self.is_some {
            Ok(Some(self.value.to_bridge()?))
        } else {
            Ok(None)
        }
    }

    /// Destroy this C ABI optional value.
    pub(crate) fn destroy(&mut self) {
        if self.is_some {
            self.value.destroy();
        }
        self.is_some = false;
        self.value = DestackPackageId::empty();
    }
}

/// C ABI bridge value.
#[repr(C)]
#[derive(Debug)]
pub struct DestackModuleId {
    /// Owning package.
    pub(crate) package: DestackPackageId,
    /// Canonical lowercase hex module key within the package.
    pub(crate) key: *mut c_char,
}

/// C ABI bridge value array.
#[repr(C)]
#[derive(Debug)]
pub struct DestackModuleIdArray {
    /// Owned value pointer.
    pub(crate) ptr: *mut DestackModuleId,
    /// Value count.
    pub(crate) len: usize,
}

/// C ABI optional bridge value.
#[repr(C)]
#[derive(Debug)]
pub struct DestackOptionalModuleId {
    /// Whether the value is present.
    pub(crate) is_some: bool,
    /// Value when present.
    pub(crate) value: DestackModuleId,
}

impl DestackModuleId {
    /// Convert one bridge value into one C ABI value.
    pub(crate) fn from_bridge(value: destack::language::ModuleId) -> Result<Self, String> {
        Ok(Self {
            package: DestackPackageId::from_bridge(value.package)?,
            key: c_string(value.key)?,
        })
    }

    /// Convert this C ABI value into one bridge value.
    pub(crate) fn to_bridge(&self) -> Result<destack::language::ModuleId, String> {
        Ok(destack::language::ModuleId {
            package: self.package.to_bridge()?,
            key: read_string(self.key)?,
        })
    }

    /// Destroy this C ABI value.
    pub(crate) fn destroy(&mut self) {
        self.package.destroy();
        destroy_string(self.key);
        self.key = ptr::null_mut();
    }

    /// Return one empty C ABI value.
    pub(crate) fn empty() -> Self {
        Self {
            package: DestackPackageId::empty(),
            key: ptr::null_mut(),
        }
    }
}

impl DestackModuleIdArray {
    /// Convert bridge values into one C ABI array.
    pub(crate) fn from_bridge(values: Vec<destack::language::ModuleId>) -> Result<Self, String> {
        let mut converted = Vec::with_capacity(values.len());
        for value in values {
            converted.push(DestackModuleId::from_bridge(value)?);
        }
        let (ptr, len) = owned_array(converted);
        Ok(Self { ptr, len })
    }

    /// Convert this C ABI array into bridge values.
    pub(crate) fn to_bridge(&self) -> Result<Vec<destack::language::ModuleId>, String> {
        if self.len == 0 {
            return Ok(Vec::new());
        }
        if self.ptr.is_null() {
            return Err("array pointer is null".to_string());
        }
        let values = unsafe { slice::from_raw_parts(self.ptr, self.len) };
        let mut converted = Vec::with_capacity(values.len());
        for value in values {
            converted.push(value.to_bridge()?);
        }
        Ok(converted)
    }

    /// Destroy this C ABI array.
    pub(crate) fn destroy(&mut self) {
        if self.ptr.is_null() {
            return;
        }
        unsafe {
            destroy_array(self.ptr, self.len, |value| value.destroy());
        }
        self.ptr = ptr::null_mut();
        self.len = 0;
    }
}

impl DestackOptionalModuleId {
    /// Convert one optional bridge value into one C ABI optional value.
    pub(crate) fn from_bridge(value: Option<destack::language::ModuleId>) -> Result<Self, String> {
        let Some(value) = value else {
            return Ok(Self {
                is_some: false,
                value: DestackModuleId::empty(),
            });
        };
        Ok(Self {
            is_some: true,
            value: DestackModuleId::from_bridge(value)?,
        })
    }

    /// Convert this C ABI optional value into one bridge optional value.
    pub(crate) fn to_bridge(&self) -> Result<Option<destack::language::ModuleId>, String> {
        if self.is_some {
            Ok(Some(self.value.to_bridge()?))
        } else {
            Ok(None)
        }
    }

    /// Destroy this C ABI optional value.
    pub(crate) fn destroy(&mut self) {
        if self.is_some {
            self.value.destroy();
        }
        self.is_some = false;
        self.value = DestackModuleId::empty();
    }
}

/// C ABI bridge value.
#[repr(C)]
#[derive(Debug)]
pub struct DestackProfileId {
    /// Canonical lowercase hex profile id.
    pub(crate) id: *mut c_char,
}

/// C ABI bridge value array.
#[repr(C)]
#[derive(Debug)]
pub struct DestackProfileIdArray {
    /// Owned value pointer.
    pub(crate) ptr: *mut DestackProfileId,
    /// Value count.
    pub(crate) len: usize,
}

/// C ABI optional bridge value.
#[repr(C)]
#[derive(Debug)]
pub struct DestackOptionalProfileId {
    /// Whether the value is present.
    pub(crate) is_some: bool,
    /// Value when present.
    pub(crate) value: DestackProfileId,
}

impl DestackProfileId {
    /// Convert one bridge value into one C ABI value.
    pub(crate) fn from_bridge(value: destack::language::ProfileId) -> Result<Self, String> {
        Ok(Self {
            id: c_string(value.id)?,
        })
    }

    /// Convert this C ABI value into one bridge value.
    pub(crate) fn to_bridge(&self) -> Result<destack::language::ProfileId, String> {
        Ok(destack::language::ProfileId {
            id: read_string(self.id)?,
        })
    }

    /// Destroy this C ABI value.
    pub(crate) fn destroy(&mut self) {
        destroy_string(self.id);
        self.id = ptr::null_mut();
    }

    /// Return one empty C ABI value.
    pub(crate) fn empty() -> Self {
        Self {
            id: ptr::null_mut(),
        }
    }
}

impl DestackProfileIdArray {
    /// Convert bridge values into one C ABI array.
    pub(crate) fn from_bridge(values: Vec<destack::language::ProfileId>) -> Result<Self, String> {
        let mut converted = Vec::with_capacity(values.len());
        for value in values {
            converted.push(DestackProfileId::from_bridge(value)?);
        }
        let (ptr, len) = owned_array(converted);
        Ok(Self { ptr, len })
    }

    /// Convert this C ABI array into bridge values.
    pub(crate) fn to_bridge(&self) -> Result<Vec<destack::language::ProfileId>, String> {
        if self.len == 0 {
            return Ok(Vec::new());
        }
        if self.ptr.is_null() {
            return Err("array pointer is null".to_string());
        }
        let values = unsafe { slice::from_raw_parts(self.ptr, self.len) };
        let mut converted = Vec::with_capacity(values.len());
        for value in values {
            converted.push(value.to_bridge()?);
        }
        Ok(converted)
    }

    /// Destroy this C ABI array.
    pub(crate) fn destroy(&mut self) {
        if self.ptr.is_null() {
            return;
        }
        unsafe {
            destroy_array(self.ptr, self.len, |value| value.destroy());
        }
        self.ptr = ptr::null_mut();
        self.len = 0;
    }
}

impl DestackOptionalProfileId {
    /// Convert one optional bridge value into one C ABI optional value.
    pub(crate) fn from_bridge(value: Option<destack::language::ProfileId>) -> Result<Self, String> {
        let Some(value) = value else {
            return Ok(Self {
                is_some: false,
                value: DestackProfileId::empty(),
            });
        };
        Ok(Self {
            is_some: true,
            value: DestackProfileId::from_bridge(value)?,
        })
    }

    /// Convert this C ABI optional value into one bridge optional value.
    pub(crate) fn to_bridge(&self) -> Result<Option<destack::language::ProfileId>, String> {
        if self.is_some {
            Ok(Some(self.value.to_bridge()?))
        } else {
            Ok(None)
        }
    }

    /// Destroy this C ABI optional value.
    pub(crate) fn destroy(&mut self) {
        if self.is_some {
            self.value.destroy();
        }
        self.is_some = false;
        self.value = DestackProfileId::empty();
    }
}

/// C ABI bridge value.
#[repr(C)]
#[derive(Debug)]
pub struct DestackDirChecked {
    /// Exact checked facade artifact version.
    pub(crate) version: DestackArtifactVersion,
    /// Checked module id.
    pub(crate) module: DestackModuleId,
    /// Checked semantic profile.
    pub(crate) profile: DestackProfileId,
    /// Component that owns the checked module output.
    pub(crate) component: DestackComponentId,
    /// Component entry module.
    pub(crate) entry: DestackModuleId,
}

/// C ABI bridge value array.
#[repr(C)]
#[derive(Debug)]
pub struct DestackDirCheckedArray {
    /// Owned value pointer.
    pub(crate) ptr: *mut DestackDirChecked,
    /// Value count.
    pub(crate) len: usize,
}

/// C ABI optional bridge value.
#[repr(C)]
#[derive(Debug)]
pub struct DestackOptionalDirChecked {
    /// Whether the value is present.
    pub(crate) is_some: bool,
    /// Value when present.
    pub(crate) value: DestackDirChecked,
}

impl DestackDirChecked {
    /// Convert one bridge value into one C ABI value.
    pub(crate) fn from_bridge(value: destack::language::DirChecked) -> Result<Self, String> {
        Ok(Self {
            version: DestackArtifactVersion::from_bridge(value.version)?,
            module: DestackModuleId::from_bridge(value.module)?,
            profile: DestackProfileId::from_bridge(value.profile)?,
            component: DestackComponentId::from_bridge(value.component)?,
            entry: DestackModuleId::from_bridge(value.entry)?,
        })
    }

    /// Convert this C ABI value into one bridge value.
    pub(crate) fn to_bridge(&self) -> Result<destack::language::DirChecked, String> {
        Ok(destack::language::DirChecked {
            version: self.version.to_bridge()?,
            module: self.module.to_bridge()?,
            profile: self.profile.to_bridge()?,
            component: self.component.to_bridge()?,
            entry: self.entry.to_bridge()?,
        })
    }

    /// Destroy this C ABI value.
    pub(crate) fn destroy(&mut self) {
        self.version.destroy();
        self.module.destroy();
        self.profile.destroy();
        self.component.destroy();
        self.entry.destroy();
    }

    /// Return one empty C ABI value.
    pub(crate) fn empty() -> Self {
        Self {
            version: DestackArtifactVersion::empty(),
            module: DestackModuleId::empty(),
            profile: DestackProfileId::empty(),
            component: DestackComponentId::empty(),
            entry: DestackModuleId::empty(),
        }
    }
}

impl DestackDirCheckedArray {
    /// Convert bridge values into one C ABI array.
    pub(crate) fn from_bridge(values: Vec<destack::language::DirChecked>) -> Result<Self, String> {
        let mut converted = Vec::with_capacity(values.len());
        for value in values {
            converted.push(DestackDirChecked::from_bridge(value)?);
        }
        let (ptr, len) = owned_array(converted);
        Ok(Self { ptr, len })
    }

    /// Convert this C ABI array into bridge values.
    pub(crate) fn to_bridge(&self) -> Result<Vec<destack::language::DirChecked>, String> {
        if self.len == 0 {
            return Ok(Vec::new());
        }
        if self.ptr.is_null() {
            return Err("array pointer is null".to_string());
        }
        let values = unsafe { slice::from_raw_parts(self.ptr, self.len) };
        let mut converted = Vec::with_capacity(values.len());
        for value in values {
            converted.push(value.to_bridge()?);
        }
        Ok(converted)
    }

    /// Destroy this C ABI array.
    pub(crate) fn destroy(&mut self) {
        if self.ptr.is_null() {
            return;
        }
        unsafe {
            destroy_array(self.ptr, self.len, |value| value.destroy());
        }
        self.ptr = ptr::null_mut();
        self.len = 0;
    }
}

impl DestackOptionalDirChecked {
    /// Convert one optional bridge value into one C ABI optional value.
    pub(crate) fn from_bridge(
        value: Option<destack::language::DirChecked>,
    ) -> Result<Self, String> {
        let Some(value) = value else {
            return Ok(Self {
                is_some: false,
                value: DestackDirChecked::empty(),
            });
        };
        Ok(Self {
            is_some: true,
            value: DestackDirChecked::from_bridge(value)?,
        })
    }

    /// Convert this C ABI optional value into one bridge optional value.
    pub(crate) fn to_bridge(&self) -> Result<Option<destack::language::DirChecked>, String> {
        if self.is_some {
            Ok(Some(self.value.to_bridge()?))
        } else {
            Ok(None)
        }
    }

    /// Destroy this C ABI optional value.
    pub(crate) fn destroy(&mut self) {
        if self.is_some {
            self.value.destroy();
        }
        self.is_some = false;
        self.value = DestackDirChecked::empty();
    }
}

/// C ABI bridge value.
#[repr(C)]
#[derive(Debug)]
pub struct DestackDirParsed {
    /// Exact parsed artifact version.
    pub(crate) version: DestackArtifactVersion,
    /// Parsed module id.
    pub(crate) module: DestackModuleId,
}

/// C ABI bridge value array.
#[repr(C)]
#[derive(Debug)]
pub struct DestackDirParsedArray {
    /// Owned value pointer.
    pub(crate) ptr: *mut DestackDirParsed,
    /// Value count.
    pub(crate) len: usize,
}

/// C ABI optional bridge value.
#[repr(C)]
#[derive(Debug)]
pub struct DestackOptionalDirParsed {
    /// Whether the value is present.
    pub(crate) is_some: bool,
    /// Value when present.
    pub(crate) value: DestackDirParsed,
}

impl DestackDirParsed {
    /// Convert one bridge value into one C ABI value.
    pub(crate) fn from_bridge(value: destack::language::DirParsed) -> Result<Self, String> {
        Ok(Self {
            version: DestackArtifactVersion::from_bridge(value.version)?,
            module: DestackModuleId::from_bridge(value.module)?,
        })
    }

    /// Convert this C ABI value into one bridge value.
    pub(crate) fn to_bridge(&self) -> Result<destack::language::DirParsed, String> {
        Ok(destack::language::DirParsed {
            version: self.version.to_bridge()?,
            module: self.module.to_bridge()?,
        })
    }

    /// Destroy this C ABI value.
    pub(crate) fn destroy(&mut self) {
        self.version.destroy();
        self.module.destroy();
    }

    /// Return one empty C ABI value.
    pub(crate) fn empty() -> Self {
        Self {
            version: DestackArtifactVersion::empty(),
            module: DestackModuleId::empty(),
        }
    }
}

impl DestackDirParsedArray {
    /// Convert bridge values into one C ABI array.
    pub(crate) fn from_bridge(values: Vec<destack::language::DirParsed>) -> Result<Self, String> {
        let mut converted = Vec::with_capacity(values.len());
        for value in values {
            converted.push(DestackDirParsed::from_bridge(value)?);
        }
        let (ptr, len) = owned_array(converted);
        Ok(Self { ptr, len })
    }

    /// Convert this C ABI array into bridge values.
    pub(crate) fn to_bridge(&self) -> Result<Vec<destack::language::DirParsed>, String> {
        if self.len == 0 {
            return Ok(Vec::new());
        }
        if self.ptr.is_null() {
            return Err("array pointer is null".to_string());
        }
        let values = unsafe { slice::from_raw_parts(self.ptr, self.len) };
        let mut converted = Vec::with_capacity(values.len());
        for value in values {
            converted.push(value.to_bridge()?);
        }
        Ok(converted)
    }

    /// Destroy this C ABI array.
    pub(crate) fn destroy(&mut self) {
        if self.ptr.is_null() {
            return;
        }
        unsafe {
            destroy_array(self.ptr, self.len, |value| value.destroy());
        }
        self.ptr = ptr::null_mut();
        self.len = 0;
    }
}

impl DestackOptionalDirParsed {
    /// Convert one optional bridge value into one C ABI optional value.
    pub(crate) fn from_bridge(value: Option<destack::language::DirParsed>) -> Result<Self, String> {
        let Some(value) = value else {
            return Ok(Self {
                is_some: false,
                value: DestackDirParsed::empty(),
            });
        };
        Ok(Self {
            is_some: true,
            value: DestackDirParsed::from_bridge(value)?,
        })
    }

    /// Convert this C ABI optional value into one bridge optional value.
    pub(crate) fn to_bridge(&self) -> Result<Option<destack::language::DirParsed>, String> {
        if self.is_some {
            Ok(Some(self.value.to_bridge()?))
        } else {
            Ok(None)
        }
    }

    /// Destroy this C ABI optional value.
    pub(crate) fn destroy(&mut self) {
        if self.is_some {
            self.value.destroy();
        }
        self.is_some = false;
        self.value = DestackDirParsed::empty();
    }
}

/// C ABI bridge value.
#[repr(C)]
#[derive(Debug)]
pub struct DestackDirResolved {
    /// Exact resolved artifact version.
    pub(crate) version: DestackArtifactVersion,
    /// Resolved module id.
    pub(crate) module: DestackModuleId,
    /// Resolved semantic profile.
    pub(crate) profile: DestackProfileId,
}

/// C ABI bridge value array.
#[repr(C)]
#[derive(Debug)]
pub struct DestackDirResolvedArray {
    /// Owned value pointer.
    pub(crate) ptr: *mut DestackDirResolved,
    /// Value count.
    pub(crate) len: usize,
}

/// C ABI optional bridge value.
#[repr(C)]
#[derive(Debug)]
pub struct DestackOptionalDirResolved {
    /// Whether the value is present.
    pub(crate) is_some: bool,
    /// Value when present.
    pub(crate) value: DestackDirResolved,
}

impl DestackDirResolved {
    /// Convert one bridge value into one C ABI value.
    pub(crate) fn from_bridge(value: destack::language::DirResolved) -> Result<Self, String> {
        Ok(Self {
            version: DestackArtifactVersion::from_bridge(value.version)?,
            module: DestackModuleId::from_bridge(value.module)?,
            profile: DestackProfileId::from_bridge(value.profile)?,
        })
    }

    /// Convert this C ABI value into one bridge value.
    pub(crate) fn to_bridge(&self) -> Result<destack::language::DirResolved, String> {
        Ok(destack::language::DirResolved {
            version: self.version.to_bridge()?,
            module: self.module.to_bridge()?,
            profile: self.profile.to_bridge()?,
        })
    }

    /// Destroy this C ABI value.
    pub(crate) fn destroy(&mut self) {
        self.version.destroy();
        self.module.destroy();
        self.profile.destroy();
    }

    /// Return one empty C ABI value.
    pub(crate) fn empty() -> Self {
        Self {
            version: DestackArtifactVersion::empty(),
            module: DestackModuleId::empty(),
            profile: DestackProfileId::empty(),
        }
    }
}

impl DestackDirResolvedArray {
    /// Convert bridge values into one C ABI array.
    pub(crate) fn from_bridge(values: Vec<destack::language::DirResolved>) -> Result<Self, String> {
        let mut converted = Vec::with_capacity(values.len());
        for value in values {
            converted.push(DestackDirResolved::from_bridge(value)?);
        }
        let (ptr, len) = owned_array(converted);
        Ok(Self { ptr, len })
    }

    /// Convert this C ABI array into bridge values.
    pub(crate) fn to_bridge(&self) -> Result<Vec<destack::language::DirResolved>, String> {
        if self.len == 0 {
            return Ok(Vec::new());
        }
        if self.ptr.is_null() {
            return Err("array pointer is null".to_string());
        }
        let values = unsafe { slice::from_raw_parts(self.ptr, self.len) };
        let mut converted = Vec::with_capacity(values.len());
        for value in values {
            converted.push(value.to_bridge()?);
        }
        Ok(converted)
    }

    /// Destroy this C ABI array.
    pub(crate) fn destroy(&mut self) {
        if self.ptr.is_null() {
            return;
        }
        unsafe {
            destroy_array(self.ptr, self.len, |value| value.destroy());
        }
        self.ptr = ptr::null_mut();
        self.len = 0;
    }
}

impl DestackOptionalDirResolved {
    /// Convert one optional bridge value into one C ABI optional value.
    pub(crate) fn from_bridge(
        value: Option<destack::language::DirResolved>,
    ) -> Result<Self, String> {
        let Some(value) = value else {
            return Ok(Self {
                is_some: false,
                value: DestackDirResolved::empty(),
            });
        };
        Ok(Self {
            is_some: true,
            value: DestackDirResolved::from_bridge(value)?,
        })
    }

    /// Convert this C ABI optional value into one bridge optional value.
    pub(crate) fn to_bridge(&self) -> Result<Option<destack::language::DirResolved>, String> {
        if self.is_some {
            Ok(Some(self.value.to_bridge()?))
        } else {
            Ok(None)
        }
    }

    /// Destroy this C ABI optional value.
    pub(crate) fn destroy(&mut self) {
        if self.is_some {
            self.value.destroy();
        }
        self.is_some = false;
        self.value = DestackDirResolved::empty();
    }
}

/// C ABI bridge value.
#[repr(C)]
#[derive(Debug)]
pub struct DestackProductId {
    /// Owning package.
    pub(crate) package: DestackPackageId,
    /// Canonical lowercase hex product key within the package.
    pub(crate) key: *mut c_char,
}

/// C ABI bridge value array.
#[repr(C)]
#[derive(Debug)]
pub struct DestackProductIdArray {
    /// Owned value pointer.
    pub(crate) ptr: *mut DestackProductId,
    /// Value count.
    pub(crate) len: usize,
}

/// C ABI optional bridge value.
#[repr(C)]
#[derive(Debug)]
pub struct DestackOptionalProductId {
    /// Whether the value is present.
    pub(crate) is_some: bool,
    /// Value when present.
    pub(crate) value: DestackProductId,
}

impl DestackProductId {
    /// Convert one bridge value into one C ABI value.
    pub(crate) fn from_bridge(value: destack::language::ProductId) -> Result<Self, String> {
        Ok(Self {
            package: DestackPackageId::from_bridge(value.package)?,
            key: c_string(value.key)?,
        })
    }

    /// Convert this C ABI value into one bridge value.
    pub(crate) fn to_bridge(&self) -> Result<destack::language::ProductId, String> {
        Ok(destack::language::ProductId {
            package: self.package.to_bridge()?,
            key: read_string(self.key)?,
        })
    }

    /// Destroy this C ABI value.
    pub(crate) fn destroy(&mut self) {
        self.package.destroy();
        destroy_string(self.key);
        self.key = ptr::null_mut();
    }

    /// Return one empty C ABI value.
    pub(crate) fn empty() -> Self {
        Self {
            package: DestackPackageId::empty(),
            key: ptr::null_mut(),
        }
    }
}

impl DestackProductIdArray {
    /// Convert bridge values into one C ABI array.
    pub(crate) fn from_bridge(values: Vec<destack::language::ProductId>) -> Result<Self, String> {
        let mut converted = Vec::with_capacity(values.len());
        for value in values {
            converted.push(DestackProductId::from_bridge(value)?);
        }
        let (ptr, len) = owned_array(converted);
        Ok(Self { ptr, len })
    }

    /// Convert this C ABI array into bridge values.
    pub(crate) fn to_bridge(&self) -> Result<Vec<destack::language::ProductId>, String> {
        if self.len == 0 {
            return Ok(Vec::new());
        }
        if self.ptr.is_null() {
            return Err("array pointer is null".to_string());
        }
        let values = unsafe { slice::from_raw_parts(self.ptr, self.len) };
        let mut converted = Vec::with_capacity(values.len());
        for value in values {
            converted.push(value.to_bridge()?);
        }
        Ok(converted)
    }

    /// Destroy this C ABI array.
    pub(crate) fn destroy(&mut self) {
        if self.ptr.is_null() {
            return;
        }
        unsafe {
            destroy_array(self.ptr, self.len, |value| value.destroy());
        }
        self.ptr = ptr::null_mut();
        self.len = 0;
    }
}

impl DestackOptionalProductId {
    /// Convert one optional bridge value into one C ABI optional value.
    pub(crate) fn from_bridge(value: Option<destack::language::ProductId>) -> Result<Self, String> {
        let Some(value) = value else {
            return Ok(Self {
                is_some: false,
                value: DestackProductId::empty(),
            });
        };
        Ok(Self {
            is_some: true,
            value: DestackProductId::from_bridge(value)?,
        })
    }

    /// Convert this C ABI optional value into one bridge optional value.
    pub(crate) fn to_bridge(&self) -> Result<Option<destack::language::ProductId>, String> {
        if self.is_some {
            Ok(Some(self.value.to_bridge()?))
        } else {
            Ok(None)
        }
    }

    /// Destroy this C ABI optional value.
    pub(crate) fn destroy(&mut self) {
        if self.is_some {
            self.value.destroy();
        }
        self.is_some = false;
        self.value = DestackProductId::empty();
    }
}

/// C ABI bridge value.
#[repr(C)]
#[derive(Debug)]
pub struct DestackRevision {
    /// Displayed repository revision id.
    pub(crate) id: *mut c_char,
}

/// C ABI bridge value array.
#[repr(C)]
#[derive(Debug)]
pub struct DestackRevisionArray {
    /// Owned value pointer.
    pub(crate) ptr: *mut DestackRevision,
    /// Value count.
    pub(crate) len: usize,
}

/// C ABI optional bridge value.
#[repr(C)]
#[derive(Debug)]
pub struct DestackOptionalRevision {
    /// Whether the value is present.
    pub(crate) is_some: bool,
    /// Value when present.
    pub(crate) value: DestackRevision,
}

impl DestackRevision {
    /// Convert one bridge value into one C ABI value.
    pub(crate) fn from_bridge(value: destack::language::Revision) -> Result<Self, String> {
        Ok(Self {
            id: c_string(value.id)?,
        })
    }

    /// Convert this C ABI value into one bridge value.
    pub(crate) fn to_bridge(&self) -> Result<destack::language::Revision, String> {
        Ok(destack::language::Revision {
            id: read_string(self.id)?,
        })
    }

    /// Destroy this C ABI value.
    pub(crate) fn destroy(&mut self) {
        destroy_string(self.id);
        self.id = ptr::null_mut();
    }

    /// Return one empty C ABI value.
    pub(crate) fn empty() -> Self {
        Self {
            id: ptr::null_mut(),
        }
    }
}

impl DestackRevisionArray {
    /// Convert bridge values into one C ABI array.
    pub(crate) fn from_bridge(values: Vec<destack::language::Revision>) -> Result<Self, String> {
        let mut converted = Vec::with_capacity(values.len());
        for value in values {
            converted.push(DestackRevision::from_bridge(value)?);
        }
        let (ptr, len) = owned_array(converted);
        Ok(Self { ptr, len })
    }

    /// Convert this C ABI array into bridge values.
    pub(crate) fn to_bridge(&self) -> Result<Vec<destack::language::Revision>, String> {
        if self.len == 0 {
            return Ok(Vec::new());
        }
        if self.ptr.is_null() {
            return Err("array pointer is null".to_string());
        }
        let values = unsafe { slice::from_raw_parts(self.ptr, self.len) };
        let mut converted = Vec::with_capacity(values.len());
        for value in values {
            converted.push(value.to_bridge()?);
        }
        Ok(converted)
    }

    /// Destroy this C ABI array.
    pub(crate) fn destroy(&mut self) {
        if self.ptr.is_null() {
            return;
        }
        unsafe {
            destroy_array(self.ptr, self.len, |value| value.destroy());
        }
        self.ptr = ptr::null_mut();
        self.len = 0;
    }
}

impl DestackOptionalRevision {
    /// Convert one optional bridge value into one C ABI optional value.
    pub(crate) fn from_bridge(value: Option<destack::language::Revision>) -> Result<Self, String> {
        let Some(value) = value else {
            return Ok(Self {
                is_some: false,
                value: DestackRevision::empty(),
            });
        };
        Ok(Self {
            is_some: true,
            value: DestackRevision::from_bridge(value)?,
        })
    }

    /// Convert this C ABI optional value into one bridge optional value.
    pub(crate) fn to_bridge(&self) -> Result<Option<destack::language::Revision>, String> {
        if self.is_some {
            Ok(Some(self.value.to_bridge()?))
        } else {
            Ok(None)
        }
    }

    /// Destroy this C ABI optional value.
    pub(crate) fn destroy(&mut self) {
        if self.is_some {
            self.value.destroy();
        }
        self.is_some = false;
        self.value = DestackRevision::empty();
    }
}

/// C ABI bridge value.
#[repr(C)]
#[derive(Debug)]
pub struct DestackTargetId {
    /// Owning package.
    pub(crate) package: DestackPackageId,
    /// Canonical lowercase hex target key within the package.
    pub(crate) key: *mut c_char,
}

/// C ABI bridge value array.
#[repr(C)]
#[derive(Debug)]
pub struct DestackTargetIdArray {
    /// Owned value pointer.
    pub(crate) ptr: *mut DestackTargetId,
    /// Value count.
    pub(crate) len: usize,
}

/// C ABI optional bridge value.
#[repr(C)]
#[derive(Debug)]
pub struct DestackOptionalTargetId {
    /// Whether the value is present.
    pub(crate) is_some: bool,
    /// Value when present.
    pub(crate) value: DestackTargetId,
}

impl DestackTargetId {
    /// Convert one bridge value into one C ABI value.
    pub(crate) fn from_bridge(value: destack::language::TargetId) -> Result<Self, String> {
        Ok(Self {
            package: DestackPackageId::from_bridge(value.package)?,
            key: c_string(value.key)?,
        })
    }

    /// Convert this C ABI value into one bridge value.
    pub(crate) fn to_bridge(&self) -> Result<destack::language::TargetId, String> {
        Ok(destack::language::TargetId {
            package: self.package.to_bridge()?,
            key: read_string(self.key)?,
        })
    }

    /// Destroy this C ABI value.
    pub(crate) fn destroy(&mut self) {
        self.package.destroy();
        destroy_string(self.key);
        self.key = ptr::null_mut();
    }

    /// Return one empty C ABI value.
    pub(crate) fn empty() -> Self {
        Self {
            package: DestackPackageId::empty(),
            key: ptr::null_mut(),
        }
    }
}

impl DestackTargetIdArray {
    /// Convert bridge values into one C ABI array.
    pub(crate) fn from_bridge(values: Vec<destack::language::TargetId>) -> Result<Self, String> {
        let mut converted = Vec::with_capacity(values.len());
        for value in values {
            converted.push(DestackTargetId::from_bridge(value)?);
        }
        let (ptr, len) = owned_array(converted);
        Ok(Self { ptr, len })
    }

    /// Convert this C ABI array into bridge values.
    pub(crate) fn to_bridge(&self) -> Result<Vec<destack::language::TargetId>, String> {
        if self.len == 0 {
            return Ok(Vec::new());
        }
        if self.ptr.is_null() {
            return Err("array pointer is null".to_string());
        }
        let values = unsafe { slice::from_raw_parts(self.ptr, self.len) };
        let mut converted = Vec::with_capacity(values.len());
        for value in values {
            converted.push(value.to_bridge()?);
        }
        Ok(converted)
    }

    /// Destroy this C ABI array.
    pub(crate) fn destroy(&mut self) {
        if self.ptr.is_null() {
            return;
        }
        unsafe {
            destroy_array(self.ptr, self.len, |value| value.destroy());
        }
        self.ptr = ptr::null_mut();
        self.len = 0;
    }
}

impl DestackOptionalTargetId {
    /// Convert one optional bridge value into one C ABI optional value.
    pub(crate) fn from_bridge(value: Option<destack::language::TargetId>) -> Result<Self, String> {
        let Some(value) = value else {
            return Ok(Self {
                is_some: false,
                value: DestackTargetId::empty(),
            });
        };
        Ok(Self {
            is_some: true,
            value: DestackTargetId::from_bridge(value)?,
        })
    }

    /// Convert this C ABI optional value into one bridge optional value.
    pub(crate) fn to_bridge(&self) -> Result<Option<destack::language::TargetId>, String> {
        if self.is_some {
            Ok(Some(self.value.to_bridge()?))
        } else {
            Ok(None)
        }
    }

    /// Destroy this C ABI optional value.
    pub(crate) fn destroy(&mut self) {
        if self.is_some {
            self.value.destroy();
        }
        self.is_some = false;
        self.value = DestackTargetId::empty();
    }
}

/// Create one artifact key handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_artifact_key_build(
    target: DestackTargetId,
    out: *mut *mut DestackArtifactKey,
    error: *mut *mut DestackError,
) -> DestackStatus {
    return_status(error, || {
        let target = target.to_bridge()?;
        let value = destack::language::ArtifactKey::Build { target };
        let key = Box::into_raw(Box::new(DestackArtifactKey { value }));
        write_out(out, key, "artifact key output is null")
    })
}

/// Create one artifact key handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_artifact_key_dir_parsed(
    module: DestackModuleId,
    out: *mut *mut DestackArtifactKey,
    error: *mut *mut DestackError,
) -> DestackStatus {
    return_status(error, || {
        let module = module.to_bridge()?;
        let value = destack::language::ArtifactKey::DirParsed { module };
        let key = Box::into_raw(Box::new(DestackArtifactKey { value }));
        write_out(out, key, "artifact key output is null")
    })
}

/// Create one artifact key handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_artifact_key_data(
    module: DestackModuleId,
    out: *mut *mut DestackArtifactKey,
    error: *mut *mut DestackError,
) -> DestackStatus {
    return_status(error, || {
        let module = module.to_bridge()?;
        let value = destack::language::ArtifactKey::Data { module };
        let key = Box::into_raw(Box::new(DestackArtifactKey { value }));
        write_out(out, key, "artifact key output is null")
    })
}

/// Create one artifact key handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_artifact_key_global_environment(
    profile: DestackProfileId,
    out: *mut *mut DestackArtifactKey,
    error: *mut *mut DestackError,
) -> DestackStatus {
    return_status(error, || {
        let profile = profile.to_bridge()?;
        let value = destack::language::ArtifactKey::GlobalEnvironment { profile };
        let key = Box::into_raw(Box::new(DestackArtifactKey { value }));
        write_out(out, key, "artifact key output is null")
    })
}

/// Create one artifact key handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_artifact_key_package_index(
    profile: DestackProfileId,
    out: *mut *mut DestackArtifactKey,
    error: *mut *mut DestackError,
) -> DestackStatus {
    return_status(error, || {
        let profile = profile.to_bridge()?;
        let value = destack::language::ArtifactKey::PackageIndex { profile };
        let key = Box::into_raw(Box::new(DestackArtifactKey { value }));
        write_out(out, key, "artifact key output is null")
    })
}

/// Create one artifact key handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_artifact_key_component_graph(
    profile: DestackProfileId,
    out: *mut *mut DestackArtifactKey,
    error: *mut *mut DestackError,
) -> DestackStatus {
    return_status(error, || {
        let profile = profile.to_bridge()?;
        let value = destack::language::ArtifactKey::ComponentGraph { profile };
        let key = Box::into_raw(Box::new(DestackArtifactKey { value }));
        write_out(out, key, "artifact key output is null")
    })
}

/// Create one artifact key handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_artifact_key_program_analysis(
    profile: DestackProfileId,
    target: DestackTargetId,
    out: *mut *mut DestackArtifactKey,
    error: *mut *mut DestackError,
) -> DestackStatus {
    return_status(error, || {
        let profile = profile.to_bridge()?;
        let target = target.to_bridge()?;
        let value = destack::language::ArtifactKey::ProgramAnalysis { profile, target };
        let key = Box::into_raw(Box::new(DestackArtifactKey { value }));
        write_out(out, key, "artifact key output is null")
    })
}

/// Create one artifact key handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_artifact_key_dir_bound(
    module: DestackModuleId,
    profile: DestackProfileId,
    out: *mut *mut DestackArtifactKey,
    error: *mut *mut DestackError,
) -> DestackStatus {
    return_status(error, || {
        let module = module.to_bridge()?;
        let profile = profile.to_bridge()?;
        let value = destack::language::ArtifactKey::DirBound { module, profile };
        let key = Box::into_raw(Box::new(DestackArtifactKey { value }));
        write_out(out, key, "artifact key output is null")
    })
}

/// Create one artifact key handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_artifact_key_dir_imported(
    module: DestackModuleId,
    profile: DestackProfileId,
    out: *mut *mut DestackArtifactKey,
    error: *mut *mut DestackError,
) -> DestackStatus {
    return_status(error, || {
        let module = module.to_bridge()?;
        let profile = profile.to_bridge()?;
        let value = destack::language::ArtifactKey::DirImported { module, profile };
        let key = Box::into_raw(Box::new(DestackArtifactKey { value }));
        write_out(out, key, "artifact key output is null")
    })
}

/// Create one artifact key handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_artifact_key_dir_expanded(
    module: DestackModuleId,
    profile: DestackProfileId,
    out: *mut *mut DestackArtifactKey,
    error: *mut *mut DestackError,
) -> DestackStatus {
    return_status(error, || {
        let module = module.to_bridge()?;
        let profile = profile.to_bridge()?;
        let value = destack::language::ArtifactKey::DirExpanded { module, profile };
        let key = Box::into_raw(Box::new(DestackArtifactKey { value }));
        write_out(out, key, "artifact key output is null")
    })
}

/// Create one artifact key handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_artifact_key_dir_exported(
    module: DestackModuleId,
    profile: DestackProfileId,
    out: *mut *mut DestackArtifactKey,
    error: *mut *mut DestackError,
) -> DestackStatus {
    return_status(error, || {
        let module = module.to_bridge()?;
        let profile = profile.to_bridge()?;
        let value = destack::language::ArtifactKey::DirExported { module, profile };
        let key = Box::into_raw(Box::new(DestackArtifactKey { value }));
        write_out(out, key, "artifact key output is null")
    })
}

/// Create one artifact key handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_artifact_key_dir_resolved(
    module: DestackModuleId,
    profile: DestackProfileId,
    out: *mut *mut DestackArtifactKey,
    error: *mut *mut DestackError,
) -> DestackStatus {
    return_status(error, || {
        let module = module.to_bridge()?;
        let profile = profile.to_bridge()?;
        let value = destack::language::ArtifactKey::DirResolved { module, profile };
        let key = Box::into_raw(Box::new(DestackArtifactKey { value }));
        write_out(out, key, "artifact key output is null")
    })
}

/// Create one artifact key handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_artifact_key_dir_checked_component(
    entry: DestackModuleId,
    component: DestackComponentId,
    profile: DestackProfileId,
    out: *mut *mut DestackArtifactKey,
    error: *mut *mut DestackError,
) -> DestackStatus {
    return_status(error, || {
        let entry = entry.to_bridge()?;
        let component = component.to_bridge()?;
        let profile = profile.to_bridge()?;
        let value = destack::language::ArtifactKey::DirCheckedComponent {
            entry,
            component,
            profile,
        };
        let key = Box::into_raw(Box::new(DestackArtifactKey { value }));
        write_out(out, key, "artifact key output is null")
    })
}

/// Create one artifact key handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_artifact_key_dir_checked(
    module: DestackModuleId,
    profile: DestackProfileId,
    out: *mut *mut DestackArtifactKey,
    error: *mut *mut DestackError,
) -> DestackStatus {
    return_status(error, || {
        let module = module.to_bridge()?;
        let profile = profile.to_bridge()?;
        let value = destack::language::ArtifactKey::DirChecked { module, profile };
        let key = Box::into_raw(Box::new(DestackArtifactKey { value }));
        write_out(out, key, "artifact key output is null")
    })
}

/// Create one artifact key handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_artifact_key_dir_materialized(
    module: DestackModuleId,
    profile: DestackProfileId,
    out: *mut *mut DestackArtifactKey,
    error: *mut *mut DestackError,
) -> DestackStatus {
    return_status(error, || {
        let module = module.to_bridge()?;
        let profile = profile.to_bridge()?;
        let value = destack::language::ArtifactKey::DirMaterialized { module, profile };
        let key = Box::into_raw(Box::new(DestackArtifactKey { value }));
        write_out(out, key, "artifact key output is null")
    })
}

/// Create one artifact key handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_artifact_key_dir_elaborated(
    module: DestackModuleId,
    profile: DestackProfileId,
    out: *mut *mut DestackArtifactKey,
    error: *mut *mut DestackError,
) -> DestackStatus {
    return_status(error, || {
        let module = module.to_bridge()?;
        let profile = profile.to_bridge()?;
        let value = destack::language::ArtifactKey::DirElaborated { module, profile };
        let key = Box::into_raw(Box::new(DestackArtifactKey { value }));
        write_out(out, key, "artifact key output is null")
    })
}

/// Create one artifact key handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_artifact_key_mir_lowered(
    module: DestackModuleId,
    profile: DestackProfileId,
    target: DestackTargetId,
    out: *mut *mut DestackArtifactKey,
    error: *mut *mut DestackError,
) -> DestackStatus {
    return_status(error, || {
        let module = module.to_bridge()?;
        let profile = profile.to_bridge()?;
        let target = target.to_bridge()?;
        let value = destack::language::ArtifactKey::MirLowered {
            module,
            profile,
            target,
        };
        let key = Box::into_raw(Box::new(DestackArtifactKey { value }));
        write_out(out, key, "artifact key output is null")
    })
}

/// Create one artifact key handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_artifact_key_mir_verified(
    module: DestackModuleId,
    profile: DestackProfileId,
    target: DestackTargetId,
    out: *mut *mut DestackArtifactKey,
    error: *mut *mut DestackError,
) -> DestackStatus {
    return_status(error, || {
        let module = module.to_bridge()?;
        let profile = profile.to_bridge()?;
        let target = target.to_bridge()?;
        let value = destack::language::ArtifactKey::MirVerified {
            module,
            profile,
            target,
        };
        let key = Box::into_raw(Box::new(DestackArtifactKey { value }));
        write_out(out, key, "artifact key output is null")
    })
}

/// Create one artifact key handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_artifact_key_mir_analyzed(
    module: DestackModuleId,
    profile: DestackProfileId,
    target: DestackTargetId,
    out: *mut *mut DestackArtifactKey,
    error: *mut *mut DestackError,
) -> DestackStatus {
    return_status(error, || {
        let module = module.to_bridge()?;
        let profile = profile.to_bridge()?;
        let target = target.to_bridge()?;
        let value = destack::language::ArtifactKey::MirAnalyzed {
            module,
            profile,
            target,
        };
        let key = Box::into_raw(Box::new(DestackArtifactKey { value }));
        write_out(out, key, "artifact key output is null")
    })
}

/// Create one artifact key handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_artifact_key_mir_optimized(
    module: DestackModuleId,
    profile: DestackProfileId,
    target: DestackTargetId,
    out: *mut *mut DestackArtifactKey,
    error: *mut *mut DestackError,
) -> DestackStatus {
    return_status(error, || {
        let module = module.to_bridge()?;
        let profile = profile.to_bridge()?;
        let target = target.to_bridge()?;
        let value = destack::language::ArtifactKey::MirOptimized {
            module,
            profile,
            target,
        };
        let key = Box::into_raw(Box::new(DestackArtifactKey { value }));
        write_out(out, key, "artifact key output is null")
    })
}

/// Create one artifact key handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_artifact_key_module_query_index(
    module: DestackModuleId,
    profile: DestackProfileId,
    out: *mut *mut DestackArtifactKey,
    error: *mut *mut DestackError,
) -> DestackStatus {
    return_status(error, || {
        let module = module.to_bridge()?;
        let profile = profile.to_bridge()?;
        let value = destack::language::ArtifactKey::ModuleQueryIndex { module, profile };
        let key = Box::into_raw(Box::new(DestackArtifactKey { value }));
        write_out(out, key, "artifact key output is null")
    })
}

/// Create one artifact key handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_artifact_key_workspace_query_index(
    profile: DestackProfileId,
    out: *mut *mut DestackArtifactKey,
    error: *mut *mut DestackError,
) -> DestackStatus {
    return_status(error, || {
        let profile = profile.to_bridge()?;
        let value = destack::language::ArtifactKey::WorkspaceQueryIndex { profile };
        let key = Box::into_raw(Box::new(DestackArtifactKey { value }));
        write_out(out, key, "artifact key output is null")
    })
}

/// Create one artifact key handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_artifact_key_script(
    module: DestackModuleId,
    target: DestackTargetId,
    out: *mut *mut DestackArtifactKey,
    error: *mut *mut DestackError,
) -> DestackStatus {
    return_status(error, || {
        let module = module.to_bridge()?;
        let target = target.to_bridge()?;
        let value = destack::language::ArtifactKey::Script { module, target };
        let key = Box::into_raw(Box::new(DestackArtifactKey { value }));
        write_out(out, key, "artifact key output is null")
    })
}

/// Create one artifact key handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_artifact_key_object(
    module: DestackModuleId,
    target: DestackTargetId,
    out: *mut *mut DestackArtifactKey,
    error: *mut *mut DestackError,
) -> DestackStatus {
    return_status(error, || {
        let module = module.to_bridge()?;
        let target = target.to_bridge()?;
        let value = destack::language::ArtifactKey::Object { module, target };
        let key = Box::into_raw(Box::new(DestackArtifactKey { value }));
        write_out(out, key, "artifact key output is null")
    })
}

/// Create one artifact key handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_artifact_key_asset(
    module: DestackModuleId,
    target: DestackTargetId,
    out: *mut *mut DestackArtifactKey,
    error: *mut *mut DestackError,
) -> DestackStatus {
    return_status(error, || {
        let module = module.to_bridge()?;
        let target = target.to_bridge()?;
        let value = destack::language::ArtifactKey::Asset { module, target };
        let key = Box::into_raw(Box::new(DestackArtifactKey { value }));
        write_out(out, key, "artifact key output is null")
    })
}

/// Create one artifact key handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_artifact_key_bundle(
    package: DestackPackageId,
    target: DestackTargetId,
    out: *mut *mut DestackArtifactKey,
    error: *mut *mut DestackError,
) -> DestackStatus {
    return_status(error, || {
        let package = package.to_bridge()?;
        let target = target.to_bridge()?;
        let value = destack::language::ArtifactKey::Bundle { package, target };
        let key = Box::into_raw(Box::new(DestackArtifactKey { value }));
        write_out(out, key, "artifact key output is null")
    })
}

/// Create one artifact key handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_artifact_key_program(
    package: DestackPackageId,
    target: DestackTargetId,
    out: *mut *mut DestackArtifactKey,
    error: *mut *mut DestackError,
) -> DestackStatus {
    return_status(error, || {
        let package = package.to_bridge()?;
        let target = target.to_bridge()?;
        let value = destack::language::ArtifactKey::Program { package, target };
        let key = Box::into_raw(Box::new(DestackArtifactKey { value }));
        write_out(out, key, "artifact key output is null")
    })
}

/// Create one artifact key handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_artifact_key_product(
    package: DestackPackageId,
    product: DestackProductId,
    out: *mut *mut DestackArtifactKey,
    error: *mut *mut DestackError,
) -> DestackStatus {
    return_status(error, || {
        let package = package.to_bridge()?;
        let product = product.to_bridge()?;
        let value = destack::language::ArtifactKey::Product { package, product };
        let key = Box::into_raw(Box::new(DestackArtifactKey { value }));
        write_out(out, key, "artifact key output is null")
    })
}

/// Create one artifact key handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_artifact_key_module_linted(
    module: DestackModuleId,
    profile: DestackProfileId,
    out: *mut *mut DestackArtifactKey,
    error: *mut *mut DestackError,
) -> DestackStatus {
    return_status(error, || {
        let module = module.to_bridge()?;
        let profile = profile.to_bridge()?;
        let value = destack::language::ArtifactKey::ModuleLinted { module, profile };
        let key = Box::into_raw(Box::new(DestackArtifactKey { value }));
        write_out(out, key, "artifact key output is null")
    })
}

/// Create one artifact key handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_artifact_key_package_linted(
    package: DestackPackageId,
    out: *mut *mut DestackArtifactKey,
    error: *mut *mut DestackError,
) -> DestackStatus {
    return_status(error, || {
        let package = package.to_bridge()?;
        let value = destack::language::ArtifactKey::PackageLinted { package };
        let key = Box::into_raw(Box::new(DestackArtifactKey { value }));
        write_out(out, key, "artifact key output is null")
    })
}

/// Create one artifact key handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_artifact_key_workspace_linted(
    out: *mut *mut DestackArtifactKey,
    error: *mut *mut DestackError,
) -> DestackStatus {
    return_status(error, || {
        let value = destack::language::ArtifactKey::WorkspaceLinted;
        let key = Box::into_raw(Box::new(DestackArtifactKey { value }));
        write_out(out, key, "artifact key output is null")
    })
}

/// Destroy one owned byte array.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_byte_array_destroy(mut array: DestackByteArray) {
    array.destroy();
}

/// Destroy one owned string array.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_string_array_destroy(mut array: DestackStringArray) {
    array.destroy();
}

/// Destroy one optional owned string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_optional_string_destroy(mut value: DestackOptionalString) {
    value.destroy();
}

/// Destroy one artifact key handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_artifact_key_destroy(value: *mut DestackArtifactKey) {
    if value.is_null() {
        return;
    }
    drop(unsafe { Box::from_raw(value) });
}

/// Destroy one C ABI bridge value.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_applicability_destroy(value: *mut DestackApplicability) {
    if let Some(value) = unsafe { value.as_mut() } {
        value.destroy();
    }
}

/// Destroy one C ABI bridge value array.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_applicability_array_destroy(mut array: DestackApplicabilityArray) {
    array.destroy();
}

/// Destroy one C ABI bridge value.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_artifact_version_destroy(value: *mut DestackArtifactVersion) {
    if let Some(value) = unsafe { value.as_mut() } {
        value.destroy();
    }
}

/// Destroy one C ABI bridge value array.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_artifact_version_array_destroy(
    mut array: DestackArtifactVersionArray,
) {
    array.destroy();
}

/// Destroy one C ABI bridge value.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_content_id_destroy(value: *mut DestackContentId) {
    if let Some(value) = unsafe { value.as_mut() } {
        value.destroy();
    }
}

/// Destroy one C ABI bridge value array.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_content_id_array_destroy(mut array: DestackContentIdArray) {
    array.destroy();
}

/// Destroy one C ABI bridge value.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_file_id_destroy(value: *mut DestackFileId) {
    if let Some(value) = unsafe { value.as_mut() } {
        value.destroy();
    }
}

/// Destroy one C ABI bridge value array.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_file_id_array_destroy(mut array: DestackFileIdArray) {
    array.destroy();
}

/// Destroy one C ABI bridge value.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_artifact_dependency_destroy(
    value: *mut DestackArtifactDependency,
) {
    if let Some(value) = unsafe { value.as_mut() } {
        value.destroy();
    }
}

/// Destroy one C ABI bridge value array.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_artifact_dependency_array_destroy(
    mut array: DestackArtifactDependencyArray,
) {
    array.destroy();
}

/// Destroy one C ABI bridge value.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_artifact_sidecar_label_destroy(
    value: *mut DestackArtifactSidecarLabel,
) {
    if let Some(value) = unsafe { value.as_mut() } {
        value.destroy();
    }
}

/// Destroy one C ABI bridge value array.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_artifact_sidecar_label_array_destroy(
    mut array: DestackArtifactSidecarLabelArray,
) {
    array.destroy();
}

/// Destroy one C ABI bridge value.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_content_destroy(value: *mut DestackContent) {
    if let Some(value) = unsafe { value.as_mut() } {
        value.destroy();
    }
}

/// Destroy one C ABI bridge value array.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_content_array_destroy(mut array: DestackContentArray) {
    array.destroy();
}

/// Destroy one C ABI bridge value.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_artifact_sidecar_destroy(value: *mut DestackArtifactSidecar) {
    if let Some(value) = unsafe { value.as_mut() } {
        value.destroy();
    }
}

/// Destroy one C ABI bridge value array.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_artifact_sidecar_array_destroy(
    mut array: DestackArtifactSidecarArray,
) {
    array.destroy();
}

/// Destroy one C ABI bridge value.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_artifact_string_destroy(value: *mut DestackArtifactString) {
    if let Some(value) = unsafe { value.as_mut() } {
        value.destroy();
    }
}

/// Destroy one C ABI bridge value array.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_artifact_string_array_destroy(
    mut array: DestackArtifactStringArray,
) {
    array.destroy();
}

/// Destroy one C ABI bridge value.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_diagnostic_help_destroy(value: *mut DestackDiagnosticHelp) {
    if let Some(value) = unsafe { value.as_mut() } {
        value.destroy();
    }
}

/// Destroy one C ABI bridge value array.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_diagnostic_help_array_destroy(
    mut array: DestackDiagnosticHelpArray,
) {
    array.destroy();
}

/// Destroy one C ABI bridge value.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_span_destroy(value: *mut DestackSpan) {
    if let Some(value) = unsafe { value.as_mut() } {
        value.destroy();
    }
}

/// Destroy one C ABI bridge value array.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_span_array_destroy(mut array: DestackSpanArray) {
    array.destroy();
}

/// Destroy one C ABI bridge value.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_diagnostic_label_destroy(value: *mut DestackDiagnosticLabel) {
    if let Some(value) = unsafe { value.as_mut() } {
        value.destroy();
    }
}

/// Destroy one C ABI bridge value array.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_diagnostic_label_array_destroy(
    mut array: DestackDiagnosticLabelArray,
) {
    array.destroy();
}

/// Destroy one C ABI bridge value.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_diagnostic_note_destroy(value: *mut DestackDiagnosticNote) {
    if let Some(value) = unsafe { value.as_mut() } {
        value.destroy();
    }
}

/// Destroy one C ABI bridge value array.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_diagnostic_note_array_destroy(
    mut array: DestackDiagnosticNoteArray,
) {
    array.destroy();
}

/// Destroy one C ABI bridge value.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_diagnostic_severity_destroy(
    value: *mut DestackDiagnosticSeverity,
) {
    if let Some(value) = unsafe { value.as_mut() } {
        value.destroy();
    }
}

/// Destroy one C ABI bridge value array.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_diagnostic_severity_array_destroy(
    mut array: DestackDiagnosticSeverityArray,
) {
    array.destroy();
}

/// Destroy one C ABI bridge value.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_patch_destroy(value: *mut DestackPatch) {
    if let Some(value) = unsafe { value.as_mut() } {
        value.destroy();
    }
}

/// Destroy one C ABI bridge value array.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_patch_array_destroy(mut array: DestackPatchArray) {
    array.destroy();
}

/// Destroy one C ABI bridge value.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_file_patch_destroy(value: *mut DestackFilePatch) {
    if let Some(value) = unsafe { value.as_mut() } {
        value.destroy();
    }
}

/// Destroy one C ABI bridge value array.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_file_patch_array_destroy(mut array: DestackFilePatchArray) {
    array.destroy();
}

/// Destroy one C ABI bridge value.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_patch_set_destroy(value: *mut DestackPatchSet) {
    if let Some(value) = unsafe { value.as_mut() } {
        value.destroy();
    }
}

/// Destroy one C ABI bridge value array.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_patch_set_array_destroy(mut array: DestackPatchSetArray) {
    array.destroy();
}

/// Destroy one C ABI bridge value.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_diagnostic_suggestion_destroy(
    value: *mut DestackDiagnosticSuggestion,
) {
    if let Some(value) = unsafe { value.as_mut() } {
        value.destroy();
    }
}

/// Destroy one C ABI bridge value array.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_diagnostic_suggestion_array_destroy(
    mut array: DestackDiagnosticSuggestionArray,
) {
    array.destroy();
}

/// Destroy one C ABI bridge value.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_diagnostic_tag_destroy(value: *mut DestackDiagnosticTag) {
    if let Some(value) = unsafe { value.as_mut() } {
        value.destroy();
    }
}

/// Destroy one C ABI bridge value array.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_diagnostic_tag_array_destroy(
    mut array: DestackDiagnosticTagArray,
) {
    array.destroy();
}

/// Destroy one C ABI bridge value.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_diagnostic_destroy(value: *mut DestackDiagnostic) {
    if let Some(value) = unsafe { value.as_mut() } {
        value.destroy();
    }
}

/// Destroy one C ABI bridge value array.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_diagnostic_array_destroy(mut array: DestackDiagnosticArray) {
    array.destroy();
}

/// Destroy one C ABI bridge value.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_artifact_record_destroy(value: *mut DestackArtifactRecord) {
    if let Some(value) = unsafe { value.as_mut() } {
        value.destroy();
    }
}

/// Destroy one C ABI bridge value array.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_artifact_record_array_destroy(
    mut array: DestackArtifactRecordArray,
) {
    array.destroy();
}

/// Destroy one C ABI bridge value.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_component_id_destroy(value: *mut DestackComponentId) {
    if let Some(value) = unsafe { value.as_mut() } {
        value.destroy();
    }
}

/// Destroy one C ABI bridge value array.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_component_id_array_destroy(mut array: DestackComponentIdArray) {
    array.destroy();
}

/// Destroy one C ABI bridge value.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_package_id_destroy(value: *mut DestackPackageId) {
    if let Some(value) = unsafe { value.as_mut() } {
        value.destroy();
    }
}

/// Destroy one C ABI bridge value array.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_package_id_array_destroy(mut array: DestackPackageIdArray) {
    array.destroy();
}

/// Destroy one C ABI bridge value.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_module_id_destroy(value: *mut DestackModuleId) {
    if let Some(value) = unsafe { value.as_mut() } {
        value.destroy();
    }
}

/// Destroy one C ABI bridge value array.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_module_id_array_destroy(mut array: DestackModuleIdArray) {
    array.destroy();
}

/// Destroy one C ABI bridge value.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_profile_id_destroy(value: *mut DestackProfileId) {
    if let Some(value) = unsafe { value.as_mut() } {
        value.destroy();
    }
}

/// Destroy one C ABI bridge value array.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_profile_id_array_destroy(mut array: DestackProfileIdArray) {
    array.destroy();
}

/// Destroy one C ABI bridge value.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_dir_checked_destroy(value: *mut DestackDirChecked) {
    if let Some(value) = unsafe { value.as_mut() } {
        value.destroy();
    }
}

/// Destroy one C ABI bridge value array.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_dir_checked_array_destroy(mut array: DestackDirCheckedArray) {
    array.destroy();
}

/// Destroy one C ABI bridge value.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_dir_parsed_destroy(value: *mut DestackDirParsed) {
    if let Some(value) = unsafe { value.as_mut() } {
        value.destroy();
    }
}

/// Destroy one C ABI bridge value array.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_dir_parsed_array_destroy(mut array: DestackDirParsedArray) {
    array.destroy();
}

/// Destroy one C ABI bridge value.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_dir_resolved_destroy(value: *mut DestackDirResolved) {
    if let Some(value) = unsafe { value.as_mut() } {
        value.destroy();
    }
}

/// Destroy one C ABI bridge value array.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_dir_resolved_array_destroy(mut array: DestackDirResolvedArray) {
    array.destroy();
}

/// Destroy one C ABI bridge value.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_product_id_destroy(value: *mut DestackProductId) {
    if let Some(value) = unsafe { value.as_mut() } {
        value.destroy();
    }
}

/// Destroy one C ABI bridge value array.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_product_id_array_destroy(mut array: DestackProductIdArray) {
    array.destroy();
}

/// Destroy one C ABI bridge value.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_revision_destroy(value: *mut DestackRevision) {
    if let Some(value) = unsafe { value.as_mut() } {
        value.destroy();
    }
}

/// Destroy one C ABI bridge value array.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_revision_array_destroy(mut array: DestackRevisionArray) {
    array.destroy();
}

/// Destroy one C ABI bridge value.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_target_id_destroy(value: *mut DestackTargetId) {
    if let Some(value) = unsafe { value.as_mut() } {
        value.destroy();
    }
}

/// Destroy one C ABI bridge value array.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_target_id_array_destroy(mut array: DestackTargetIdArray) {
    array.destroy();
}
