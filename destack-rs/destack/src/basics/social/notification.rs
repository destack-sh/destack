//! destack.basics.social.notification@2025.08.15.1

#![destack::partial(destack.basics.social.notification, file)]

#[destack::generated(NotificationStatus, , block)]
/// A Status of a Notification.
pub enum NotificationStatus {
    /// Pending
    Unread = 1,
    /// Read
    Read = 2,
    /// Dismissed
    Dismissed = 3,
    /// Expired
    Expired = 4,
    /// Rescinded
    Rescinded = 5,
}
