use std::io;

use super::BuildId;
use super::id::CURRENT_BUILD_ID;

impl BuildId {
    /// Initialize the current build from the exact loaded WebAssembly module bytes.
    pub fn initialize_current(bytes: &[u8]) -> io::Result<Self> {
        let build_id = Self::from_bytes(bytes);
        let initialized = CURRENT_BUILD_ID.get_or_init(|| build_id);

        // preserve one exact identity for the lifetime of this module instance
        if *initialized == build_id {
            Ok(*initialized)
        } else {
            Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                "WebAssembly build identity is already initialized",
            ))
        }
    }

    /// Require the host to provide the exact loaded WebAssembly module identity.
    pub(super) fn read() -> io::Result<Self> {
        Err(io::Error::new(
            io::ErrorKind::NotFound,
            "WebAssembly host did not initialize the loaded module build identity",
        ))
    }
}
