use tspp_artifact::BuildId;

use super::Workspace;

const BUILD_ID_BYTES: &[u8; 16] = include_bytes!(concat!(env!("OUT_DIR"), "/destack-build-id"));

impl Workspace {
    /// The toolchain build containing this workspace implementation.
    pub const BUILD_ID: BuildId = BuildId::new(*BUILD_ID_BYTES);
}
