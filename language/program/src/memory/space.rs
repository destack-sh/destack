use std::mem::size_of;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tspp_core::{SectionBuilder, SectionEntry, SectionImage, SectionSlice};
use tspp_memory::{MemoryError, MemoryMap, MemoryRange, MemoryResult};
use tspp_native::abi;
use tspp_serde::Reflect;

use crate::{Global, GlobalAddress, GlobalLocation};

/// One static address word rebased when its image is materialized.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry)]
pub struct StaticRelocation {
    /// The byte offset of the address word inside its source image.
    pub byte_offset: u64,
    /// The static location containing the referenced global.
    pub location: GlobalLocation,
    /// Reserved relocation word.
    reserved: u32,
}

/// Section-backed static memory image carried by a program.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry)]
pub struct StaticImage {
    /// Static bytes.
    bytes: SectionSlice<u8>,
    /// Static address words rebased when the image is materialized.
    relocations: SectionSlice<StaticRelocation>,
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
    /// Static address words rebased when the image is materialized.
    relocations: Vec<StaticRelocation>,
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
    /// Create initialized static bytes with address words to rebase.
    pub fn new(bytes: Vec<u8>, relocations: Vec<StaticRelocation>, alignment: usize) -> Self {
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

impl StaticRelocation {
    /// Create one static address relocation.
    pub(crate) const fn new(byte_offset: usize, location: GlobalLocation) -> Self {
        Self {
            byte_offset: byte_offset as u64,
            location,
            reserved: 0,
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

    /// Materialize this image into runtime static memory.
    pub(crate) fn materialize(
        &self,
        sections: SectionImage<'_>,
        memory: Arc<MemoryMap>,
    ) -> MemoryResult<StaticSpace> {
        StaticSpace::new(memory, sections.entries(self.bytes), self.alignment())
    }

    /// Materialize this image into dedicated mapping frames.
    pub(crate) fn materialize_constant(
        &self,
        sections: SectionImage<'_>,
        memory: Arc<MemoryMap>,
    ) -> MemoryResult<StaticSpace> {
        StaticSpace::constant(memory, sections.entries(self.bytes), self.alignment())
    }

    /// Borrow the static address relocations.
    pub(crate) fn relocations<'a>(&self, sections: SectionImage<'a>) -> &'a [StaticRelocation] {
        sections.entries(self.relocations)
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
}

/// Durable static memory image.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct StaticSpaceImage {
    /// The static byte offset inside world memory.
    pub memory_offset: usize,
    /// The static byte count inside world memory.
    pub byte_len: usize,
}

/// Runtime-owned static memory.
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

    /// Create constant memory in dedicated mapping frames.
    fn constant(memory: Arc<MemoryMap>, bytes: &[u8], alignment: usize) -> MemoryResult<Self> {
        let frame_byte_len = memory.frame_size_bytes();
        let byte_len = bytes.len().next_multiple_of(frame_byte_len);
        let alignment = alignment.max(frame_byte_len);
        let range = memory.allocate(byte_len, alignment)?;
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

    /// Capture static memory.
    pub fn image(&self) -> StaticSpaceImage {
        StaticSpaceImage {
            memory_offset: self.range.offset,
            byte_len: self.range.byte_len,
        }
    }

    /// Rebase static address words against their concrete target spaces.
    pub(crate) fn relocate(
        &self,
        relocations: &[StaticRelocation],
        base: impl Fn(GlobalLocation) -> MemoryResult<usize>,
    ) -> MemoryResult<()> {
        for relocation in relocations {
            // locate the address word inside this static space
            let byte_offset = usize::try_from(relocation.byte_offset).map_err(|_| {
                MemoryError::internal("static relocation exceeds host address width")
            })?;
            let byte_end = byte_offset
                .checked_add(GlobalAddress::BYTE_LEN)
                .ok_or_else(|| MemoryError::internal("static relocation offset overflow"))?;
            if byte_end > self.range.byte_len {
                return Err(MemoryError::internal(
                    "static relocation lies outside its source space",
                ));
            }
            let word_offset = self.range.offset + byte_offset;
            let bytes = self
                .memory
                .read_bytes(word_offset, GlobalAddress::BYTE_LEN)?;
            let mut word = [0u8; GlobalAddress::BYTE_LEN];
            word.copy_from_slice(&bytes);

            // rebase the target into world memory
            let target = usize::try_from(u64::from_le_bytes(word))
                .map_err(|_| MemoryError::internal("static target exceeds host address width"))?;
            let target = target
                .checked_add(base(relocation.location)?)
                .ok_or_else(|| MemoryError::internal("static target address overflow"))?;
            self.memory
                .write_bytes(word_offset, &(target as u64).to_le_bytes())?;
        }

        Ok(())
    }

    /// Make this static range immutable.
    pub(crate) fn freeze(&self) -> MemoryResult<()> {
        if self.range.byte_len == 0 {
            return Ok(());
        }

        self.memory.freeze(self.range)
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

const _: () = assert!(size_of::<StaticRelocation>() == 16);

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
