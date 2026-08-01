use std::{io, mem, ptr, slice};

use super::BuildId;

const LC_UUID: u32 = 0x1b;
const MACH_HEADER_32_BYTES: usize = 28;
const MACH_HEADER_64_BYTES: usize = 32;
const MH_MAGIC: u32 = 0xfeed_face;
const MH_MAGIC_64: u32 = 0xfeed_facf;
const LOAD_COMMAND_BYTES: usize = 8;
const UUID_BYTES: usize = 16;
const UUID_COMMAND_BYTES: usize = LOAD_COMMAND_BYTES + UUID_BYTES;

impl BuildId {
    /// Read the Mach O UUID of the image containing this implementation.
    pub(super) fn read() -> io::Result<Self> {
        // resolve this implementation to its loaded image
        let mut image = unsafe { mem::zeroed::<libc::Dl_info>() };
        let address = Self::current as *const () as *const libc::c_void;
        let is_resolved = unsafe { libc::dladdr(address, &mut image) };
        if is_resolved == 0 || image.dli_fbase.is_null() {
            return Err(io::Error::other("dyld could not resolve the Destack image"));
        }
        let header = image.dli_fbase.cast::<u8>();
        let magic = unsafe { ptr::read_unaligned(header.cast::<u32>()) };
        let header_bytes = match magic {
            MH_MAGIC => MACH_HEADER_32_BYTES,
            MH_MAGIC_64 => MACH_HEADER_64_BYTES,
            _ => {
                return Err(io::Error::other("Destack image is not native Mach O"));
            }
        };

        // bound the load command view by the header's declared byte count
        let command_count = unsafe { ptr::read_unaligned(header.add(16).cast::<u32>()) } as usize;
        let command_bytes = unsafe { ptr::read_unaligned(header.add(20).cast::<u32>()) } as usize;
        let commands = unsafe { slice::from_raw_parts(header.add(header_bytes), command_bytes) };
        let mut offset = 0;

        // find the linker supplied UUID command
        for _ in 0..command_count {
            if commands.len().saturating_sub(offset) < LOAD_COMMAND_BYTES {
                return Err(io::Error::other(
                    "Mach O load commands exceed their declared size",
                ));
            }
            let command =
                unsafe { ptr::read_unaligned(commands.as_ptr().add(offset).cast::<u32>()) };
            let command_bytes = unsafe {
                ptr::read_unaligned(commands.as_ptr().add(offset + 4).cast::<u32>()) as usize
            };
            if command_bytes < LOAD_COMMAND_BYTES || command_bytes > commands.len() - offset {
                return Err(io::Error::other("Mach O load command has an invalid size"));
            }

            // consume the exact UUID payload
            if command == LC_UUID {
                if command_bytes < UUID_COMMAND_BYTES {
                    return Err(io::Error::other("Mach O UUID command has an invalid size"));
                }
                let uuid = &commands[offset + LOAD_COMMAND_BYTES..offset + UUID_COMMAND_BYTES];

                return Ok(Self::from_bytes(uuid));
            }

            offset += command_bytes;
        }

        Err(io::Error::other("Destack image has no Mach O UUID"))
    }
}
