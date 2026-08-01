use std::sync::OnceLock;
use std::{fmt, io};

const BUILD_ID_BYTES: usize = 16;

static CURRENT_BUILD_ID: OnceLock<BuildId> = OnceLock::new();

/// The Destack build that produces derived artifacts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(transparent)]
pub struct BuildId([u8; BUILD_ID_BYTES]);

impl BuildId {
    /// Create an id from exact producer identity bytes.
    pub fn from_bytes(bytes: &[u8]) -> Self {
        let hash = blake3::hash(bytes);
        let mut build_id = [0; BUILD_ID_BYTES];
        build_id.copy_from_slice(&hash.as_bytes()[..BUILD_ID_BYTES]);

        Self(build_id)
    }

    /// Return the linker id of the binary image containing this artifact implementation.
    pub fn current() -> io::Result<Self> {
        if let Some(build_id) = CURRENT_BUILD_ID.get() {
            return Ok(*build_id);
        }

        let build_id = Self::read()?;
        let build_id = CURRENT_BUILD_ID.get_or_init(|| build_id);

        Ok(*build_id)
    }

    /// Return the shared build id for isolated tests.
    pub fn test() -> Self {
        Self::from_bytes(b"destack test build")
    }

    /// Return the build id bytes.
    pub(crate) fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

impl fmt::Display for BuildId {
    /// Format the build id as lowercase hexadecimal.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        for byte in self.0 {
            write!(formatter, "{byte:02x}")?;
        }

        Ok(())
    }
}
