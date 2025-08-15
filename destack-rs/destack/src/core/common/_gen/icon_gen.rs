//! destack.core.common.icon@2025.08.15.1

#![destack::generated(destack.core.common.icon, file)]

use crate::IconType;

#[destack::generated(IconType, Debug, block)]
impl std::fmt::Debug for IconType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IconType::Emoji => write!(f, "EMOJI"),
            IconType::File => write!(f, "FILE"),
            IconType::FileUrl => write!(f, "FILE_URL"),
        }
    }
}
