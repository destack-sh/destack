use std::collections::BTreeMap;
use std::ffi::c_void;
use std::mem::transmute;
use std::ops::Range;
use std::sync::OnceLock;

use parking_lot::RwLock;

use super::{Error, LoadOperation};

const ADD_SYMBOL: &[u8] = b"__unw_add_find_dynamic_unwind_sections\0";

static REGISTRY: OnceLock<Registry> = OnceLock::new();
static REGISTRATION: OnceLock<Result<(), Error>> = OnceLock::new();

/// One Darwin dynamic unwind registration.
#[derive(Debug)]
pub(super) struct Registration {
    /// Registered executable range start.
    start: usize,
}

/// Process-wide Darwin dynamic unwind section registry.
#[derive(Debug, Default)]
struct Registry {
    /// Unwind sections keyed by executable range start.
    entries: RwLock<BTreeMap<usize, Entry>>,
}

/// Unwind sections for one executable range.
#[derive(Debug, Clone, Copy)]
struct Entry {
    /// Executable range end.
    end: usize,
    /// Darwin libunwind section addresses.
    sections: Sections,
}

/// Darwin libunwind dynamic sections.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct Sections {
    /// Dynamic image base address.
    dso_base: usize,
    /// DWARF `.eh_frame` address.
    dwarf_section: usize,
    /// DWARF `.eh_frame` byte length.
    dwarf_section_length: usize,
    /// Compact unwind section address.
    compact_unwind_section: usize,
    /// Compact unwind section byte length.
    compact_unwind_section_length: usize,
}

type AddCallback = unsafe extern "C" fn(*const c_void) -> libc::c_int;

impl Registration {
    /// Register mapped Darwin unwind sections for one executable range.
    pub(super) fn register(
        code: Range<usize>,
        dso_base: usize,
        dwarf: Range<usize>,
    ) -> Result<Self, Error> {
        let registry = REGISTRY.get_or_init(Registry::default);
        REGISTRATION
            .get_or_init(register_callback)
            .as_ref()
            .map_err(|error| error.clone())?;

        // reject overlapping ranges because lookup selects one predecessor
        let mut entries = registry.entries.write();
        let previous = entries.range(..=code.start).next_back();
        let next = entries.range(code.start..).next();
        let overlaps_previous = previous.is_some_and(|(_, entry)| code.start < entry.end);
        let overlaps_next = next.is_some_and(|(start, _)| *start < code.end);
        if overlaps_previous || overlaps_next {
            return Err(load_error(
                "executable unwind range overlaps a live mapping",
            ));
        }

        // publish the complete immutable section record
        let sections = Sections {
            dso_base,
            dwarf_section: dwarf.start,
            dwarf_section_length: dwarf.len(),
            compact_unwind_section: 0,
            compact_unwind_section_length: 0,
        };
        entries.insert(
            code.start,
            Entry {
                end: code.end,
                sections,
            },
        );

        Ok(Self { start: code.start })
    }
}

impl Drop for Registration {
    /// Remove this executable range before its mapping is released.
    fn drop(&mut self) {
        let Some(registry) = REGISTRY.get() else {
            return;
        };

        registry.entries.write().remove(&self.start);
    }
}

/// Return Darwin unwind sections for one instruction address.
extern "C" fn find(address: usize, output: *mut Sections) -> libc::c_int {
    let Some(registry) = REGISTRY.get() else {
        return 0;
    };
    if output.is_null() {
        return 0;
    }

    // select the containing executable range
    let entries = registry.entries.read();
    let Some((_, entry)) = entries.range(..=address).next_back() else {
        return 0;
    };
    if address >= entry.end {
        return 0;
    }

    // SAFETY: libunwind supplied one writable Sections output pointer.
    unsafe { output.write(entry.sections) };

    1
}

/// Register the process-wide Darwin unwind lookup callback.
fn register_callback() -> Result<(), Error> {
    // resolve the API dynamically because older Darwin runtimes omit it
    let symbol = unsafe { libc::dlsym(libc::RTLD_DEFAULT, ADD_SYMBOL.as_ptr().cast()) };
    if symbol.is_null() {
        return Err(load_error(
            "dynamic unwind section registration is unavailable",
        ));
    }
    // SAFETY: dlsym returned the named Darwin callback registration operation.
    let add: AddCallback = unsafe { transmute(symbol) };

    // publish the callback before any generated code can execute
    let result = unsafe { add(find as *const c_void) };
    if result != 0 {
        return Err(load_error(
            "dynamic unwind section callback registration failed",
        ));
    }

    Ok(())
}

/// Build one Darwin unwind registration error.
fn load_error(message: &str) -> Error {
    Error::NativeLoad {
        operation: LoadOperation::Register,
        message: message.to_owned(),
    }
}
