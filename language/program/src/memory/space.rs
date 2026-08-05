use std::sync::Arc;

use destack_core::{SectionBuilder, SectionEntry, SectionImage, SectionSlice};
use destack_memory::{MemoryMap, MemoryRange, MemoryResult};
use destack_native::abi;
use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use crate::{Global, GlobalAddress};

/// Section-backed static memory image carried by a program.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry)]
pub struct StaticImage {
    /// Static bytes.
    bytes: SectionSlice<u8>,
    /// Image-relative word offsets rebased to the placed range at materialization.
    relocations: SectionSlice<u64>,
    /// Required byte alignment.
    alignment: u32,
    /// Reserved image word.
    reserved: u32,
}

/// Static bytes before Program image packing.
#[derive(Debug)]
pub struct StaticBytes {
    /// Initialized bytes.
    bytes: Vec<u8>,
    /// Image-relative word offsets holding image-relative target offsets.
    relocations: Vec<u64>,
    /// Required byte alignment.
    alignment: usize,
}

impl Default for StaticBytes {
    /// Create empty static bytes with byte alignment.
    fn default() -> Self {
        Self {
            bytes: Vec::new(),
            relocations: Vec::new(),
            alignment: 1,
        }
    }
}

impl Default for StaticImage {
    /// Create one empty static image.
    fn default() -> Self {
        Self {
            bytes: SectionSlice::empty(),
            relocations: SectionSlice::empty(),
            alignment: 1,
            reserved: 0,
        }
    }
}

impl StaticBytes {
    /// Create initialized static bytes with one required alignment.
    pub fn new(bytes: Vec<u8>, alignment: usize) -> Self {
        Self::relocated(bytes, Vec::new(), alignment)
    }

    /// Create initialized static bytes with address words to rebase.
    pub fn relocated(bytes: Vec<u8>, relocations: Vec<u64>, alignment: usize) -> Self {
        assert!(
            alignment.is_power_of_two(),
            "static alignment must be a power of two"
        );
        assert!(
            u32::try_from(alignment).is_ok(),
            "static alignment must fit its Program representation"
        );

        Self {
            bytes,
            relocations,
            alignment,
        }
    }
}

impl StaticImage {
    /// Pack one static image.
    pub(crate) fn pack(sections: &mut SectionBuilder, bytes: StaticBytes) -> Self {
        let alignment = bytes.alignment;
        let relocations = sections.insert(bytes.relocations);
        let bytes = sections.insert_bytes(bytes.bytes, alignment);

        Self {
            bytes,
            relocations,
            alignment: alignment as u32,
            reserved: 0,
        }
    }

    /// Borrow one global byte range.
    pub fn bytes<'a>(&self, sections: SectionImage<'a>, global: &Global) -> Option<&'a [u8]> {
        let end = global.offset() + global.byte_len();

        sections.entries(self.bytes).get(global.offset()..end)
    }

    /// Return one stable reference to a global in this image.
    pub fn reference(&self, global: &Global) -> GlobalAddress {
        GlobalAddress::new(global.offset())
    }

    /// Return a native address for one static byte range.
    pub fn address(
        &self,
        sections: SectionImage<'_>,
        address: GlobalAddress,
        byte_len: usize,
    ) -> Option<usize> {
        let start = address.offset()?;
        let end = start.checked_add(byte_len)?;
        let bytes = sections.entries(self.bytes);
        if end > bytes.len() {
            return None;
        }

        Some(bytes.as_ptr() as usize + start)
    }

    /// Return whether static memory owns one byte range.
    pub fn owns_address_range(
        &self,
        sections: SectionImage<'_>,
        address: GlobalAddress,
        byte_len: usize,
    ) -> bool {
        self.address(sections, address, byte_len).is_some()
    }

    /// Materialize this image into runtime static memory with rebased addresses.
    pub fn materialize(
        &self,
        sections: SectionImage<'_>,
        memory: Arc<MemoryMap>,
    ) -> MemoryResult<StaticSpace> {
        let space = StaticSpace::new(memory, sections.entries(self.bytes), self.alignment())?;
        space.rebase(sections.entries(self.relocations))?;

        Ok(space)
    }

    /// Return whether no static bytes exist.
    pub fn is_empty(&self, sections: SectionImage<'_>) -> bool {
        sections.entries(self.bytes).is_empty()
    }

    /// Return the static byte count.
    pub fn byte_len(&self, sections: SectionImage<'_>) -> usize {
        sections.entries(self.bytes).len()
    }

    /// Return the required byte alignment.
    pub const fn alignment(&self) -> usize {
        self.alignment as usize
    }

    /// Return whether the stored bytes satisfy their declared alignment.
    pub(crate) fn is_aligned(&self, sections: SectionImage<'_>) -> bool {
        let alignment = self.alignment();
        if !alignment.is_power_of_two() {
            return false;
        }

        // accept empty images without inspecting their synthetic slice pointer
        let bytes = sections.entries(self.bytes);
        bytes.is_empty() || (bytes.as_ptr() as usize).is_multiple_of(alignment)
    }

    /// Return a native projection of this constant image.
    pub fn native(&self, sections: SectionImage<'_>) -> abi::ConstantSpace {
        let bytes = sections.entries(self.bytes);

        abi::ConstantSpace {
            bytes: bytes.as_ptr(),
            byte_len: bytes.len(),
        }
    }
}

/// Durable mutable static memory image.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct StaticSpaceImage {
    /// The static byte offset inside world memory.
    pub memory_offset: usize,
    /// The static byte count inside world memory.
    pub byte_len: usize,
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
    pub fn new(memory: Arc<MemoryMap>, bytes: &[u8], alignment: usize) -> MemoryResult<Self> {
        let range = memory.allocate(bytes.len(), alignment)?;
        let space = Self { memory, range };
        space.memory.write_bytes(space.range.offset, bytes)?;

        Ok(space)
    }

    /// Restore static memory from one image.
    pub fn from_image(memory: Arc<MemoryMap>, image: &StaticSpaceImage) -> Self {
        let range = MemoryRange {
            offset: image.memory_offset,
            byte_len: image.byte_len,
        };

        Self { memory, range }
    }

    /// Fork static memory over an already forked world map.
    pub fn fork(&self, memory: Arc<MemoryMap>) -> Self {
        Self {
            memory,
            range: self.range,
        }
    }

    /// Capture mutable static memory.
    pub fn image(&self) -> StaticSpaceImage {
        StaticSpaceImage {
            memory_offset: self.range.offset,
            byte_len: self.range.byte_len,
        }
    }

    /// Rebase relocated address words by this space's world offset.
    ///
    /// Each relocation names a space-relative word holding a space-relative
    /// target offset; after rebasing, the word holds a world offset.
    pub fn rebase(&self, relocations: &[u64]) -> MemoryResult<()> {
        for &relocation in relocations {
            // read the space-relative target offset
            let word_offset = self.range.offset + relocation as usize;
            let bytes = self.memory.read_bytes(word_offset, GlobalAddress::BYTE_LEN)?;
            let mut word = [0u8; GlobalAddress::BYTE_LEN];
            word.copy_from_slice(&bytes);

            // rebase the target into world memory
            let target = u64::from_le_bytes(word) + self.range.offset as u64;
            self.memory.write_bytes(word_offset, &target.to_le_bytes())?;
        }

        Ok(())
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

    /// Return one stable reference to a global in this space.
    pub fn reference(&self, global: &Global) -> GlobalAddress {
        GlobalAddress::new(self.range.offset + global.offset())
    }

    /// Return a native address for one static byte range.
    pub fn address(&self, address: GlobalAddress, byte_len: usize) -> Option<usize> {
        let start = address.offset()?;
        let end = start.checked_add(byte_len)?;
        if start < self.range.offset || end > self.range.end() {
            return None;
        }

        Some(self.memory.base_address() + start)
    }

    /// Return a mutable native address for one static byte range.
    pub fn address_mut(
        &mut self,
        address: GlobalAddress,
        byte_len: usize,
    ) -> MemoryResult<Option<usize>> {
        let Some(start) = address.offset() else {
            return Ok(None);
        };
        let Some(end) = start.checked_add(byte_len) else {
            return Ok(None);
        };
        if start < self.range.offset || end > self.range.end() {
            return Ok(None);
        }

        self.memory.make_writable(start, byte_len)?;

        Ok(Some(self.memory.base_address() + start))
    }

    /// Return the static byte count.
    pub fn byte_len(&self) -> usize {
        self.range.byte_len
    }

    /// Return the static range offset inside world memory.
    pub fn offset(&self) -> usize {
        self.range.offset
    }

    /// Return a native projection of this static space.
    pub fn native(&self) -> abi::StaticSpace {
        abi::StaticSpace {
            offset: self.range.offset,
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
        let source = StaticSpace::new(source_memory.clone(), &[1, 2, 3, 4], 1)
            .expect("source statics should materialize");
        let memory_image = source_memory
            .capture()
            .expect("memory image should capture");
        let image = source.image();
        let target_memory = Arc::new(memory_image.restore().expect("memory should restore"));

        let restored = StaticSpace::from_image(target_memory.clone(), &image);
        let restored_image = restored.image();

        assert_eq!(restored_image, image);
        assert_eq!(
            target_memory
                .read_bytes(image.memory_offset, image.byte_len)
                .expect("restored statics should read"),
            [1, 2, 3, 4]
        );
    }

    /// Isolate forked mutable static bytes on first write.
    #[test]
    fn test_fork_static_space_isolates_writes() {
        let memory = Arc::new(
            MemoryMap::reserve(64 * 1024, 8 * 1024).expect("parent memory should reserve"),
        );
        let parent = StaticSpace::new(memory.clone(), &[1, 2, 3, 4], 1)
            .expect("parent statics should materialize");
        let fork_memory = Arc::new(memory.fork_lazy().expect("world memory should fork"));
        let fork = parent.fork(fork_memory);

        fork.memory
            .write_bytes(fork.range.offset, &[9, 8, 7, 6])
            .expect("fork statics should write");

        assert_eq!(
            memory
                .read_bytes(parent.range.offset, parent.range.byte_len)
                .expect("parent statics should read"),
            [1, 2, 3, 4]
        );
        assert_eq!(
            fork.memory
                .read_bytes(fork.range.offset, fork.range.byte_len)
                .expect("fork statics should read"),
            [9, 8, 7, 6]
        );
    }

    /// Reuse static memory ranges after their owner is dropped.
    #[test]
    fn test_drop_static_space_releases_range() {
        let memory =
            Arc::new(MemoryMap::reserve(64 * 1024, 8 * 1024).expect("test memory should reserve"));
        let first_offset = {
            let space = StaticSpace::new(memory.clone(), &[1, 2, 3, 4], 1)
                .expect("first statics should materialize");

            space.range.offset
        };
        let second =
            StaticSpace::new(memory, &[5, 6, 7, 8], 1).expect("second statics should materialize");

        assert_eq!(second.range.offset, first_offset);
    }
}
