use std::ffi::{c_int, c_void};
use std::{io, ptr, slice};

use super::BuildId;

const ELF_NOTE_HEADER_BYTES: usize = 12;
const ELF_NOTE_ALIGNMENT: usize = 4;
const GNU_BUILD_ID: u32 = 3;

/// ELF build id search over the current process images.
struct ElfBuildIdSearch {
    /// One address within the selected image.
    address: usize,
    /// The discovered build id.
    build_id: Option<BuildId>,
    /// The malformed selected image error.
    error: Option<&'static str>,
}

impl ElfBuildIdSearch {
    /// Visit one loaded ELF image.
    unsafe extern "C" fn visit(
        image: *mut libc::dl_phdr_info,
        _size: usize,
        context: *mut c_void,
    ) -> c_int {
        let image = unsafe { &*image };
        let context = unsafe { &mut *context.cast::<Self>() };

        // select the image containing this artifact implementation
        let headers = unsafe { slice::from_raw_parts(image.dlpi_phdr, image.dlpi_phnum as usize) };
        let is_selected = headers.iter().any(|header| {
            if header.p_type != libc::PT_LOAD {
                return false;
            }
            let start = (image.dlpi_addr as usize).wrapping_add(header.p_vaddr as usize);
            let end = start.wrapping_add(header.p_memsz as usize);

            context.address >= start && context.address < end
        });
        if !is_selected {
            return 0;
        }

        // inspect each note segment in the selected image
        for header in headers {
            if header.p_type != libc::PT_NOTE {
                continue;
            }
            let address =
                (image.dlpi_addr as usize).wrapping_add(header.p_vaddr as usize) as *const u8;
            let notes = unsafe { slice::from_raw_parts(address, header.p_memsz as usize) };

            match Self::read_notes(notes) {
                Ok(Some(build_id)) => {
                    context.build_id = Some(build_id);

                    return 1;
                }
                Ok(None) => {}
                Err(error) => {
                    context.error = Some(error);

                    return 1;
                }
            }
        }

        1
    }

    /// Read a GNU build id from one ELF note segment.
    fn read_notes(notes: &[u8]) -> Result<Option<BuildId>, &'static str> {
        let mut offset = 0;

        while notes.len().saturating_sub(offset) >= ELF_NOTE_HEADER_BYTES {
            let name_bytes = Self::read_u32(notes, offset) as usize;
            let value_bytes = Self::read_u32(notes, offset + 4) as usize;
            let note_type = Self::read_u32(notes, offset + 8);
            let name_start = offset + ELF_NOTE_HEADER_BYTES;
            let name_end = name_start + name_bytes;
            let value_start = Self::align(name_end);
            let value_end = value_start + value_bytes;
            if value_end > notes.len() {
                return Err("ELF note exceeds its declared segment");
            }

            // consume the exact GNU build id note
            let name = &notes[name_start..name_end];
            if note_type == GNU_BUILD_ID && name == b"GNU\0" {
                return Ok(Some(BuildId::from_bytes(&notes[value_start..value_end])));
            }

            offset = Self::align(value_end);
        }

        Ok(None)
    }

    /// Align one ELF note offset.
    fn align(offset: usize) -> usize {
        (offset + ELF_NOTE_ALIGNMENT - 1) & !(ELF_NOTE_ALIGNMENT - 1)
    }

    /// Read one native endian ELF word.
    fn read_u32(bytes: &[u8], offset: usize) -> u32 {
        unsafe { ptr::read_unaligned(bytes.as_ptr().add(offset).cast::<u32>()) }
    }
}

impl BuildId {
    /// Read the GNU build id of the image containing this implementation.
    pub(super) fn read() -> io::Result<Self> {
        let mut search = ElfBuildIdSearch {
            address: Self::current as *const () as usize,
            build_id: None,
            error: None,
        };

        // visit the images already mapped by the platform loader
        unsafe {
            libc::dl_iterate_phdr(
                Some(ElfBuildIdSearch::visit),
                (&mut search as *mut ElfBuildIdSearch).cast(),
            );
        }

        if let Some(error) = search.error {
            Err(io::Error::other(error))
        } else if let Some(build_id) = search.build_id {
            Ok(build_id)
        } else {
            Err(io::Error::other("Destack image has no GNU build id"))
        }
    }
}
