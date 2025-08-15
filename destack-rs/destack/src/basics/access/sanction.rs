//! destack.basics.access.sanction@2025.08.15.1

#![destack::partial(destack.basics.access.sanction, file)]

#[destack::generated(SanctionType, enum, block)]
/// A Type of Sanction.
pub enum SanctionType {
    /// A Ban
    Ban = 1,
    /// A Mute
    Mute = 2
}