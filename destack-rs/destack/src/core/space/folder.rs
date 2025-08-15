//! destack.core.space.folder@2025.08.15.1

#![destack::partial(destack.core.space.folder, file)]

#[destack::generated(FolderType, enum, block)]
/// FolderType
pub enum FolderType {
    /// The root folder of a Space
    System = 1,
    /// The home folder of a Space
    Home = 2,
    /// A general folder
    General = 3,
    /// A module
    Module = 4,
    /// An app folder
    App = 5,
}
