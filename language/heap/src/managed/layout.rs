use destack_mir::LayoutId;
use serde::{Deserialize, Serialize};

/// One compact stored layout identifier for heap metadata.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[repr(transparent)]
pub(crate) struct StoredLayoutId(u32);

impl StoredLayoutId {
    /// Return one empty stored layout identifier.
    pub(crate) const fn none() -> Self {
        Self(0)
    }

    /// Encode one optional layout identifier.
    pub(crate) fn from_option(layout_id: Option<LayoutId>) -> Self {
        match layout_id {
            Some(layout_id) => Self(layout_id.raw()),
            None => Self::none(),
        }
    }

    /// Decode this stored layout identifier.
    pub(crate) fn to_option(self) -> Option<LayoutId> {
        if self.0 == 0 {
            None
        } else {
            Some(LayoutId::new(self.0))
        }
    }
}
