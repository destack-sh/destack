use destack_core::{SectionImage, SectionPacker, SectionSlice};
use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use crate::{Global, GlobalAddress};

/// Native projection of immutable constant memory.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeConstantSpace {
    /// The first byte in the constant space.
    pub bytes: *const u8,
    /// The constant space byte count.
    pub byte_len: usize,
}

/// Native projection of mutable static memory.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeStaticSpace {
    /// The first byte in the static space.
    pub bytes: *mut u8,
    /// The static space byte count.
    pub byte_len: usize,
}

/// Section-backed static memory image carried by a program.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct StaticImage {
    /// Static bytes.
    bytes: SectionSlice<u8>,
}

impl StaticImage {
    /// Pack one static image.
    pub(crate) fn pack(sections: &mut SectionPacker, bytes: Vec<u8>) -> Self {
        let bytes = sections.insert(bytes);

        Self { bytes }
    }

    /// Borrow one global byte range.
    pub fn bytes<'a>(&self, sections: SectionImage<'a>, global: &Global) -> Option<&'a [u8]> {
        let end = global.offset() + global.byte_len();

        sections.entries(self.bytes).get(global.offset()..end)
    }

    /// Return a native address for one static byte range.
    pub fn native_address(
        &self,
        sections: SectionImage<'_>,
        global: &Global,
        address: GlobalAddress,
        byte_len: usize,
    ) -> Option<usize> {
        let start = address.byte_offset();
        let end = start.checked_add(byte_len)?;
        if end > global.byte_len() {
            return None;
        }

        Some(sections.entries(self.bytes).as_ptr() as usize + global.offset() + start)
    }

    /// Return whether static memory owns one byte range.
    pub fn owns_address_range(
        &self,
        sections: SectionImage<'_>,
        global: &Global,
        address: GlobalAddress,
        byte_len: usize,
    ) -> bool {
        self.native_address(sections, global, address, byte_len)
            .is_some()
    }

    /// Materialize this image into mutable runtime static memory.
    pub fn materialize(&self, sections: SectionImage<'_>) -> StaticSpace {
        StaticSpace::new(sections.entries(self.bytes).to_vec().into_boxed_slice())
    }

    /// Return whether no static bytes exist.
    pub fn is_empty(&self, sections: SectionImage<'_>) -> bool {
        sections.entries(self.bytes).is_empty()
    }

    /// Return the static byte count.
    pub fn byte_len(&self, sections: SectionImage<'_>) -> usize {
        sections.entries(self.bytes).len()
    }

    /// Return a native projection of this constant image.
    pub fn as_native_constants(&self, sections: SectionImage<'_>) -> NativeConstantSpace {
        let bytes = sections.entries(self.bytes);

        NativeConstantSpace {
            bytes: bytes.as_ptr(),
            byte_len: bytes.len(),
        }
    }
}

/// Runtime-owned mutable static memory.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct StaticSpace {
    /// The static bytes.
    bytes: Box<[u8]>,
}

impl StaticSpace {
    /// Create empty static memory.
    pub fn empty() -> Self {
        Self {
            bytes: Box::default(),
        }
    }

    /// Create static memory.
    fn new(bytes: Box<[u8]>) -> Self {
        Self { bytes }
    }

    /// Borrow one global byte range.
    pub fn bytes(&self, global: &Global) -> Option<&[u8]> {
        let end = global.offset() + global.byte_len();

        self.bytes.get(global.offset()..end)
    }

    /// Borrow one global byte range mutably.
    pub fn bytes_mut(&mut self, global: &Global) -> Option<&mut [u8]> {
        let end = global.offset() + global.byte_len();

        self.bytes.get_mut(global.offset()..end)
    }

    /// Return a native address for one static byte range.
    pub fn native_address(
        &self,
        global: &Global,
        address: GlobalAddress,
        byte_len: usize,
    ) -> Option<usize> {
        let start = address.byte_offset();
        let end = start.checked_add(byte_len)?;
        if end > global.byte_len() {
            return None;
        }

        Some(self.bytes.as_ptr() as usize + global.offset() + start)
    }

    /// Return a mutable native address for one static byte range.
    pub fn native_address_mut(
        &mut self,
        global: &Global,
        address: GlobalAddress,
        byte_len: usize,
    ) -> Option<usize> {
        let start = address.byte_offset();
        let end = start.checked_add(byte_len)?;
        if end > global.byte_len() {
            return None;
        }

        Some(self.bytes.as_mut_ptr() as usize + global.offset() + start)
    }

    /// Return whether static memory owns one byte range.
    pub fn owns_address_range(
        &self,
        global: &Global,
        address: GlobalAddress,
        byte_len: usize,
    ) -> bool {
        self.native_address(global, address, byte_len).is_some()
    }

    /// Return whether no static bytes exist.
    pub fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }

    /// Return the static byte count.
    pub fn byte_len(&self) -> usize {
        self.bytes.len()
    }

    /// Return a native projection of this constant space.
    pub fn as_native_constants(&self) -> NativeConstantSpace {
        NativeConstantSpace {
            bytes: self.bytes.as_ptr(),
            byte_len: self.bytes.len(),
        }
    }

    /// Return a native projection of this static space.
    pub fn as_native_statics(&mut self) -> NativeStaticSpace {
        NativeStaticSpace {
            bytes: self.bytes.as_mut_ptr(),
            byte_len: self.bytes.len(),
        }
    }
}
