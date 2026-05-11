use crate::runtime::action::HostAction;

/// Return the static desktop contact action ids for the Unix host family.
pub(crate) fn desktop_actions() -> [HostAction; 2] {
    [HostAction::OsContactRead, HostAction::OsContactWrite]
}
