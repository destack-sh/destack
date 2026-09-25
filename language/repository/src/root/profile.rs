use crate::config::Mode;

/// The stable profile id.
pub use tspp_source::ProfileId;

/// Return whether one active mode is present.
pub fn has_mode(modes: &[String], mode: Mode) -> bool {
    modes.iter().any(|active| active == mode.name)
}
