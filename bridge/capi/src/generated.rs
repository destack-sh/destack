// generated bridge target, do not edit

use std::ffi::c_char;
use std::ptr;

use destack as rust;

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
        let values = unsafe { std::slice::from_raw_parts(self.ptr, self.len) };
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
    pub(crate) value: rust::ArtifactKey,
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
    pub(crate) fn from_bridge(value: rust::Applicability) -> Result<Self, String> {
        Ok(match value {
            rust::Applicability::Automatic => Self::Automatic,
            rust::Applicability::Unsafe => Self::Unsafe,
            rust::Applicability::Dangerous => Self::Dangerous,
        })
    }

    /// Convert this C ABI enum into one bridge enum.
    pub(crate) fn to_bridge(&self) -> Result<rust::Applicability, String> {
        Ok(match *self {
            Self::Automatic => rust::Applicability::Automatic,
            Self::Unsafe => rust::Applicability::Unsafe,
            Self::Dangerous => rust::Applicability::Dangerous,
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
    pub(crate) fn from_bridge(values: Vec<rust::Applicability>) -> Result<Self, String> {
        let mut converted = Vec::with_capacity(values.len());
        for value in values {
            converted.push(DestackApplicability::from_bridge(value)?);
        }
        let (ptr, len) = owned_array(converted);
        Ok(Self { ptr, len })
    }

    /// Convert this C ABI array into bridge values.
    pub(crate) fn to_bridge(&self) -> Result<Vec<rust::Applicability>, String> {
        if self.len == 0 {
            return Ok(Vec::new());
        }
        if self.ptr.is_null() {
            return Err("array pointer is null".to_string());
        }
        let values = unsafe { std::slice::from_raw_parts(self.ptr, self.len) };
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
    pub(crate) fn from_bridge(value: Option<rust::Applicability>) -> Result<Self, String> {
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
    pub(crate) fn to_bridge(&self) -> Result<Option<rust::Applicability>, String> {
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

/// C ABI bridge enum.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DestackArtifactPathState {
    /// The path did not exist.
    Missing = 0,
    /// The path was a regular file.
    File = 1,
    /// The path was a directory.
    Directory = 2,
    /// The path was a symbolic link.
    Symlink = 3,
    /// The path existed with another host-specific kind.
    Other = 4,
}

/// C ABI bridge enum array.
#[repr(C)]
#[derive(Debug)]
pub struct DestackArtifactPathStateArray {
    /// Owned value pointer.
    pub(crate) ptr: *mut DestackArtifactPathState,
    /// Value count.
    pub(crate) len: usize,
}

/// C ABI optional bridge enum.
#[repr(C)]
#[derive(Debug)]
pub struct DestackOptionalArtifactPathState {
    /// Whether the value is present.
    pub(crate) is_some: bool,
    /// Value when present.
    pub(crate) value: DestackArtifactPathState,
}

impl DestackArtifactPathState {
    /// Convert one bridge enum into one C ABI enum.
    pub(crate) fn from_bridge(value: rust::ArtifactPathState) -> Result<Self, String> {
        Ok(match value {
            rust::ArtifactPathState::Missing => Self::Missing,
            rust::ArtifactPathState::File => Self::File,
            rust::ArtifactPathState::Directory => Self::Directory,
            rust::ArtifactPathState::Symlink => Self::Symlink,
            rust::ArtifactPathState::Other => Self::Other,
        })
    }

    /// Convert this C ABI enum into one bridge enum.
    pub(crate) fn to_bridge(&self) -> Result<rust::ArtifactPathState, String> {
        Ok(match *self {
            Self::Missing => rust::ArtifactPathState::Missing,
            Self::File => rust::ArtifactPathState::File,
            Self::Directory => rust::ArtifactPathState::Directory,
            Self::Symlink => rust::ArtifactPathState::Symlink,
            Self::Other => rust::ArtifactPathState::Other,
        })
    }

    /// Destroy this C ABI enum.
    pub(crate) fn destroy(&mut self) {}
    /// Return one empty C ABI enum.
    pub(crate) fn empty() -> Self {
        Self::Missing
    }
}

impl DestackArtifactPathStateArray {
    /// Convert bridge values into one C ABI array.
    pub(crate) fn from_bridge(values: Vec<rust::ArtifactPathState>) -> Result<Self, String> {
        let mut converted = Vec::with_capacity(values.len());
        for value in values {
            converted.push(DestackArtifactPathState::from_bridge(value)?);
        }
        let (ptr, len) = owned_array(converted);
        Ok(Self { ptr, len })
    }

    /// Convert this C ABI array into bridge values.
    pub(crate) fn to_bridge(&self) -> Result<Vec<rust::ArtifactPathState>, String> {
        if self.len == 0 {
            return Ok(Vec::new());
        }
        if self.ptr.is_null() {
            return Err("array pointer is null".to_string());
        }
        let values = unsafe { std::slice::from_raw_parts(self.ptr, self.len) };
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

impl DestackOptionalArtifactPathState {
    /// Convert one optional bridge value into one C ABI optional value.
    pub(crate) fn from_bridge(value: Option<rust::ArtifactPathState>) -> Result<Self, String> {
        let Some(value) = value else {
            return Ok(Self {
                is_some: false,
                value: DestackArtifactPathState::empty(),
            });
        };
        Ok(Self {
            is_some: true,
            value: DestackArtifactPathState::from_bridge(value)?,
        })
    }

    /// Convert this C ABI optional value into one bridge optional value.
    pub(crate) fn to_bridge(&self) -> Result<Option<rust::ArtifactPathState>, String> {
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
        self.value = DestackArtifactPathState::empty();
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
    pub(crate) fn from_bridge(value: rust::FileId) -> Result<Self, String> {
        Ok(Self {
            id: c_string(value.id)?,
        })
    }

    /// Convert this C ABI value into one bridge value.
    pub(crate) fn to_bridge(&self) -> Result<rust::FileId, String> {
        Ok(rust::FileId {
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
    pub(crate) fn from_bridge(values: Vec<rust::FileId>) -> Result<Self, String> {
        let mut converted = Vec::with_capacity(values.len());
        for value in values {
            converted.push(DestackFileId::from_bridge(value)?);
        }
        let (ptr, len) = owned_array(converted);
        Ok(Self { ptr, len })
    }

    /// Convert this C ABI array into bridge values.
    pub(crate) fn to_bridge(&self) -> Result<Vec<rust::FileId>, String> {
        if self.len == 0 {
            return Ok(Vec::new());
        }
        if self.ptr.is_null() {
            return Err("array pointer is null".to_string());
        }
        let values = unsafe { std::slice::from_raw_parts(self.ptr, self.len) };
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
    pub(crate) fn from_bridge(value: Option<rust::FileId>) -> Result<Self, String> {
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
    pub(crate) fn to_bridge(&self) -> Result<Option<rust::FileId>, String> {
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

/// C ABI bridge value.
#[repr(C)]
#[derive(Debug)]
pub struct DestackArtifactDirectoryEntry {
    /// The entry path identity.
    pub(crate) path: DestackFileId,
    /// The exact entry path state.
    pub(crate) state: DestackArtifactPathState,
}

/// C ABI bridge value array.
#[repr(C)]
#[derive(Debug)]
pub struct DestackArtifactDirectoryEntryArray {
    /// Owned value pointer.
    pub(crate) ptr: *mut DestackArtifactDirectoryEntry,
    /// Value count.
    pub(crate) len: usize,
}

/// C ABI optional bridge value.
#[repr(C)]
#[derive(Debug)]
pub struct DestackOptionalArtifactDirectoryEntry {
    /// Whether the value is present.
    pub(crate) is_some: bool,
    /// Value when present.
    pub(crate) value: DestackArtifactDirectoryEntry,
}

impl DestackArtifactDirectoryEntry {
    /// Convert one bridge value into one C ABI value.
    pub(crate) fn from_bridge(value: rust::ArtifactDirectoryEntry) -> Result<Self, String> {
        Ok(Self {
            path: DestackFileId::from_bridge(value.path)?,
            state: DestackArtifactPathState::from_bridge(value.state)?,
        })
    }

    /// Convert this C ABI value into one bridge value.
    pub(crate) fn to_bridge(&self) -> Result<rust::ArtifactDirectoryEntry, String> {
        Ok(rust::ArtifactDirectoryEntry {
            path: self.path.to_bridge()?,
            state: self.state.to_bridge()?,
        })
    }

    /// Destroy this C ABI value.
    pub(crate) fn destroy(&mut self) {
        self.path.destroy();
        self.state.destroy();
    }

    /// Return one empty C ABI value.
    pub(crate) fn empty() -> Self {
        Self {
            path: DestackFileId::empty(),
            state: DestackArtifactPathState::empty(),
        }
    }
}

impl DestackArtifactDirectoryEntryArray {
    /// Convert bridge values into one C ABI array.
    pub(crate) fn from_bridge(values: Vec<rust::ArtifactDirectoryEntry>) -> Result<Self, String> {
        let mut converted = Vec::with_capacity(values.len());
        for value in values {
            converted.push(DestackArtifactDirectoryEntry::from_bridge(value)?);
        }
        let (ptr, len) = owned_array(converted);
        Ok(Self { ptr, len })
    }

    /// Convert this C ABI array into bridge values.
    pub(crate) fn to_bridge(&self) -> Result<Vec<rust::ArtifactDirectoryEntry>, String> {
        if self.len == 0 {
            return Ok(Vec::new());
        }
        if self.ptr.is_null() {
            return Err("array pointer is null".to_string());
        }
        let values = unsafe { std::slice::from_raw_parts(self.ptr, self.len) };
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

impl DestackOptionalArtifactDirectoryEntry {
    /// Convert one optional bridge value into one C ABI optional value.
    pub(crate) fn from_bridge(value: Option<rust::ArtifactDirectoryEntry>) -> Result<Self, String> {
        let Some(value) = value else {
            return Ok(Self {
                is_some: false,
                value: DestackArtifactDirectoryEntry::empty(),
            });
        };
        Ok(Self {
            is_some: true,
            value: DestackArtifactDirectoryEntry::from_bridge(value)?,
        })
    }

    /// Convert this C ABI optional value into one bridge optional value.
    pub(crate) fn to_bridge(&self) -> Result<Option<rust::ArtifactDirectoryEntry>, String> {
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
        self.value = DestackArtifactDirectoryEntry::empty();
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
    pub(crate) fn from_bridge(value: rust::ContentId) -> Result<Self, String> {
        Ok(Self {
            id: c_string(value.id)?,
        })
    }

    /// Convert this C ABI value into one bridge value.
    pub(crate) fn to_bridge(&self) -> Result<rust::ContentId, String> {
        Ok(rust::ContentId {
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
    pub(crate) fn from_bridge(values: Vec<rust::ContentId>) -> Result<Self, String> {
        let mut converted = Vec::with_capacity(values.len());
        for value in values {
            converted.push(DestackContentId::from_bridge(value)?);
        }
        let (ptr, len) = owned_array(converted);
        Ok(Self { ptr, len })
    }

    /// Convert this C ABI array into bridge values.
    pub(crate) fn to_bridge(&self) -> Result<Vec<rust::ContentId>, String> {
        if self.len == 0 {
            return Ok(Vec::new());
        }
        if self.ptr.is_null() {
            return Err("array pointer is null".to_string());
        }
        let values = unsafe { std::slice::from_raw_parts(self.ptr, self.len) };
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
    pub(crate) fn from_bridge(value: Option<rust::ContentId>) -> Result<Self, String> {
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
    pub(crate) fn to_bridge(&self) -> Result<Option<rust::ContentId>, String> {
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

/// C ABI bridge enum kind.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DestackArtifactSourceDependencyKind {
    /// The exact state observed for one source path.
    PathState = 0,
    /// The exact direct entries observed for one directory.
    DirectoryEntries = 1,
    /// The exact source content read for one file.
    FileContent = 2,
}

/// C ABI bridge enum.
#[repr(C)]
#[derive(Debug)]
pub struct DestackArtifactSourceDependency {
    /// Active enum variant.
    pub(crate) kind: DestackArtifactSourceDependencyKind,
    pub(crate) path: DestackFileId,
    pub(crate) state: DestackArtifactPathState,
    pub(crate) directory: DestackFileId,
    pub(crate) entries: DestackArtifactDirectoryEntryArray,
    pub(crate) file: DestackFileId,
    pub(crate) content: DestackContentId,
}

/// C ABI bridge enum array.
#[repr(C)]
#[derive(Debug)]
pub struct DestackArtifactSourceDependencyArray {
    /// Owned value pointer.
    pub(crate) ptr: *mut DestackArtifactSourceDependency,
    /// Value count.
    pub(crate) len: usize,
}

/// C ABI optional bridge enum.
#[repr(C)]
#[derive(Debug)]
pub struct DestackOptionalArtifactSourceDependency {
    /// Whether the value is present.
    pub(crate) is_some: bool,
    /// Value when present.
    pub(crate) value: DestackArtifactSourceDependency,
}

impl DestackArtifactSourceDependency {
    /// Convert one bridge enum into one C ABI enum.
    pub(crate) fn from_bridge(value: rust::ArtifactSourceDependency) -> Result<Self, String> {
        Ok(match value {
            rust::ArtifactSourceDependency::PathState { path, state } => Self {
                kind: DestackArtifactSourceDependencyKind::PathState,
                path: DestackFileId::from_bridge(path)?,
                state: DestackArtifactPathState::from_bridge(state)?,
                directory: DestackFileId::empty(),
                entries: DestackArtifactDirectoryEntryArray {
                    ptr: ptr::null_mut(),
                    len: 0,
                },
                file: DestackFileId::empty(),
                content: DestackContentId::empty(),
            },
            rust::ArtifactSourceDependency::DirectoryEntries { directory, entries } => Self {
                kind: DestackArtifactSourceDependencyKind::DirectoryEntries,
                path: DestackFileId::empty(),
                state: DestackArtifactPathState::empty(),
                directory: DestackFileId::from_bridge(directory)?,
                entries: DestackArtifactDirectoryEntryArray::from_bridge(entries)?,
                file: DestackFileId::empty(),
                content: DestackContentId::empty(),
            },
            rust::ArtifactSourceDependency::FileContent { file, content } => Self {
                kind: DestackArtifactSourceDependencyKind::FileContent,
                path: DestackFileId::empty(),
                state: DestackArtifactPathState::empty(),
                directory: DestackFileId::empty(),
                entries: DestackArtifactDirectoryEntryArray {
                    ptr: ptr::null_mut(),
                    len: 0,
                },
                file: DestackFileId::from_bridge(file)?,
                content: DestackContentId::from_bridge(content)?,
            },
        })
    }

    /// Convert this C ABI enum into one bridge enum.
    pub(crate) fn to_bridge(&self) -> Result<rust::ArtifactSourceDependency, String> {
        match self.kind {
            DestackArtifactSourceDependencyKind::PathState => {
                let path = self.path.to_bridge()?;
                let state = self.state.to_bridge()?;
                Ok(rust::ArtifactSourceDependency::PathState {
                    path: path,
                    state: state,
                })
            }
            DestackArtifactSourceDependencyKind::DirectoryEntries => {
                let directory = self.directory.to_bridge()?;
                let entries = self.entries.to_bridge()?;
                Ok(rust::ArtifactSourceDependency::DirectoryEntries {
                    directory: directory,
                    entries: entries,
                })
            }
            DestackArtifactSourceDependencyKind::FileContent => {
                let file = self.file.to_bridge()?;
                let content = self.content.to_bridge()?;
                Ok(rust::ArtifactSourceDependency::FileContent {
                    file: file,
                    content: content,
                })
            }
        }
    }

    /// Destroy this C ABI enum.
    pub(crate) fn destroy(&mut self) {
        self.path.destroy();
        self.state.destroy();
        self.directory.destroy();
        self.entries.destroy();
        self.file.destroy();
        self.content.destroy();
    }

    /// Return one empty C ABI enum.
    pub(crate) fn empty() -> Self {
        Self {
            kind: DestackArtifactSourceDependencyKind::PathState,
            path: DestackFileId::empty(),
            state: DestackArtifactPathState::empty(),
            directory: DestackFileId::empty(),
            entries: DestackArtifactDirectoryEntryArray {
                ptr: ptr::null_mut(),
                len: 0,
            },
            file: DestackFileId::empty(),
            content: DestackContentId::empty(),
        }
    }
}

impl DestackArtifactSourceDependencyArray {
    /// Convert bridge values into one C ABI array.
    pub(crate) fn from_bridge(values: Vec<rust::ArtifactSourceDependency>) -> Result<Self, String> {
        let mut converted = Vec::with_capacity(values.len());
        for value in values {
            converted.push(DestackArtifactSourceDependency::from_bridge(value)?);
        }
        let (ptr, len) = owned_array(converted);
        Ok(Self { ptr, len })
    }

    /// Convert this C ABI array into bridge values.
    pub(crate) fn to_bridge(&self) -> Result<Vec<rust::ArtifactSourceDependency>, String> {
        if self.len == 0 {
            return Ok(Vec::new());
        }
        if self.ptr.is_null() {
            return Err("array pointer is null".to_string());
        }
        let values = unsafe { std::slice::from_raw_parts(self.ptr, self.len) };
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

impl DestackOptionalArtifactSourceDependency {
    /// Convert one optional bridge value into one C ABI optional value.
    pub(crate) fn from_bridge(
        value: Option<rust::ArtifactSourceDependency>,
    ) -> Result<Self, String> {
        let Some(value) = value else {
            return Ok(Self {
                is_some: false,
                value: DestackArtifactSourceDependency::empty(),
            });
        };
        Ok(Self {
            is_some: true,
            value: DestackArtifactSourceDependency::from_bridge(value)?,
        })
    }

    /// Convert this C ABI optional value into one bridge optional value.
    pub(crate) fn to_bridge(&self) -> Result<Option<rust::ArtifactSourceDependency>, String> {
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
        self.value = DestackArtifactSourceDependency::empty();
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
    pub(crate) fn from_bridge(value: rust::ArtifactVersion) -> Result<Self, String> {
        Ok(Self {
            key: Box::into_raw(Box::new(DestackArtifactKey { value: value.key })),
            fingerprint: c_string(value.fingerprint)?,
        })
    }

    /// Convert this C ABI value into one bridge value.
    pub(crate) fn to_bridge(&self) -> Result<rust::ArtifactVersion, String> {
        Ok(rust::ArtifactVersion {
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
    pub(crate) fn from_bridge(values: Vec<rust::ArtifactVersion>) -> Result<Self, String> {
        let mut converted = Vec::with_capacity(values.len());
        for value in values {
            converted.push(DestackArtifactVersion::from_bridge(value)?);
        }
        let (ptr, len) = owned_array(converted);
        Ok(Self { ptr, len })
    }

    /// Convert this C ABI array into bridge values.
    pub(crate) fn to_bridge(&self) -> Result<Vec<rust::ArtifactVersion>, String> {
        if self.len == 0 {
            return Ok(Vec::new());
        }
        if self.ptr.is_null() {
            return Err("array pointer is null".to_string());
        }
        let values = unsafe { std::slice::from_raw_parts(self.ptr, self.len) };
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
    pub(crate) fn from_bridge(value: Option<rust::ArtifactVersion>) -> Result<Self, String> {
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
    pub(crate) fn to_bridge(&self) -> Result<Option<rust::ArtifactVersion>, String> {
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
    pub(crate) dependency: DestackArtifactSourceDependency,
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
    pub(crate) fn from_bridge(value: rust::ArtifactDependency) -> Result<Self, String> {
        Ok(match value {
            rust::ArtifactDependency::Artifact { version } => Self {
                kind: DestackArtifactDependencyKind::Artifact,
                version: DestackArtifactVersion::from_bridge(version)?,
                dependency: DestackArtifactSourceDependency::empty(),
            },
            rust::ArtifactDependency::Source { dependency } => Self {
                kind: DestackArtifactDependencyKind::Source,
                version: DestackArtifactVersion::empty(),
                dependency: DestackArtifactSourceDependency::from_bridge(dependency)?,
            },
        })
    }

    /// Convert this C ABI enum into one bridge enum.
    pub(crate) fn to_bridge(&self) -> Result<rust::ArtifactDependency, String> {
        match self.kind {
            DestackArtifactDependencyKind::Artifact => {
                let version = self.version.to_bridge()?;
                Ok(rust::ArtifactDependency::Artifact { version: version })
            }
            DestackArtifactDependencyKind::Source => {
                let dependency = self.dependency.to_bridge()?;
                Ok(rust::ArtifactDependency::Source {
                    dependency: dependency,
                })
            }
        }
    }

    /// Destroy this C ABI enum.
    pub(crate) fn destroy(&mut self) {
        self.version.destroy();
        self.dependency.destroy();
    }

    /// Return one empty C ABI enum.
    pub(crate) fn empty() -> Self {
        Self {
            kind: DestackArtifactDependencyKind::Artifact,
            version: DestackArtifactVersion::empty(),
            dependency: DestackArtifactSourceDependency::empty(),
        }
    }
}

impl DestackArtifactDependencyArray {
    /// Convert bridge values into one C ABI array.
    pub(crate) fn from_bridge(values: Vec<rust::ArtifactDependency>) -> Result<Self, String> {
        let mut converted = Vec::with_capacity(values.len());
        for value in values {
            converted.push(DestackArtifactDependency::from_bridge(value)?);
        }
        let (ptr, len) = owned_array(converted);
        Ok(Self { ptr, len })
    }

    /// Convert this C ABI array into bridge values.
    pub(crate) fn to_bridge(&self) -> Result<Vec<rust::ArtifactDependency>, String> {
        if self.len == 0 {
            return Ok(Vec::new());
        }
        if self.ptr.is_null() {
            return Err("array pointer is null".to_string());
        }
        let values = unsafe { std::slice::from_raw_parts(self.ptr, self.len) };
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
    pub(crate) fn from_bridge(value: Option<rust::ArtifactDependency>) -> Result<Self, String> {
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
    pub(crate) fn to_bridge(&self) -> Result<Option<rust::ArtifactDependency>, String> {
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
    pub(crate) fn from_bridge(value: rust::ArtifactSidecarLabel) -> Result<Self, String> {
        Ok(Self {
            key: c_string(value.key)?,
            value: c_string(value.value)?,
        })
    }

    /// Convert this C ABI value into one bridge value.
    pub(crate) fn to_bridge(&self) -> Result<rust::ArtifactSidecarLabel, String> {
        Ok(rust::ArtifactSidecarLabel {
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
    pub(crate) fn from_bridge(values: Vec<rust::ArtifactSidecarLabel>) -> Result<Self, String> {
        let mut converted = Vec::with_capacity(values.len());
        for value in values {
            converted.push(DestackArtifactSidecarLabel::from_bridge(value)?);
        }
        let (ptr, len) = owned_array(converted);
        Ok(Self { ptr, len })
    }

    /// Convert this C ABI array into bridge values.
    pub(crate) fn to_bridge(&self) -> Result<Vec<rust::ArtifactSidecarLabel>, String> {
        if self.len == 0 {
            return Ok(Vec::new());
        }
        if self.ptr.is_null() {
            return Err("array pointer is null".to_string());
        }
        let values = unsafe { std::slice::from_raw_parts(self.ptr, self.len) };
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
    pub(crate) fn from_bridge(value: Option<rust::ArtifactSidecarLabel>) -> Result<Self, String> {
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
    pub(crate) fn to_bridge(&self) -> Result<Option<rust::ArtifactSidecarLabel>, String> {
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
    pub(crate) fn from_bridge(value: rust::Content) -> Result<Self, String> {
        Ok(match value {
            rust::Content::Text { content } => Self {
                kind: DestackContentKind::Text,
                text_content: c_string(content)?,
                binary_content: DestackByteArray {
                    ptr: ptr::null_mut(),
                    len: 0,
                },
            },
            rust::Content::Binary { content } => Self {
                kind: DestackContentKind::Binary,
                text_content: ptr::null_mut(),
                binary_content: DestackByteArray::from_vec(content),
            },
        })
    }

    /// Convert this C ABI enum into one bridge enum.
    pub(crate) fn to_bridge(&self) -> Result<rust::Content, String> {
        match self.kind {
            DestackContentKind::Text => {
                let text_content = read_string(self.text_content)?;
                Ok(rust::Content::Text {
                    content: text_content,
                })
            }
            DestackContentKind::Binary => {
                let binary_content = read_bytes(
                    self.binary_content.ptr.cast_const(),
                    self.binary_content.len,
                )?;
                Ok(rust::Content::Binary {
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
    pub(crate) fn from_bridge(values: Vec<rust::Content>) -> Result<Self, String> {
        let mut converted = Vec::with_capacity(values.len());
        for value in values {
            converted.push(DestackContent::from_bridge(value)?);
        }
        let (ptr, len) = owned_array(converted);
        Ok(Self { ptr, len })
    }

    /// Convert this C ABI array into bridge values.
    pub(crate) fn to_bridge(&self) -> Result<Vec<rust::Content>, String> {
        if self.len == 0 {
            return Ok(Vec::new());
        }
        if self.ptr.is_null() {
            return Err("array pointer is null".to_string());
        }
        let values = unsafe { std::slice::from_raw_parts(self.ptr, self.len) };
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
    pub(crate) fn from_bridge(value: Option<rust::Content>) -> Result<Self, String> {
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
    pub(crate) fn to_bridge(&self) -> Result<Option<rust::Content>, String> {
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
    pub(crate) fn from_bridge(value: rust::ArtifactSidecar) -> Result<Self, String> {
        Ok(Self {
            name: c_string(value.name)?,
            labels: DestackArtifactSidecarLabelArray::from_bridge(value.labels)?,
            content: DestackContent::from_bridge(value.content)?,
        })
    }

    /// Convert this C ABI value into one bridge value.
    pub(crate) fn to_bridge(&self) -> Result<rust::ArtifactSidecar, String> {
        Ok(rust::ArtifactSidecar {
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
    pub(crate) fn from_bridge(values: Vec<rust::ArtifactSidecar>) -> Result<Self, String> {
        let mut converted = Vec::with_capacity(values.len());
        for value in values {
            converted.push(DestackArtifactSidecar::from_bridge(value)?);
        }
        let (ptr, len) = owned_array(converted);
        Ok(Self { ptr, len })
    }

    /// Convert this C ABI array into bridge values.
    pub(crate) fn to_bridge(&self) -> Result<Vec<rust::ArtifactSidecar>, String> {
        if self.len == 0 {
            return Ok(Vec::new());
        }
        if self.ptr.is_null() {
            return Err("array pointer is null".to_string());
        }
        let values = unsafe { std::slice::from_raw_parts(self.ptr, self.len) };
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
    pub(crate) fn from_bridge(value: Option<rust::ArtifactSidecar>) -> Result<Self, String> {
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
    pub(crate) fn to_bridge(&self) -> Result<Option<rust::ArtifactSidecar>, String> {
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
    pub(crate) fn from_bridge(value: rust::ArtifactString) -> Result<Self, String> {
        Ok(Self {
            id: c_string(value.id)?,
            text: c_string(value.text)?,
        })
    }

    /// Convert this C ABI value into one bridge value.
    pub(crate) fn to_bridge(&self) -> Result<rust::ArtifactString, String> {
        Ok(rust::ArtifactString {
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
    pub(crate) fn from_bridge(values: Vec<rust::ArtifactString>) -> Result<Self, String> {
        let mut converted = Vec::with_capacity(values.len());
        for value in values {
            converted.push(DestackArtifactString::from_bridge(value)?);
        }
        let (ptr, len) = owned_array(converted);
        Ok(Self { ptr, len })
    }

    /// Convert this C ABI array into bridge values.
    pub(crate) fn to_bridge(&self) -> Result<Vec<rust::ArtifactString>, String> {
        if self.len == 0 {
            return Ok(Vec::new());
        }
        if self.ptr.is_null() {
            return Err("array pointer is null".to_string());
        }
        let values = unsafe { std::slice::from_raw_parts(self.ptr, self.len) };
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
    pub(crate) fn from_bridge(value: Option<rust::ArtifactString>) -> Result<Self, String> {
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
    pub(crate) fn to_bridge(&self) -> Result<Option<rust::ArtifactString>, String> {
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
    pub(crate) fn from_bridge(value: rust::DiagnosticHelp) -> Result<Self, String> {
        Ok(Self {
            message: c_string(value.message)?,
        })
    }

    /// Convert this C ABI value into one bridge value.
    pub(crate) fn to_bridge(&self) -> Result<rust::DiagnosticHelp, String> {
        Ok(rust::DiagnosticHelp {
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
    pub(crate) fn from_bridge(values: Vec<rust::DiagnosticHelp>) -> Result<Self, String> {
        let mut converted = Vec::with_capacity(values.len());
        for value in values {
            converted.push(DestackDiagnosticHelp::from_bridge(value)?);
        }
        let (ptr, len) = owned_array(converted);
        Ok(Self { ptr, len })
    }

    /// Convert this C ABI array into bridge values.
    pub(crate) fn to_bridge(&self) -> Result<Vec<rust::DiagnosticHelp>, String> {
        if self.len == 0 {
            return Ok(Vec::new());
        }
        if self.ptr.is_null() {
            return Err("array pointer is null".to_string());
        }
        let values = unsafe { std::slice::from_raw_parts(self.ptr, self.len) };
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
    pub(crate) fn from_bridge(value: Option<rust::DiagnosticHelp>) -> Result<Self, String> {
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
    pub(crate) fn to_bridge(&self) -> Result<Option<rust::DiagnosticHelp>, String> {
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
    pub(crate) fn from_bridge(value: rust::Span) -> Result<Self, String> {
        Ok(Self {
            file: DestackFileId::from_bridge(value.file)?,
            start: value.start,
            end: value.end,
        })
    }

    /// Convert this C ABI value into one bridge value.
    pub(crate) fn to_bridge(&self) -> Result<rust::Span, String> {
        Ok(rust::Span {
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
    pub(crate) fn from_bridge(values: Vec<rust::Span>) -> Result<Self, String> {
        let mut converted = Vec::with_capacity(values.len());
        for value in values {
            converted.push(DestackSpan::from_bridge(value)?);
        }
        let (ptr, len) = owned_array(converted);
        Ok(Self { ptr, len })
    }

    /// Convert this C ABI array into bridge values.
    pub(crate) fn to_bridge(&self) -> Result<Vec<rust::Span>, String> {
        if self.len == 0 {
            return Ok(Vec::new());
        }
        if self.ptr.is_null() {
            return Err("array pointer is null".to_string());
        }
        let values = unsafe { std::slice::from_raw_parts(self.ptr, self.len) };
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
    pub(crate) fn from_bridge(value: Option<rust::Span>) -> Result<Self, String> {
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
    pub(crate) fn to_bridge(&self) -> Result<Option<rust::Span>, String> {
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
    pub(crate) fn from_bridge(value: rust::DiagnosticLabel) -> Result<Self, String> {
        Ok(Self {
            content: DestackContentId::from_bridge(value.content)?,
            span: DestackSpan::from_bridge(value.span)?,
            message: DestackOptionalString::from_bridge(value.message)?,
        })
    }

    /// Convert this C ABI value into one bridge value.
    pub(crate) fn to_bridge(&self) -> Result<rust::DiagnosticLabel, String> {
        Ok(rust::DiagnosticLabel {
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
    pub(crate) fn from_bridge(values: Vec<rust::DiagnosticLabel>) -> Result<Self, String> {
        let mut converted = Vec::with_capacity(values.len());
        for value in values {
            converted.push(DestackDiagnosticLabel::from_bridge(value)?);
        }
        let (ptr, len) = owned_array(converted);
        Ok(Self { ptr, len })
    }

    /// Convert this C ABI array into bridge values.
    pub(crate) fn to_bridge(&self) -> Result<Vec<rust::DiagnosticLabel>, String> {
        if self.len == 0 {
            return Ok(Vec::new());
        }
        if self.ptr.is_null() {
            return Err("array pointer is null".to_string());
        }
        let values = unsafe { std::slice::from_raw_parts(self.ptr, self.len) };
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
    pub(crate) fn from_bridge(value: Option<rust::DiagnosticLabel>) -> Result<Self, String> {
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
    pub(crate) fn to_bridge(&self) -> Result<Option<rust::DiagnosticLabel>, String> {
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
    pub(crate) fn from_bridge(value: rust::DiagnosticNote) -> Result<Self, String> {
        Ok(Self {
            message: c_string(value.message)?,
        })
    }

    /// Convert this C ABI value into one bridge value.
    pub(crate) fn to_bridge(&self) -> Result<rust::DiagnosticNote, String> {
        Ok(rust::DiagnosticNote {
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
    pub(crate) fn from_bridge(values: Vec<rust::DiagnosticNote>) -> Result<Self, String> {
        let mut converted = Vec::with_capacity(values.len());
        for value in values {
            converted.push(DestackDiagnosticNote::from_bridge(value)?);
        }
        let (ptr, len) = owned_array(converted);
        Ok(Self { ptr, len })
    }

    /// Convert this C ABI array into bridge values.
    pub(crate) fn to_bridge(&self) -> Result<Vec<rust::DiagnosticNote>, String> {
        if self.len == 0 {
            return Ok(Vec::new());
        }
        if self.ptr.is_null() {
            return Err("array pointer is null".to_string());
        }
        let values = unsafe { std::slice::from_raw_parts(self.ptr, self.len) };
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
    pub(crate) fn from_bridge(value: Option<rust::DiagnosticNote>) -> Result<Self, String> {
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
    pub(crate) fn to_bridge(&self) -> Result<Option<rust::DiagnosticNote>, String> {
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
    pub(crate) fn from_bridge(value: rust::DiagnosticSeverity) -> Result<Self, String> {
        Ok(match value {
            rust::DiagnosticSeverity::Note => Self::Note,
            rust::DiagnosticSeverity::Warning => Self::Warning,
            rust::DiagnosticSeverity::Error => Self::Error,
        })
    }

    /// Convert this C ABI enum into one bridge enum.
    pub(crate) fn to_bridge(&self) -> Result<rust::DiagnosticSeverity, String> {
        Ok(match *self {
            Self::Note => rust::DiagnosticSeverity::Note,
            Self::Warning => rust::DiagnosticSeverity::Warning,
            Self::Error => rust::DiagnosticSeverity::Error,
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
    pub(crate) fn from_bridge(values: Vec<rust::DiagnosticSeverity>) -> Result<Self, String> {
        let mut converted = Vec::with_capacity(values.len());
        for value in values {
            converted.push(DestackDiagnosticSeverity::from_bridge(value)?);
        }
        let (ptr, len) = owned_array(converted);
        Ok(Self { ptr, len })
    }

    /// Convert this C ABI array into bridge values.
    pub(crate) fn to_bridge(&self) -> Result<Vec<rust::DiagnosticSeverity>, String> {
        if self.len == 0 {
            return Ok(Vec::new());
        }
        if self.ptr.is_null() {
            return Err("array pointer is null".to_string());
        }
        let values = unsafe { std::slice::from_raw_parts(self.ptr, self.len) };
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
    pub(crate) fn from_bridge(value: Option<rust::DiagnosticSeverity>) -> Result<Self, String> {
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
    pub(crate) fn to_bridge(&self) -> Result<Option<rust::DiagnosticSeverity>, String> {
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
pub struct DestackReplacement {
    /// Source span to replace.
    pub(crate) span: DestackSpan,
    /// Replacement text.
    pub(crate) new_text: *mut c_char,
}

/// C ABI bridge value array.
#[repr(C)]
#[derive(Debug)]
pub struct DestackReplacementArray {
    /// Owned value pointer.
    pub(crate) ptr: *mut DestackReplacement,
    /// Value count.
    pub(crate) len: usize,
}

/// C ABI optional bridge value.
#[repr(C)]
#[derive(Debug)]
pub struct DestackOptionalReplacement {
    /// Whether the value is present.
    pub(crate) is_some: bool,
    /// Value when present.
    pub(crate) value: DestackReplacement,
}

impl DestackReplacement {
    /// Convert one bridge value into one C ABI value.
    pub(crate) fn from_bridge(value: rust::Replacement) -> Result<Self, String> {
        Ok(Self {
            span: DestackSpan::from_bridge(value.span)?,
            new_text: c_string(value.new_text)?,
        })
    }

    /// Convert this C ABI value into one bridge value.
    pub(crate) fn to_bridge(&self) -> Result<rust::Replacement, String> {
        Ok(rust::Replacement {
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

impl DestackReplacementArray {
    /// Convert bridge values into one C ABI array.
    pub(crate) fn from_bridge(values: Vec<rust::Replacement>) -> Result<Self, String> {
        let mut converted = Vec::with_capacity(values.len());
        for value in values {
            converted.push(DestackReplacement::from_bridge(value)?);
        }
        let (ptr, len) = owned_array(converted);
        Ok(Self { ptr, len })
    }

    /// Convert this C ABI array into bridge values.
    pub(crate) fn to_bridge(&self) -> Result<Vec<rust::Replacement>, String> {
        if self.len == 0 {
            return Ok(Vec::new());
        }
        if self.ptr.is_null() {
            return Err("array pointer is null".to_string());
        }
        let values = unsafe { std::slice::from_raw_parts(self.ptr, self.len) };
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

impl DestackOptionalReplacement {
    /// Convert one optional bridge value into one C ABI optional value.
    pub(crate) fn from_bridge(value: Option<rust::Replacement>) -> Result<Self, String> {
        let Some(value) = value else {
            return Ok(Self {
                is_some: false,
                value: DestackReplacement::empty(),
            });
        };
        Ok(Self {
            is_some: true,
            value: DestackReplacement::from_bridge(value)?,
        })
    }

    /// Convert this C ABI optional value into one bridge optional value.
    pub(crate) fn to_bridge(&self) -> Result<Option<rust::Replacement>, String> {
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
        self.value = DestackReplacement::empty();
    }
}

/// C ABI bridge value.
#[repr(C)]
#[derive(Debug)]
pub struct DestackFilePatch {
    /// Edited file.
    pub(crate) file: DestackFileId,
    /// Source replacements.
    pub(crate) replacements: DestackReplacementArray,
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
    pub(crate) fn from_bridge(value: rust::FilePatch) -> Result<Self, String> {
        Ok(Self {
            file: DestackFileId::from_bridge(value.file)?,
            replacements: DestackReplacementArray::from_bridge(value.replacements)?,
        })
    }

    /// Convert this C ABI value into one bridge value.
    pub(crate) fn to_bridge(&self) -> Result<rust::FilePatch, String> {
        Ok(rust::FilePatch {
            file: self.file.to_bridge()?,
            replacements: self.replacements.to_bridge()?,
        })
    }

    /// Destroy this C ABI value.
    pub(crate) fn destroy(&mut self) {
        self.file.destroy();
        self.replacements.destroy();
    }

    /// Return one empty C ABI value.
    pub(crate) fn empty() -> Self {
        Self {
            file: DestackFileId::empty(),
            replacements: DestackReplacementArray {
                ptr: ptr::null_mut(),
                len: 0,
            },
        }
    }
}

impl DestackFilePatchArray {
    /// Convert bridge values into one C ABI array.
    pub(crate) fn from_bridge(values: Vec<rust::FilePatch>) -> Result<Self, String> {
        let mut converted = Vec::with_capacity(values.len());
        for value in values {
            converted.push(DestackFilePatch::from_bridge(value)?);
        }
        let (ptr, len) = owned_array(converted);
        Ok(Self { ptr, len })
    }

    /// Convert this C ABI array into bridge values.
    pub(crate) fn to_bridge(&self) -> Result<Vec<rust::FilePatch>, String> {
        if self.len == 0 {
            return Ok(Vec::new());
        }
        if self.ptr.is_null() {
            return Err("array pointer is null".to_string());
        }
        let values = unsafe { std::slice::from_raw_parts(self.ptr, self.len) };
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
    pub(crate) fn from_bridge(value: Option<rust::FilePatch>) -> Result<Self, String> {
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
    pub(crate) fn to_bridge(&self) -> Result<Option<rust::FilePatch>, String> {
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
pub struct DestackBatchEdit {
    /// Per-file edits.
    pub(crate) files: DestackFilePatchArray,
}

/// C ABI bridge value array.
#[repr(C)]
#[derive(Debug)]
pub struct DestackBatchEditArray {
    /// Owned value pointer.
    pub(crate) ptr: *mut DestackBatchEdit,
    /// Value count.
    pub(crate) len: usize,
}

/// C ABI optional bridge value.
#[repr(C)]
#[derive(Debug)]
pub struct DestackOptionalBatchEdit {
    /// Whether the value is present.
    pub(crate) is_some: bool,
    /// Value when present.
    pub(crate) value: DestackBatchEdit,
}

impl DestackBatchEdit {
    /// Convert one bridge value into one C ABI value.
    pub(crate) fn from_bridge(value: rust::BatchEdit) -> Result<Self, String> {
        Ok(Self {
            files: DestackFilePatchArray::from_bridge(value.files)?,
        })
    }

    /// Convert this C ABI value into one bridge value.
    pub(crate) fn to_bridge(&self) -> Result<rust::BatchEdit, String> {
        Ok(rust::BatchEdit {
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

impl DestackBatchEditArray {
    /// Convert bridge values into one C ABI array.
    pub(crate) fn from_bridge(values: Vec<rust::BatchEdit>) -> Result<Self, String> {
        let mut converted = Vec::with_capacity(values.len());
        for value in values {
            converted.push(DestackBatchEdit::from_bridge(value)?);
        }
        let (ptr, len) = owned_array(converted);
        Ok(Self { ptr, len })
    }

    /// Convert this C ABI array into bridge values.
    pub(crate) fn to_bridge(&self) -> Result<Vec<rust::BatchEdit>, String> {
        if self.len == 0 {
            return Ok(Vec::new());
        }
        if self.ptr.is_null() {
            return Err("array pointer is null".to_string());
        }
        let values = unsafe { std::slice::from_raw_parts(self.ptr, self.len) };
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

impl DestackOptionalBatchEdit {
    /// Convert one optional bridge value into one C ABI optional value.
    pub(crate) fn from_bridge(value: Option<rust::BatchEdit>) -> Result<Self, String> {
        let Some(value) = value else {
            return Ok(Self {
                is_some: false,
                value: DestackBatchEdit::empty(),
            });
        };
        Ok(Self {
            is_some: true,
            value: DestackBatchEdit::from_bridge(value)?,
        })
    }

    /// Convert this C ABI optional value into one bridge optional value.
    pub(crate) fn to_bridge(&self) -> Result<Option<rust::BatchEdit>, String> {
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
        self.value = DestackBatchEdit::empty();
    }
}

/// C ABI bridge value.
#[repr(C)]
#[derive(Debug)]
pub struct DestackDiagnosticSuggestion {
    /// Exact source edits for machine application.
    pub(crate) edits: DestackBatchEdit,
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
    pub(crate) fn from_bridge(value: rust::DiagnosticSuggestion) -> Result<Self, String> {
        Ok(Self {
            edits: DestackBatchEdit::from_bridge(value.edits)?,
            labels: DestackDiagnosticLabelArray::from_bridge(value.labels)?,
            message: c_string(value.message)?,
            applicability: DestackApplicability::from_bridge(value.applicability)?,
        })
    }

    /// Convert this C ABI value into one bridge value.
    pub(crate) fn to_bridge(&self) -> Result<rust::DiagnosticSuggestion, String> {
        Ok(rust::DiagnosticSuggestion {
            edits: self.edits.to_bridge()?,
            labels: self.labels.to_bridge()?,
            message: read_string(self.message)?,
            applicability: self.applicability.to_bridge()?,
        })
    }

    /// Destroy this C ABI value.
    pub(crate) fn destroy(&mut self) {
        self.edits.destroy();
        self.labels.destroy();
        destroy_string(self.message);
        self.message = ptr::null_mut();
        self.applicability.destroy();
    }

    /// Return one empty C ABI value.
    pub(crate) fn empty() -> Self {
        Self {
            edits: DestackBatchEdit::empty(),
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
    pub(crate) fn from_bridge(values: Vec<rust::DiagnosticSuggestion>) -> Result<Self, String> {
        let mut converted = Vec::with_capacity(values.len());
        for value in values {
            converted.push(DestackDiagnosticSuggestion::from_bridge(value)?);
        }
        let (ptr, len) = owned_array(converted);
        Ok(Self { ptr, len })
    }

    /// Convert this C ABI array into bridge values.
    pub(crate) fn to_bridge(&self) -> Result<Vec<rust::DiagnosticSuggestion>, String> {
        if self.len == 0 {
            return Ok(Vec::new());
        }
        if self.ptr.is_null() {
            return Err("array pointer is null".to_string());
        }
        let values = unsafe { std::slice::from_raw_parts(self.ptr, self.len) };
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
    pub(crate) fn from_bridge(value: Option<rust::DiagnosticSuggestion>) -> Result<Self, String> {
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
    pub(crate) fn to_bridge(&self) -> Result<Option<rust::DiagnosticSuggestion>, String> {
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
    pub(crate) fn from_bridge(value: rust::DiagnosticTag) -> Result<Self, String> {
        Ok(match value {
            rust::DiagnosticTag::Unnecessary => Self::Unnecessary,
            rust::DiagnosticTag::Deprecated => Self::Deprecated,
        })
    }

    /// Convert this C ABI enum into one bridge enum.
    pub(crate) fn to_bridge(&self) -> Result<rust::DiagnosticTag, String> {
        Ok(match *self {
            Self::Unnecessary => rust::DiagnosticTag::Unnecessary,
            Self::Deprecated => rust::DiagnosticTag::Deprecated,
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
    pub(crate) fn from_bridge(values: Vec<rust::DiagnosticTag>) -> Result<Self, String> {
        let mut converted = Vec::with_capacity(values.len());
        for value in values {
            converted.push(DestackDiagnosticTag::from_bridge(value)?);
        }
        let (ptr, len) = owned_array(converted);
        Ok(Self { ptr, len })
    }

    /// Convert this C ABI array into bridge values.
    pub(crate) fn to_bridge(&self) -> Result<Vec<rust::DiagnosticTag>, String> {
        if self.len == 0 {
            return Ok(Vec::new());
        }
        if self.ptr.is_null() {
            return Err("array pointer is null".to_string());
        }
        let values = unsafe { std::slice::from_raw_parts(self.ptr, self.len) };
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
    pub(crate) fn from_bridge(value: Option<rust::DiagnosticTag>) -> Result<Self, String> {
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
    pub(crate) fn to_bridge(&self) -> Result<Option<rust::DiagnosticTag>, String> {
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
    pub(crate) fn from_bridge(value: rust::Diagnostic) -> Result<Self, String> {
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
    pub(crate) fn to_bridge(&self) -> Result<rust::Diagnostic, String> {
        Ok(rust::Diagnostic {
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
    pub(crate) fn from_bridge(values: Vec<rust::Diagnostic>) -> Result<Self, String> {
        let mut converted = Vec::with_capacity(values.len());
        for value in values {
            converted.push(DestackDiagnostic::from_bridge(value)?);
        }
        let (ptr, len) = owned_array(converted);
        Ok(Self { ptr, len })
    }

    /// Convert this C ABI array into bridge values.
    pub(crate) fn to_bridge(&self) -> Result<Vec<rust::Diagnostic>, String> {
        if self.len == 0 {
            return Ok(Vec::new());
        }
        if self.ptr.is_null() {
            return Err("array pointer is null".to_string());
        }
        let values = unsafe { std::slice::from_raw_parts(self.ptr, self.len) };
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
    pub(crate) fn from_bridge(value: Option<rust::Diagnostic>) -> Result<Self, String> {
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
    pub(crate) fn to_bridge(&self) -> Result<Option<rust::Diagnostic>, String> {
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
    pub(crate) fn from_bridge(value: rust::ArtifactRecord) -> Result<Self, String> {
        Ok(Self {
            version: DestackArtifactVersion::from_bridge(value.version)?,
            payload: DestackByteArray::from_vec(value.payload),
            strings: DestackArtifactStringArray::from_bridge(value.strings)?,
            dependencies: DestackArtifactDependencyArray::from_bridge(value.dependencies)?,
            diagnostics: DestackDiagnosticArray::from_bridge(value.diagnostics)?,
            sidecars: DestackArtifactSidecarArray::from_bridge(value.sidecars)?,
        })
    }

    /// Convert this C ABI value into one bridge value.
    pub(crate) fn to_bridge(&self) -> Result<rust::ArtifactRecord, String> {
        Ok(rust::ArtifactRecord {
            version: self.version.to_bridge()?,
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
    pub(crate) fn from_bridge(values: Vec<rust::ArtifactRecord>) -> Result<Self, String> {
        let mut converted = Vec::with_capacity(values.len());
        for value in values {
            converted.push(DestackArtifactRecord::from_bridge(value)?);
        }
        let (ptr, len) = owned_array(converted);
        Ok(Self { ptr, len })
    }

    /// Convert this C ABI array into bridge values.
    pub(crate) fn to_bridge(&self) -> Result<Vec<rust::ArtifactRecord>, String> {
        if self.len == 0 {
            return Ok(Vec::new());
        }
        if self.ptr.is_null() {
            return Err("array pointer is null".to_string());
        }
        let values = unsafe { std::slice::from_raw_parts(self.ptr, self.len) };
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
    pub(crate) fn from_bridge(value: Option<rust::ArtifactRecord>) -> Result<Self, String> {
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
    pub(crate) fn to_bridge(&self) -> Result<Option<rust::ArtifactRecord>, String> {
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

/// C ABI bridge enum.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DestackFileType {
    /// `.ds`.
    Destack = 0,
    /// `.d.ds`.
    DestackDeclaration = 1,
    /// `.js`.
    JavaScript = 2,
    /// `.jsx`.
    JavaScriptXml = 3,
    /// `.ts`.
    TypeScript = 4,
    /// `.tsx`.
    TypeScriptXml = 5,
    /// `.d.ts`.
    TypeScriptDeclaration = 6,
    /// Text file.
    Text = 7,
    /// TOML file.
    Toml = 8,
    /// YAML file.
    Yaml = 9,
    /// JSON file.
    Json = 10,
    /// Environment file.
    Env = 11,
    /// HTML file.
    Html = 12,
    /// Markdown file.
    Markdown = 13,
    /// CSS file.
    Css = 14,
    /// SVG file.
    Svg = 15,
    /// WebAssembly payload.
    Wasm = 16,
    /// Node native module.
    Node = 17,
    /// Source map file.
    SourceMap = 18,
    /// Native object file.
    Object = 19,
    /// Image asset.
    Image = 20,
    /// Font asset.
    Font = 21,
    /// Audio asset.
    Audio = 22,
    /// Video asset.
    Video = 23,
    /// 3D model asset.
    Model = 24,
    /// AI model asset.
    Neural = 25,
    /// Document asset.
    Document = 26,
    /// Unknown binary file.
    Binary = 27,
    /// Unknown file type.
    Unknown = 28,
}

/// C ABI bridge enum array.
#[repr(C)]
#[derive(Debug)]
pub struct DestackFileTypeArray {
    /// Owned value pointer.
    pub(crate) ptr: *mut DestackFileType,
    /// Value count.
    pub(crate) len: usize,
}

/// C ABI optional bridge enum.
#[repr(C)]
#[derive(Debug)]
pub struct DestackOptionalFileType {
    /// Whether the value is present.
    pub(crate) is_some: bool,
    /// Value when present.
    pub(crate) value: DestackFileType,
}

impl DestackFileType {
    /// Convert one bridge enum into one C ABI enum.
    pub(crate) fn from_bridge(value: rust::FileType) -> Result<Self, String> {
        Ok(match value {
            rust::FileType::Destack => Self::Destack,
            rust::FileType::DestackDeclaration => Self::DestackDeclaration,
            rust::FileType::JavaScript => Self::JavaScript,
            rust::FileType::JavaScriptXml => Self::JavaScriptXml,
            rust::FileType::TypeScript => Self::TypeScript,
            rust::FileType::TypeScriptXml => Self::TypeScriptXml,
            rust::FileType::TypeScriptDeclaration => Self::TypeScriptDeclaration,
            rust::FileType::Text => Self::Text,
            rust::FileType::Toml => Self::Toml,
            rust::FileType::Yaml => Self::Yaml,
            rust::FileType::Json => Self::Json,
            rust::FileType::Env => Self::Env,
            rust::FileType::Html => Self::Html,
            rust::FileType::Markdown => Self::Markdown,
            rust::FileType::Css => Self::Css,
            rust::FileType::Svg => Self::Svg,
            rust::FileType::Wasm => Self::Wasm,
            rust::FileType::Node => Self::Node,
            rust::FileType::SourceMap => Self::SourceMap,
            rust::FileType::Object => Self::Object,
            rust::FileType::Image => Self::Image,
            rust::FileType::Font => Self::Font,
            rust::FileType::Audio => Self::Audio,
            rust::FileType::Video => Self::Video,
            rust::FileType::Model => Self::Model,
            rust::FileType::Neural => Self::Neural,
            rust::FileType::Document => Self::Document,
            rust::FileType::Binary => Self::Binary,
            rust::FileType::Unknown => Self::Unknown,
        })
    }

    /// Convert this C ABI enum into one bridge enum.
    pub(crate) fn to_bridge(&self) -> Result<rust::FileType, String> {
        Ok(match *self {
            Self::Destack => rust::FileType::Destack,
            Self::DestackDeclaration => rust::FileType::DestackDeclaration,
            Self::JavaScript => rust::FileType::JavaScript,
            Self::JavaScriptXml => rust::FileType::JavaScriptXml,
            Self::TypeScript => rust::FileType::TypeScript,
            Self::TypeScriptXml => rust::FileType::TypeScriptXml,
            Self::TypeScriptDeclaration => rust::FileType::TypeScriptDeclaration,
            Self::Text => rust::FileType::Text,
            Self::Toml => rust::FileType::Toml,
            Self::Yaml => rust::FileType::Yaml,
            Self::Json => rust::FileType::Json,
            Self::Env => rust::FileType::Env,
            Self::Html => rust::FileType::Html,
            Self::Markdown => rust::FileType::Markdown,
            Self::Css => rust::FileType::Css,
            Self::Svg => rust::FileType::Svg,
            Self::Wasm => rust::FileType::Wasm,
            Self::Node => rust::FileType::Node,
            Self::SourceMap => rust::FileType::SourceMap,
            Self::Object => rust::FileType::Object,
            Self::Image => rust::FileType::Image,
            Self::Font => rust::FileType::Font,
            Self::Audio => rust::FileType::Audio,
            Self::Video => rust::FileType::Video,
            Self::Model => rust::FileType::Model,
            Self::Neural => rust::FileType::Neural,
            Self::Document => rust::FileType::Document,
            Self::Binary => rust::FileType::Binary,
            Self::Unknown => rust::FileType::Unknown,
        })
    }

    /// Destroy this C ABI enum.
    pub(crate) fn destroy(&mut self) {}
    /// Return one empty C ABI enum.
    pub(crate) fn empty() -> Self {
        Self::Destack
    }
}

impl DestackFileTypeArray {
    /// Convert bridge values into one C ABI array.
    pub(crate) fn from_bridge(values: Vec<rust::FileType>) -> Result<Self, String> {
        let mut converted = Vec::with_capacity(values.len());
        for value in values {
            converted.push(DestackFileType::from_bridge(value)?);
        }
        let (ptr, len) = owned_array(converted);
        Ok(Self { ptr, len })
    }

    /// Convert this C ABI array into bridge values.
    pub(crate) fn to_bridge(&self) -> Result<Vec<rust::FileType>, String> {
        if self.len == 0 {
            return Ok(Vec::new());
        }
        if self.ptr.is_null() {
            return Err("array pointer is null".to_string());
        }
        let values = unsafe { std::slice::from_raw_parts(self.ptr, self.len) };
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

impl DestackOptionalFileType {
    /// Convert one optional bridge value into one C ABI optional value.
    pub(crate) fn from_bridge(value: Option<rust::FileType>) -> Result<Self, String> {
        let Some(value) = value else {
            return Ok(Self {
                is_some: false,
                value: DestackFileType::empty(),
            });
        };
        Ok(Self {
            is_some: true,
            value: DestackFileType::from_bridge(value)?,
        })
    }

    /// Convert this C ABI optional value into one bridge optional value.
    pub(crate) fn to_bridge(&self) -> Result<Option<rust::FileType>, String> {
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
        self.value = DestackFileType::empty();
    }
}

/// C ABI bridge value.
#[repr(C)]
#[derive(Debug)]
pub struct DestackSourceMapSource {
    /// The mapped source names.
    pub(crate) name: *mut c_char,
    /// The embedded source contents when they exist.
    pub(crate) content: DestackOptionalString,
}

/// C ABI bridge value array.
#[repr(C)]
#[derive(Debug)]
pub struct DestackSourceMapSourceArray {
    /// Owned value pointer.
    pub(crate) ptr: *mut DestackSourceMapSource,
    /// Value count.
    pub(crate) len: usize,
}

/// C ABI optional bridge value.
#[repr(C)]
#[derive(Debug)]
pub struct DestackOptionalSourceMapSource {
    /// Whether the value is present.
    pub(crate) is_some: bool,
    /// Value when present.
    pub(crate) value: DestackSourceMapSource,
}

impl DestackSourceMapSource {
    /// Convert one bridge value into one C ABI value.
    pub(crate) fn from_bridge(value: rust::SourceMapSource) -> Result<Self, String> {
        Ok(Self {
            name: c_string(value.name)?,
            content: DestackOptionalString::from_bridge(value.content)?,
        })
    }

    /// Convert this C ABI value into one bridge value.
    pub(crate) fn to_bridge(&self) -> Result<rust::SourceMapSource, String> {
        Ok(rust::SourceMapSource {
            name: read_string(self.name)?,
            content: self.content.to_bridge()?,
        })
    }

    /// Destroy this C ABI value.
    pub(crate) fn destroy(&mut self) {
        destroy_string(self.name);
        self.name = ptr::null_mut();
        self.content.destroy();
    }

    /// Return one empty C ABI value.
    pub(crate) fn empty() -> Self {
        Self {
            name: ptr::null_mut(),
            content: DestackOptionalString::empty(),
        }
    }
}

impl DestackSourceMapSourceArray {
    /// Convert bridge values into one C ABI array.
    pub(crate) fn from_bridge(values: Vec<rust::SourceMapSource>) -> Result<Self, String> {
        let mut converted = Vec::with_capacity(values.len());
        for value in values {
            converted.push(DestackSourceMapSource::from_bridge(value)?);
        }
        let (ptr, len) = owned_array(converted);
        Ok(Self { ptr, len })
    }

    /// Convert this C ABI array into bridge values.
    pub(crate) fn to_bridge(&self) -> Result<Vec<rust::SourceMapSource>, String> {
        if self.len == 0 {
            return Ok(Vec::new());
        }
        if self.ptr.is_null() {
            return Err("array pointer is null".to_string());
        }
        let values = unsafe { std::slice::from_raw_parts(self.ptr, self.len) };
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

impl DestackOptionalSourceMapSource {
    /// Convert one optional bridge value into one C ABI optional value.
    pub(crate) fn from_bridge(value: Option<rust::SourceMapSource>) -> Result<Self, String> {
        let Some(value) = value else {
            return Ok(Self {
                is_some: false,
                value: DestackSourceMapSource::empty(),
            });
        };
        Ok(Self {
            is_some: true,
            value: DestackSourceMapSource::from_bridge(value)?,
        })
    }

    /// Convert this C ABI optional value into one bridge optional value.
    pub(crate) fn to_bridge(&self) -> Result<Option<rust::SourceMapSource>, String> {
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
        self.value = DestackSourceMapSource::empty();
    }
}

/// C ABI bridge value.
#[repr(C)]
#[derive(Debug)]
pub struct DestackSourceMap {
    /// The source map version.
    pub(crate) version: u32,
    /// The emitted file name when one exists.
    pub(crate) file: DestackOptionalString,
    /// The source root when one exists.
    pub(crate) source_root: DestackOptionalString,
    /// The mapped sources.
    pub(crate) sources: DestackSourceMapSourceArray,
    /// The recorded symbol names.
    pub(crate) names: DestackStringArray,
    /// The VLQ mapping payload.
    pub(crate) mappings: *mut c_char,
    /// The debug id when one exists.
    pub(crate) debug_id: DestackOptionalString,
}

/// C ABI bridge value array.
#[repr(C)]
#[derive(Debug)]
pub struct DestackSourceMapArray {
    /// Owned value pointer.
    pub(crate) ptr: *mut DestackSourceMap,
    /// Value count.
    pub(crate) len: usize,
}

/// C ABI optional bridge value.
#[repr(C)]
#[derive(Debug)]
pub struct DestackOptionalSourceMap {
    /// Whether the value is present.
    pub(crate) is_some: bool,
    /// Value when present.
    pub(crate) value: DestackSourceMap,
}

impl DestackSourceMap {
    /// Convert one bridge value into one C ABI value.
    pub(crate) fn from_bridge(value: rust::SourceMap) -> Result<Self, String> {
        Ok(Self {
            version: value.version,
            file: DestackOptionalString::from_bridge(value.file)?,
            source_root: DestackOptionalString::from_bridge(value.source_root)?,
            sources: DestackSourceMapSourceArray::from_bridge(value.sources)?,
            names: DestackStringArray::from_bridge(value.names)?,
            mappings: c_string(value.mappings)?,
            debug_id: DestackOptionalString::from_bridge(value.debug_id)?,
        })
    }

    /// Convert this C ABI value into one bridge value.
    pub(crate) fn to_bridge(&self) -> Result<rust::SourceMap, String> {
        Ok(rust::SourceMap {
            version: self.version,
            file: self.file.to_bridge()?,
            source_root: self.source_root.to_bridge()?,
            sources: self.sources.to_bridge()?,
            names: self.names.to_bridge()?,
            mappings: read_string(self.mappings)?,
            debug_id: self.debug_id.to_bridge()?,
        })
    }

    /// Destroy this C ABI value.
    pub(crate) fn destroy(&mut self) {
        self.file.destroy();
        self.source_root.destroy();
        self.sources.destroy();
        self.names.destroy();
        destroy_string(self.mappings);
        self.mappings = ptr::null_mut();
        self.debug_id.destroy();
    }

    /// Return one empty C ABI value.
    pub(crate) fn empty() -> Self {
        Self {
            version: 0,
            file: DestackOptionalString::empty(),
            source_root: DestackOptionalString::empty(),
            sources: DestackSourceMapSourceArray {
                ptr: ptr::null_mut(),
                len: 0,
            },
            names: DestackStringArray {
                ptr: ptr::null_mut(),
                len: 0,
            },
            mappings: ptr::null_mut(),
            debug_id: DestackOptionalString::empty(),
        }
    }
}

impl DestackSourceMapArray {
    /// Convert bridge values into one C ABI array.
    pub(crate) fn from_bridge(values: Vec<rust::SourceMap>) -> Result<Self, String> {
        let mut converted = Vec::with_capacity(values.len());
        for value in values {
            converted.push(DestackSourceMap::from_bridge(value)?);
        }
        let (ptr, len) = owned_array(converted);
        Ok(Self { ptr, len })
    }

    /// Convert this C ABI array into bridge values.
    pub(crate) fn to_bridge(&self) -> Result<Vec<rust::SourceMap>, String> {
        if self.len == 0 {
            return Ok(Vec::new());
        }
        if self.ptr.is_null() {
            return Err("array pointer is null".to_string());
        }
        let values = unsafe { std::slice::from_raw_parts(self.ptr, self.len) };
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

impl DestackOptionalSourceMap {
    /// Convert one optional bridge value into one C ABI optional value.
    pub(crate) fn from_bridge(value: Option<rust::SourceMap>) -> Result<Self, String> {
        let Some(value) = value else {
            return Ok(Self {
                is_some: false,
                value: DestackSourceMap::empty(),
            });
        };
        Ok(Self {
            is_some: true,
            value: DestackSourceMap::from_bridge(value)?,
        })
    }

    /// Convert this C ABI optional value into one bridge optional value.
    pub(crate) fn to_bridge(&self) -> Result<Option<rust::SourceMap>, String> {
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
        self.value = DestackSourceMap::empty();
    }
}

/// C ABI bridge value.
#[repr(C)]
#[derive(Debug)]
pub struct DestackAsset {
    /// The asset file type.
    pub(crate) file_type: DestackFileType,
    /// The asset content identity.
    pub(crate) content: DestackContentId,
    /// The source module URI when one exists.
    pub(crate) source: DestackOptionalString,
    /// The source map when one exists.
    pub(crate) map: DestackOptionalSourceMap,
}

/// C ABI bridge value array.
#[repr(C)]
#[derive(Debug)]
pub struct DestackAssetArray {
    /// Owned value pointer.
    pub(crate) ptr: *mut DestackAsset,
    /// Value count.
    pub(crate) len: usize,
}

/// C ABI optional bridge value.
#[repr(C)]
#[derive(Debug)]
pub struct DestackOptionalAsset {
    /// Whether the value is present.
    pub(crate) is_some: bool,
    /// Value when present.
    pub(crate) value: DestackAsset,
}

impl DestackAsset {
    /// Convert one bridge value into one C ABI value.
    pub(crate) fn from_bridge(value: rust::Asset) -> Result<Self, String> {
        Ok(Self {
            file_type: DestackFileType::from_bridge(value.file_type)?,
            content: DestackContentId::from_bridge(value.content)?,
            source: DestackOptionalString::from_bridge(value.source)?,
            map: DestackOptionalSourceMap::from_bridge(value.map)?,
        })
    }

    /// Convert this C ABI value into one bridge value.
    pub(crate) fn to_bridge(&self) -> Result<rust::Asset, String> {
        Ok(rust::Asset {
            file_type: self.file_type.to_bridge()?,
            content: self.content.to_bridge()?,
            source: self.source.to_bridge()?,
            map: self.map.to_bridge()?,
        })
    }

    /// Destroy this C ABI value.
    pub(crate) fn destroy(&mut self) {
        self.file_type.destroy();
        self.content.destroy();
        self.source.destroy();
        self.map.destroy();
    }

    /// Return one empty C ABI value.
    pub(crate) fn empty() -> Self {
        Self {
            file_type: DestackFileType::empty(),
            content: DestackContentId::empty(),
            source: DestackOptionalString::empty(),
            map: DestackOptionalSourceMap {
                is_some: false,
                value: DestackSourceMap::empty(),
            },
        }
    }
}

impl DestackAssetArray {
    /// Convert bridge values into one C ABI array.
    pub(crate) fn from_bridge(values: Vec<rust::Asset>) -> Result<Self, String> {
        let mut converted = Vec::with_capacity(values.len());
        for value in values {
            converted.push(DestackAsset::from_bridge(value)?);
        }
        let (ptr, len) = owned_array(converted);
        Ok(Self { ptr, len })
    }

    /// Convert this C ABI array into bridge values.
    pub(crate) fn to_bridge(&self) -> Result<Vec<rust::Asset>, String> {
        if self.len == 0 {
            return Ok(Vec::new());
        }
        if self.ptr.is_null() {
            return Err("array pointer is null".to_string());
        }
        let values = unsafe { std::slice::from_raw_parts(self.ptr, self.len) };
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

impl DestackOptionalAsset {
    /// Convert one optional bridge value into one C ABI optional value.
    pub(crate) fn from_bridge(value: Option<rust::Asset>) -> Result<Self, String> {
        let Some(value) = value else {
            return Ok(Self {
                is_some: false,
                value: DestackAsset::empty(),
            });
        };
        Ok(Self {
            is_some: true,
            value: DestackAsset::from_bridge(value)?,
        })
    }

    /// Convert this C ABI optional value into one bridge optional value.
    pub(crate) fn to_bridge(&self) -> Result<Option<rust::Asset>, String> {
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
        self.value = DestackAsset::empty();
    }
}

/// C ABI bridge enum.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DestackBuildLinkage {
    /// Ship a portable Destack payload consumed by a runtime.
    Portable = 0,
    /// Link the build payload into the produced platform binary.
    Static = 1,
    /// Ship the build payload as a dynamic library.
    Dynamic = 2,
}

/// C ABI bridge enum array.
#[repr(C)]
#[derive(Debug)]
pub struct DestackBuildLinkageArray {
    /// Owned value pointer.
    pub(crate) ptr: *mut DestackBuildLinkage,
    /// Value count.
    pub(crate) len: usize,
}

/// C ABI optional bridge enum.
#[repr(C)]
#[derive(Debug)]
pub struct DestackOptionalBuildLinkage {
    /// Whether the value is present.
    pub(crate) is_some: bool,
    /// Value when present.
    pub(crate) value: DestackBuildLinkage,
}

impl DestackBuildLinkage {
    /// Convert one bridge enum into one C ABI enum.
    pub(crate) fn from_bridge(value: rust::BuildLinkage) -> Result<Self, String> {
        Ok(match value {
            rust::BuildLinkage::Portable => Self::Portable,
            rust::BuildLinkage::Static => Self::Static,
            rust::BuildLinkage::Dynamic => Self::Dynamic,
        })
    }

    /// Convert this C ABI enum into one bridge enum.
    pub(crate) fn to_bridge(&self) -> Result<rust::BuildLinkage, String> {
        Ok(match *self {
            Self::Portable => rust::BuildLinkage::Portable,
            Self::Static => rust::BuildLinkage::Static,
            Self::Dynamic => rust::BuildLinkage::Dynamic,
        })
    }

    /// Destroy this C ABI enum.
    pub(crate) fn destroy(&mut self) {}
    /// Return one empty C ABI enum.
    pub(crate) fn empty() -> Self {
        Self::Portable
    }
}

impl DestackBuildLinkageArray {
    /// Convert bridge values into one C ABI array.
    pub(crate) fn from_bridge(values: Vec<rust::BuildLinkage>) -> Result<Self, String> {
        let mut converted = Vec::with_capacity(values.len());
        for value in values {
            converted.push(DestackBuildLinkage::from_bridge(value)?);
        }
        let (ptr, len) = owned_array(converted);
        Ok(Self { ptr, len })
    }

    /// Convert this C ABI array into bridge values.
    pub(crate) fn to_bridge(&self) -> Result<Vec<rust::BuildLinkage>, String> {
        if self.len == 0 {
            return Ok(Vec::new());
        }
        if self.ptr.is_null() {
            return Err("array pointer is null".to_string());
        }
        let values = unsafe { std::slice::from_raw_parts(self.ptr, self.len) };
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

impl DestackOptionalBuildLinkage {
    /// Convert one optional bridge value into one C ABI optional value.
    pub(crate) fn from_bridge(value: Option<rust::BuildLinkage>) -> Result<Self, String> {
        let Some(value) = value else {
            return Ok(Self {
                is_some: false,
                value: DestackBuildLinkage::empty(),
            });
        };
        Ok(Self {
            is_some: true,
            value: DestackBuildLinkage::from_bridge(value)?,
        })
    }

    /// Convert this C ABI optional value into one bridge optional value.
    pub(crate) fn to_bridge(&self) -> Result<Option<rust::BuildLinkage>, String> {
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
        self.value = DestackBuildLinkage::empty();
    }
}

/// C ABI bridge enum.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DestackBuildProfile {
    /// Full Destack build.
    Full = 0,
    /// Smaller Destack build with optional services omitted.
    Minimal = 1,
    /// Freestanding output without the normal Destack runtime contract.
    Freestanding = 2,
}

/// C ABI bridge enum array.
#[repr(C)]
#[derive(Debug)]
pub struct DestackBuildProfileArray {
    /// Owned value pointer.
    pub(crate) ptr: *mut DestackBuildProfile,
    /// Value count.
    pub(crate) len: usize,
}

/// C ABI optional bridge enum.
#[repr(C)]
#[derive(Debug)]
pub struct DestackOptionalBuildProfile {
    /// Whether the value is present.
    pub(crate) is_some: bool,
    /// Value when present.
    pub(crate) value: DestackBuildProfile,
}

impl DestackBuildProfile {
    /// Convert one bridge enum into one C ABI enum.
    pub(crate) fn from_bridge(value: rust::BuildProfile) -> Result<Self, String> {
        Ok(match value {
            rust::BuildProfile::Full => Self::Full,
            rust::BuildProfile::Minimal => Self::Minimal,
            rust::BuildProfile::Freestanding => Self::Freestanding,
        })
    }

    /// Convert this C ABI enum into one bridge enum.
    pub(crate) fn to_bridge(&self) -> Result<rust::BuildProfile, String> {
        Ok(match *self {
            Self::Full => rust::BuildProfile::Full,
            Self::Minimal => rust::BuildProfile::Minimal,
            Self::Freestanding => rust::BuildProfile::Freestanding,
        })
    }

    /// Destroy this C ABI enum.
    pub(crate) fn destroy(&mut self) {}
    /// Return one empty C ABI enum.
    pub(crate) fn empty() -> Self {
        Self::Full
    }
}

impl DestackBuildProfileArray {
    /// Convert bridge values into one C ABI array.
    pub(crate) fn from_bridge(values: Vec<rust::BuildProfile>) -> Result<Self, String> {
        let mut converted = Vec::with_capacity(values.len());
        for value in values {
            converted.push(DestackBuildProfile::from_bridge(value)?);
        }
        let (ptr, len) = owned_array(converted);
        Ok(Self { ptr, len })
    }

    /// Convert this C ABI array into bridge values.
    pub(crate) fn to_bridge(&self) -> Result<Vec<rust::BuildProfile>, String> {
        if self.len == 0 {
            return Ok(Vec::new());
        }
        if self.ptr.is_null() {
            return Err("array pointer is null".to_string());
        }
        let values = unsafe { std::slice::from_raw_parts(self.ptr, self.len) };
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

impl DestackOptionalBuildProfile {
    /// Convert one optional bridge value into one C ABI optional value.
    pub(crate) fn from_bridge(value: Option<rust::BuildProfile>) -> Result<Self, String> {
        let Some(value) = value else {
            return Ok(Self {
                is_some: false,
                value: DestackBuildProfile::empty(),
            });
        };
        Ok(Self {
            is_some: true,
            value: DestackBuildProfile::from_bridge(value)?,
        })
    }

    /// Convert this C ABI optional value into one bridge optional value.
    pub(crate) fn to_bridge(&self) -> Result<Option<rust::BuildProfile>, String> {
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
        self.value = DestackBuildProfile::empty();
    }
}

/// C ABI bridge value.
#[repr(C)]
#[derive(Debug)]
pub struct DestackBuild {
    /// The build distribution profile.
    pub(crate) profile: DestackBuildProfile,
    /// The build linkage.
    pub(crate) linkage: DestackBuildLinkage,
    /// The encoded build content.
    pub(crate) content: DestackContentId,
}

/// C ABI bridge value array.
#[repr(C)]
#[derive(Debug)]
pub struct DestackBuildArray {
    /// Owned value pointer.
    pub(crate) ptr: *mut DestackBuild,
    /// Value count.
    pub(crate) len: usize,
}

/// C ABI optional bridge value.
#[repr(C)]
#[derive(Debug)]
pub struct DestackOptionalBuild {
    /// Whether the value is present.
    pub(crate) is_some: bool,
    /// Value when present.
    pub(crate) value: DestackBuild,
}

impl DestackBuild {
    /// Convert one bridge value into one C ABI value.
    pub(crate) fn from_bridge(value: rust::Build) -> Result<Self, String> {
        Ok(Self {
            profile: DestackBuildProfile::from_bridge(value.profile)?,
            linkage: DestackBuildLinkage::from_bridge(value.linkage)?,
            content: DestackContentId::from_bridge(value.content)?,
        })
    }

    /// Convert this C ABI value into one bridge value.
    pub(crate) fn to_bridge(&self) -> Result<rust::Build, String> {
        Ok(rust::Build {
            profile: self.profile.to_bridge()?,
            linkage: self.linkage.to_bridge()?,
            content: self.content.to_bridge()?,
        })
    }

    /// Destroy this C ABI value.
    pub(crate) fn destroy(&mut self) {
        self.profile.destroy();
        self.linkage.destroy();
        self.content.destroy();
    }

    /// Return one empty C ABI value.
    pub(crate) fn empty() -> Self {
        Self {
            profile: DestackBuildProfile::empty(),
            linkage: DestackBuildLinkage::empty(),
            content: DestackContentId::empty(),
        }
    }
}

impl DestackBuildArray {
    /// Convert bridge values into one C ABI array.
    pub(crate) fn from_bridge(values: Vec<rust::Build>) -> Result<Self, String> {
        let mut converted = Vec::with_capacity(values.len());
        for value in values {
            converted.push(DestackBuild::from_bridge(value)?);
        }
        let (ptr, len) = owned_array(converted);
        Ok(Self { ptr, len })
    }

    /// Convert this C ABI array into bridge values.
    pub(crate) fn to_bridge(&self) -> Result<Vec<rust::Build>, String> {
        if self.len == 0 {
            return Ok(Vec::new());
        }
        if self.ptr.is_null() {
            return Err("array pointer is null".to_string());
        }
        let values = unsafe { std::slice::from_raw_parts(self.ptr, self.len) };
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

impl DestackOptionalBuild {
    /// Convert one optional bridge value into one C ABI optional value.
    pub(crate) fn from_bridge(value: Option<rust::Build>) -> Result<Self, String> {
        let Some(value) = value else {
            return Ok(Self {
                is_some: false,
                value: DestackBuild::empty(),
            });
        };
        Ok(Self {
            is_some: true,
            value: DestackBuild::from_bridge(value)?,
        })
    }

    /// Convert this C ABI optional value into one bridge optional value.
    pub(crate) fn to_bridge(&self) -> Result<Option<rust::Build>, String> {
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
        self.value = DestackBuild::empty();
    }
}

/// C ABI bridge enum.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DestackBundleSection {
    /// Per-module library output.
    Module = 0,
    /// Primary runnable entry output.
    Entry = 1,
    /// Declaration or type surface.
    Declaration = 2,
    /// Asset collection emitted by this target.
    Asset = 3,
    /// Build manifest or output index.
    Manifest = 4,
    /// Source maps or debug maps.
    SourceMap = 5,
    /// Native object or wasm payload.
    Native = 6,
}

/// C ABI bridge enum array.
#[repr(C)]
#[derive(Debug)]
pub struct DestackBundleSectionArray {
    /// Owned value pointer.
    pub(crate) ptr: *mut DestackBundleSection,
    /// Value count.
    pub(crate) len: usize,
}

/// C ABI optional bridge enum.
#[repr(C)]
#[derive(Debug)]
pub struct DestackOptionalBundleSection {
    /// Whether the value is present.
    pub(crate) is_some: bool,
    /// Value when present.
    pub(crate) value: DestackBundleSection,
}

impl DestackBundleSection {
    /// Convert one bridge enum into one C ABI enum.
    pub(crate) fn from_bridge(value: rust::BundleSection) -> Result<Self, String> {
        Ok(match value {
            rust::BundleSection::Module => Self::Module,
            rust::BundleSection::Entry => Self::Entry,
            rust::BundleSection::Declaration => Self::Declaration,
            rust::BundleSection::Asset => Self::Asset,
            rust::BundleSection::Manifest => Self::Manifest,
            rust::BundleSection::SourceMap => Self::SourceMap,
            rust::BundleSection::Native => Self::Native,
        })
    }

    /// Convert this C ABI enum into one bridge enum.
    pub(crate) fn to_bridge(&self) -> Result<rust::BundleSection, String> {
        Ok(match *self {
            Self::Module => rust::BundleSection::Module,
            Self::Entry => rust::BundleSection::Entry,
            Self::Declaration => rust::BundleSection::Declaration,
            Self::Asset => rust::BundleSection::Asset,
            Self::Manifest => rust::BundleSection::Manifest,
            Self::SourceMap => rust::BundleSection::SourceMap,
            Self::Native => rust::BundleSection::Native,
        })
    }

    /// Destroy this C ABI enum.
    pub(crate) fn destroy(&mut self) {}
    /// Return one empty C ABI enum.
    pub(crate) fn empty() -> Self {
        Self::Module
    }
}

impl DestackBundleSectionArray {
    /// Convert bridge values into one C ABI array.
    pub(crate) fn from_bridge(values: Vec<rust::BundleSection>) -> Result<Self, String> {
        let mut converted = Vec::with_capacity(values.len());
        for value in values {
            converted.push(DestackBundleSection::from_bridge(value)?);
        }
        let (ptr, len) = owned_array(converted);
        Ok(Self { ptr, len })
    }

    /// Convert this C ABI array into bridge values.
    pub(crate) fn to_bridge(&self) -> Result<Vec<rust::BundleSection>, String> {
        if self.len == 0 {
            return Ok(Vec::new());
        }
        if self.ptr.is_null() {
            return Err("array pointer is null".to_string());
        }
        let values = unsafe { std::slice::from_raw_parts(self.ptr, self.len) };
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

impl DestackOptionalBundleSection {
    /// Convert one optional bridge value into one C ABI optional value.
    pub(crate) fn from_bridge(value: Option<rust::BundleSection>) -> Result<Self, String> {
        let Some(value) = value else {
            return Ok(Self {
                is_some: false,
                value: DestackBundleSection::empty(),
            });
        };
        Ok(Self {
            is_some: true,
            value: DestackBundleSection::from_bridge(value)?,
        })
    }

    /// Convert this C ABI optional value into one bridge optional value.
    pub(crate) fn to_bridge(&self) -> Result<Option<rust::BundleSection>, String> {
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
        self.value = DestackBundleSection::empty();
    }
}

/// C ABI bridge value.
#[repr(C)]
#[derive(Debug)]
pub struct DestackBundleFile {
    /// The bundle section this file belongs to.
    pub(crate) section: DestackBundleSection,
    /// The output URI.
    pub(crate) uri: *mut c_char,
    /// The emitted file type.
    pub(crate) file_type: DestackFileType,
    /// The output content identity.
    pub(crate) content: DestackContentId,
    /// The related source URI when one exists.
    pub(crate) source: DestackOptionalString,
}

/// C ABI bridge value array.
#[repr(C)]
#[derive(Debug)]
pub struct DestackBundleFileArray {
    /// Owned value pointer.
    pub(crate) ptr: *mut DestackBundleFile,
    /// Value count.
    pub(crate) len: usize,
}

/// C ABI optional bridge value.
#[repr(C)]
#[derive(Debug)]
pub struct DestackOptionalBundleFile {
    /// Whether the value is present.
    pub(crate) is_some: bool,
    /// Value when present.
    pub(crate) value: DestackBundleFile,
}

impl DestackBundleFile {
    /// Convert one bridge value into one C ABI value.
    pub(crate) fn from_bridge(value: rust::BundleFile) -> Result<Self, String> {
        Ok(Self {
            section: DestackBundleSection::from_bridge(value.section)?,
            uri: c_string(value.uri)?,
            file_type: DestackFileType::from_bridge(value.file_type)?,
            content: DestackContentId::from_bridge(value.content)?,
            source: DestackOptionalString::from_bridge(value.source)?,
        })
    }

    /// Convert this C ABI value into one bridge value.
    pub(crate) fn to_bridge(&self) -> Result<rust::BundleFile, String> {
        Ok(rust::BundleFile {
            section: self.section.to_bridge()?,
            uri: read_string(self.uri)?,
            file_type: self.file_type.to_bridge()?,
            content: self.content.to_bridge()?,
            source: self.source.to_bridge()?,
        })
    }

    /// Destroy this C ABI value.
    pub(crate) fn destroy(&mut self) {
        self.section.destroy();
        destroy_string(self.uri);
        self.uri = ptr::null_mut();
        self.file_type.destroy();
        self.content.destroy();
        self.source.destroy();
    }

    /// Return one empty C ABI value.
    pub(crate) fn empty() -> Self {
        Self {
            section: DestackBundleSection::empty(),
            uri: ptr::null_mut(),
            file_type: DestackFileType::empty(),
            content: DestackContentId::empty(),
            source: DestackOptionalString::empty(),
        }
    }
}

impl DestackBundleFileArray {
    /// Convert bridge values into one C ABI array.
    pub(crate) fn from_bridge(values: Vec<rust::BundleFile>) -> Result<Self, String> {
        let mut converted = Vec::with_capacity(values.len());
        for value in values {
            converted.push(DestackBundleFile::from_bridge(value)?);
        }
        let (ptr, len) = owned_array(converted);
        Ok(Self { ptr, len })
    }

    /// Convert this C ABI array into bridge values.
    pub(crate) fn to_bridge(&self) -> Result<Vec<rust::BundleFile>, String> {
        if self.len == 0 {
            return Ok(Vec::new());
        }
        if self.ptr.is_null() {
            return Err("array pointer is null".to_string());
        }
        let values = unsafe { std::slice::from_raw_parts(self.ptr, self.len) };
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

impl DestackOptionalBundleFile {
    /// Convert one optional bridge value into one C ABI optional value.
    pub(crate) fn from_bridge(value: Option<rust::BundleFile>) -> Result<Self, String> {
        let Some(value) = value else {
            return Ok(Self {
                is_some: false,
                value: DestackBundleFile::empty(),
            });
        };
        Ok(Self {
            is_some: true,
            value: DestackBundleFile::from_bridge(value)?,
        })
    }

    /// Convert this C ABI optional value into one bridge optional value.
    pub(crate) fn to_bridge(&self) -> Result<Option<rust::BundleFile>, String> {
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
        self.value = DestackBundleFile::empty();
    }
}

/// C ABI bridge enum.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DestackBundleMode {
    /// Per-module assets without target-level assembly.
    PreserveModules = 0,
    /// One assembled output file.
    SingleFile = 1,
    /// Multiple assembled output files.
    Chunked = 2,
}

/// C ABI bridge enum array.
#[repr(C)]
#[derive(Debug)]
pub struct DestackBundleModeArray {
    /// Owned value pointer.
    pub(crate) ptr: *mut DestackBundleMode,
    /// Value count.
    pub(crate) len: usize,
}

/// C ABI optional bridge enum.
#[repr(C)]
#[derive(Debug)]
pub struct DestackOptionalBundleMode {
    /// Whether the value is present.
    pub(crate) is_some: bool,
    /// Value when present.
    pub(crate) value: DestackBundleMode,
}

impl DestackBundleMode {
    /// Convert one bridge enum into one C ABI enum.
    pub(crate) fn from_bridge(value: rust::BundleMode) -> Result<Self, String> {
        Ok(match value {
            rust::BundleMode::PreserveModules => Self::PreserveModules,
            rust::BundleMode::SingleFile => Self::SingleFile,
            rust::BundleMode::Chunked => Self::Chunked,
        })
    }

    /// Convert this C ABI enum into one bridge enum.
    pub(crate) fn to_bridge(&self) -> Result<rust::BundleMode, String> {
        Ok(match *self {
            Self::PreserveModules => rust::BundleMode::PreserveModules,
            Self::SingleFile => rust::BundleMode::SingleFile,
            Self::Chunked => rust::BundleMode::Chunked,
        })
    }

    /// Destroy this C ABI enum.
    pub(crate) fn destroy(&mut self) {}
    /// Return one empty C ABI enum.
    pub(crate) fn empty() -> Self {
        Self::PreserveModules
    }
}

impl DestackBundleModeArray {
    /// Convert bridge values into one C ABI array.
    pub(crate) fn from_bridge(values: Vec<rust::BundleMode>) -> Result<Self, String> {
        let mut converted = Vec::with_capacity(values.len());
        for value in values {
            converted.push(DestackBundleMode::from_bridge(value)?);
        }
        let (ptr, len) = owned_array(converted);
        Ok(Self { ptr, len })
    }

    /// Convert this C ABI array into bridge values.
    pub(crate) fn to_bridge(&self) -> Result<Vec<rust::BundleMode>, String> {
        if self.len == 0 {
            return Ok(Vec::new());
        }
        if self.ptr.is_null() {
            return Err("array pointer is null".to_string());
        }
        let values = unsafe { std::slice::from_raw_parts(self.ptr, self.len) };
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

impl DestackOptionalBundleMode {
    /// Convert one optional bridge value into one C ABI optional value.
    pub(crate) fn from_bridge(value: Option<rust::BundleMode>) -> Result<Self, String> {
        let Some(value) = value else {
            return Ok(Self {
                is_some: false,
                value: DestackBundleMode::empty(),
            });
        };
        Ok(Self {
            is_some: true,
            value: DestackBundleMode::from_bridge(value)?,
        })
    }

    /// Convert this C ABI optional value into one bridge optional value.
    pub(crate) fn to_bridge(&self) -> Result<Option<rust::BundleMode>, String> {
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
        self.value = DestackBundleMode::empty();
    }
}

/// C ABI bridge enum.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DestackEmitFormat {
    /// JavaScript output.
    Js = 0,
    /// TypeScript output.
    Ts = 1,
    /// WebAssembly output.
    Wasm = 2,
    /// Native binary output.
    Native = 3,
}

/// C ABI bridge enum array.
#[repr(C)]
#[derive(Debug)]
pub struct DestackEmitFormatArray {
    /// Owned value pointer.
    pub(crate) ptr: *mut DestackEmitFormat,
    /// Value count.
    pub(crate) len: usize,
}

/// C ABI optional bridge enum.
#[repr(C)]
#[derive(Debug)]
pub struct DestackOptionalEmitFormat {
    /// Whether the value is present.
    pub(crate) is_some: bool,
    /// Value when present.
    pub(crate) value: DestackEmitFormat,
}

impl DestackEmitFormat {
    /// Convert one bridge enum into one C ABI enum.
    pub(crate) fn from_bridge(value: rust::EmitFormat) -> Result<Self, String> {
        Ok(match value {
            rust::EmitFormat::Js => Self::Js,
            rust::EmitFormat::Ts => Self::Ts,
            rust::EmitFormat::Wasm => Self::Wasm,
            rust::EmitFormat::Native => Self::Native,
        })
    }

    /// Convert this C ABI enum into one bridge enum.
    pub(crate) fn to_bridge(&self) -> Result<rust::EmitFormat, String> {
        Ok(match *self {
            Self::Js => rust::EmitFormat::Js,
            Self::Ts => rust::EmitFormat::Ts,
            Self::Wasm => rust::EmitFormat::Wasm,
            Self::Native => rust::EmitFormat::Native,
        })
    }

    /// Destroy this C ABI enum.
    pub(crate) fn destroy(&mut self) {}
    /// Return one empty C ABI enum.
    pub(crate) fn empty() -> Self {
        Self::Js
    }
}

impl DestackEmitFormatArray {
    /// Convert bridge values into one C ABI array.
    pub(crate) fn from_bridge(values: Vec<rust::EmitFormat>) -> Result<Self, String> {
        let mut converted = Vec::with_capacity(values.len());
        for value in values {
            converted.push(DestackEmitFormat::from_bridge(value)?);
        }
        let (ptr, len) = owned_array(converted);
        Ok(Self { ptr, len })
    }

    /// Convert this C ABI array into bridge values.
    pub(crate) fn to_bridge(&self) -> Result<Vec<rust::EmitFormat>, String> {
        if self.len == 0 {
            return Ok(Vec::new());
        }
        if self.ptr.is_null() {
            return Err("array pointer is null".to_string());
        }
        let values = unsafe { std::slice::from_raw_parts(self.ptr, self.len) };
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

impl DestackOptionalEmitFormat {
    /// Convert one optional bridge value into one C ABI optional value.
    pub(crate) fn from_bridge(value: Option<rust::EmitFormat>) -> Result<Self, String> {
        let Some(value) = value else {
            return Ok(Self {
                is_some: false,
                value: DestackEmitFormat::empty(),
            });
        };
        Ok(Self {
            is_some: true,
            value: DestackEmitFormat::from_bridge(value)?,
        })
    }

    /// Convert this C ABI optional value into one bridge optional value.
    pub(crate) fn to_bridge(&self) -> Result<Option<rust::EmitFormat>, String> {
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
        self.value = DestackEmitFormat::empty();
    }
}

/// C ABI bridge value.
#[repr(C)]
#[derive(Debug)]
pub struct DestackBundle {
    /// The emitted artifact family.
    pub(crate) emit: DestackEmitFormat,
    /// The target-level assembly mode.
    pub(crate) mode: DestackBundleMode,
    /// The files in this bundle.
    pub(crate) files: DestackBundleFileArray,
}

/// C ABI bridge value array.
#[repr(C)]
#[derive(Debug)]
pub struct DestackBundleArray {
    /// Owned value pointer.
    pub(crate) ptr: *mut DestackBundle,
    /// Value count.
    pub(crate) len: usize,
}

/// C ABI optional bridge value.
#[repr(C)]
#[derive(Debug)]
pub struct DestackOptionalBundle {
    /// Whether the value is present.
    pub(crate) is_some: bool,
    /// Value when present.
    pub(crate) value: DestackBundle,
}

impl DestackBundle {
    /// Convert one bridge value into one C ABI value.
    pub(crate) fn from_bridge(value: rust::Bundle) -> Result<Self, String> {
        Ok(Self {
            emit: DestackEmitFormat::from_bridge(value.emit)?,
            mode: DestackBundleMode::from_bridge(value.mode)?,
            files: DestackBundleFileArray::from_bridge(value.files)?,
        })
    }

    /// Convert this C ABI value into one bridge value.
    pub(crate) fn to_bridge(&self) -> Result<rust::Bundle, String> {
        Ok(rust::Bundle {
            emit: self.emit.to_bridge()?,
            mode: self.mode.to_bridge()?,
            files: self.files.to_bridge()?,
        })
    }

    /// Destroy this C ABI value.
    pub(crate) fn destroy(&mut self) {
        self.emit.destroy();
        self.mode.destroy();
        self.files.destroy();
    }

    /// Return one empty C ABI value.
    pub(crate) fn empty() -> Self {
        Self {
            emit: DestackEmitFormat::empty(),
            mode: DestackBundleMode::empty(),
            files: DestackBundleFileArray {
                ptr: ptr::null_mut(),
                len: 0,
            },
        }
    }
}

impl DestackBundleArray {
    /// Convert bridge values into one C ABI array.
    pub(crate) fn from_bridge(values: Vec<rust::Bundle>) -> Result<Self, String> {
        let mut converted = Vec::with_capacity(values.len());
        for value in values {
            converted.push(DestackBundle::from_bridge(value)?);
        }
        let (ptr, len) = owned_array(converted);
        Ok(Self { ptr, len })
    }

    /// Convert this C ABI array into bridge values.
    pub(crate) fn to_bridge(&self) -> Result<Vec<rust::Bundle>, String> {
        if self.len == 0 {
            return Ok(Vec::new());
        }
        if self.ptr.is_null() {
            return Err("array pointer is null".to_string());
        }
        let values = unsafe { std::slice::from_raw_parts(self.ptr, self.len) };
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

impl DestackOptionalBundle {
    /// Convert one optional bridge value into one C ABI optional value.
    pub(crate) fn from_bridge(value: Option<rust::Bundle>) -> Result<Self, String> {
        let Some(value) = value else {
            return Ok(Self {
                is_some: false,
                value: DestackBundle::empty(),
            });
        };
        Ok(Self {
            is_some: true,
            value: DestackBundle::from_bridge(value)?,
        })
    }

    /// Convert this C ABI optional value into one bridge optional value.
    pub(crate) fn to_bridge(&self) -> Result<Option<rust::Bundle>, String> {
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
        self.value = DestackBundle::empty();
    }
}

/// C ABI bridge enum.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DestackObjectFormat {
    /// Native relocatable object file.
    Object = 0,
    /// WebAssembly object or module payload.
    Wasm = 1,
}

/// C ABI bridge enum array.
#[repr(C)]
#[derive(Debug)]
pub struct DestackObjectFormatArray {
    /// Owned value pointer.
    pub(crate) ptr: *mut DestackObjectFormat,
    /// Value count.
    pub(crate) len: usize,
}

/// C ABI optional bridge enum.
#[repr(C)]
#[derive(Debug)]
pub struct DestackOptionalObjectFormat {
    /// Whether the value is present.
    pub(crate) is_some: bool,
    /// Value when present.
    pub(crate) value: DestackObjectFormat,
}

impl DestackObjectFormat {
    /// Convert one bridge enum into one C ABI enum.
    pub(crate) fn from_bridge(value: rust::ObjectFormat) -> Result<Self, String> {
        Ok(match value {
            rust::ObjectFormat::Object => Self::Object,
            rust::ObjectFormat::Wasm => Self::Wasm,
        })
    }

    /// Convert this C ABI enum into one bridge enum.
    pub(crate) fn to_bridge(&self) -> Result<rust::ObjectFormat, String> {
        Ok(match *self {
            Self::Object => rust::ObjectFormat::Object,
            Self::Wasm => rust::ObjectFormat::Wasm,
        })
    }

    /// Destroy this C ABI enum.
    pub(crate) fn destroy(&mut self) {}
    /// Return one empty C ABI enum.
    pub(crate) fn empty() -> Self {
        Self::Object
    }
}

impl DestackObjectFormatArray {
    /// Convert bridge values into one C ABI array.
    pub(crate) fn from_bridge(values: Vec<rust::ObjectFormat>) -> Result<Self, String> {
        let mut converted = Vec::with_capacity(values.len());
        for value in values {
            converted.push(DestackObjectFormat::from_bridge(value)?);
        }
        let (ptr, len) = owned_array(converted);
        Ok(Self { ptr, len })
    }

    /// Convert this C ABI array into bridge values.
    pub(crate) fn to_bridge(&self) -> Result<Vec<rust::ObjectFormat>, String> {
        if self.len == 0 {
            return Ok(Vec::new());
        }
        if self.ptr.is_null() {
            return Err("array pointer is null".to_string());
        }
        let values = unsafe { std::slice::from_raw_parts(self.ptr, self.len) };
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

impl DestackOptionalObjectFormat {
    /// Convert one optional bridge value into one C ABI optional value.
    pub(crate) fn from_bridge(value: Option<rust::ObjectFormat>) -> Result<Self, String> {
        let Some(value) = value else {
            return Ok(Self {
                is_some: false,
                value: DestackObjectFormat::empty(),
            });
        };
        Ok(Self {
            is_some: true,
            value: DestackObjectFormat::from_bridge(value)?,
        })
    }

    /// Convert this C ABI optional value into one bridge optional value.
    pub(crate) fn to_bridge(&self) -> Result<Option<rust::ObjectFormat>, String> {
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
        self.value = DestackObjectFormat::empty();
    }
}

/// C ABI bridge value.
#[repr(C)]
#[derive(Debug)]
pub struct DestackObject {
    /// The compiled-code object format.
    pub(crate) format: DestackObjectFormat,
    /// The encoded object content identity.
    pub(crate) content: DestackContentId,
    /// The source map when one exists.
    pub(crate) map: DestackOptionalSourceMap,
}

/// C ABI bridge value array.
#[repr(C)]
#[derive(Debug)]
pub struct DestackObjectArray {
    /// Owned value pointer.
    pub(crate) ptr: *mut DestackObject,
    /// Value count.
    pub(crate) len: usize,
}

/// C ABI optional bridge value.
#[repr(C)]
#[derive(Debug)]
pub struct DestackOptionalObject {
    /// Whether the value is present.
    pub(crate) is_some: bool,
    /// Value when present.
    pub(crate) value: DestackObject,
}

impl DestackObject {
    /// Convert one bridge value into one C ABI value.
    pub(crate) fn from_bridge(value: rust::Object) -> Result<Self, String> {
        Ok(Self {
            format: DestackObjectFormat::from_bridge(value.format)?,
            content: DestackContentId::from_bridge(value.content)?,
            map: DestackOptionalSourceMap::from_bridge(value.map)?,
        })
    }

    /// Convert this C ABI value into one bridge value.
    pub(crate) fn to_bridge(&self) -> Result<rust::Object, String> {
        Ok(rust::Object {
            format: self.format.to_bridge()?,
            content: self.content.to_bridge()?,
            map: self.map.to_bridge()?,
        })
    }

    /// Destroy this C ABI value.
    pub(crate) fn destroy(&mut self) {
        self.format.destroy();
        self.content.destroy();
        self.map.destroy();
    }

    /// Return one empty C ABI value.
    pub(crate) fn empty() -> Self {
        Self {
            format: DestackObjectFormat::empty(),
            content: DestackContentId::empty(),
            map: DestackOptionalSourceMap {
                is_some: false,
                value: DestackSourceMap::empty(),
            },
        }
    }
}

impl DestackObjectArray {
    /// Convert bridge values into one C ABI array.
    pub(crate) fn from_bridge(values: Vec<rust::Object>) -> Result<Self, String> {
        let mut converted = Vec::with_capacity(values.len());
        for value in values {
            converted.push(DestackObject::from_bridge(value)?);
        }
        let (ptr, len) = owned_array(converted);
        Ok(Self { ptr, len })
    }

    /// Convert this C ABI array into bridge values.
    pub(crate) fn to_bridge(&self) -> Result<Vec<rust::Object>, String> {
        if self.len == 0 {
            return Ok(Vec::new());
        }
        if self.ptr.is_null() {
            return Err("array pointer is null".to_string());
        }
        let values = unsafe { std::slice::from_raw_parts(self.ptr, self.len) };
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

impl DestackOptionalObject {
    /// Convert one optional bridge value into one C ABI optional value.
    pub(crate) fn from_bridge(value: Option<rust::Object>) -> Result<Self, String> {
        let Some(value) = value else {
            return Ok(Self {
                is_some: false,
                value: DestackObject::empty(),
            });
        };
        Ok(Self {
            is_some: true,
            value: DestackObject::from_bridge(value)?,
        })
    }

    /// Convert this C ABI optional value into one bridge optional value.
    pub(crate) fn to_bridge(&self) -> Result<Option<rust::Object>, String> {
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
        self.value = DestackObject::empty();
    }
}

/// C ABI bridge enum.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DestackHost {
    /// Native host environment.
    Native = 0,
    /// Browser host environment.
    Browser = 1,
    /// WASI host environment.
    Wasi = 2,
    /// Emscripten host environment.
    Emscripten = 3,
    /// Freestanding target without host imports.
    Freestanding = 4,
}

/// C ABI bridge enum array.
#[repr(C)]
#[derive(Debug)]
pub struct DestackHostArray {
    /// Owned value pointer.
    pub(crate) ptr: *mut DestackHost,
    /// Value count.
    pub(crate) len: usize,
}

/// C ABI optional bridge enum.
#[repr(C)]
#[derive(Debug)]
pub struct DestackOptionalHost {
    /// Whether the value is present.
    pub(crate) is_some: bool,
    /// Value when present.
    pub(crate) value: DestackHost,
}

impl DestackHost {
    /// Convert one bridge enum into one C ABI enum.
    pub(crate) fn from_bridge(value: rust::Host) -> Result<Self, String> {
        Ok(match value {
            rust::Host::Native => Self::Native,
            rust::Host::Browser => Self::Browser,
            rust::Host::Wasi => Self::Wasi,
            rust::Host::Emscripten => Self::Emscripten,
            rust::Host::Freestanding => Self::Freestanding,
        })
    }

    /// Convert this C ABI enum into one bridge enum.
    pub(crate) fn to_bridge(&self) -> Result<rust::Host, String> {
        Ok(match *self {
            Self::Native => rust::Host::Native,
            Self::Browser => rust::Host::Browser,
            Self::Wasi => rust::Host::Wasi,
            Self::Emscripten => rust::Host::Emscripten,
            Self::Freestanding => rust::Host::Freestanding,
        })
    }

    /// Destroy this C ABI enum.
    pub(crate) fn destroy(&mut self) {}
    /// Return one empty C ABI enum.
    pub(crate) fn empty() -> Self {
        Self::Native
    }
}

impl DestackHostArray {
    /// Convert bridge values into one C ABI array.
    pub(crate) fn from_bridge(values: Vec<rust::Host>) -> Result<Self, String> {
        let mut converted = Vec::with_capacity(values.len());
        for value in values {
            converted.push(DestackHost::from_bridge(value)?);
        }
        let (ptr, len) = owned_array(converted);
        Ok(Self { ptr, len })
    }

    /// Convert this C ABI array into bridge values.
    pub(crate) fn to_bridge(&self) -> Result<Vec<rust::Host>, String> {
        if self.len == 0 {
            return Ok(Vec::new());
        }
        if self.ptr.is_null() {
            return Err("array pointer is null".to_string());
        }
        let values = unsafe { std::slice::from_raw_parts(self.ptr, self.len) };
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

impl DestackOptionalHost {
    /// Convert one optional bridge value into one C ABI optional value.
    pub(crate) fn from_bridge(value: Option<rust::Host>) -> Result<Self, String> {
        let Some(value) = value else {
            return Ok(Self {
                is_some: false,
                value: DestackHost::empty(),
            });
        };
        Ok(Self {
            is_some: true,
            value: DestackHost::from_bridge(value)?,
        })
    }

    /// Convert this C ABI optional value into one bridge optional value.
    pub(crate) fn to_bridge(&self) -> Result<Option<rust::Host>, String> {
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
        self.value = DestackHost::empty();
    }
}

/// C ABI bridge enum.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DestackRuntime {
    /// Destack native runtime.
    Destack = 0,
    /// JavaScript host runtime.
    Js = 1,
}

/// C ABI bridge enum array.
#[repr(C)]
#[derive(Debug)]
pub struct DestackRuntimeArray {
    /// Owned value pointer.
    pub(crate) ptr: *mut DestackRuntime,
    /// Value count.
    pub(crate) len: usize,
}

/// C ABI optional bridge enum.
#[repr(C)]
#[derive(Debug)]
pub struct DestackOptionalRuntime {
    /// Whether the value is present.
    pub(crate) is_some: bool,
    /// Value when present.
    pub(crate) value: DestackRuntime,
}

impl DestackRuntime {
    /// Convert one bridge enum into one C ABI enum.
    pub(crate) fn from_bridge(value: rust::Runtime) -> Result<Self, String> {
        Ok(match value {
            rust::Runtime::Destack => Self::Destack,
            rust::Runtime::Js => Self::Js,
        })
    }

    /// Convert this C ABI enum into one bridge enum.
    pub(crate) fn to_bridge(&self) -> Result<rust::Runtime, String> {
        Ok(match *self {
            Self::Destack => rust::Runtime::Destack,
            Self::Js => rust::Runtime::Js,
        })
    }

    /// Destroy this C ABI enum.
    pub(crate) fn destroy(&mut self) {}
    /// Return one empty C ABI enum.
    pub(crate) fn empty() -> Self {
        Self::Destack
    }
}

impl DestackRuntimeArray {
    /// Convert bridge values into one C ABI array.
    pub(crate) fn from_bridge(values: Vec<rust::Runtime>) -> Result<Self, String> {
        let mut converted = Vec::with_capacity(values.len());
        for value in values {
            converted.push(DestackRuntime::from_bridge(value)?);
        }
        let (ptr, len) = owned_array(converted);
        Ok(Self { ptr, len })
    }

    /// Convert this C ABI array into bridge values.
    pub(crate) fn to_bridge(&self) -> Result<Vec<rust::Runtime>, String> {
        if self.len == 0 {
            return Ok(Vec::new());
        }
        if self.ptr.is_null() {
            return Err("array pointer is null".to_string());
        }
        let values = unsafe { std::slice::from_raw_parts(self.ptr, self.len) };
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

impl DestackOptionalRuntime {
    /// Convert one optional bridge value into one C ABI optional value.
    pub(crate) fn from_bridge(value: Option<rust::Runtime>) -> Result<Self, String> {
        let Some(value) = value else {
            return Ok(Self {
                is_some: false,
                value: DestackRuntime::empty(),
            });
        };
        Ok(Self {
            is_some: true,
            value: DestackRuntime::from_bridge(value)?,
        })
    }

    /// Convert this C ABI optional value into one bridge optional value.
    pub(crate) fn to_bridge(&self) -> Result<Option<rust::Runtime>, String> {
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
        self.value = DestackRuntime::empty();
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
    pub(crate) fn from_bridge(value: rust::PackageId) -> Result<Self, String> {
        Ok(Self {
            id: c_string(value.id)?,
        })
    }

    /// Convert this C ABI value into one bridge value.
    pub(crate) fn to_bridge(&self) -> Result<rust::PackageId, String> {
        Ok(rust::PackageId {
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
    pub(crate) fn from_bridge(values: Vec<rust::PackageId>) -> Result<Self, String> {
        let mut converted = Vec::with_capacity(values.len());
        for value in values {
            converted.push(DestackPackageId::from_bridge(value)?);
        }
        let (ptr, len) = owned_array(converted);
        Ok(Self { ptr, len })
    }

    /// Convert this C ABI array into bridge values.
    pub(crate) fn to_bridge(&self) -> Result<Vec<rust::PackageId>, String> {
        if self.len == 0 {
            return Ok(Vec::new());
        }
        if self.ptr.is_null() {
            return Err("array pointer is null".to_string());
        }
        let values = unsafe { std::slice::from_raw_parts(self.ptr, self.len) };
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
    pub(crate) fn from_bridge(value: Option<rust::PackageId>) -> Result<Self, String> {
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
    pub(crate) fn to_bridge(&self) -> Result<Option<rust::PackageId>, String> {
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
    pub(crate) fn from_bridge(value: rust::TargetId) -> Result<Self, String> {
        Ok(Self {
            package: DestackPackageId::from_bridge(value.package)?,
            key: c_string(value.key)?,
        })
    }

    /// Convert this C ABI value into one bridge value.
    pub(crate) fn to_bridge(&self) -> Result<rust::TargetId, String> {
        Ok(rust::TargetId {
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
    pub(crate) fn from_bridge(values: Vec<rust::TargetId>) -> Result<Self, String> {
        let mut converted = Vec::with_capacity(values.len());
        for value in values {
            converted.push(DestackTargetId::from_bridge(value)?);
        }
        let (ptr, len) = owned_array(converted);
        Ok(Self { ptr, len })
    }

    /// Convert this C ABI array into bridge values.
    pub(crate) fn to_bridge(&self) -> Result<Vec<rust::TargetId>, String> {
        if self.len == 0 {
            return Ok(Vec::new());
        }
        if self.ptr.is_null() {
            return Err("array pointer is null".to_string());
        }
        let values = unsafe { std::slice::from_raw_parts(self.ptr, self.len) };
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
    pub(crate) fn from_bridge(value: Option<rust::TargetId>) -> Result<Self, String> {
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
    pub(crate) fn to_bridge(&self) -> Result<Option<rust::TargetId>, String> {
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

/// C ABI bridge value.
#[repr(C)]
#[derive(Debug)]
pub struct DestackProductTarget {
    /// The configured product target name.
    pub(crate) name: *mut c_char,
    /// The repository target assembled into this product.
    pub(crate) target: DestackTargetId,
    /// The runtime contract this target expects.
    pub(crate) runtime: DestackRuntime,
    /// The host environment this target expects.
    pub(crate) host: DestackHost,
    /// The platform this target expects.
    pub(crate) platform: *mut c_char,
    /// Whether this product target includes its toolchain build payload.
    pub(crate) includes_build: bool,
    /// Whether this product target includes its linked bundle.
    pub(crate) includes_bundle: bool,
    /// Whether this product target includes its executable program.
    pub(crate) includes_program: bool,
}

/// C ABI bridge value array.
#[repr(C)]
#[derive(Debug)]
pub struct DestackProductTargetArray {
    /// Owned value pointer.
    pub(crate) ptr: *mut DestackProductTarget,
    /// Value count.
    pub(crate) len: usize,
}

/// C ABI optional bridge value.
#[repr(C)]
#[derive(Debug)]
pub struct DestackOptionalProductTarget {
    /// Whether the value is present.
    pub(crate) is_some: bool,
    /// Value when present.
    pub(crate) value: DestackProductTarget,
}

impl DestackProductTarget {
    /// Convert one bridge value into one C ABI value.
    pub(crate) fn from_bridge(value: rust::ProductTarget) -> Result<Self, String> {
        Ok(Self {
            name: c_string(value.name)?,
            target: DestackTargetId::from_bridge(value.target)?,
            runtime: DestackRuntime::from_bridge(value.runtime)?,
            host: DestackHost::from_bridge(value.host)?,
            platform: c_string(value.platform)?,
            includes_build: value.includes_build,
            includes_bundle: value.includes_bundle,
            includes_program: value.includes_program,
        })
    }

    /// Convert this C ABI value into one bridge value.
    pub(crate) fn to_bridge(&self) -> Result<rust::ProductTarget, String> {
        Ok(rust::ProductTarget {
            name: read_string(self.name)?,
            target: self.target.to_bridge()?,
            runtime: self.runtime.to_bridge()?,
            host: self.host.to_bridge()?,
            platform: read_string(self.platform)?,
            includes_build: self.includes_build,
            includes_bundle: self.includes_bundle,
            includes_program: self.includes_program,
        })
    }

    /// Destroy this C ABI value.
    pub(crate) fn destroy(&mut self) {
        destroy_string(self.name);
        self.name = ptr::null_mut();
        self.target.destroy();
        self.runtime.destroy();
        self.host.destroy();
        destroy_string(self.platform);
        self.platform = ptr::null_mut();
    }

    /// Return one empty C ABI value.
    pub(crate) fn empty() -> Self {
        Self {
            name: ptr::null_mut(),
            target: DestackTargetId::empty(),
            runtime: DestackRuntime::empty(),
            host: DestackHost::empty(),
            platform: ptr::null_mut(),
            includes_build: false,
            includes_bundle: false,
            includes_program: false,
        }
    }
}

impl DestackProductTargetArray {
    /// Convert bridge values into one C ABI array.
    pub(crate) fn from_bridge(values: Vec<rust::ProductTarget>) -> Result<Self, String> {
        let mut converted = Vec::with_capacity(values.len());
        for value in values {
            converted.push(DestackProductTarget::from_bridge(value)?);
        }
        let (ptr, len) = owned_array(converted);
        Ok(Self { ptr, len })
    }

    /// Convert this C ABI array into bridge values.
    pub(crate) fn to_bridge(&self) -> Result<Vec<rust::ProductTarget>, String> {
        if self.len == 0 {
            return Ok(Vec::new());
        }
        if self.ptr.is_null() {
            return Err("array pointer is null".to_string());
        }
        let values = unsafe { std::slice::from_raw_parts(self.ptr, self.len) };
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

impl DestackOptionalProductTarget {
    /// Convert one optional bridge value into one C ABI optional value.
    pub(crate) fn from_bridge(value: Option<rust::ProductTarget>) -> Result<Self, String> {
        let Some(value) = value else {
            return Ok(Self {
                is_some: false,
                value: DestackProductTarget::empty(),
            });
        };
        Ok(Self {
            is_some: true,
            value: DestackProductTarget::from_bridge(value)?,
        })
    }

    /// Convert this C ABI optional value into one bridge optional value.
    pub(crate) fn to_bridge(&self) -> Result<Option<rust::ProductTarget>, String> {
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
        self.value = DestackProductTarget::empty();
    }
}

/// C ABI bridge value.
#[repr(C)]
#[derive(Debug)]
pub struct DestackProduct {
    /// The configured product name.
    pub(crate) name: *mut c_char,
    /// The linked targets in deterministic order.
    pub(crate) targets: DestackProductTargetArray,
}

/// C ABI bridge value array.
#[repr(C)]
#[derive(Debug)]
pub struct DestackProductArray {
    /// Owned value pointer.
    pub(crate) ptr: *mut DestackProduct,
    /// Value count.
    pub(crate) len: usize,
}

/// C ABI optional bridge value.
#[repr(C)]
#[derive(Debug)]
pub struct DestackOptionalProduct {
    /// Whether the value is present.
    pub(crate) is_some: bool,
    /// Value when present.
    pub(crate) value: DestackProduct,
}

impl DestackProduct {
    /// Convert one bridge value into one C ABI value.
    pub(crate) fn from_bridge(value: rust::Product) -> Result<Self, String> {
        Ok(Self {
            name: c_string(value.name)?,
            targets: DestackProductTargetArray::from_bridge(value.targets)?,
        })
    }

    /// Convert this C ABI value into one bridge value.
    pub(crate) fn to_bridge(&self) -> Result<rust::Product, String> {
        Ok(rust::Product {
            name: read_string(self.name)?,
            targets: self.targets.to_bridge()?,
        })
    }

    /// Destroy this C ABI value.
    pub(crate) fn destroy(&mut self) {
        destroy_string(self.name);
        self.name = ptr::null_mut();
        self.targets.destroy();
    }

    /// Return one empty C ABI value.
    pub(crate) fn empty() -> Self {
        Self {
            name: ptr::null_mut(),
            targets: DestackProductTargetArray {
                ptr: ptr::null_mut(),
                len: 0,
            },
        }
    }
}

impl DestackProductArray {
    /// Convert bridge values into one C ABI array.
    pub(crate) fn from_bridge(values: Vec<rust::Product>) -> Result<Self, String> {
        let mut converted = Vec::with_capacity(values.len());
        for value in values {
            converted.push(DestackProduct::from_bridge(value)?);
        }
        let (ptr, len) = owned_array(converted);
        Ok(Self { ptr, len })
    }

    /// Convert this C ABI array into bridge values.
    pub(crate) fn to_bridge(&self) -> Result<Vec<rust::Product>, String> {
        if self.len == 0 {
            return Ok(Vec::new());
        }
        if self.ptr.is_null() {
            return Err("array pointer is null".to_string());
        }
        let values = unsafe { std::slice::from_raw_parts(self.ptr, self.len) };
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

impl DestackOptionalProduct {
    /// Convert one optional bridge value into one C ABI optional value.
    pub(crate) fn from_bridge(value: Option<rust::Product>) -> Result<Self, String> {
        let Some(value) = value else {
            return Ok(Self {
                is_some: false,
                value: DestackProduct::empty(),
            });
        };
        Ok(Self {
            is_some: true,
            value: DestackProduct::from_bridge(value)?,
        })
    }

    /// Convert this C ABI optional value into one bridge optional value.
    pub(crate) fn to_bridge(&self) -> Result<Option<rust::Product>, String> {
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
        self.value = DestackProduct::empty();
    }
}

/// C ABI bridge enum.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DestackProgramFormat {
    /// VM executable program.
    Vm = 0,
    /// Native executable program.
    Native = 1,
}

/// C ABI bridge enum array.
#[repr(C)]
#[derive(Debug)]
pub struct DestackProgramFormatArray {
    /// Owned value pointer.
    pub(crate) ptr: *mut DestackProgramFormat,
    /// Value count.
    pub(crate) len: usize,
}

/// C ABI optional bridge enum.
#[repr(C)]
#[derive(Debug)]
pub struct DestackOptionalProgramFormat {
    /// Whether the value is present.
    pub(crate) is_some: bool,
    /// Value when present.
    pub(crate) value: DestackProgramFormat,
}

impl DestackProgramFormat {
    /// Convert one bridge enum into one C ABI enum.
    pub(crate) fn from_bridge(value: rust::ProgramFormat) -> Result<Self, String> {
        Ok(match value {
            rust::ProgramFormat::Vm => Self::Vm,
            rust::ProgramFormat::Native => Self::Native,
        })
    }

    /// Convert this C ABI enum into one bridge enum.
    pub(crate) fn to_bridge(&self) -> Result<rust::ProgramFormat, String> {
        Ok(match *self {
            Self::Vm => rust::ProgramFormat::Vm,
            Self::Native => rust::ProgramFormat::Native,
        })
    }

    /// Destroy this C ABI enum.
    pub(crate) fn destroy(&mut self) {}
    /// Return one empty C ABI enum.
    pub(crate) fn empty() -> Self {
        Self::Vm
    }
}

impl DestackProgramFormatArray {
    /// Convert bridge values into one C ABI array.
    pub(crate) fn from_bridge(values: Vec<rust::ProgramFormat>) -> Result<Self, String> {
        let mut converted = Vec::with_capacity(values.len());
        for value in values {
            converted.push(DestackProgramFormat::from_bridge(value)?);
        }
        let (ptr, len) = owned_array(converted);
        Ok(Self { ptr, len })
    }

    /// Convert this C ABI array into bridge values.
    pub(crate) fn to_bridge(&self) -> Result<Vec<rust::ProgramFormat>, String> {
        if self.len == 0 {
            return Ok(Vec::new());
        }
        if self.ptr.is_null() {
            return Err("array pointer is null".to_string());
        }
        let values = unsafe { std::slice::from_raw_parts(self.ptr, self.len) };
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

impl DestackOptionalProgramFormat {
    /// Convert one optional bridge value into one C ABI optional value.
    pub(crate) fn from_bridge(value: Option<rust::ProgramFormat>) -> Result<Self, String> {
        let Some(value) = value else {
            return Ok(Self {
                is_some: false,
                value: DestackProgramFormat::empty(),
            });
        };
        Ok(Self {
            is_some: true,
            value: DestackProgramFormat::from_bridge(value)?,
        })
    }

    /// Convert this C ABI optional value into one bridge optional value.
    pub(crate) fn to_bridge(&self) -> Result<Option<rust::ProgramFormat>, String> {
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
        self.value = DestackProgramFormat::empty();
    }
}

/// C ABI bridge value.
#[repr(C)]
#[derive(Debug)]
pub struct DestackProgramHeader {
    /// Human-facing program name.
    pub(crate) name: DestackOptionalString,
    /// Build fingerprint that produced this program.
    pub(crate) fingerprint: DestackOptionalString,
    /// Target triple or equivalent target identity.
    pub(crate) target: DestackOptionalString,
}

/// C ABI bridge value array.
#[repr(C)]
#[derive(Debug)]
pub struct DestackProgramHeaderArray {
    /// Owned value pointer.
    pub(crate) ptr: *mut DestackProgramHeader,
    /// Value count.
    pub(crate) len: usize,
}

/// C ABI optional bridge value.
#[repr(C)]
#[derive(Debug)]
pub struct DestackOptionalProgramHeader {
    /// Whether the value is present.
    pub(crate) is_some: bool,
    /// Value when present.
    pub(crate) value: DestackProgramHeader,
}

impl DestackProgramHeader {
    /// Convert one bridge value into one C ABI value.
    pub(crate) fn from_bridge(value: rust::ProgramHeader) -> Result<Self, String> {
        Ok(Self {
            name: DestackOptionalString::from_bridge(value.name)?,
            fingerprint: DestackOptionalString::from_bridge(value.fingerprint)?,
            target: DestackOptionalString::from_bridge(value.target)?,
        })
    }

    /// Convert this C ABI value into one bridge value.
    pub(crate) fn to_bridge(&self) -> Result<rust::ProgramHeader, String> {
        Ok(rust::ProgramHeader {
            name: self.name.to_bridge()?,
            fingerprint: self.fingerprint.to_bridge()?,
            target: self.target.to_bridge()?,
        })
    }

    /// Destroy this C ABI value.
    pub(crate) fn destroy(&mut self) {
        self.name.destroy();
        self.fingerprint.destroy();
        self.target.destroy();
    }

    /// Return one empty C ABI value.
    pub(crate) fn empty() -> Self {
        Self {
            name: DestackOptionalString::empty(),
            fingerprint: DestackOptionalString::empty(),
            target: DestackOptionalString::empty(),
        }
    }
}

impl DestackProgramHeaderArray {
    /// Convert bridge values into one C ABI array.
    pub(crate) fn from_bridge(values: Vec<rust::ProgramHeader>) -> Result<Self, String> {
        let mut converted = Vec::with_capacity(values.len());
        for value in values {
            converted.push(DestackProgramHeader::from_bridge(value)?);
        }
        let (ptr, len) = owned_array(converted);
        Ok(Self { ptr, len })
    }

    /// Convert this C ABI array into bridge values.
    pub(crate) fn to_bridge(&self) -> Result<Vec<rust::ProgramHeader>, String> {
        if self.len == 0 {
            return Ok(Vec::new());
        }
        if self.ptr.is_null() {
            return Err("array pointer is null".to_string());
        }
        let values = unsafe { std::slice::from_raw_parts(self.ptr, self.len) };
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

impl DestackOptionalProgramHeader {
    /// Convert one optional bridge value into one C ABI optional value.
    pub(crate) fn from_bridge(value: Option<rust::ProgramHeader>) -> Result<Self, String> {
        let Some(value) = value else {
            return Ok(Self {
                is_some: false,
                value: DestackProgramHeader::empty(),
            });
        };
        Ok(Self {
            is_some: true,
            value: DestackProgramHeader::from_bridge(value)?,
        })
    }

    /// Convert this C ABI optional value into one bridge optional value.
    pub(crate) fn to_bridge(&self) -> Result<Option<rust::ProgramHeader>, String> {
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
        self.value = DestackProgramHeader::empty();
    }
}

/// C ABI bridge value.
#[repr(C)]
#[derive(Debug)]
pub struct DestackProgram {
    /// The program identity and compatibility header.
    pub(crate) header: DestackProgramHeader,
    /// The executable format.
    pub(crate) format: DestackProgramFormat,
    /// Content blobs referenced by the executable payload.
    pub(crate) contents: DestackContentIdArray,
}

/// C ABI bridge value array.
#[repr(C)]
#[derive(Debug)]
pub struct DestackProgramArray {
    /// Owned value pointer.
    pub(crate) ptr: *mut DestackProgram,
    /// Value count.
    pub(crate) len: usize,
}

/// C ABI optional bridge value.
#[repr(C)]
#[derive(Debug)]
pub struct DestackOptionalProgram {
    /// Whether the value is present.
    pub(crate) is_some: bool,
    /// Value when present.
    pub(crate) value: DestackProgram,
}

impl DestackProgram {
    /// Convert one bridge value into one C ABI value.
    pub(crate) fn from_bridge(value: rust::Program) -> Result<Self, String> {
        Ok(Self {
            header: DestackProgramHeader::from_bridge(value.header)?,
            format: DestackProgramFormat::from_bridge(value.format)?,
            contents: DestackContentIdArray::from_bridge(value.contents)?,
        })
    }

    /// Convert this C ABI value into one bridge value.
    pub(crate) fn to_bridge(&self) -> Result<rust::Program, String> {
        Ok(rust::Program {
            header: self.header.to_bridge()?,
            format: self.format.to_bridge()?,
            contents: self.contents.to_bridge()?,
        })
    }

    /// Destroy this C ABI value.
    pub(crate) fn destroy(&mut self) {
        self.header.destroy();
        self.format.destroy();
        self.contents.destroy();
    }

    /// Return one empty C ABI value.
    pub(crate) fn empty() -> Self {
        Self {
            header: DestackProgramHeader::empty(),
            format: DestackProgramFormat::empty(),
            contents: DestackContentIdArray {
                ptr: ptr::null_mut(),
                len: 0,
            },
        }
    }
}

impl DestackProgramArray {
    /// Convert bridge values into one C ABI array.
    pub(crate) fn from_bridge(values: Vec<rust::Program>) -> Result<Self, String> {
        let mut converted = Vec::with_capacity(values.len());
        for value in values {
            converted.push(DestackProgram::from_bridge(value)?);
        }
        let (ptr, len) = owned_array(converted);
        Ok(Self { ptr, len })
    }

    /// Convert this C ABI array into bridge values.
    pub(crate) fn to_bridge(&self) -> Result<Vec<rust::Program>, String> {
        if self.len == 0 {
            return Ok(Vec::new());
        }
        if self.ptr.is_null() {
            return Err("array pointer is null".to_string());
        }
        let values = unsafe { std::slice::from_raw_parts(self.ptr, self.len) };
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

impl DestackOptionalProgram {
    /// Convert one optional bridge value into one C ABI optional value.
    pub(crate) fn from_bridge(value: Option<rust::Program>) -> Result<Self, String> {
        let Some(value) = value else {
            return Ok(Self {
                is_some: false,
                value: DestackProgram::empty(),
            });
        };
        Ok(Self {
            is_some: true,
            value: DestackProgram::from_bridge(value)?,
        })
    }

    /// Convert this C ABI optional value into one bridge optional value.
    pub(crate) fn to_bridge(&self) -> Result<Option<rust::Program>, String> {
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
        self.value = DestackProgram::empty();
    }
}

/// C ABI bridge value.
#[repr(C)]
#[derive(Debug)]
pub struct DestackDeclaration {
    /// The declaration text.
    pub(crate) text: *mut c_char,
}

/// C ABI bridge value array.
#[repr(C)]
#[derive(Debug)]
pub struct DestackDeclarationArray {
    /// Owned value pointer.
    pub(crate) ptr: *mut DestackDeclaration,
    /// Value count.
    pub(crate) len: usize,
}

/// C ABI optional bridge value.
#[repr(C)]
#[derive(Debug)]
pub struct DestackOptionalDeclaration {
    /// Whether the value is present.
    pub(crate) is_some: bool,
    /// Value when present.
    pub(crate) value: DestackDeclaration,
}

impl DestackDeclaration {
    /// Convert one bridge value into one C ABI value.
    pub(crate) fn from_bridge(value: rust::Declaration) -> Result<Self, String> {
        Ok(Self {
            text: c_string(value.text)?,
        })
    }

    /// Convert this C ABI value into one bridge value.
    pub(crate) fn to_bridge(&self) -> Result<rust::Declaration, String> {
        Ok(rust::Declaration {
            text: read_string(self.text)?,
        })
    }

    /// Destroy this C ABI value.
    pub(crate) fn destroy(&mut self) {
        destroy_string(self.text);
        self.text = ptr::null_mut();
    }

    /// Return one empty C ABI value.
    pub(crate) fn empty() -> Self {
        Self {
            text: ptr::null_mut(),
        }
    }
}

impl DestackDeclarationArray {
    /// Convert bridge values into one C ABI array.
    pub(crate) fn from_bridge(values: Vec<rust::Declaration>) -> Result<Self, String> {
        let mut converted = Vec::with_capacity(values.len());
        for value in values {
            converted.push(DestackDeclaration::from_bridge(value)?);
        }
        let (ptr, len) = owned_array(converted);
        Ok(Self { ptr, len })
    }

    /// Convert this C ABI array into bridge values.
    pub(crate) fn to_bridge(&self) -> Result<Vec<rust::Declaration>, String> {
        if self.len == 0 {
            return Ok(Vec::new());
        }
        if self.ptr.is_null() {
            return Err("array pointer is null".to_string());
        }
        let values = unsafe { std::slice::from_raw_parts(self.ptr, self.len) };
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

impl DestackOptionalDeclaration {
    /// Convert one optional bridge value into one C ABI optional value.
    pub(crate) fn from_bridge(value: Option<rust::Declaration>) -> Result<Self, String> {
        let Some(value) = value else {
            return Ok(Self {
                is_some: false,
                value: DestackDeclaration::empty(),
            });
        };
        Ok(Self {
            is_some: true,
            value: DestackDeclaration::from_bridge(value)?,
        })
    }

    /// Convert this C ABI optional value into one bridge optional value.
    pub(crate) fn to_bridge(&self) -> Result<Option<rust::Declaration>, String> {
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
        self.value = DestackDeclaration::empty();
    }
}

/// C ABI bridge enum.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DestackScriptLanguage {
    /// JavaScript output.
    JavaScript = 0,
    /// TypeScript output.
    TypeScript = 1,
}

/// C ABI bridge enum array.
#[repr(C)]
#[derive(Debug)]
pub struct DestackScriptLanguageArray {
    /// Owned value pointer.
    pub(crate) ptr: *mut DestackScriptLanguage,
    /// Value count.
    pub(crate) len: usize,
}

/// C ABI optional bridge enum.
#[repr(C)]
#[derive(Debug)]
pub struct DestackOptionalScriptLanguage {
    /// Whether the value is present.
    pub(crate) is_some: bool,
    /// Value when present.
    pub(crate) value: DestackScriptLanguage,
}

impl DestackScriptLanguage {
    /// Convert one bridge enum into one C ABI enum.
    pub(crate) fn from_bridge(value: rust::ScriptLanguage) -> Result<Self, String> {
        Ok(match value {
            rust::ScriptLanguage::JavaScript => Self::JavaScript,
            rust::ScriptLanguage::TypeScript => Self::TypeScript,
        })
    }

    /// Convert this C ABI enum into one bridge enum.
    pub(crate) fn to_bridge(&self) -> Result<rust::ScriptLanguage, String> {
        Ok(match *self {
            Self::JavaScript => rust::ScriptLanguage::JavaScript,
            Self::TypeScript => rust::ScriptLanguage::TypeScript,
        })
    }

    /// Destroy this C ABI enum.
    pub(crate) fn destroy(&mut self) {}
    /// Return one empty C ABI enum.
    pub(crate) fn empty() -> Self {
        Self::JavaScript
    }
}

impl DestackScriptLanguageArray {
    /// Convert bridge values into one C ABI array.
    pub(crate) fn from_bridge(values: Vec<rust::ScriptLanguage>) -> Result<Self, String> {
        let mut converted = Vec::with_capacity(values.len());
        for value in values {
            converted.push(DestackScriptLanguage::from_bridge(value)?);
        }
        let (ptr, len) = owned_array(converted);
        Ok(Self { ptr, len })
    }

    /// Convert this C ABI array into bridge values.
    pub(crate) fn to_bridge(&self) -> Result<Vec<rust::ScriptLanguage>, String> {
        if self.len == 0 {
            return Ok(Vec::new());
        }
        if self.ptr.is_null() {
            return Err("array pointer is null".to_string());
        }
        let values = unsafe { std::slice::from_raw_parts(self.ptr, self.len) };
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

impl DestackOptionalScriptLanguage {
    /// Convert one optional bridge value into one C ABI optional value.
    pub(crate) fn from_bridge(value: Option<rust::ScriptLanguage>) -> Result<Self, String> {
        let Some(value) = value else {
            return Ok(Self {
                is_some: false,
                value: DestackScriptLanguage::empty(),
            });
        };
        Ok(Self {
            is_some: true,
            value: DestackScriptLanguage::from_bridge(value)?,
        })
    }

    /// Convert this C ABI optional value into one bridge optional value.
    pub(crate) fn to_bridge(&self) -> Result<Option<rust::ScriptLanguage>, String> {
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
        self.value = DestackScriptLanguage::empty();
    }
}

/// C ABI bridge value.
#[repr(C)]
#[derive(Debug)]
pub struct DestackScript {
    /// The target language of this script.
    pub(crate) language: DestackScriptLanguage,
    /// The emitted declaration when one exists.
    pub(crate) declaration: DestackOptionalDeclaration,
    /// The source map when one exists.
    pub(crate) map: DestackOptionalSourceMap,
    /// Whether this script has top-level side effects.
    pub(crate) has_top_level_side_effects: bool,
}

/// C ABI bridge value array.
#[repr(C)]
#[derive(Debug)]
pub struct DestackScriptArray {
    /// Owned value pointer.
    pub(crate) ptr: *mut DestackScript,
    /// Value count.
    pub(crate) len: usize,
}

/// C ABI optional bridge value.
#[repr(C)]
#[derive(Debug)]
pub struct DestackOptionalScript {
    /// Whether the value is present.
    pub(crate) is_some: bool,
    /// Value when present.
    pub(crate) value: DestackScript,
}

impl DestackScript {
    /// Convert one bridge value into one C ABI value.
    pub(crate) fn from_bridge(value: rust::Script) -> Result<Self, String> {
        Ok(Self {
            language: DestackScriptLanguage::from_bridge(value.language)?,
            declaration: DestackOptionalDeclaration::from_bridge(value.declaration)?,
            map: DestackOptionalSourceMap::from_bridge(value.map)?,
            has_top_level_side_effects: value.has_top_level_side_effects,
        })
    }

    /// Convert this C ABI value into one bridge value.
    pub(crate) fn to_bridge(&self) -> Result<rust::Script, String> {
        Ok(rust::Script {
            language: self.language.to_bridge()?,
            declaration: self.declaration.to_bridge()?,
            map: self.map.to_bridge()?,
            has_top_level_side_effects: self.has_top_level_side_effects,
        })
    }

    /// Destroy this C ABI value.
    pub(crate) fn destroy(&mut self) {
        self.language.destroy();
        self.declaration.destroy();
        self.map.destroy();
    }

    /// Return one empty C ABI value.
    pub(crate) fn empty() -> Self {
        Self {
            language: DestackScriptLanguage::empty(),
            declaration: DestackOptionalDeclaration {
                is_some: false,
                value: DestackDeclaration::empty(),
            },
            map: DestackOptionalSourceMap {
                is_some: false,
                value: DestackSourceMap::empty(),
            },
            has_top_level_side_effects: false,
        }
    }
}

impl DestackScriptArray {
    /// Convert bridge values into one C ABI array.
    pub(crate) fn from_bridge(values: Vec<rust::Script>) -> Result<Self, String> {
        let mut converted = Vec::with_capacity(values.len());
        for value in values {
            converted.push(DestackScript::from_bridge(value)?);
        }
        let (ptr, len) = owned_array(converted);
        Ok(Self { ptr, len })
    }

    /// Convert this C ABI array into bridge values.
    pub(crate) fn to_bridge(&self) -> Result<Vec<rust::Script>, String> {
        if self.len == 0 {
            return Ok(Vec::new());
        }
        if self.ptr.is_null() {
            return Err("array pointer is null".to_string());
        }
        let values = unsafe { std::slice::from_raw_parts(self.ptr, self.len) };
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

impl DestackOptionalScript {
    /// Convert one optional bridge value into one C ABI optional value.
    pub(crate) fn from_bridge(value: Option<rust::Script>) -> Result<Self, String> {
        let Some(value) = value else {
            return Ok(Self {
                is_some: false,
                value: DestackScript::empty(),
            });
        };
        Ok(Self {
            is_some: true,
            value: DestackScript::from_bridge(value)?,
        })
    }

    /// Convert this C ABI optional value into one bridge optional value.
    pub(crate) fn to_bridge(&self) -> Result<Option<rust::Script>, String> {
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
        self.value = DestackScript::empty();
    }
}

/// C ABI bridge enum kind.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DestackBuildOutputKind {
    /// Built script artifact.
    Script = 0,
    /// Built object artifact.
    Object = 1,
    /// Built asset artifact.
    Asset = 2,
    /// Built toolchain payload artifact.
    Build = 3,
    /// Built bundle artifact.
    Bundle = 4,
    /// Built program artifact.
    Program = 5,
    /// Built product artifact.
    Product = 6,
}

/// C ABI bridge enum.
#[repr(C)]
#[derive(Debug)]
pub struct DestackBuildOutput {
    /// Active enum variant.
    pub(crate) kind: DestackBuildOutputKind,
    pub(crate) version: DestackArtifactVersion,
    pub(crate) script_script: DestackScript,
    pub(crate) object_object: DestackObject,
    pub(crate) asset_asset: DestackAsset,
    pub(crate) build_build: DestackBuild,
    pub(crate) bundle_bundle: DestackBundle,
    pub(crate) program_program: DestackProgram,
    pub(crate) product_product: DestackProduct,
}

/// C ABI bridge enum array.
#[repr(C)]
#[derive(Debug)]
pub struct DestackBuildOutputArray {
    /// Owned value pointer.
    pub(crate) ptr: *mut DestackBuildOutput,
    /// Value count.
    pub(crate) len: usize,
}

/// C ABI optional bridge enum.
#[repr(C)]
#[derive(Debug)]
pub struct DestackOptionalBuildOutput {
    /// Whether the value is present.
    pub(crate) is_some: bool,
    /// Value when present.
    pub(crate) value: DestackBuildOutput,
}

impl DestackBuildOutput {
    /// Convert one bridge enum into one C ABI enum.
    pub(crate) fn from_bridge(value: rust::BuildOutput) -> Result<Self, String> {
        Ok(match value {
            rust::BuildOutput::Script { version, script } => Self {
                kind: DestackBuildOutputKind::Script,
                version: DestackArtifactVersion::from_bridge(version)?,
                script_script: DestackScript::from_bridge(script)?,
                object_object: DestackObject::empty(),
                asset_asset: DestackAsset::empty(),
                build_build: DestackBuild::empty(),
                bundle_bundle: DestackBundle::empty(),
                program_program: DestackProgram::empty(),
                product_product: DestackProduct::empty(),
            },
            rust::BuildOutput::Object { version, object } => Self {
                kind: DestackBuildOutputKind::Object,
                version: DestackArtifactVersion::from_bridge(version)?,
                script_script: DestackScript::empty(),
                object_object: DestackObject::from_bridge(object)?,
                asset_asset: DestackAsset::empty(),
                build_build: DestackBuild::empty(),
                bundle_bundle: DestackBundle::empty(),
                program_program: DestackProgram::empty(),
                product_product: DestackProduct::empty(),
            },
            rust::BuildOutput::Asset { version, asset } => Self {
                kind: DestackBuildOutputKind::Asset,
                version: DestackArtifactVersion::from_bridge(version)?,
                script_script: DestackScript::empty(),
                object_object: DestackObject::empty(),
                asset_asset: DestackAsset::from_bridge(asset)?,
                build_build: DestackBuild::empty(),
                bundle_bundle: DestackBundle::empty(),
                program_program: DestackProgram::empty(),
                product_product: DestackProduct::empty(),
            },
            rust::BuildOutput::Build { version, build } => Self {
                kind: DestackBuildOutputKind::Build,
                version: DestackArtifactVersion::from_bridge(version)?,
                script_script: DestackScript::empty(),
                object_object: DestackObject::empty(),
                asset_asset: DestackAsset::empty(),
                build_build: DestackBuild::from_bridge(build)?,
                bundle_bundle: DestackBundle::empty(),
                program_program: DestackProgram::empty(),
                product_product: DestackProduct::empty(),
            },
            rust::BuildOutput::Bundle { version, bundle } => Self {
                kind: DestackBuildOutputKind::Bundle,
                version: DestackArtifactVersion::from_bridge(version)?,
                script_script: DestackScript::empty(),
                object_object: DestackObject::empty(),
                asset_asset: DestackAsset::empty(),
                build_build: DestackBuild::empty(),
                bundle_bundle: DestackBundle::from_bridge(bundle)?,
                program_program: DestackProgram::empty(),
                product_product: DestackProduct::empty(),
            },
            rust::BuildOutput::Program { version, program } => Self {
                kind: DestackBuildOutputKind::Program,
                version: DestackArtifactVersion::from_bridge(version)?,
                script_script: DestackScript::empty(),
                object_object: DestackObject::empty(),
                asset_asset: DestackAsset::empty(),
                build_build: DestackBuild::empty(),
                bundle_bundle: DestackBundle::empty(),
                program_program: DestackProgram::from_bridge(program)?,
                product_product: DestackProduct::empty(),
            },
            rust::BuildOutput::Product { version, product } => Self {
                kind: DestackBuildOutputKind::Product,
                version: DestackArtifactVersion::from_bridge(version)?,
                script_script: DestackScript::empty(),
                object_object: DestackObject::empty(),
                asset_asset: DestackAsset::empty(),
                build_build: DestackBuild::empty(),
                bundle_bundle: DestackBundle::empty(),
                program_program: DestackProgram::empty(),
                product_product: DestackProduct::from_bridge(product)?,
            },
        })
    }

    /// Convert this C ABI enum into one bridge enum.
    pub(crate) fn to_bridge(&self) -> Result<rust::BuildOutput, String> {
        match self.kind {
            DestackBuildOutputKind::Script => {
                let version = self.version.to_bridge()?;
                let script_script = self.script_script.to_bridge()?;
                Ok(rust::BuildOutput::Script {
                    version: version,
                    script: script_script,
                })
            }
            DestackBuildOutputKind::Object => {
                let version = self.version.to_bridge()?;
                let object_object = self.object_object.to_bridge()?;
                Ok(rust::BuildOutput::Object {
                    version: version,
                    object: object_object,
                })
            }
            DestackBuildOutputKind::Asset => {
                let version = self.version.to_bridge()?;
                let asset_asset = self.asset_asset.to_bridge()?;
                Ok(rust::BuildOutput::Asset {
                    version: version,
                    asset: asset_asset,
                })
            }
            DestackBuildOutputKind::Build => {
                let version = self.version.to_bridge()?;
                let build_build = self.build_build.to_bridge()?;
                Ok(rust::BuildOutput::Build {
                    version: version,
                    build: build_build,
                })
            }
            DestackBuildOutputKind::Bundle => {
                let version = self.version.to_bridge()?;
                let bundle_bundle = self.bundle_bundle.to_bridge()?;
                Ok(rust::BuildOutput::Bundle {
                    version: version,
                    bundle: bundle_bundle,
                })
            }
            DestackBuildOutputKind::Program => {
                let version = self.version.to_bridge()?;
                let program_program = self.program_program.to_bridge()?;
                Ok(rust::BuildOutput::Program {
                    version: version,
                    program: program_program,
                })
            }
            DestackBuildOutputKind::Product => {
                let version = self.version.to_bridge()?;
                let product_product = self.product_product.to_bridge()?;
                Ok(rust::BuildOutput::Product {
                    version: version,
                    product: product_product,
                })
            }
        }
    }

    /// Destroy this C ABI enum.
    pub(crate) fn destroy(&mut self) {
        self.version.destroy();
        self.script_script.destroy();
        self.object_object.destroy();
        self.asset_asset.destroy();
        self.build_build.destroy();
        self.bundle_bundle.destroy();
        self.program_program.destroy();
        self.product_product.destroy();
    }

    /// Return one empty C ABI enum.
    pub(crate) fn empty() -> Self {
        Self {
            kind: DestackBuildOutputKind::Script,
            version: DestackArtifactVersion::empty(),
            script_script: DestackScript::empty(),
            object_object: DestackObject::empty(),
            asset_asset: DestackAsset::empty(),
            build_build: DestackBuild::empty(),
            bundle_bundle: DestackBundle::empty(),
            program_program: DestackProgram::empty(),
            product_product: DestackProduct::empty(),
        }
    }
}

impl DestackBuildOutputArray {
    /// Convert bridge values into one C ABI array.
    pub(crate) fn from_bridge(values: Vec<rust::BuildOutput>) -> Result<Self, String> {
        let mut converted = Vec::with_capacity(values.len());
        for value in values {
            converted.push(DestackBuildOutput::from_bridge(value)?);
        }
        let (ptr, len) = owned_array(converted);
        Ok(Self { ptr, len })
    }

    /// Convert this C ABI array into bridge values.
    pub(crate) fn to_bridge(&self) -> Result<Vec<rust::BuildOutput>, String> {
        if self.len == 0 {
            return Ok(Vec::new());
        }
        if self.ptr.is_null() {
            return Err("array pointer is null".to_string());
        }
        let values = unsafe { std::slice::from_raw_parts(self.ptr, self.len) };
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

impl DestackOptionalBuildOutput {
    /// Convert one optional bridge value into one C ABI optional value.
    pub(crate) fn from_bridge(value: Option<rust::BuildOutput>) -> Result<Self, String> {
        let Some(value) = value else {
            return Ok(Self {
                is_some: false,
                value: DestackBuildOutput::empty(),
            });
        };
        Ok(Self {
            is_some: true,
            value: DestackBuildOutput::from_bridge(value)?,
        })
    }

    /// Convert this C ABI optional value into one bridge optional value.
    pub(crate) fn to_bridge(&self) -> Result<Option<rust::BuildOutput>, String> {
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
        self.value = DestackBuildOutput::empty();
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
    pub(crate) fn from_bridge(value: rust::ModuleId) -> Result<Self, String> {
        Ok(Self {
            package: DestackPackageId::from_bridge(value.package)?,
            key: c_string(value.key)?,
        })
    }

    /// Convert this C ABI value into one bridge value.
    pub(crate) fn to_bridge(&self) -> Result<rust::ModuleId, String> {
        Ok(rust::ModuleId {
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
    pub(crate) fn from_bridge(values: Vec<rust::ModuleId>) -> Result<Self, String> {
        let mut converted = Vec::with_capacity(values.len());
        for value in values {
            converted.push(DestackModuleId::from_bridge(value)?);
        }
        let (ptr, len) = owned_array(converted);
        Ok(Self { ptr, len })
    }

    /// Convert this C ABI array into bridge values.
    pub(crate) fn to_bridge(&self) -> Result<Vec<rust::ModuleId>, String> {
        if self.len == 0 {
            return Ok(Vec::new());
        }
        if self.ptr.is_null() {
            return Err("array pointer is null".to_string());
        }
        let values = unsafe { std::slice::from_raw_parts(self.ptr, self.len) };
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
    pub(crate) fn from_bridge(value: Option<rust::ModuleId>) -> Result<Self, String> {
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
    pub(crate) fn to_bridge(&self) -> Result<Option<rust::ModuleId>, String> {
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
pub struct DestackModule {
    /// Stable source module id.
    pub(crate) id: DestackModuleId,
}

/// C ABI bridge value array.
#[repr(C)]
#[derive(Debug)]
pub struct DestackModuleArray {
    /// Owned value pointer.
    pub(crate) ptr: *mut DestackModule,
    /// Value count.
    pub(crate) len: usize,
}

/// C ABI optional bridge value.
#[repr(C)]
#[derive(Debug)]
pub struct DestackOptionalModule {
    /// Whether the value is present.
    pub(crate) is_some: bool,
    /// Value when present.
    pub(crate) value: DestackModule,
}

impl DestackModule {
    /// Convert one bridge value into one C ABI value.
    pub(crate) fn from_bridge(value: rust::Module) -> Result<Self, String> {
        Ok(Self {
            id: DestackModuleId::from_bridge(value.id)?,
        })
    }

    /// Convert this C ABI value into one bridge value.
    pub(crate) fn to_bridge(&self) -> Result<rust::Module, String> {
        Ok(rust::Module {
            id: self.id.to_bridge()?,
        })
    }

    /// Destroy this C ABI value.
    pub(crate) fn destroy(&mut self) {
        self.id.destroy();
    }

    /// Return one empty C ABI value.
    pub(crate) fn empty() -> Self {
        Self {
            id: DestackModuleId::empty(),
        }
    }
}

impl DestackModuleArray {
    /// Convert bridge values into one C ABI array.
    pub(crate) fn from_bridge(values: Vec<rust::Module>) -> Result<Self, String> {
        let mut converted = Vec::with_capacity(values.len());
        for value in values {
            converted.push(DestackModule::from_bridge(value)?);
        }
        let (ptr, len) = owned_array(converted);
        Ok(Self { ptr, len })
    }

    /// Convert this C ABI array into bridge values.
    pub(crate) fn to_bridge(&self) -> Result<Vec<rust::Module>, String> {
        if self.len == 0 {
            return Ok(Vec::new());
        }
        if self.ptr.is_null() {
            return Err("array pointer is null".to_string());
        }
        let values = unsafe { std::slice::from_raw_parts(self.ptr, self.len) };
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

impl DestackOptionalModule {
    /// Convert one optional bridge value into one C ABI optional value.
    pub(crate) fn from_bridge(value: Option<rust::Module>) -> Result<Self, String> {
        let Some(value) = value else {
            return Ok(Self {
                is_some: false,
                value: DestackModule::empty(),
            });
        };
        Ok(Self {
            is_some: true,
            value: DestackModule::from_bridge(value)?,
        })
    }

    /// Convert this C ABI optional value into one bridge optional value.
    pub(crate) fn to_bridge(&self) -> Result<Option<rust::Module>, String> {
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
        self.value = DestackModule::empty();
    }
}

/// C ABI bridge enum.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DestackModuleBuildKind {
    /// Build the structured script artifact.
    Script = 0,
    /// Build the compiled object artifact.
    Object = 1,
    /// Build the opaque asset artifact.
    Asset = 2,
}

/// C ABI bridge enum array.
#[repr(C)]
#[derive(Debug)]
pub struct DestackModuleBuildKindArray {
    /// Owned value pointer.
    pub(crate) ptr: *mut DestackModuleBuildKind,
    /// Value count.
    pub(crate) len: usize,
}

/// C ABI optional bridge enum.
#[repr(C)]
#[derive(Debug)]
pub struct DestackOptionalModuleBuildKind {
    /// Whether the value is present.
    pub(crate) is_some: bool,
    /// Value when present.
    pub(crate) value: DestackModuleBuildKind,
}

impl DestackModuleBuildKind {
    /// Convert one bridge enum into one C ABI enum.
    pub(crate) fn from_bridge(value: rust::ModuleBuildKind) -> Result<Self, String> {
        Ok(match value {
            rust::ModuleBuildKind::Script => Self::Script,
            rust::ModuleBuildKind::Object => Self::Object,
            rust::ModuleBuildKind::Asset => Self::Asset,
        })
    }

    /// Convert this C ABI enum into one bridge enum.
    pub(crate) fn to_bridge(&self) -> Result<rust::ModuleBuildKind, String> {
        Ok(match *self {
            Self::Script => rust::ModuleBuildKind::Script,
            Self::Object => rust::ModuleBuildKind::Object,
            Self::Asset => rust::ModuleBuildKind::Asset,
        })
    }

    /// Destroy this C ABI enum.
    pub(crate) fn destroy(&mut self) {}
    /// Return one empty C ABI enum.
    pub(crate) fn empty() -> Self {
        Self::Script
    }
}

impl DestackModuleBuildKindArray {
    /// Convert bridge values into one C ABI array.
    pub(crate) fn from_bridge(values: Vec<rust::ModuleBuildKind>) -> Result<Self, String> {
        let mut converted = Vec::with_capacity(values.len());
        for value in values {
            converted.push(DestackModuleBuildKind::from_bridge(value)?);
        }
        let (ptr, len) = owned_array(converted);
        Ok(Self { ptr, len })
    }

    /// Convert this C ABI array into bridge values.
    pub(crate) fn to_bridge(&self) -> Result<Vec<rust::ModuleBuildKind>, String> {
        if self.len == 0 {
            return Ok(Vec::new());
        }
        if self.ptr.is_null() {
            return Err("array pointer is null".to_string());
        }
        let values = unsafe { std::slice::from_raw_parts(self.ptr, self.len) };
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

impl DestackOptionalModuleBuildKind {
    /// Convert one optional bridge value into one C ABI optional value.
    pub(crate) fn from_bridge(value: Option<rust::ModuleBuildKind>) -> Result<Self, String> {
        let Some(value) = value else {
            return Ok(Self {
                is_some: false,
                value: DestackModuleBuildKind::empty(),
            });
        };
        Ok(Self {
            is_some: true,
            value: DestackModuleBuildKind::from_bridge(value)?,
        })
    }

    /// Convert this C ABI optional value into one bridge optional value.
    pub(crate) fn to_bridge(&self) -> Result<Option<rust::ModuleBuildKind>, String> {
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
        self.value = DestackModuleBuildKind::empty();
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
    pub(crate) fn from_bridge(value: rust::ProductId) -> Result<Self, String> {
        Ok(Self {
            package: DestackPackageId::from_bridge(value.package)?,
            key: c_string(value.key)?,
        })
    }

    /// Convert this C ABI value into one bridge value.
    pub(crate) fn to_bridge(&self) -> Result<rust::ProductId, String> {
        Ok(rust::ProductId {
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
    pub(crate) fn from_bridge(values: Vec<rust::ProductId>) -> Result<Self, String> {
        let mut converted = Vec::with_capacity(values.len());
        for value in values {
            converted.push(DestackProductId::from_bridge(value)?);
        }
        let (ptr, len) = owned_array(converted);
        Ok(Self { ptr, len })
    }

    /// Convert this C ABI array into bridge values.
    pub(crate) fn to_bridge(&self) -> Result<Vec<rust::ProductId>, String> {
        if self.len == 0 {
            return Ok(Vec::new());
        }
        if self.ptr.is_null() {
            return Err("array pointer is null".to_string());
        }
        let values = unsafe { std::slice::from_raw_parts(self.ptr, self.len) };
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
    pub(crate) fn from_bridge(value: Option<rust::ProductId>) -> Result<Self, String> {
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
    pub(crate) fn to_bridge(&self) -> Result<Option<rust::ProductId>, String> {
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

/// C ABI bridge enum kind.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DestackBuildRequestKind {
    /// Build one module artifact.
    Module = 0,
    /// Build one target build payload.
    Build = 1,
    /// Build one package target.
    Target = 2,
    /// Build one product.
    Product = 3,
}

/// C ABI bridge enum.
#[repr(C)]
#[derive(Debug)]
pub struct DestackBuildRequest {
    /// Active enum variant.
    pub(crate) kind: DestackBuildRequestKind,
    pub(crate) module_module: DestackModule,
    pub(crate) target: DestackTargetId,
    pub(crate) output: DestackModuleBuildKind,
    pub(crate) target_target: DestackTargetId,
    pub(crate) product_product: DestackProductId,
}

/// C ABI bridge enum array.
#[repr(C)]
#[derive(Debug)]
pub struct DestackBuildRequestArray {
    /// Owned value pointer.
    pub(crate) ptr: *mut DestackBuildRequest,
    /// Value count.
    pub(crate) len: usize,
}

/// C ABI optional bridge enum.
#[repr(C)]
#[derive(Debug)]
pub struct DestackOptionalBuildRequest {
    /// Whether the value is present.
    pub(crate) is_some: bool,
    /// Value when present.
    pub(crate) value: DestackBuildRequest,
}

impl DestackBuildRequest {
    /// Convert one bridge enum into one C ABI enum.
    pub(crate) fn from_bridge(value: rust::BuildRequest) -> Result<Self, String> {
        Ok(match value {
            rust::BuildRequest::Module {
                module,
                target,
                output,
            } => Self {
                kind: DestackBuildRequestKind::Module,
                module_module: DestackModule::from_bridge(module)?,
                target: DestackTargetId::from_bridge(target)?,
                output: DestackModuleBuildKind::from_bridge(output)?,
                target_target: DestackTargetId::empty(),
                product_product: DestackProductId::empty(),
            },
            rust::BuildRequest::Build { target } => Self {
                kind: DestackBuildRequestKind::Build,
                module_module: DestackModule::empty(),
                target: DestackTargetId::from_bridge(target)?,
                output: DestackModuleBuildKind::empty(),
                target_target: DestackTargetId::empty(),
                product_product: DestackProductId::empty(),
            },
            rust::BuildRequest::Target { target } => Self {
                kind: DestackBuildRequestKind::Target,
                module_module: DestackModule::empty(),
                target: DestackTargetId::empty(),
                output: DestackModuleBuildKind::empty(),
                target_target: DestackTargetId::from_bridge(target)?,
                product_product: DestackProductId::empty(),
            },
            rust::BuildRequest::Product { product } => Self {
                kind: DestackBuildRequestKind::Product,
                module_module: DestackModule::empty(),
                target: DestackTargetId::empty(),
                output: DestackModuleBuildKind::empty(),
                target_target: DestackTargetId::empty(),
                product_product: DestackProductId::from_bridge(product)?,
            },
        })
    }

    /// Convert this C ABI enum into one bridge enum.
    pub(crate) fn to_bridge(&self) -> Result<rust::BuildRequest, String> {
        match self.kind {
            DestackBuildRequestKind::Module => {
                let module_module = self.module_module.to_bridge()?;
                let target = self.target.to_bridge()?;
                let output = self.output.to_bridge()?;
                Ok(rust::BuildRequest::Module {
                    module: module_module,
                    target: target,
                    output: output,
                })
            }
            DestackBuildRequestKind::Build => {
                let target = self.target.to_bridge()?;
                Ok(rust::BuildRequest::Build { target: target })
            }
            DestackBuildRequestKind::Target => {
                let target_target = self.target_target.to_bridge()?;
                Ok(rust::BuildRequest::Target {
                    target: target_target,
                })
            }
            DestackBuildRequestKind::Product => {
                let product_product = self.product_product.to_bridge()?;
                Ok(rust::BuildRequest::Product {
                    product: product_product,
                })
            }
        }
    }

    /// Destroy this C ABI enum.
    pub(crate) fn destroy(&mut self) {
        self.module_module.destroy();
        self.target.destroy();
        self.output.destroy();
        self.target_target.destroy();
        self.product_product.destroy();
    }

    /// Return one empty C ABI enum.
    pub(crate) fn empty() -> Self {
        Self {
            kind: DestackBuildRequestKind::Module,
            module_module: DestackModule::empty(),
            target: DestackTargetId::empty(),
            output: DestackModuleBuildKind::empty(),
            target_target: DestackTargetId::empty(),
            product_product: DestackProductId::empty(),
        }
    }
}

impl DestackBuildRequestArray {
    /// Convert bridge values into one C ABI array.
    pub(crate) fn from_bridge(values: Vec<rust::BuildRequest>) -> Result<Self, String> {
        let mut converted = Vec::with_capacity(values.len());
        for value in values {
            converted.push(DestackBuildRequest::from_bridge(value)?);
        }
        let (ptr, len) = owned_array(converted);
        Ok(Self { ptr, len })
    }

    /// Convert this C ABI array into bridge values.
    pub(crate) fn to_bridge(&self) -> Result<Vec<rust::BuildRequest>, String> {
        if self.len == 0 {
            return Ok(Vec::new());
        }
        if self.ptr.is_null() {
            return Err("array pointer is null".to_string());
        }
        let values = unsafe { std::slice::from_raw_parts(self.ptr, self.len) };
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

impl DestackOptionalBuildRequest {
    /// Convert one optional bridge value into one C ABI optional value.
    pub(crate) fn from_bridge(value: Option<rust::BuildRequest>) -> Result<Self, String> {
        let Some(value) = value else {
            return Ok(Self {
                is_some: false,
                value: DestackBuildRequest::empty(),
            });
        };
        Ok(Self {
            is_some: true,
            value: DestackBuildRequest::from_bridge(value)?,
        })
    }

    /// Convert this C ABI optional value into one bridge optional value.
    pub(crate) fn to_bridge(&self) -> Result<Option<rust::BuildRequest>, String> {
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
        self.value = DestackBuildRequest::empty();
    }
}

/// C ABI bridge value.
#[repr(C)]
#[derive(Debug)]
pub struct DestackChange {
    /// Repository logical path.
    pub(crate) path: *mut c_char,
    /// External file URI.
    pub(crate) uri: *mut c_char,
    /// Whether the file was removed.
    pub(crate) is_removed: bool,
    /// Updated module id when known.
    pub(crate) module_id: DestackOptionalModuleId,
}

/// C ABI bridge value array.
#[repr(C)]
#[derive(Debug)]
pub struct DestackChangeArray {
    /// Owned value pointer.
    pub(crate) ptr: *mut DestackChange,
    /// Value count.
    pub(crate) len: usize,
}

/// C ABI optional bridge value.
#[repr(C)]
#[derive(Debug)]
pub struct DestackOptionalChange {
    /// Whether the value is present.
    pub(crate) is_some: bool,
    /// Value when present.
    pub(crate) value: DestackChange,
}

impl DestackChange {
    /// Convert one bridge value into one C ABI value.
    pub(crate) fn from_bridge(value: rust::Change) -> Result<Self, String> {
        Ok(Self {
            path: c_string(value.path)?,
            uri: c_string(value.uri)?,
            is_removed: value.is_removed,
            module_id: DestackOptionalModuleId::from_bridge(value.module_id)?,
        })
    }

    /// Convert this C ABI value into one bridge value.
    pub(crate) fn to_bridge(&self) -> Result<rust::Change, String> {
        Ok(rust::Change {
            path: read_string(self.path)?,
            uri: read_string(self.uri)?,
            is_removed: self.is_removed,
            module_id: self.module_id.to_bridge()?,
        })
    }

    /// Destroy this C ABI value.
    pub(crate) fn destroy(&mut self) {
        destroy_string(self.path);
        self.path = ptr::null_mut();
        destroy_string(self.uri);
        self.uri = ptr::null_mut();
        self.module_id.destroy();
    }

    /// Return one empty C ABI value.
    pub(crate) fn empty() -> Self {
        Self {
            path: ptr::null_mut(),
            uri: ptr::null_mut(),
            is_removed: false,
            module_id: DestackOptionalModuleId {
                is_some: false,
                value: DestackModuleId::empty(),
            },
        }
    }
}

impl DestackChangeArray {
    /// Convert bridge values into one C ABI array.
    pub(crate) fn from_bridge(values: Vec<rust::Change>) -> Result<Self, String> {
        let mut converted = Vec::with_capacity(values.len());
        for value in values {
            converted.push(DestackChange::from_bridge(value)?);
        }
        let (ptr, len) = owned_array(converted);
        Ok(Self { ptr, len })
    }

    /// Convert this C ABI array into bridge values.
    pub(crate) fn to_bridge(&self) -> Result<Vec<rust::Change>, String> {
        if self.len == 0 {
            return Ok(Vec::new());
        }
        if self.ptr.is_null() {
            return Err("array pointer is null".to_string());
        }
        let values = unsafe { std::slice::from_raw_parts(self.ptr, self.len) };
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

impl DestackOptionalChange {
    /// Convert one optional bridge value into one C ABI optional value.
    pub(crate) fn from_bridge(value: Option<rust::Change>) -> Result<Self, String> {
        let Some(value) = value else {
            return Ok(Self {
                is_some: false,
                value: DestackChange::empty(),
            });
        };
        Ok(Self {
            is_some: true,
            value: DestackChange::from_bridge(value)?,
        })
    }

    /// Convert this C ABI optional value into one bridge optional value.
    pub(crate) fn to_bridge(&self) -> Result<Option<rust::Change>, String> {
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
        self.value = DestackChange::empty();
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
    pub(crate) fn from_bridge(value: rust::Revision) -> Result<Self, String> {
        Ok(Self {
            id: c_string(value.id)?,
        })
    }

    /// Convert this C ABI value into one bridge value.
    pub(crate) fn to_bridge(&self) -> Result<rust::Revision, String> {
        Ok(rust::Revision {
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
    pub(crate) fn from_bridge(values: Vec<rust::Revision>) -> Result<Self, String> {
        let mut converted = Vec::with_capacity(values.len());
        for value in values {
            converted.push(DestackRevision::from_bridge(value)?);
        }
        let (ptr, len) = owned_array(converted);
        Ok(Self { ptr, len })
    }

    /// Convert this C ABI array into bridge values.
    pub(crate) fn to_bridge(&self) -> Result<Vec<rust::Revision>, String> {
        if self.len == 0 {
            return Ok(Vec::new());
        }
        if self.ptr.is_null() {
            return Err("array pointer is null".to_string());
        }
        let values = unsafe { std::slice::from_raw_parts(self.ptr, self.len) };
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
    pub(crate) fn from_bridge(value: Option<rust::Revision>) -> Result<Self, String> {
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
    pub(crate) fn to_bridge(&self) -> Result<Option<rust::Revision>, String> {
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
pub struct DestackCommit {
    /// Previous revision.
    pub(crate) before: DestackRevision,
    /// Updated revision.
    pub(crate) after: DestackRevision,
    /// Changed files.
    pub(crate) changes: DestackChangeArray,
}

/// C ABI bridge value array.
#[repr(C)]
#[derive(Debug)]
pub struct DestackCommitArray {
    /// Owned value pointer.
    pub(crate) ptr: *mut DestackCommit,
    /// Value count.
    pub(crate) len: usize,
}

/// C ABI optional bridge value.
#[repr(C)]
#[derive(Debug)]
pub struct DestackOptionalCommit {
    /// Whether the value is present.
    pub(crate) is_some: bool,
    /// Value when present.
    pub(crate) value: DestackCommit,
}

impl DestackCommit {
    /// Convert one bridge value into one C ABI value.
    pub(crate) fn from_bridge(value: rust::Commit) -> Result<Self, String> {
        Ok(Self {
            before: DestackRevision::from_bridge(value.before)?,
            after: DestackRevision::from_bridge(value.after)?,
            changes: DestackChangeArray::from_bridge(value.changes)?,
        })
    }

    /// Convert this C ABI value into one bridge value.
    pub(crate) fn to_bridge(&self) -> Result<rust::Commit, String> {
        Ok(rust::Commit {
            before: self.before.to_bridge()?,
            after: self.after.to_bridge()?,
            changes: self.changes.to_bridge()?,
        })
    }

    /// Destroy this C ABI value.
    pub(crate) fn destroy(&mut self) {
        self.before.destroy();
        self.after.destroy();
        self.changes.destroy();
    }

    /// Return one empty C ABI value.
    pub(crate) fn empty() -> Self {
        Self {
            before: DestackRevision::empty(),
            after: DestackRevision::empty(),
            changes: DestackChangeArray {
                ptr: ptr::null_mut(),
                len: 0,
            },
        }
    }
}

impl DestackCommitArray {
    /// Convert bridge values into one C ABI array.
    pub(crate) fn from_bridge(values: Vec<rust::Commit>) -> Result<Self, String> {
        let mut converted = Vec::with_capacity(values.len());
        for value in values {
            converted.push(DestackCommit::from_bridge(value)?);
        }
        let (ptr, len) = owned_array(converted);
        Ok(Self { ptr, len })
    }

    /// Convert this C ABI array into bridge values.
    pub(crate) fn to_bridge(&self) -> Result<Vec<rust::Commit>, String> {
        if self.len == 0 {
            return Ok(Vec::new());
        }
        if self.ptr.is_null() {
            return Err("array pointer is null".to_string());
        }
        let values = unsafe { std::slice::from_raw_parts(self.ptr, self.len) };
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

impl DestackOptionalCommit {
    /// Convert one optional bridge value into one C ABI optional value.
    pub(crate) fn from_bridge(value: Option<rust::Commit>) -> Result<Self, String> {
        let Some(value) = value else {
            return Ok(Self {
                is_some: false,
                value: DestackCommit::empty(),
            });
        };
        Ok(Self {
            is_some: true,
            value: DestackCommit::from_bridge(value)?,
        })
    }

    /// Convert this C ABI optional value into one bridge optional value.
    pub(crate) fn to_bridge(&self) -> Result<Option<rust::Commit>, String> {
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
        self.value = DestackCommit::empty();
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
    pub(crate) fn from_bridge(value: rust::ComponentId) -> Result<Self, String> {
        Ok(Self {
            id: c_string(value.id)?,
        })
    }

    /// Convert this C ABI value into one bridge value.
    pub(crate) fn to_bridge(&self) -> Result<rust::ComponentId, String> {
        Ok(rust::ComponentId {
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
    pub(crate) fn from_bridge(values: Vec<rust::ComponentId>) -> Result<Self, String> {
        let mut converted = Vec::with_capacity(values.len());
        for value in values {
            converted.push(DestackComponentId::from_bridge(value)?);
        }
        let (ptr, len) = owned_array(converted);
        Ok(Self { ptr, len })
    }

    /// Convert this C ABI array into bridge values.
    pub(crate) fn to_bridge(&self) -> Result<Vec<rust::ComponentId>, String> {
        if self.len == 0 {
            return Ok(Vec::new());
        }
        if self.ptr.is_null() {
            return Err("array pointer is null".to_string());
        }
        let values = unsafe { std::slice::from_raw_parts(self.ptr, self.len) };
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
    pub(crate) fn from_bridge(value: Option<rust::ComponentId>) -> Result<Self, String> {
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
    pub(crate) fn to_bridge(&self) -> Result<Option<rust::ComponentId>, String> {
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
    pub(crate) fn from_bridge(value: rust::ProfileId) -> Result<Self, String> {
        Ok(Self {
            id: c_string(value.id)?,
        })
    }

    /// Convert this C ABI value into one bridge value.
    pub(crate) fn to_bridge(&self) -> Result<rust::ProfileId, String> {
        Ok(rust::ProfileId {
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
    pub(crate) fn from_bridge(values: Vec<rust::ProfileId>) -> Result<Self, String> {
        let mut converted = Vec::with_capacity(values.len());
        for value in values {
            converted.push(DestackProfileId::from_bridge(value)?);
        }
        let (ptr, len) = owned_array(converted);
        Ok(Self { ptr, len })
    }

    /// Convert this C ABI array into bridge values.
    pub(crate) fn to_bridge(&self) -> Result<Vec<rust::ProfileId>, String> {
        if self.len == 0 {
            return Ok(Vec::new());
        }
        if self.ptr.is_null() {
            return Err("array pointer is null".to_string());
        }
        let values = unsafe { std::slice::from_raw_parts(self.ptr, self.len) };
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
    pub(crate) fn from_bridge(value: Option<rust::ProfileId>) -> Result<Self, String> {
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
    pub(crate) fn to_bridge(&self) -> Result<Option<rust::ProfileId>, String> {
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
    pub(crate) fn from_bridge(value: rust::DirChecked) -> Result<Self, String> {
        Ok(Self {
            version: DestackArtifactVersion::from_bridge(value.version)?,
            module: DestackModuleId::from_bridge(value.module)?,
            profile: DestackProfileId::from_bridge(value.profile)?,
            component: DestackComponentId::from_bridge(value.component)?,
            entry: DestackModuleId::from_bridge(value.entry)?,
        })
    }

    /// Convert this C ABI value into one bridge value.
    pub(crate) fn to_bridge(&self) -> Result<rust::DirChecked, String> {
        Ok(rust::DirChecked {
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
    pub(crate) fn from_bridge(values: Vec<rust::DirChecked>) -> Result<Self, String> {
        let mut converted = Vec::with_capacity(values.len());
        for value in values {
            converted.push(DestackDirChecked::from_bridge(value)?);
        }
        let (ptr, len) = owned_array(converted);
        Ok(Self { ptr, len })
    }

    /// Convert this C ABI array into bridge values.
    pub(crate) fn to_bridge(&self) -> Result<Vec<rust::DirChecked>, String> {
        if self.len == 0 {
            return Ok(Vec::new());
        }
        if self.ptr.is_null() {
            return Err("array pointer is null".to_string());
        }
        let values = unsafe { std::slice::from_raw_parts(self.ptr, self.len) };
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
    pub(crate) fn from_bridge(value: Option<rust::DirChecked>) -> Result<Self, String> {
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
    pub(crate) fn to_bridge(&self) -> Result<Option<rust::DirChecked>, String> {
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
    pub(crate) fn from_bridge(value: rust::DirParsed) -> Result<Self, String> {
        Ok(Self {
            version: DestackArtifactVersion::from_bridge(value.version)?,
            module: DestackModuleId::from_bridge(value.module)?,
        })
    }

    /// Convert this C ABI value into one bridge value.
    pub(crate) fn to_bridge(&self) -> Result<rust::DirParsed, String> {
        Ok(rust::DirParsed {
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
    pub(crate) fn from_bridge(values: Vec<rust::DirParsed>) -> Result<Self, String> {
        let mut converted = Vec::with_capacity(values.len());
        for value in values {
            converted.push(DestackDirParsed::from_bridge(value)?);
        }
        let (ptr, len) = owned_array(converted);
        Ok(Self { ptr, len })
    }

    /// Convert this C ABI array into bridge values.
    pub(crate) fn to_bridge(&self) -> Result<Vec<rust::DirParsed>, String> {
        if self.len == 0 {
            return Ok(Vec::new());
        }
        if self.ptr.is_null() {
            return Err("array pointer is null".to_string());
        }
        let values = unsafe { std::slice::from_raw_parts(self.ptr, self.len) };
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
    pub(crate) fn from_bridge(value: Option<rust::DirParsed>) -> Result<Self, String> {
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
    pub(crate) fn to_bridge(&self) -> Result<Option<rust::DirParsed>, String> {
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
    pub(crate) fn from_bridge(value: rust::DirResolved) -> Result<Self, String> {
        Ok(Self {
            version: DestackArtifactVersion::from_bridge(value.version)?,
            module: DestackModuleId::from_bridge(value.module)?,
            profile: DestackProfileId::from_bridge(value.profile)?,
        })
    }

    /// Convert this C ABI value into one bridge value.
    pub(crate) fn to_bridge(&self) -> Result<rust::DirResolved, String> {
        Ok(rust::DirResolved {
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
    pub(crate) fn from_bridge(values: Vec<rust::DirResolved>) -> Result<Self, String> {
        let mut converted = Vec::with_capacity(values.len());
        for value in values {
            converted.push(DestackDirResolved::from_bridge(value)?);
        }
        let (ptr, len) = owned_array(converted);
        Ok(Self { ptr, len })
    }

    /// Convert this C ABI array into bridge values.
    pub(crate) fn to_bridge(&self) -> Result<Vec<rust::DirResolved>, String> {
        if self.len == 0 {
            return Ok(Vec::new());
        }
        if self.ptr.is_null() {
            return Err("array pointer is null".to_string());
        }
        let values = unsafe { std::slice::from_raw_parts(self.ptr, self.len) };
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
    pub(crate) fn from_bridge(value: Option<rust::DirResolved>) -> Result<Self, String> {
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
    pub(crate) fn to_bridge(&self) -> Result<Option<rust::DirResolved>, String> {
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

/// C ABI bridge enum kind.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DestackDocumentKind {
    /// Repository module at one immutable revision.
    Module = 0,
    /// Ad hoc source text.
    Text = 1,
}

/// C ABI bridge enum.
#[repr(C)]
#[derive(Debug)]
pub struct DestackDocument {
    /// Active enum variant.
    pub(crate) kind: DestackDocumentKind,
    pub(crate) module_module: DestackModule,
    pub(crate) path: *mut c_char,
    pub(crate) text_text: *mut c_char,
}

/// C ABI bridge enum array.
#[repr(C)]
#[derive(Debug)]
pub struct DestackDocumentArray {
    /// Owned value pointer.
    pub(crate) ptr: *mut DestackDocument,
    /// Value count.
    pub(crate) len: usize,
}

/// C ABI optional bridge enum.
#[repr(C)]
#[derive(Debug)]
pub struct DestackOptionalDocument {
    /// Whether the value is present.
    pub(crate) is_some: bool,
    /// Value when present.
    pub(crate) value: DestackDocument,
}

impl DestackDocument {
    /// Convert one bridge enum into one C ABI enum.
    pub(crate) fn from_bridge(value: rust::Document) -> Result<Self, String> {
        Ok(match value {
            rust::Document::Module { module } => Self {
                kind: DestackDocumentKind::Module,
                module_module: DestackModule::from_bridge(module)?,
                path: ptr::null_mut(),
                text_text: ptr::null_mut(),
            },
            rust::Document::Text { path, text } => Self {
                kind: DestackDocumentKind::Text,
                module_module: DestackModule::empty(),
                path: c_string(path)?,
                text_text: c_string(text)?,
            },
        })
    }

    /// Convert this C ABI enum into one bridge enum.
    pub(crate) fn to_bridge(&self) -> Result<rust::Document, String> {
        match self.kind {
            DestackDocumentKind::Module => {
                let module_module = self.module_module.to_bridge()?;
                Ok(rust::Document::Module {
                    module: module_module,
                })
            }
            DestackDocumentKind::Text => {
                let path = read_string(self.path)?;
                let text_text = read_string(self.text_text)?;
                Ok(rust::Document::Text {
                    path: path,
                    text: text_text,
                })
            }
        }
    }

    /// Destroy this C ABI enum.
    pub(crate) fn destroy(&mut self) {
        self.module_module.destroy();
        destroy_string(self.path);
        self.path = ptr::null_mut();
        destroy_string(self.text_text);
        self.text_text = ptr::null_mut();
    }

    /// Return one empty C ABI enum.
    pub(crate) fn empty() -> Self {
        Self {
            kind: DestackDocumentKind::Module,
            module_module: DestackModule::empty(),
            path: ptr::null_mut(),
            text_text: ptr::null_mut(),
        }
    }
}

impl DestackDocumentArray {
    /// Convert bridge values into one C ABI array.
    pub(crate) fn from_bridge(values: Vec<rust::Document>) -> Result<Self, String> {
        let mut converted = Vec::with_capacity(values.len());
        for value in values {
            converted.push(DestackDocument::from_bridge(value)?);
        }
        let (ptr, len) = owned_array(converted);
        Ok(Self { ptr, len })
    }

    /// Convert this C ABI array into bridge values.
    pub(crate) fn to_bridge(&self) -> Result<Vec<rust::Document>, String> {
        if self.len == 0 {
            return Ok(Vec::new());
        }
        if self.ptr.is_null() {
            return Err("array pointer is null".to_string());
        }
        let values = unsafe { std::slice::from_raw_parts(self.ptr, self.len) };
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

impl DestackOptionalDocument {
    /// Convert one optional bridge value into one C ABI optional value.
    pub(crate) fn from_bridge(value: Option<rust::Document>) -> Result<Self, String> {
        let Some(value) = value else {
            return Ok(Self {
                is_some: false,
                value: DestackDocument::empty(),
            });
        };
        Ok(Self {
            is_some: true,
            value: DestackDocument::from_bridge(value)?,
        })
    }

    /// Convert this C ABI optional value into one bridge optional value.
    pub(crate) fn to_bridge(&self) -> Result<Option<rust::Document>, String> {
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
        self.value = DestackDocument::empty();
    }
}

/// C ABI bridge value.
#[repr(C)]
#[derive(Debug)]
pub struct DestackFormatOutput {
    /// Formatted source text.
    pub(crate) text: *mut c_char,
}

/// C ABI bridge value array.
#[repr(C)]
#[derive(Debug)]
pub struct DestackFormatOutputArray {
    /// Owned value pointer.
    pub(crate) ptr: *mut DestackFormatOutput,
    /// Value count.
    pub(crate) len: usize,
}

/// C ABI optional bridge value.
#[repr(C)]
#[derive(Debug)]
pub struct DestackOptionalFormatOutput {
    /// Whether the value is present.
    pub(crate) is_some: bool,
    /// Value when present.
    pub(crate) value: DestackFormatOutput,
}

impl DestackFormatOutput {
    /// Convert one bridge value into one C ABI value.
    pub(crate) fn from_bridge(value: rust::FormatOutput) -> Result<Self, String> {
        Ok(Self {
            text: c_string(value.text)?,
        })
    }

    /// Convert this C ABI value into one bridge value.
    pub(crate) fn to_bridge(&self) -> Result<rust::FormatOutput, String> {
        Ok(rust::FormatOutput {
            text: read_string(self.text)?,
        })
    }

    /// Destroy this C ABI value.
    pub(crate) fn destroy(&mut self) {
        destroy_string(self.text);
        self.text = ptr::null_mut();
    }

    /// Return one empty C ABI value.
    pub(crate) fn empty() -> Self {
        Self {
            text: ptr::null_mut(),
        }
    }
}

impl DestackFormatOutputArray {
    /// Convert bridge values into one C ABI array.
    pub(crate) fn from_bridge(values: Vec<rust::FormatOutput>) -> Result<Self, String> {
        let mut converted = Vec::with_capacity(values.len());
        for value in values {
            converted.push(DestackFormatOutput::from_bridge(value)?);
        }
        let (ptr, len) = owned_array(converted);
        Ok(Self { ptr, len })
    }

    /// Convert this C ABI array into bridge values.
    pub(crate) fn to_bridge(&self) -> Result<Vec<rust::FormatOutput>, String> {
        if self.len == 0 {
            return Ok(Vec::new());
        }
        if self.ptr.is_null() {
            return Err("array pointer is null".to_string());
        }
        let values = unsafe { std::slice::from_raw_parts(self.ptr, self.len) };
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

impl DestackOptionalFormatOutput {
    /// Convert one optional bridge value into one C ABI optional value.
    pub(crate) fn from_bridge(value: Option<rust::FormatOutput>) -> Result<Self, String> {
        let Some(value) = value else {
            return Ok(Self {
                is_some: false,
                value: DestackFormatOutput::empty(),
            });
        };
        Ok(Self {
            is_some: true,
            value: DestackFormatOutput::from_bridge(value)?,
        })
    }

    /// Convert this C ABI optional value into one bridge optional value.
    pub(crate) fn to_bridge(&self) -> Result<Option<rust::FormatOutput>, String> {
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
        self.value = DestackFormatOutput::empty();
    }
}

/// C ABI bridge value.
#[repr(C)]
#[derive(Debug)]
pub struct DestackFormatRequest {
    /// Document to format.
    pub(crate) document: DestackDocument,
}

/// C ABI bridge value array.
#[repr(C)]
#[derive(Debug)]
pub struct DestackFormatRequestArray {
    /// Owned value pointer.
    pub(crate) ptr: *mut DestackFormatRequest,
    /// Value count.
    pub(crate) len: usize,
}

/// C ABI optional bridge value.
#[repr(C)]
#[derive(Debug)]
pub struct DestackOptionalFormatRequest {
    /// Whether the value is present.
    pub(crate) is_some: bool,
    /// Value when present.
    pub(crate) value: DestackFormatRequest,
}

impl DestackFormatRequest {
    /// Convert one bridge value into one C ABI value.
    pub(crate) fn from_bridge(value: rust::FormatRequest) -> Result<Self, String> {
        Ok(Self {
            document: DestackDocument::from_bridge(value.document)?,
        })
    }

    /// Convert this C ABI value into one bridge value.
    pub(crate) fn to_bridge(&self) -> Result<rust::FormatRequest, String> {
        Ok(rust::FormatRequest {
            document: self.document.to_bridge()?,
        })
    }

    /// Destroy this C ABI value.
    pub(crate) fn destroy(&mut self) {
        self.document.destroy();
    }

    /// Return one empty C ABI value.
    pub(crate) fn empty() -> Self {
        Self {
            document: DestackDocument::empty(),
        }
    }
}

impl DestackFormatRequestArray {
    /// Convert bridge values into one C ABI array.
    pub(crate) fn from_bridge(values: Vec<rust::FormatRequest>) -> Result<Self, String> {
        let mut converted = Vec::with_capacity(values.len());
        for value in values {
            converted.push(DestackFormatRequest::from_bridge(value)?);
        }
        let (ptr, len) = owned_array(converted);
        Ok(Self { ptr, len })
    }

    /// Convert this C ABI array into bridge values.
    pub(crate) fn to_bridge(&self) -> Result<Vec<rust::FormatRequest>, String> {
        if self.len == 0 {
            return Ok(Vec::new());
        }
        if self.ptr.is_null() {
            return Err("array pointer is null".to_string());
        }
        let values = unsafe { std::slice::from_raw_parts(self.ptr, self.len) };
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

impl DestackOptionalFormatRequest {
    /// Convert one optional bridge value into one C ABI optional value.
    pub(crate) fn from_bridge(value: Option<rust::FormatRequest>) -> Result<Self, String> {
        let Some(value) = value else {
            return Ok(Self {
                is_some: false,
                value: DestackFormatRequest::empty(),
            });
        };
        Ok(Self {
            is_some: true,
            value: DestackFormatRequest::from_bridge(value)?,
        })
    }

    /// Convert this C ABI optional value into one bridge optional value.
    pub(crate) fn to_bridge(&self) -> Result<Option<rust::FormatRequest>, String> {
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
        self.value = DestackFormatRequest::empty();
    }
}

/// C ABI bridge value.
#[repr(C)]
#[derive(Debug)]
pub struct DestackLintOutput {
    /// Diagnostics emitted by lint rules.
    pub(crate) diagnostics: DestackDiagnosticArray,
}

/// C ABI bridge value array.
#[repr(C)]
#[derive(Debug)]
pub struct DestackLintOutputArray {
    /// Owned value pointer.
    pub(crate) ptr: *mut DestackLintOutput,
    /// Value count.
    pub(crate) len: usize,
}

/// C ABI optional bridge value.
#[repr(C)]
#[derive(Debug)]
pub struct DestackOptionalLintOutput {
    /// Whether the value is present.
    pub(crate) is_some: bool,
    /// Value when present.
    pub(crate) value: DestackLintOutput,
}

impl DestackLintOutput {
    /// Convert one bridge value into one C ABI value.
    pub(crate) fn from_bridge(value: rust::LintOutput) -> Result<Self, String> {
        Ok(Self {
            diagnostics: DestackDiagnosticArray::from_bridge(value.diagnostics)?,
        })
    }

    /// Convert this C ABI value into one bridge value.
    pub(crate) fn to_bridge(&self) -> Result<rust::LintOutput, String> {
        Ok(rust::LintOutput {
            diagnostics: self.diagnostics.to_bridge()?,
        })
    }

    /// Destroy this C ABI value.
    pub(crate) fn destroy(&mut self) {
        self.diagnostics.destroy();
    }

    /// Return one empty C ABI value.
    pub(crate) fn empty() -> Self {
        Self {
            diagnostics: DestackDiagnosticArray {
                ptr: ptr::null_mut(),
                len: 0,
            },
        }
    }
}

impl DestackLintOutputArray {
    /// Convert bridge values into one C ABI array.
    pub(crate) fn from_bridge(values: Vec<rust::LintOutput>) -> Result<Self, String> {
        let mut converted = Vec::with_capacity(values.len());
        for value in values {
            converted.push(DestackLintOutput::from_bridge(value)?);
        }
        let (ptr, len) = owned_array(converted);
        Ok(Self { ptr, len })
    }

    /// Convert this C ABI array into bridge values.
    pub(crate) fn to_bridge(&self) -> Result<Vec<rust::LintOutput>, String> {
        if self.len == 0 {
            return Ok(Vec::new());
        }
        if self.ptr.is_null() {
            return Err("array pointer is null".to_string());
        }
        let values = unsafe { std::slice::from_raw_parts(self.ptr, self.len) };
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

impl DestackOptionalLintOutput {
    /// Convert one optional bridge value into one C ABI optional value.
    pub(crate) fn from_bridge(value: Option<rust::LintOutput>) -> Result<Self, String> {
        let Some(value) = value else {
            return Ok(Self {
                is_some: false,
                value: DestackLintOutput::empty(),
            });
        };
        Ok(Self {
            is_some: true,
            value: DestackLintOutput::from_bridge(value)?,
        })
    }

    /// Convert this C ABI optional value into one bridge optional value.
    pub(crate) fn to_bridge(&self) -> Result<Option<rust::LintOutput>, String> {
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
        self.value = DestackLintOutput::empty();
    }
}

/// C ABI bridge enum kind.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DestackScopeKind {
    /// One module profile.
    Module = 0,
    /// One source package.
    Package = 1,
    /// Whole workspace.
    Workspace = 2,
}

/// C ABI bridge enum.
#[repr(C)]
#[derive(Debug)]
pub struct DestackScope {
    /// Active enum variant.
    pub(crate) kind: DestackScopeKind,
    pub(crate) module_module: DestackModule,
    pub(crate) profile: DestackProfileId,
    pub(crate) package_package: DestackPackageId,
}

/// C ABI bridge enum array.
#[repr(C)]
#[derive(Debug)]
pub struct DestackScopeArray {
    /// Owned value pointer.
    pub(crate) ptr: *mut DestackScope,
    /// Value count.
    pub(crate) len: usize,
}

/// C ABI optional bridge enum.
#[repr(C)]
#[derive(Debug)]
pub struct DestackOptionalScope {
    /// Whether the value is present.
    pub(crate) is_some: bool,
    /// Value when present.
    pub(crate) value: DestackScope,
}

impl DestackScope {
    /// Convert one bridge enum into one C ABI enum.
    pub(crate) fn from_bridge(value: rust::Scope) -> Result<Self, String> {
        Ok(match value {
            rust::Scope::Module { module, profile } => Self {
                kind: DestackScopeKind::Module,
                module_module: DestackModule::from_bridge(module)?,
                profile: DestackProfileId::from_bridge(profile)?,
                package_package: DestackPackageId::empty(),
            },
            rust::Scope::Package { package } => Self {
                kind: DestackScopeKind::Package,
                module_module: DestackModule::empty(),
                profile: DestackProfileId::empty(),
                package_package: DestackPackageId::from_bridge(package)?,
            },
            rust::Scope::Workspace => Self {
                kind: DestackScopeKind::Workspace,
                module_module: DestackModule::empty(),
                profile: DestackProfileId::empty(),
                package_package: DestackPackageId::empty(),
            },
        })
    }

    /// Convert this C ABI enum into one bridge enum.
    pub(crate) fn to_bridge(&self) -> Result<rust::Scope, String> {
        match self.kind {
            DestackScopeKind::Module => {
                let module_module = self.module_module.to_bridge()?;
                let profile = self.profile.to_bridge()?;
                Ok(rust::Scope::Module {
                    module: module_module,
                    profile: profile,
                })
            }
            DestackScopeKind::Package => {
                let package_package = self.package_package.to_bridge()?;
                Ok(rust::Scope::Package {
                    package: package_package,
                })
            }
            DestackScopeKind::Workspace => Ok(rust::Scope::Workspace),
        }
    }

    /// Destroy this C ABI enum.
    pub(crate) fn destroy(&mut self) {
        self.module_module.destroy();
        self.profile.destroy();
        self.package_package.destroy();
    }

    /// Return one empty C ABI enum.
    pub(crate) fn empty() -> Self {
        Self {
            kind: DestackScopeKind::Module,
            module_module: DestackModule::empty(),
            profile: DestackProfileId::empty(),
            package_package: DestackPackageId::empty(),
        }
    }
}

impl DestackScopeArray {
    /// Convert bridge values into one C ABI array.
    pub(crate) fn from_bridge(values: Vec<rust::Scope>) -> Result<Self, String> {
        let mut converted = Vec::with_capacity(values.len());
        for value in values {
            converted.push(DestackScope::from_bridge(value)?);
        }
        let (ptr, len) = owned_array(converted);
        Ok(Self { ptr, len })
    }

    /// Convert this C ABI array into bridge values.
    pub(crate) fn to_bridge(&self) -> Result<Vec<rust::Scope>, String> {
        if self.len == 0 {
            return Ok(Vec::new());
        }
        if self.ptr.is_null() {
            return Err("array pointer is null".to_string());
        }
        let values = unsafe { std::slice::from_raw_parts(self.ptr, self.len) };
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

impl DestackOptionalScope {
    /// Convert one optional bridge value into one C ABI optional value.
    pub(crate) fn from_bridge(value: Option<rust::Scope>) -> Result<Self, String> {
        let Some(value) = value else {
            return Ok(Self {
                is_some: false,
                value: DestackScope::empty(),
            });
        };
        Ok(Self {
            is_some: true,
            value: DestackScope::from_bridge(value)?,
        })
    }

    /// Convert this C ABI optional value into one bridge optional value.
    pub(crate) fn to_bridge(&self) -> Result<Option<rust::Scope>, String> {
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
        self.value = DestackScope::empty();
    }
}

/// C ABI bridge value.
#[repr(C)]
#[derive(Debug)]
pub struct DestackLintRequest {
    /// Scope to lint.
    pub(crate) scope: DestackScope,
}

/// C ABI bridge value array.
#[repr(C)]
#[derive(Debug)]
pub struct DestackLintRequestArray {
    /// Owned value pointer.
    pub(crate) ptr: *mut DestackLintRequest,
    /// Value count.
    pub(crate) len: usize,
}

/// C ABI optional bridge value.
#[repr(C)]
#[derive(Debug)]
pub struct DestackOptionalLintRequest {
    /// Whether the value is present.
    pub(crate) is_some: bool,
    /// Value when present.
    pub(crate) value: DestackLintRequest,
}

impl DestackLintRequest {
    /// Convert one bridge value into one C ABI value.
    pub(crate) fn from_bridge(value: rust::LintRequest) -> Result<Self, String> {
        Ok(Self {
            scope: DestackScope::from_bridge(value.scope)?,
        })
    }

    /// Convert this C ABI value into one bridge value.
    pub(crate) fn to_bridge(&self) -> Result<rust::LintRequest, String> {
        Ok(rust::LintRequest {
            scope: self.scope.to_bridge()?,
        })
    }

    /// Destroy this C ABI value.
    pub(crate) fn destroy(&mut self) {
        self.scope.destroy();
    }

    /// Return one empty C ABI value.
    pub(crate) fn empty() -> Self {
        Self {
            scope: DestackScope::empty(),
        }
    }
}

impl DestackLintRequestArray {
    /// Convert bridge values into one C ABI array.
    pub(crate) fn from_bridge(values: Vec<rust::LintRequest>) -> Result<Self, String> {
        let mut converted = Vec::with_capacity(values.len());
        for value in values {
            converted.push(DestackLintRequest::from_bridge(value)?);
        }
        let (ptr, len) = owned_array(converted);
        Ok(Self { ptr, len })
    }

    /// Convert this C ABI array into bridge values.
    pub(crate) fn to_bridge(&self) -> Result<Vec<rust::LintRequest>, String> {
        if self.len == 0 {
            return Ok(Vec::new());
        }
        if self.ptr.is_null() {
            return Err("array pointer is null".to_string());
        }
        let values = unsafe { std::slice::from_raw_parts(self.ptr, self.len) };
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

impl DestackOptionalLintRequest {
    /// Convert one optional bridge value into one C ABI optional value.
    pub(crate) fn from_bridge(value: Option<rust::LintRequest>) -> Result<Self, String> {
        let Some(value) = value else {
            return Ok(Self {
                is_some: false,
                value: DestackLintRequest::empty(),
            });
        };
        Ok(Self {
            is_some: true,
            value: DestackLintRequest::from_bridge(value)?,
        })
    }

    /// Convert this C ABI optional value into one bridge optional value.
    pub(crate) fn to_bridge(&self) -> Result<Option<rust::LintRequest>, String> {
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
        self.value = DestackLintRequest::empty();
    }
}

/// C ABI bridge value.
#[repr(C)]
#[derive(Debug)]
pub struct DestackSessionFile {
    /// Repository logical path.
    pub(crate) path: *mut c_char,
}

/// C ABI bridge value array.
#[repr(C)]
#[derive(Debug)]
pub struct DestackSessionFileArray {
    /// Owned value pointer.
    pub(crate) ptr: *mut DestackSessionFile,
    /// Value count.
    pub(crate) len: usize,
}

/// C ABI optional bridge value.
#[repr(C)]
#[derive(Debug)]
pub struct DestackOptionalSessionFile {
    /// Whether the value is present.
    pub(crate) is_some: bool,
    /// Value when present.
    pub(crate) value: DestackSessionFile,
}

impl DestackSessionFile {
    /// Convert one bridge value into one C ABI value.
    pub(crate) fn from_bridge(value: rust::SessionFile) -> Result<Self, String> {
        Ok(Self {
            path: c_string(value.path)?,
        })
    }

    /// Convert this C ABI value into one bridge value.
    pub(crate) fn to_bridge(&self) -> Result<rust::SessionFile, String> {
        Ok(rust::SessionFile {
            path: read_string(self.path)?,
        })
    }

    /// Destroy this C ABI value.
    pub(crate) fn destroy(&mut self) {
        destroy_string(self.path);
        self.path = ptr::null_mut();
    }

    /// Return one empty C ABI value.
    pub(crate) fn empty() -> Self {
        Self {
            path: ptr::null_mut(),
        }
    }
}

impl DestackSessionFileArray {
    /// Convert bridge values into one C ABI array.
    pub(crate) fn from_bridge(values: Vec<rust::SessionFile>) -> Result<Self, String> {
        let mut converted = Vec::with_capacity(values.len());
        for value in values {
            converted.push(DestackSessionFile::from_bridge(value)?);
        }
        let (ptr, len) = owned_array(converted);
        Ok(Self { ptr, len })
    }

    /// Convert this C ABI array into bridge values.
    pub(crate) fn to_bridge(&self) -> Result<Vec<rust::SessionFile>, String> {
        if self.len == 0 {
            return Ok(Vec::new());
        }
        if self.ptr.is_null() {
            return Err("array pointer is null".to_string());
        }
        let values = unsafe { std::slice::from_raw_parts(self.ptr, self.len) };
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

impl DestackOptionalSessionFile {
    /// Convert one optional bridge value into one C ABI optional value.
    pub(crate) fn from_bridge(value: Option<rust::SessionFile>) -> Result<Self, String> {
        let Some(value) = value else {
            return Ok(Self {
                is_some: false,
                value: DestackSessionFile::empty(),
            });
        };
        Ok(Self {
            is_some: true,
            value: DestackSessionFile::from_bridge(value)?,
        })
    }

    /// Convert this C ABI optional value into one bridge optional value.
    pub(crate) fn to_bridge(&self) -> Result<Option<rust::SessionFile>, String> {
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
        self.value = DestackSessionFile::empty();
    }
}

/// C ABI bridge value.
#[repr(C)]
#[derive(Debug)]
pub struct DestackTextRange {
    /// Inclusive start byte offset.
    pub(crate) start: u32,
    /// Exclusive end byte offset.
    pub(crate) end: u32,
}

/// C ABI bridge value array.
#[repr(C)]
#[derive(Debug)]
pub struct DestackTextRangeArray {
    /// Owned value pointer.
    pub(crate) ptr: *mut DestackTextRange,
    /// Value count.
    pub(crate) len: usize,
}

/// C ABI optional bridge value.
#[repr(C)]
#[derive(Debug)]
pub struct DestackOptionalTextRange {
    /// Whether the value is present.
    pub(crate) is_some: bool,
    /// Value when present.
    pub(crate) value: DestackTextRange,
}

impl DestackTextRange {
    /// Convert one bridge value into one C ABI value.
    pub(crate) fn from_bridge(value: rust::TextRange) -> Result<Self, String> {
        Ok(Self {
            start: value.start,
            end: value.end,
        })
    }

    /// Convert this C ABI value into one bridge value.
    pub(crate) fn to_bridge(&self) -> Result<rust::TextRange, String> {
        Ok(rust::TextRange {
            start: self.start,
            end: self.end,
        })
    }

    /// Destroy this C ABI value.
    pub(crate) fn destroy(&mut self) {}
    /// Return one empty C ABI value.
    pub(crate) fn empty() -> Self {
        Self { start: 0, end: 0 }
    }
}

impl DestackTextRangeArray {
    /// Convert bridge values into one C ABI array.
    pub(crate) fn from_bridge(values: Vec<rust::TextRange>) -> Result<Self, String> {
        let mut converted = Vec::with_capacity(values.len());
        for value in values {
            converted.push(DestackTextRange::from_bridge(value)?);
        }
        let (ptr, len) = owned_array(converted);
        Ok(Self { ptr, len })
    }

    /// Convert this C ABI array into bridge values.
    pub(crate) fn to_bridge(&self) -> Result<Vec<rust::TextRange>, String> {
        if self.len == 0 {
            return Ok(Vec::new());
        }
        if self.ptr.is_null() {
            return Err("array pointer is null".to_string());
        }
        let values = unsafe { std::slice::from_raw_parts(self.ptr, self.len) };
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

impl DestackOptionalTextRange {
    /// Convert one optional bridge value into one C ABI optional value.
    pub(crate) fn from_bridge(value: Option<rust::TextRange>) -> Result<Self, String> {
        let Some(value) = value else {
            return Ok(Self {
                is_some: false,
                value: DestackTextRange::empty(),
            });
        };
        Ok(Self {
            is_some: true,
            value: DestackTextRange::from_bridge(value)?,
        })
    }

    /// Convert this C ABI optional value into one bridge optional value.
    pub(crate) fn to_bridge(&self) -> Result<Option<rust::TextRange>, String> {
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
        self.value = DestackTextRange::empty();
    }
}

/// C ABI bridge value.
#[repr(C)]
#[derive(Debug)]
pub struct DestackTextEdit {
    /// Replaced byte range.
    pub(crate) range: DestackTextRange,
    /// Replacement text.
    pub(crate) text: *mut c_char,
}

/// C ABI bridge value array.
#[repr(C)]
#[derive(Debug)]
pub struct DestackTextEditArray {
    /// Owned value pointer.
    pub(crate) ptr: *mut DestackTextEdit,
    /// Value count.
    pub(crate) len: usize,
}

/// C ABI optional bridge value.
#[repr(C)]
#[derive(Debug)]
pub struct DestackOptionalTextEdit {
    /// Whether the value is present.
    pub(crate) is_some: bool,
    /// Value when present.
    pub(crate) value: DestackTextEdit,
}

impl DestackTextEdit {
    /// Convert one bridge value into one C ABI value.
    pub(crate) fn from_bridge(value: rust::TextEdit) -> Result<Self, String> {
        Ok(Self {
            range: DestackTextRange::from_bridge(value.range)?,
            text: c_string(value.text)?,
        })
    }

    /// Convert this C ABI value into one bridge value.
    pub(crate) fn to_bridge(&self) -> Result<rust::TextEdit, String> {
        Ok(rust::TextEdit {
            range: self.range.to_bridge()?,
            text: read_string(self.text)?,
        })
    }

    /// Destroy this C ABI value.
    pub(crate) fn destroy(&mut self) {
        self.range.destroy();
        destroy_string(self.text);
        self.text = ptr::null_mut();
    }

    /// Return one empty C ABI value.
    pub(crate) fn empty() -> Self {
        Self {
            range: DestackTextRange::empty(),
            text: ptr::null_mut(),
        }
    }
}

impl DestackTextEditArray {
    /// Convert bridge values into one C ABI array.
    pub(crate) fn from_bridge(values: Vec<rust::TextEdit>) -> Result<Self, String> {
        let mut converted = Vec::with_capacity(values.len());
        for value in values {
            converted.push(DestackTextEdit::from_bridge(value)?);
        }
        let (ptr, len) = owned_array(converted);
        Ok(Self { ptr, len })
    }

    /// Convert this C ABI array into bridge values.
    pub(crate) fn to_bridge(&self) -> Result<Vec<rust::TextEdit>, String> {
        if self.len == 0 {
            return Ok(Vec::new());
        }
        if self.ptr.is_null() {
            return Err("array pointer is null".to_string());
        }
        let values = unsafe { std::slice::from_raw_parts(self.ptr, self.len) };
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

impl DestackOptionalTextEdit {
    /// Convert one optional bridge value into one C ABI optional value.
    pub(crate) fn from_bridge(value: Option<rust::TextEdit>) -> Result<Self, String> {
        let Some(value) = value else {
            return Ok(Self {
                is_some: false,
                value: DestackTextEdit::empty(),
            });
        };
        Ok(Self {
            is_some: true,
            value: DestackTextEdit::from_bridge(value)?,
        })
    }

    /// Convert this C ABI optional value into one bridge optional value.
    pub(crate) fn to_bridge(&self) -> Result<Option<rust::TextEdit>, String> {
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
        self.value = DestackTextEdit::empty();
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
        let value = rust::ArtifactKey::Build { target };
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
        let value = rust::ArtifactKey::DirParsed { module };
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
        let value = rust::ArtifactKey::Data { module };
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
        let value = rust::ArtifactKey::GlobalEnvironment { profile };
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
        let value = rust::ArtifactKey::PackageIndex { profile };
        let key = Box::into_raw(Box::new(DestackArtifactKey { value }));
        write_out(out, key, "artifact key output is null")
    })
}

/// Create one artifact key handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_artifact_key_module_index(
    profile: DestackProfileId,
    out: *mut *mut DestackArtifactKey,
    error: *mut *mut DestackError,
) -> DestackStatus {
    return_status(error, || {
        let profile = profile.to_bridge()?;
        let value = rust::ArtifactKey::ModuleIndex { profile };
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
        let value = rust::ArtifactKey::ComponentGraph { profile };
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
        let value = rust::ArtifactKey::ProgramAnalysis { profile, target };
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
        let value = rust::ArtifactKey::DirBound { module, profile };
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
        let value = rust::ArtifactKey::DirImported { module, profile };
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
        let value = rust::ArtifactKey::DirExpanded { module, profile };
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
        let value = rust::ArtifactKey::DirExported { module, profile };
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
        let value = rust::ArtifactKey::DirResolved { module, profile };
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
        let value = rust::ArtifactKey::DirCheckedComponent {
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
        let value = rust::ArtifactKey::DirChecked { module, profile };
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
        let value = rust::ArtifactKey::DirMaterialized { module, profile };
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
        let value = rust::ArtifactKey::DirElaborated { module, profile };
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
        let value = rust::ArtifactKey::MirLowered {
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
        let value = rust::ArtifactKey::MirVerified {
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
        let value = rust::ArtifactKey::MirAnalyzed {
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
        let value = rust::ArtifactKey::MirOptimized {
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
        let value = rust::ArtifactKey::ModuleQueryIndex { module, profile };
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
        let value = rust::ArtifactKey::WorkspaceQueryIndex { profile };
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
        let value = rust::ArtifactKey::Script { module, target };
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
        let value = rust::ArtifactKey::Object { module, target };
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
        let value = rust::ArtifactKey::Asset { module, target };
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
        let value = rust::ArtifactKey::Bundle { package, target };
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
        let value = rust::ArtifactKey::Program { package, target };
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
        let value = rust::ArtifactKey::Product { package, product };
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
        let value = rust::ArtifactKey::ModuleLinted { module, profile };
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
        let value = rust::ArtifactKey::PackageLinted { package };
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
        let value = rust::ArtifactKey::WorkspaceLinted;
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
pub unsafe extern "C" fn destack_artifact_path_state_destroy(value: *mut DestackArtifactPathState) {
    if let Some(value) = unsafe { value.as_mut() } {
        value.destroy();
    }
}

/// Destroy one C ABI bridge value array.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_artifact_path_state_array_destroy(
    mut array: DestackArtifactPathStateArray,
) {
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
pub unsafe extern "C" fn destack_artifact_directory_entry_destroy(
    value: *mut DestackArtifactDirectoryEntry,
) {
    if let Some(value) = unsafe { value.as_mut() } {
        value.destroy();
    }
}

/// Destroy one C ABI bridge value array.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_artifact_directory_entry_array_destroy(
    mut array: DestackArtifactDirectoryEntryArray,
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
pub unsafe extern "C" fn destack_artifact_source_dependency_destroy(
    value: *mut DestackArtifactSourceDependency,
) {
    if let Some(value) = unsafe { value.as_mut() } {
        value.destroy();
    }
}

/// Destroy one C ABI bridge value array.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_artifact_source_dependency_array_destroy(
    mut array: DestackArtifactSourceDependencyArray,
) {
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
pub unsafe extern "C" fn destack_replacement_destroy(value: *mut DestackReplacement) {
    if let Some(value) = unsafe { value.as_mut() } {
        value.destroy();
    }
}

/// Destroy one C ABI bridge value array.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_replacement_array_destroy(mut array: DestackReplacementArray) {
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
pub unsafe extern "C" fn destack_batch_edit_destroy(value: *mut DestackBatchEdit) {
    if let Some(value) = unsafe { value.as_mut() } {
        value.destroy();
    }
}

/// Destroy one C ABI bridge value array.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_batch_edit_array_destroy(mut array: DestackBatchEditArray) {
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
pub unsafe extern "C" fn destack_file_type_destroy(value: *mut DestackFileType) {
    if let Some(value) = unsafe { value.as_mut() } {
        value.destroy();
    }
}

/// Destroy one C ABI bridge value array.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_file_type_array_destroy(mut array: DestackFileTypeArray) {
    array.destroy();
}

/// Destroy one C ABI bridge value.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_source_map_source_destroy(value: *mut DestackSourceMapSource) {
    if let Some(value) = unsafe { value.as_mut() } {
        value.destroy();
    }
}

/// Destroy one C ABI bridge value array.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_source_map_source_array_destroy(
    mut array: DestackSourceMapSourceArray,
) {
    array.destroy();
}

/// Destroy one C ABI bridge value.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_source_map_destroy(value: *mut DestackSourceMap) {
    if let Some(value) = unsafe { value.as_mut() } {
        value.destroy();
    }
}

/// Destroy one C ABI bridge value array.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_source_map_array_destroy(mut array: DestackSourceMapArray) {
    array.destroy();
}

/// Destroy one C ABI bridge value.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_asset_destroy(value: *mut DestackAsset) {
    if let Some(value) = unsafe { value.as_mut() } {
        value.destroy();
    }
}

/// Destroy one C ABI bridge value array.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_asset_array_destroy(mut array: DestackAssetArray) {
    array.destroy();
}

/// Destroy one C ABI bridge value.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_build_linkage_destroy(value: *mut DestackBuildLinkage) {
    if let Some(value) = unsafe { value.as_mut() } {
        value.destroy();
    }
}

/// Destroy one C ABI bridge value array.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_build_linkage_array_destroy(mut array: DestackBuildLinkageArray) {
    array.destroy();
}

/// Destroy one C ABI bridge value.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_build_profile_destroy(value: *mut DestackBuildProfile) {
    if let Some(value) = unsafe { value.as_mut() } {
        value.destroy();
    }
}

/// Destroy one C ABI bridge value array.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_build_profile_array_destroy(mut array: DestackBuildProfileArray) {
    array.destroy();
}

/// Destroy one C ABI bridge value.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_build_destroy(value: *mut DestackBuild) {
    if let Some(value) = unsafe { value.as_mut() } {
        value.destroy();
    }
}

/// Destroy one C ABI bridge value array.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_build_array_destroy(mut array: DestackBuildArray) {
    array.destroy();
}

/// Destroy one C ABI bridge value.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_bundle_section_destroy(value: *mut DestackBundleSection) {
    if let Some(value) = unsafe { value.as_mut() } {
        value.destroy();
    }
}

/// Destroy one C ABI bridge value array.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_bundle_section_array_destroy(
    mut array: DestackBundleSectionArray,
) {
    array.destroy();
}

/// Destroy one C ABI bridge value.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_bundle_file_destroy(value: *mut DestackBundleFile) {
    if let Some(value) = unsafe { value.as_mut() } {
        value.destroy();
    }
}

/// Destroy one C ABI bridge value array.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_bundle_file_array_destroy(mut array: DestackBundleFileArray) {
    array.destroy();
}

/// Destroy one C ABI bridge value.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_bundle_mode_destroy(value: *mut DestackBundleMode) {
    if let Some(value) = unsafe { value.as_mut() } {
        value.destroy();
    }
}

/// Destroy one C ABI bridge value array.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_bundle_mode_array_destroy(mut array: DestackBundleModeArray) {
    array.destroy();
}

/// Destroy one C ABI bridge value.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_emit_format_destroy(value: *mut DestackEmitFormat) {
    if let Some(value) = unsafe { value.as_mut() } {
        value.destroy();
    }
}

/// Destroy one C ABI bridge value array.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_emit_format_array_destroy(mut array: DestackEmitFormatArray) {
    array.destroy();
}

/// Destroy one C ABI bridge value.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_bundle_destroy(value: *mut DestackBundle) {
    if let Some(value) = unsafe { value.as_mut() } {
        value.destroy();
    }
}

/// Destroy one C ABI bridge value array.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_bundle_array_destroy(mut array: DestackBundleArray) {
    array.destroy();
}

/// Destroy one C ABI bridge value.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_object_format_destroy(value: *mut DestackObjectFormat) {
    if let Some(value) = unsafe { value.as_mut() } {
        value.destroy();
    }
}

/// Destroy one C ABI bridge value array.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_object_format_array_destroy(mut array: DestackObjectFormatArray) {
    array.destroy();
}

/// Destroy one C ABI bridge value.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_object_destroy(value: *mut DestackObject) {
    if let Some(value) = unsafe { value.as_mut() } {
        value.destroy();
    }
}

/// Destroy one C ABI bridge value array.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_object_array_destroy(mut array: DestackObjectArray) {
    array.destroy();
}

/// Destroy one C ABI bridge value.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_host_destroy(value: *mut DestackHost) {
    if let Some(value) = unsafe { value.as_mut() } {
        value.destroy();
    }
}

/// Destroy one C ABI bridge value array.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_host_array_destroy(mut array: DestackHostArray) {
    array.destroy();
}

/// Destroy one C ABI bridge value.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_runtime_destroy(value: *mut DestackRuntime) {
    if let Some(value) = unsafe { value.as_mut() } {
        value.destroy();
    }
}

/// Destroy one C ABI bridge value array.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_runtime_array_destroy(mut array: DestackRuntimeArray) {
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

/// Destroy one C ABI bridge value.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_product_target_destroy(value: *mut DestackProductTarget) {
    if let Some(value) = unsafe { value.as_mut() } {
        value.destroy();
    }
}

/// Destroy one C ABI bridge value array.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_product_target_array_destroy(
    mut array: DestackProductTargetArray,
) {
    array.destroy();
}

/// Destroy one C ABI bridge value.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_product_destroy(value: *mut DestackProduct) {
    if let Some(value) = unsafe { value.as_mut() } {
        value.destroy();
    }
}

/// Destroy one C ABI bridge value array.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_product_array_destroy(mut array: DestackProductArray) {
    array.destroy();
}

/// Destroy one C ABI bridge value.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_program_format_destroy(value: *mut DestackProgramFormat) {
    if let Some(value) = unsafe { value.as_mut() } {
        value.destroy();
    }
}

/// Destroy one C ABI bridge value array.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_program_format_array_destroy(
    mut array: DestackProgramFormatArray,
) {
    array.destroy();
}

/// Destroy one C ABI bridge value.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_program_header_destroy(value: *mut DestackProgramHeader) {
    if let Some(value) = unsafe { value.as_mut() } {
        value.destroy();
    }
}

/// Destroy one C ABI bridge value array.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_program_header_array_destroy(
    mut array: DestackProgramHeaderArray,
) {
    array.destroy();
}

/// Destroy one C ABI bridge value.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_program_destroy(value: *mut DestackProgram) {
    if let Some(value) = unsafe { value.as_mut() } {
        value.destroy();
    }
}

/// Destroy one C ABI bridge value array.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_program_array_destroy(mut array: DestackProgramArray) {
    array.destroy();
}

/// Destroy one C ABI bridge value.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_declaration_destroy(value: *mut DestackDeclaration) {
    if let Some(value) = unsafe { value.as_mut() } {
        value.destroy();
    }
}

/// Destroy one C ABI bridge value array.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_declaration_array_destroy(mut array: DestackDeclarationArray) {
    array.destroy();
}

/// Destroy one C ABI bridge value.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_script_language_destroy(value: *mut DestackScriptLanguage) {
    if let Some(value) = unsafe { value.as_mut() } {
        value.destroy();
    }
}

/// Destroy one C ABI bridge value array.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_script_language_array_destroy(
    mut array: DestackScriptLanguageArray,
) {
    array.destroy();
}

/// Destroy one C ABI bridge value.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_script_destroy(value: *mut DestackScript) {
    if let Some(value) = unsafe { value.as_mut() } {
        value.destroy();
    }
}

/// Destroy one C ABI bridge value array.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_script_array_destroy(mut array: DestackScriptArray) {
    array.destroy();
}

/// Destroy one C ABI bridge value.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_build_output_destroy(value: *mut DestackBuildOutput) {
    if let Some(value) = unsafe { value.as_mut() } {
        value.destroy();
    }
}

/// Destroy one C ABI bridge value array.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_build_output_array_destroy(mut array: DestackBuildOutputArray) {
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
pub unsafe extern "C" fn destack_module_destroy(value: *mut DestackModule) {
    if let Some(value) = unsafe { value.as_mut() } {
        value.destroy();
    }
}

/// Destroy one C ABI bridge value array.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_module_array_destroy(mut array: DestackModuleArray) {
    array.destroy();
}

/// Destroy one C ABI bridge value.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_module_build_kind_destroy(value: *mut DestackModuleBuildKind) {
    if let Some(value) = unsafe { value.as_mut() } {
        value.destroy();
    }
}

/// Destroy one C ABI bridge value array.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_module_build_kind_array_destroy(
    mut array: DestackModuleBuildKindArray,
) {
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
pub unsafe extern "C" fn destack_build_request_destroy(value: *mut DestackBuildRequest) {
    if let Some(value) = unsafe { value.as_mut() } {
        value.destroy();
    }
}

/// Destroy one C ABI bridge value array.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_build_request_array_destroy(mut array: DestackBuildRequestArray) {
    array.destroy();
}

/// Destroy one C ABI bridge value.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_change_destroy(value: *mut DestackChange) {
    if let Some(value) = unsafe { value.as_mut() } {
        value.destroy();
    }
}

/// Destroy one C ABI bridge value array.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_change_array_destroy(mut array: DestackChangeArray) {
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
pub unsafe extern "C" fn destack_commit_destroy(value: *mut DestackCommit) {
    if let Some(value) = unsafe { value.as_mut() } {
        value.destroy();
    }
}

/// Destroy one C ABI bridge value array.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_commit_array_destroy(mut array: DestackCommitArray) {
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
pub unsafe extern "C" fn destack_document_destroy(value: *mut DestackDocument) {
    if let Some(value) = unsafe { value.as_mut() } {
        value.destroy();
    }
}

/// Destroy one C ABI bridge value array.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_document_array_destroy(mut array: DestackDocumentArray) {
    array.destroy();
}

/// Destroy one C ABI bridge value.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_format_output_destroy(value: *mut DestackFormatOutput) {
    if let Some(value) = unsafe { value.as_mut() } {
        value.destroy();
    }
}

/// Destroy one C ABI bridge value array.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_format_output_array_destroy(mut array: DestackFormatOutputArray) {
    array.destroy();
}

/// Destroy one C ABI bridge value.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_format_request_destroy(value: *mut DestackFormatRequest) {
    if let Some(value) = unsafe { value.as_mut() } {
        value.destroy();
    }
}

/// Destroy one C ABI bridge value array.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_format_request_array_destroy(
    mut array: DestackFormatRequestArray,
) {
    array.destroy();
}

/// Destroy one C ABI bridge value.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_lint_output_destroy(value: *mut DestackLintOutput) {
    if let Some(value) = unsafe { value.as_mut() } {
        value.destroy();
    }
}

/// Destroy one C ABI bridge value array.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_lint_output_array_destroy(mut array: DestackLintOutputArray) {
    array.destroy();
}

/// Destroy one C ABI bridge value.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_scope_destroy(value: *mut DestackScope) {
    if let Some(value) = unsafe { value.as_mut() } {
        value.destroy();
    }
}

/// Destroy one C ABI bridge value array.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_scope_array_destroy(mut array: DestackScopeArray) {
    array.destroy();
}

/// Destroy one C ABI bridge value.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_lint_request_destroy(value: *mut DestackLintRequest) {
    if let Some(value) = unsafe { value.as_mut() } {
        value.destroy();
    }
}

/// Destroy one C ABI bridge value array.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_lint_request_array_destroy(mut array: DestackLintRequestArray) {
    array.destroy();
}

/// Destroy one C ABI bridge value.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_session_file_destroy(value: *mut DestackSessionFile) {
    if let Some(value) = unsafe { value.as_mut() } {
        value.destroy();
    }
}

/// Destroy one C ABI bridge value array.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_session_file_array_destroy(mut array: DestackSessionFileArray) {
    array.destroy();
}

/// Destroy one C ABI bridge value.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_text_range_destroy(value: *mut DestackTextRange) {
    if let Some(value) = unsafe { value.as_mut() } {
        value.destroy();
    }
}

/// Destroy one C ABI bridge value array.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_text_range_array_destroy(mut array: DestackTextRangeArray) {
    array.destroy();
}

/// Destroy one C ABI bridge value.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_text_edit_destroy(value: *mut DestackTextEdit) {
    if let Some(value) = unsafe { value.as_mut() } {
        value.destroy();
    }
}

/// Destroy one C ABI bridge value array.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_text_edit_array_destroy(mut array: DestackTextEditArray) {
    array.destroy();
}
