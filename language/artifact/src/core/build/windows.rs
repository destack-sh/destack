use std::{io, mem, ptr, slice};

use windows_sys::Win32::Foundation::HMODULE;
use windows_sys::Win32::System::LibraryLoader::{
    GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS, GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT,
    GetModuleHandleExW,
};
use windows_sys::Win32::System::ProcessStatus::{K32GetModuleInformation, MODULEINFO};
use windows_sys::Win32::System::Threading::GetCurrentProcess;

use super::BuildId;

const CODEVIEW_DEBUG_TYPE: u32 = 2;
const DEBUG_DIRECTORY_BYTES: usize = 28;
const DEBUG_DIRECTORY_INDEX: usize = 6;
const DOS_HEADER_POINTER_OFFSET: usize = 0x3c;
const PE_32_MAGIC: u16 = 0x10b;
const PE_32_DATA_DIRECTORIES_OFFSET: usize = 96;
const PE_32_DIRECTORY_COUNT_OFFSET: usize = 92;
const PE_64_MAGIC: u16 = 0x20b;
const PE_64_DATA_DIRECTORIES_OFFSET: usize = 112;
const PE_64_DIRECTORY_COUNT_OFFSET: usize = 108;
const PE_HEADER_BYTES: usize = 24;
const REPRO_DEBUG_TYPE: u32 = 16;
const RSDS_SIGNATURE: &[u8; 4] = b"RSDS";

/// One loaded portable executable image.
struct PeImage<'a> {
    /// The complete mapped image bytes.
    bytes: &'a [u8],
}

impl PeImage<'_> {
    /// Read the linker identity from the image debug directory.
    fn build_id(&self) -> io::Result<BuildId> {
        // locate and verify the portable executable header
        if self.range(0, 2)? != b"MZ" {
            return Err(io::Error::other("Destack image has no DOS header"));
        }
        let pe_offset = self.read_u32(DOS_HEADER_POINTER_OFFSET)? as usize;
        if self.range(pe_offset, 4)? != b"PE\0\0" {
            return Err(io::Error::other("Destack image has no PE header"));
        }
        let optional_bytes = self.read_u16(pe_offset + 20)? as usize;
        let optional_offset = pe_offset + PE_HEADER_BYTES;

        // locate the debug data directory for the exact optional header format
        let magic = self.read_u16(optional_offset)?;
        let (directory_count_offset, directories_offset) = match magic {
            PE_32_MAGIC => (PE_32_DIRECTORY_COUNT_OFFSET, PE_32_DATA_DIRECTORIES_OFFSET),
            PE_64_MAGIC => (PE_64_DIRECTORY_COUNT_OFFSET, PE_64_DATA_DIRECTORIES_OFFSET),
            _ => return Err(io::Error::other("Destack image has an unknown PE format")),
        };
        let required_optional_bytes = directories_offset + (DEBUG_DIRECTORY_INDEX + 1) * 8;
        if optional_bytes < required_optional_bytes {
            return Err(io::Error::other("PE optional header is truncated"));
        }
        let directory_count = self.read_u32(optional_offset + directory_count_offset)? as usize;
        if directory_count <= DEBUG_DIRECTORY_INDEX {
            return Err(io::Error::other("Destack image has no PE debug directory"));
        }
        let debug_entry = optional_offset + directories_offset + DEBUG_DIRECTORY_INDEX * 8;
        let debug_offset = self.read_u32(debug_entry)? as usize;
        let debug_bytes = self.read_u32(debug_entry + 4)? as usize;
        if debug_bytes % DEBUG_DIRECTORY_BYTES != 0 {
            return Err(io::Error::other("PE debug directory has an invalid size"));
        }
        self.range(debug_offset, debug_bytes)?;

        // read CodeView or reproducible build identities from each debug entry
        for relative_offset in (0..debug_bytes).step_by(DEBUG_DIRECTORY_BYTES) {
            let entry_offset = debug_offset + relative_offset;
            let debug_type = self.read_u32(entry_offset + 12)?;
            let identity_bytes = self.read_u32(entry_offset + 16)? as usize;
            let identity_offset = self.read_u32(entry_offset + 20)? as usize;
            if debug_type == CODEVIEW_DEBUG_TYPE {
                let identity = self.range(identity_offset, identity_bytes)?;
                if identity.len() < 24 || &identity[..4] != RSDS_SIGNATURE {
                    return Err(io::Error::other("PE CodeView identity is not RSDS"));
                }

                return Ok(BuildId::from_bytes(&identity[4..24]));
            } else if debug_type == REPRO_DEBUG_TYPE && identity_bytes != 0 {
                let identity = self.range(identity_offset, identity_bytes)?;

                return Ok(BuildId::from_bytes(identity));
            }
        }

        Err(io::Error::other("Destack image has no PE build identity"))
    }

    /// Return one bounded image byte range.
    fn range(&self, offset: usize, bytes: usize) -> io::Result<&[u8]> {
        let end = offset
            .checked_add(bytes)
            .filter(|end| *end <= self.bytes.len())
            .ok_or_else(|| io::Error::other("PE field exceeds the mapped executable image"))?;

        Ok(&self.bytes[offset..end])
    }

    /// Read one little endian PE word.
    fn read_u16(&self, offset: usize) -> io::Result<u16> {
        let bytes = self.range(offset, mem::size_of::<u16>())?;
        let bytes = <[u8; 2]>::try_from(bytes)
            .map_err(|_| io::Error::other("PE word has an invalid size"))?;

        Ok(u16::from_le_bytes(bytes))
    }

    /// Read one little endian PE double word.
    fn read_u32(&self, offset: usize) -> io::Result<u32> {
        let bytes = self.range(offset, mem::size_of::<u32>())?;
        let bytes = <[u8; 4]>::try_from(bytes)
            .map_err(|_| io::Error::other("PE double word has an invalid size"))?;

        Ok(u32::from_le_bytes(bytes))
    }
}

impl BuildId {
    /// Read the build id of the image containing this implementation.
    pub(super) fn read() -> io::Result<Self> {
        // resolve this implementation to its loaded module
        let flags =
            GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS | GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT;
        let address = Self::current as *const () as *const u16;
        let mut module: HMODULE = 0;
        let is_resolved = unsafe { GetModuleHandleExW(flags, address, &mut module) };
        if is_resolved == 0 {
            return Err(io::Error::last_os_error());
        }

        // read the module's mapped image range
        let mut module_info = MODULEINFO {
            lpBaseOfDll: ptr::null_mut(),
            SizeOfImage: 0,
            EntryPoint: ptr::null_mut(),
        };
        let is_loaded = unsafe {
            K32GetModuleInformation(
                GetCurrentProcess(),
                module,
                &mut module_info,
                mem::size_of::<MODULEINFO>() as u32,
            )
        };
        if is_loaded == 0 {
            return Err(io::Error::last_os_error());
        }

        // read the identity from the loader validated mapped image
        let bytes = unsafe {
            slice::from_raw_parts(
                module_info.lpBaseOfDll.cast::<u8>(),
                module_info.SizeOfImage as usize,
            )
        };
        let image = PeImage { bytes };

        image.build_id()
    }
}
