//! destack.basics.access.sanction@2025.08.15.1

#![destack::generated(destack.basics.access.sanction, file)]

#[destack::generated(SanctionType, Debug, block)]
impl std::fmt::Debug for SanctionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SanctionType::Ban => write!(f, "BAN"),
            SanctionType::Mute => write!(f, "MUTE"),
        }
    }
}
