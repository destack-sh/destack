use std::io;

use super::BuildId;

impl BuildId {
    /// Return an identity for one process local browser module.
    pub(super) fn read() -> io::Result<Self> {
        Ok(Self::from_bytes(env!("CARGO_PKG_VERSION").as_bytes()))
    }
}
