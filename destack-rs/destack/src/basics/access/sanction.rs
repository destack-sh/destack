//! destack.basics.access.sanction@2025.08.14.0

#![destack::partial(destack.basics.access.sanction, file)]

#[destack::generated(SanctionType, enum, block)]
/// A Type of Sanction.
pub enum SanctionType {
    /// A Ban
    BAN = 1,
    /// A Mute
    MUTE = 2
}