use std::fmt;
use std::ptr::{copy_nonoverlapping, null_mut};

use tspp_native as native;
use tspp_native::abi;
use tspp_program::Program;

use super::{Code, Error, LoadOperation, Loader, Mapping};

#[cfg(target_vendor = "apple")]
use super::darwin;

/// Native code loader for the current process.
#[derive(Debug, Clone, Copy, Default)]
pub struct Platform;

#[cfg(unix)]
unsafe extern "C" {
    #[cfg(not(target_vendor = "apple"))]
    fn __register_frame(frame: *const u8);
    #[cfg(not(target_vendor = "apple"))]
    fn __deregister_frame(frame: *const u8);
}

#[cfg(all(target_arch = "aarch64", target_vendor = "apple"))]
unsafe extern "C" {
    fn sys_icache_invalidate(start: *const u8, byte_len: usize);
}

#[cfg(unix)]
unsafe extern "C-unwind" {
    fn _Unwind_Resume(unwind: *mut abi::Unwind) -> !;
}

/// One owned Unix executable mapping.
#[cfg(unix)]
struct Allocation {
    /// Mapped page address.
    address: *mut u8,
    /// Mapped page byte length.
    byte_len: usize,
    /// Darwin unwind registration owned by this mapping.
    #[cfg(target_vendor = "apple")]
    unwind: Option<darwin::Registration>,
    /// System V unwind section owned by this mapping.
    #[cfg(not(target_vendor = "apple"))]
    unwind: Option<*const u8>,
}

// SAFETY: executable mappings are immutable after construction.
#[cfg(unix)]
unsafe impl Send for Allocation {}

// SAFETY: executable mappings are immutable after construction.
#[cfg(unix)]
unsafe impl Sync for Allocation {}

#[cfg(unix)]
impl fmt::Debug for Allocation {
    /// Format one executable mapping without exposing process addresses.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Allocation")
            .field("byte_len", &self.byte_len)
            .field("has_unwind", &self.unwind.is_some())
            .finish()
    }
}

#[cfg(unix)]
impl Drop for Allocation {
    /// Unregister unwind tables before releasing executable pages.
    fn drop(&mut self) {
        // unregister references into this mapping before unmapping it
        #[cfg(target_vendor = "apple")]
        drop(self.unwind.take());

        #[cfg(not(target_vendor = "apple"))]
        if let Some(frame) = self.unwind.take() {
            // SAFETY: this exact frame section was registered by Platform::load_code.
            unsafe { __deregister_frame(frame) };
        }

        // SAFETY: address and byte length are the exact successful mmap result
        unsafe { libc::munmap(self.address.cast(), self.byte_len) };
    }
}

#[cfg(unix)]
impl Loader for Platform {
    /// Map and prepare one durable native code image.
    fn load_code(&self, program: &Program, code: &native::Code) -> Result<Code, Error> {
        let sections = program.sections();
        let bytes = code.bytes(sections);
        let page_size = Self::page_size()?;
        let byte_len = bytes.len().next_multiple_of(page_size);

        // reserve writable pages for relocation and initialization
        let address = unsafe {
            libc::mmap(
                null_mut(),
                byte_len,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_PRIVATE | libc::MAP_ANON,
                -1,
                0,
            )
        };
        if address == libc::MAP_FAILED {
            return Err(Self::load_error(LoadOperation::Map));
        }
        let address = address.cast::<u8>();
        let mut allocation = Allocation {
            address,
            byte_len,
            unwind: None,
        };

        // copy durable bytes and resolve every process-local import
        unsafe { copy_nonoverlapping(bytes.as_ptr(), address, bytes.len()) };
        for import in code.imports(sections) {
            Self::patch_import(address, bytes.len(), *import)?;
        }
        Self::flush(address, bytes.len());

        // publish immutable executable pages
        let result =
            unsafe { libc::mprotect(address.cast(), byte_len, libc::PROT_READ | libc::PROT_EXEC) };
        if result != 0 {
            return Err(Self::load_error(LoadOperation::Protect));
        }

        // register linked DWARF sections after every process address is final
        if let Some(unwind) = code.unwind() {
            let frame = unwind
                .sections(sections)
                .iter()
                .find(|section| section.kind == native::UnwindSectionKind::DwarfFrame);
            if let Some(section) = frame {
                let frame = unsafe { address.add(section.byte_offset() as usize) };

                #[cfg(target_vendor = "apple")]
                {
                    let range = address as usize..address as usize + bytes.len();
                    let dwarf = frame as usize..frame as usize + section.byte_len() as usize;
                    allocation.unwind = Some(darwin::Registration::register(
                        range,
                        address as usize,
                        dwarf,
                    )?);
                }

                #[cfg(not(target_vendor = "apple"))]
                {
                    // SAFETY: the linked section ends with the canonical zero-length record
                    unsafe { __register_frame(frame) };
                    allocation.unwind = Some(frame);
                }
            }
        }

        let mapping = Mapping::new(address as usize, bytes.len(), allocation);

        Code::mapped(mapping, program, code)
    }
}

#[cfg(unix)]
impl Platform {
    /// Return the current process page size.
    fn page_size() -> Result<usize, Error> {
        let page_size = unsafe { libc::sysconf(libc::_SC_PAGESIZE) };
        if page_size <= 0 {
            return Err(Self::load_error(LoadOperation::Map));
        }

        Ok(page_size as usize)
    }

    /// Continue one active platform unwind.
    pub(super) unsafe fn resume(unwind: *mut abi::Unwind) -> ! {
        // SAFETY: generated cleanup code passes the active platform unwind object
        unsafe { _Unwind_Resume(unwind) }
    }

    /// Publish generated instructions to the current processor.
    fn flush(address: *mut u8, byte_len: usize) {
        #[cfg(all(target_arch = "aarch64", target_vendor = "apple"))]
        // SAFETY: the initialized mapping contains exactly byte_len generated bytes.
        unsafe {
            sys_icache_invalidate(address, byte_len);
        }

        #[cfg(not(all(target_arch = "aarch64", target_vendor = "apple")))]
        let _ = (address, byte_len);
    }

    /// Build one platform loading diagnostic.
    fn load_error(operation: LoadOperation) -> Error {
        Error::NativeLoad {
            operation,
            message: std::io::Error::last_os_error().to_string(),
        }
    }
}
