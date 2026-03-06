#[cfg(target_os = "linux")]
use crate::platform::core::load_dynamic_symbol_named;
use crate::platform::core::{close_dynamic_library, load_dynamic_symbol, open_dynamic_library};

use std::ffi::{CStr, c_void};
use std::fmt::{Debug, Formatter};

/// Build one typed API table from one byte-string symbol map.
#[cfg(any(target_os = "linux", target_os = "android"))]
macro_rules! load_dll_api_bytes {
    (
        $library:expr,
        $candidate:expr,
        $api:ident {
            $($field:ident => $name:expr),+ $(,)?
        }
    ) => {{
        Ok($api {
            $(
                $field: $library.load_required_symbol($candidate, $name)?,
            )+
        })
    }};
}

/// Build one typed API table from one UTF-8 symbol-name map.
#[cfg(target_os = "linux")]
macro_rules! load_dll_api_named {
    (
        $library:expr,
        $candidate:expr,
        $api:ident {
            $($field:ident => $name:expr),+ $(,)?
        }
    ) => {{
        Ok($api {
            $(
                $field: $library.load_required_symbol_named($candidate, $name)?,
            )+
        })
    }};
}

#[cfg(any(target_os = "linux", target_os = "android"))]
pub(crate) use load_dll_api_bytes;
#[cfg(target_os = "linux")]
pub(crate) use load_dll_api_named;

/// One owned dynamic-library handle.
pub(crate) struct DynamicLibrary {
    /// Native dynamic-library handle.
    handle: *mut c_void,
}

impl DynamicLibrary {
    /// Open one dynamic library by one file name.
    pub(crate) fn open(name: &str) -> Result<Self, String> {
        let handle = open_dynamic_library(name)?;

        Ok(Self { handle })
    }

    /// Return one typed symbol from this loaded library.
    pub(crate) fn load_symbol<T>(&self, name: &[u8]) -> Result<T, String>
    where
        T: Copy,
    {
        load_dynamic_symbol(self.handle, name)
    }

    /// Return one required typed symbol with one candidate-qualified diagnostic.
    pub(crate) fn load_required_symbol<T>(&self, candidate: &str, name: &[u8]) -> Result<T, String>
    where
        T: Copy,
    {
        let symbol_name = CStr::from_bytes_with_nul(name)
            .map(|symbol| symbol.to_string_lossy().into_owned())
            .unwrap_or_else(|_| format!("{name:?}"));

        self.load_symbol(name).map_err(|error| {
            format!("failed to load symbol {symbol_name} from {candidate}: {error}")
        })
    }

    /// Return one typed symbol from this loaded library by one UTF-8 symbol name.
    #[cfg(target_os = "linux")]
    pub(crate) fn load_symbol_named<T>(&self, name: &str) -> Result<T, String>
    where
        T: Copy,
    {
        load_dynamic_symbol_named(self.handle, name)
    }

    /// Return one required typed symbol by one UTF-8 symbol name with one candidate-qualified diagnostic.
    #[cfg(target_os = "linux")]
    pub(crate) fn load_required_symbol_named<T>(
        &self,
        candidate: &str,
        name: &str,
    ) -> Result<T, String>
    where
        T: Copy,
    {
        self.load_symbol_named(name)
            .map_err(|error| format!("failed to load symbol {name} from {candidate}: {error}"))
    }
}

impl Debug for DynamicLibrary {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("DynamicLibrary")
            .field("handle", &self.handle)
            .finish()
    }
}

impl Drop for DynamicLibrary {
    fn drop(&mut self) {
        close_dynamic_library(self.handle);
    }
}

unsafe impl Send for DynamicLibrary {}
unsafe impl Sync for DynamicLibrary {}

/// Load one dynamic library and one api table from candidate file names.
pub(crate) fn load_library_with_api<Api, Loader>(
    candidates: &[&str],
    mut load_api: Loader,
) -> Result<(DynamicLibrary, Api), String>
where
    Loader: FnMut(&DynamicLibrary, &str) -> Result<Api, String>,
{
    if candidates.is_empty() {
        return Err(String::from(
            "no dynamic library candidates were provided to the loader",
        ));
    }

    let mut failure_messages = Vec::with_capacity(candidates.len());

    // iterate candidates in deterministic order
    for candidate in candidates {
        let library = match DynamicLibrary::open(candidate) {
            Ok(library) => library,
            Err(error) => {
                failure_messages.push(format!("{candidate}: {error}"));
                continue;
            }
        };

        // load the api table for this candidate
        let api = match load_api(&library, candidate) {
            Ok(api) => api,
            Err(error) => {
                failure_messages.push(format!("{candidate}: {error}"));
                continue;
            }
        };

        return Ok((library, api));
    }

    // report one aggregated probe diagnostic for all failed candidates
    if failure_messages.is_empty() {
        return Err(String::from(
            "all dynamic library candidates failed without one reported reason",
        ));
    }

    Err(format!(
        "all dynamic library candidates failed: {}",
        failure_messages.join("; ")
    ))
}
