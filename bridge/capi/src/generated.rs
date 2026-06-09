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
pub struct DestackFileContentId {
    /// Canonical lowercase hex file content id.
    pub(crate) id: *mut c_char,
}

/// C ABI bridge value array.
#[repr(C)]
#[derive(Debug)]
pub struct DestackFileContentIdArray {
    /// Owned value pointer.
    pub(crate) ptr: *mut DestackFileContentId,
    /// Value count.
    pub(crate) len: usize,
}

/// C ABI optional bridge value.
#[repr(C)]
#[derive(Debug)]
pub struct DestackOptionalFileContentId {
    /// Whether the value is present.
    pub(crate) is_some: bool,
    /// Value when present.
    pub(crate) value: DestackFileContentId,
}

impl DestackFileContentId {
    /// Convert one bridge value into one C ABI value.
    pub(crate) fn from_bridge(value: rust::FileContentId) -> Result<Self, String> {
        Ok(Self {
            id: c_string(value.id)?,
        })
    }

    /// Convert this C ABI value into one bridge value.
    pub(crate) fn to_bridge(&self) -> Result<rust::FileContentId, String> {
        Ok(rust::FileContentId {
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

impl DestackFileContentIdArray {
    /// Convert bridge values into one C ABI array.
    pub(crate) fn from_bridge(values: Vec<rust::FileContentId>) -> Result<Self, String> {
        let mut converted = Vec::with_capacity(values.len());
        for value in values {
            converted.push(DestackFileContentId::from_bridge(value)?);
        }
        let (ptr, len) = owned_array(converted);
        Ok(Self { ptr, len })
    }

    /// Convert this C ABI array into bridge values.
    pub(crate) fn to_bridge(&self) -> Result<Vec<rust::FileContentId>, String> {
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

impl DestackOptionalFileContentId {
    /// Convert one optional bridge value into one C ABI optional value.
    pub(crate) fn from_bridge(value: Option<rust::FileContentId>) -> Result<Self, String> {
        let Some(value) = value else {
            return Ok(Self {
                is_some: false,
                value: DestackFileContentId::empty(),
            });
        };
        Ok(Self {
            is_some: true,
            value: DestackFileContentId::from_bridge(value)?,
        })
    }

    /// Convert this C ABI optional value into one bridge optional value.
    pub(crate) fn to_bridge(&self) -> Result<Option<rust::FileContentId>, String> {
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
        self.value = DestackFileContentId::empty();
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
    pub(crate) content: DestackFileContentId,
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
                content: DestackFileContentId::empty(),
            },
            rust::ArtifactSourceDependency::DirectoryEntries { directory, entries } => Self {
                kind: DestackArtifactSourceDependencyKind::DirectoryEntries,
                path: DestackFileId::empty(),
                state: DestackArtifactPathState::empty(),
                directory: DestackFileId::from_bridge(directory)?,
                entries: DestackArtifactDirectoryEntryArray::from_bridge(entries)?,
                file: DestackFileId::empty(),
                content: DestackFileContentId::empty(),
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
                content: DestackFileContentId::from_bridge(content)?,
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
            content: DestackFileContentId::empty(),
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
pub enum DestackFileContentKind {
    /// Text file content.
    Text = 0,
    /// Binary file content.
    Binary = 1,
}

/// C ABI bridge enum.
#[repr(C)]
#[derive(Debug)]
pub struct DestackFileContent {
    /// Active enum variant.
    pub(crate) kind: DestackFileContentKind,
    pub(crate) text_content: *mut c_char,
    pub(crate) binary_content: DestackByteArray,
}

/// C ABI bridge enum array.
#[repr(C)]
#[derive(Debug)]
pub struct DestackFileContentArray {
    /// Owned value pointer.
    pub(crate) ptr: *mut DestackFileContent,
    /// Value count.
    pub(crate) len: usize,
}

/// C ABI optional bridge enum.
#[repr(C)]
#[derive(Debug)]
pub struct DestackOptionalFileContent {
    /// Whether the value is present.
    pub(crate) is_some: bool,
    /// Value when present.
    pub(crate) value: DestackFileContent,
}

impl DestackFileContent {
    /// Convert one bridge enum into one C ABI enum.
    pub(crate) fn from_bridge(value: rust::FileContent) -> Result<Self, String> {
        Ok(match value {
            rust::FileContent::Text { content } => Self {
                kind: DestackFileContentKind::Text,
                text_content: c_string(content)?,
                binary_content: DestackByteArray {
                    ptr: ptr::null_mut(),
                    len: 0,
                },
            },
            rust::FileContent::Binary { content } => Self {
                kind: DestackFileContentKind::Binary,
                text_content: ptr::null_mut(),
                binary_content: DestackByteArray::from_vec(content),
            },
        })
    }

    /// Convert this C ABI enum into one bridge enum.
    pub(crate) fn to_bridge(&self) -> Result<rust::FileContent, String> {
        match self.kind {
            DestackFileContentKind::Text => {
                let text_content = read_string(self.text_content)?;
                Ok(rust::FileContent::Text {
                    content: text_content,
                })
            }
            DestackFileContentKind::Binary => {
                let binary_content = read_bytes(
                    self.binary_content.ptr.cast_const(),
                    self.binary_content.len,
                )?;
                Ok(rust::FileContent::Binary {
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
            kind: DestackFileContentKind::Text,
            text_content: ptr::null_mut(),
            binary_content: DestackByteArray {
                ptr: ptr::null_mut(),
                len: 0,
            },
        }
    }
}

impl DestackFileContentArray {
    /// Convert bridge values into one C ABI array.
    pub(crate) fn from_bridge(values: Vec<rust::FileContent>) -> Result<Self, String> {
        let mut converted = Vec::with_capacity(values.len());
        for value in values {
            converted.push(DestackFileContent::from_bridge(value)?);
        }
        let (ptr, len) = owned_array(converted);
        Ok(Self { ptr, len })
    }

    /// Convert this C ABI array into bridge values.
    pub(crate) fn to_bridge(&self) -> Result<Vec<rust::FileContent>, String> {
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

impl DestackOptionalFileContent {
    /// Convert one optional bridge value into one C ABI optional value.
    pub(crate) fn from_bridge(value: Option<rust::FileContent>) -> Result<Self, String> {
        let Some(value) = value else {
            return Ok(Self {
                is_some: false,
                value: DestackFileContent::empty(),
            });
        };
        Ok(Self {
            is_some: true,
            value: DestackFileContent::from_bridge(value)?,
        })
    }

    /// Convert this C ABI optional value into one bridge optional value.
    pub(crate) fn to_bridge(&self) -> Result<Option<rust::FileContent>, String> {
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
        self.value = DestackFileContent::empty();
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
    pub(crate) content: DestackFileContent,
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
            content: DestackFileContent::from_bridge(value.content)?,
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
            content: DestackFileContent::empty(),
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
    /// Exact file content containing the span.
    pub(crate) content: DestackFileContentId,
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
            content: DestackFileContentId::from_bridge(value.content)?,
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
            content: DestackFileContentId::empty(),
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
    /// Serialized artifact image bytes.
    pub(crate) image: DestackByteArray,
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
            image: DestackByteArray::from_vec(value.image),
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
            image: read_bytes(self.image.ptr.cast_const(), self.image.len)?,
            strings: self.strings.to_bridge()?,
            dependencies: self.dependencies.to_bridge()?,
            diagnostics: self.diagnostics.to_bridge()?,
            sidecars: self.sidecars.to_bridge()?,
        })
    }

    /// Destroy this C ABI value.
    pub(crate) fn destroy(&mut self) {
        self.version.destroy();
        self.image.destroy();
        self.strings.destroy();
        self.dependencies.destroy();
        self.diagnostics.destroy();
        self.sidecars.destroy();
    }

    /// Return one empty C ABI value.
    pub(crate) fn empty() -> Self {
        Self {
            version: DestackArtifactVersion::empty(),
            image: DestackByteArray {
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
pub unsafe extern "C" fn destack_artifact_key_dependency_index(
    profile: DestackProfileId,
    out: *mut *mut DestackArtifactKey,
    error: *mut *mut DestackError,
) -> DestackStatus {
    return_status(error, || {
        let profile = profile.to_bridge()?;
        let value = rust::ArtifactKey::DependencyIndex { profile };
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
pub unsafe extern "C" fn destack_artifact_key_module_output(
    module: DestackModuleId,
    target: DestackTargetId,
    out: *mut *mut DestackArtifactKey,
    error: *mut *mut DestackError,
) -> DestackStatus {
    return_status(error, || {
        let module = module.to_bridge()?;
        let target = target.to_bridge()?;
        let value = rust::ArtifactKey::ModuleOutput { module, target };
        let key = Box::into_raw(Box::new(DestackArtifactKey { value }));
        write_out(out, key, "artifact key output is null")
    })
}

/// Create one artifact key handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_artifact_key_package_output(
    package: DestackPackageId,
    target: DestackTargetId,
    out: *mut *mut DestackArtifactKey,
    error: *mut *mut DestackError,
) -> DestackStatus {
    return_status(error, || {
        let package = package.to_bridge()?;
        let target = target.to_bridge()?;
        let value = rust::ArtifactKey::PackageOutput { package, target };
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
pub unsafe extern "C" fn destack_file_content_id_destroy(value: *mut DestackFileContentId) {
    if let Some(value) = unsafe { value.as_mut() } {
        value.destroy();
    }
}

/// Destroy one C ABI bridge value array.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_file_content_id_array_destroy(
    mut array: DestackFileContentIdArray,
) {
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
pub unsafe extern "C" fn destack_file_content_destroy(value: *mut DestackFileContent) {
    if let Some(value) = unsafe { value.as_mut() } {
        value.destroy();
    }
}

/// Destroy one C ABI bridge value array.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_file_content_array_destroy(mut array: DestackFileContentArray) {
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
