//! destack.core.builtin.casing@2025.08.15.1

#![destack::generated(destack.core.builtin.casing, file)]

use crate::StringCasing;

#[destack::generated(StringCasing, Debug, block)]
impl std::fmt::Debug for StringCasing {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StringCasing::Snake => write!(f, "SNAKE"),
            StringCasing::UpperCamel => write!(f, "UPPER_CAMEL"),
            StringCasing::LowerCamel => write!(f, "LOWER_CAMEL"),
            StringCasing::AllCaps => write!(f, "ALL_CAPS"),
        }
    }
}
