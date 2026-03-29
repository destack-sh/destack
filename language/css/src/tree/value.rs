use crate::ComponentValueList;
use serde::{Deserialize, Serialize};

/// One authored CSS declaration value.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeclarationValue {
    /// The declaration value component values.
    pub components: ComponentValueList,
}

impl DeclarationValue {
    /// Return this declaration value as component values.
    pub fn components(&self) -> &ComponentValueList {
        &self.components
    }

    /// Return this declaration value as mutable component values.
    pub fn components_mut(&mut self) -> &mut ComponentValueList {
        &mut self.components
    }
}
