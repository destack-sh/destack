use destack_source as source;

use crate::source::parse_u128;
use crate::{SourceIdParseError, bridge};

/// External component id crossing bridge boundaries.
#[bridge]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ComponentId {
    /// Canonical lowercase hex component id.
    pub id: String,
}

impl ComponentId {
    /// Convert one source component id into one bridge component id.
    pub fn from_source(id: source::ComponentId) -> Self {
        Self {
            id: format!("{:032x}", id.raw()),
        }
    }

    /// Convert this bridge component id into one source component id.
    pub fn into_source(self) -> Result<source::ComponentId, SourceIdParseError> {
        let id = parse_u128("component", &self.id)?;

        Ok(source::ComponentId::new(id))
    }
}

impl From<source::ComponentId> for ComponentId {
    /// Convert one source component id into one bridge component id.
    fn from(id: source::ComponentId) -> Self {
        Self::from_source(id)
    }
}

impl TryFrom<ComponentId> for source::ComponentId {
    type Error = SourceIdParseError;

    /// Convert one bridge component id into one source component id.
    fn try_from(id: ComponentId) -> Result<Self, Self::Error> {
        id.into_source()
    }
}
