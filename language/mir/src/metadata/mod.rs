mod debug;
mod dispatch;
mod layout;
mod memory;
mod provenance;
mod r#type;
mod union;

pub use debug::*;
pub use dispatch::*;
pub(crate) use layout::complete_layout_metadata;
pub use layout::*;
pub use memory::*;
pub use provenance::*;
pub use r#type::*;
pub use union::*;

use serde::{Deserialize, Serialize};

/// Structured MIR metadata domains.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Metadata {
    /// Canonical layout facts.
    pub layout: LayoutMetadata,
    /// Canonical dispatch facts.
    pub dispatch: DispatchMetadata,
    /// Provenance and source-tracking facts.
    pub provenance: Provenance,
    /// Debug metadata.
    pub debug: Debug,
    /// Memory and alias metadata.
    pub memory: Memory,
}
