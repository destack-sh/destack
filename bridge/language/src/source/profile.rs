use destack_source as source;

use crate::source::parse_u128;
use crate::{SourceIdParseError, bridge};

/// External profile id crossing bridge boundaries.
#[bridge]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ProfileId {
    /// Canonical lowercase hex profile id.
    pub id: String,
}

impl ProfileId {
    /// Convert one source profile id into one bridge profile id.
    pub fn from_source(id: source::ProfileId) -> Self {
        Self {
            id: format!("{:032x}", id.raw()),
        }
    }

    /// Convert this bridge profile id into one source profile id.
    pub fn into_source(self) -> Result<source::ProfileId, SourceIdParseError> {
        let id = parse_u128("profile", &self.id)?;

        Ok(source::ProfileId::new(id))
    }
}

impl From<source::ProfileId> for ProfileId {
    /// Convert one source profile id into one bridge profile id.
    fn from(id: source::ProfileId) -> Self {
        Self::from_source(id)
    }
}

impl TryFrom<ProfileId> for source::ProfileId {
    type Error = SourceIdParseError;

    /// Convert one bridge profile id into one source profile id.
    fn try_from(id: ProfileId) -> Result<Self, Self::Error> {
        id.into_source()
    }
}
