use std::io;

use super::BuildId;

impl BuildId {
    /// Reject targets without a loaded executable build identifier.
    pub(super) fn read() -> io::Result<Self> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "this target does not expose a loaded executable build id",
        ))
    }
}
