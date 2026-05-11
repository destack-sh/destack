use crate::runtime::action::{HostAction, HostActionSet};

/// Return the static desktop calendar action ids for the Unix host family.
pub(crate) fn desktop_actions() -> [HostAction; 2] {
    [HostAction::OsCalendarRead, HostAction::OsCalendarWrite]
}

/// Return dynamic Unix calendar request actions.
pub(crate) fn request_actions() -> HostActionSet {
    #[cfg(all(not(test), target_os = "linux"))]
    {
        return super::linux::request_actions();
    }

    #[cfg(all(test, target_os = "linux"))]
    {
        return super::test::request_actions();
    }

    #[cfg(any(
        all(not(test), not(target_os = "linux")),
        all(test, not(target_os = "linux"))
    ))]
    {
        super::unsupported::request_actions()
    }
}
