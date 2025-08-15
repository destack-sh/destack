//! destack.core.space.folder@2025.08.14.0

#![destack::partial(destack.core.space.folder, file)]

#[destack::generated(FolderType, enum, block)]
/// FolderType
pub enum FolderType {
    /// The root folder of a Space
    SYSTEM = 1,
    /// The home folder of a Space
    HOME = 2,
    /// A general folder
    GENERAL = 3,
    /// A module
    MODULE = 4,
    /// An app folder
    APP = 5
}