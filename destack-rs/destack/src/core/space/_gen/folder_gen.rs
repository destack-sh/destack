//! destack.core.space.folder

#![destack::generated(destack.core.space.folder, file)]

use crate::FolderType;

#[destack::generated(FolderType, Debug, block)]
impl std::fmt::Debug for FolderType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FolderType::System => write!(f, "SYSTEM"),
            FolderType::Home => write!(f, "HOME"),
            FolderType::General => write!(f, "GENERAL"),
            FolderType::Module => write!(f, "MODULE"),
            FolderType::App => write!(f, "APP"),
        }
    }
}
