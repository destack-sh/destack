use std::sync::Arc;

use destack_core::{SectionImage, SectionPacker, SectionSlice};
use destack_memory::{MemoryError, MemoryMap, MemoryRange, MemoryResult};
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
    pub fn materialize(
        &self,
        sections: SectionImage<'_>,
        memory: Arc<MemoryMap>,
    ) -> MemoryResult<StaticSpace> {
        StaticSpace::new(memory, sections.entries(self.bytes))
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

/// Durable mutable static memory image.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct StaticSpaceImage {
    /// The static byte offset inside world memory.
    pub memory_offset: u64,
    /// The captured static bytes.
    pub bytes: Vec<u8>,
}

/// Runtime-owned mutable static memory.
#[derive(Debug)]
pub struct StaticSpace {
    /// The world memory map.
    memory: Arc<MemoryMap>,
    /// The static byte range inside world memory.
    range: MemoryRange,
}

impl StaticSpace {
    /// Create static memory from initial bytes.
    pub fn new(memory: Arc<MemoryMap>, bytes: &[u8]) -> MemoryResult<Self> {
        let range = memory.allocate(bytes.len(), 1)?;
        let space = Self { memory, range };
        space.memory.write_bytes(space.range.offset, bytes)?;

        Ok(space)
    }

    /// Restore static memory from one image.
    pub fn from_image(memory: Arc<MemoryMap>, image: &StaticSpaceImage) -> MemoryResult<Self> {
        let offset =
            usize::try_from(image.memory_offset).map_err(|_| MemoryError::OffsetOverflow {
                offset: image.memory_offset,
            })?;
        let range = MemoryRange {
            offset,
            byte_len: image.bytes.len(),
        };
        memory.claim(range)?;
        let space = Self { memory, range };
        space.memory.write_bytes(space.range.offset, &image.bytes)?;

        Ok(space)
    }

    /// Fork static memory over an already forked world map.
    pub fn fork(&self, memory: Arc<MemoryMap>) -> Self {
        Self {
            memory,
            range: self.range,
        }
    }

    /// Capture mutable static memory.
    pub fn image(&self) -> MemoryResult<StaticSpaceImage> {
        let bytes = self
            .memory
            .read_bytes(self.range.offset, self.range.byte_len)?;

        Ok(StaticSpaceImage {
            memory_offset: self.range.offset as u64,
            bytes,
        })
    }

    /// Borrow one mapped global byte range mutably.
    ///
    /// # Safety
    ///
    /// No other access to this memory map may overlap the returned byte range.
    pub(crate) unsafe fn bytes_mut(&mut self, global: &Global) -> Option<&mut [u8]> {
        let end = global.offset() + global.byte_len();

        if end > self.range.byte_len {
            return None;
        }

        // SAFETY: &mut self grants exclusive access to this static range
        let bytes = unsafe {
            self.memory
                .mapped_bytes_mut(self.range.offset, self.range.byte_len)
        };

        bytes.get_mut(global.offset()..end)
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

        Some(self.memory.base_address() + self.range.offset + global.offset() + start)
    }

    /// Return a mutable native address for one static byte range.
    pub fn native_address_mut(
        &mut self,
        global: &Global,
        address: GlobalAddress,
        byte_len: usize,
    ) -> MemoryResult<Option<usize>> {
        let start = address.byte_offset();
        let Some(end) = start.checked_add(byte_len) else {
            return Ok(None);
        };
        if end > global.byte_len() {
            return Ok(None);
        }

        self.memory
            .make_writable(self.range.offset + global.offset() + start, byte_len)?;

        Ok(Some(
            self.memory.base_address() + self.range.offset + global.offset() + start,
        ))
    }

    /// Return the static byte count.
    pub fn byte_len(&self) -> usize {
        self.range.byte_len
    }

    /// Return a native projection of this static space.
    pub fn as_native_statics(&mut self) -> NativeStaticSpace {
        NativeStaticSpace {
            bytes: (self.memory.base_address() + self.range.offset) as *mut u8,
            byte_len: self.range.byte_len,
        }
    }
}

impl Drop for StaticSpace {
    fn drop(&mut self) {
        // abort because an owned range becoming invalid means memory state is corrupt
        if self.memory.release(self.range).is_err() {
            std::process::abort();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Restore mutable static bytes at their captured world offset.
    #[test]
    fn test_restore_static_space_image() {
        let source_memory = Arc::new(
            MemoryMap::reserve(64 * 1024, 8 * 1024).expect("source memory should reserve"),
        );
        let source = StaticSpace::new(source_memory, &[1, 2, 3, 4])
            .expect("source statics should materialize");
        let image = source.image().expect("static image should capture");
        let target_memory = Arc::new(
            MemoryMap::reserve(64 * 1024, 8 * 1024).expect("target memory should reserve"),
        );

        let restored =
            StaticSpace::from_image(target_memory, &image).expect("static image should restore");
        let restored_image = restored.image().expect("restored image should capture");

        assert_eq!(restored_image, image);
    }

    /// Isolate forked mutable static bytes on first write.
    #[test]
    fn test_fork_static_space_isolates_writes() {
        let memory = Arc::new(
            MemoryMap::reserve(64 * 1024, 8 * 1024).expect("parent memory should reserve"),
        );
        let parent = StaticSpace::new(memory.clone(), &[1, 2, 3, 4])
            .expect("parent statics should materialize");
        let fork_memory = Arc::new(memory.fork_lazy().expect("world memory should fork"));
        let fork = parent.fork(fork_memory);

        fork.memory
            .write_bytes(fork.range.offset, &[9, 8, 7, 6])
            .expect("fork statics should write");

        assert_eq!(
            parent.image().expect("parent image should capture").bytes,
            [1, 2, 3, 4]
        );
        assert_eq!(
            fork.image().expect("fork image should capture").bytes,
            [9, 8, 7, 6]
        );
    }

    /// Reuse static memory ranges after their owner is dropped.
    #[test]
    fn test_drop_static_space_releases_range() {
        let memory =
            Arc::new(MemoryMap::reserve(64 * 1024, 8 * 1024).expect("test memory should reserve"));
        let first_offset = {
            let space = StaticSpace::new(memory.clone(), &[1, 2, 3, 4])
                .expect("first statics should materialize");

            space.range.offset
        };
        let second =
            StaticSpace::new(memory, &[5, 6, 7, 8]).expect("second statics should materialize");

        assert_eq!(second.range.offset, first_offset);
    }
}
