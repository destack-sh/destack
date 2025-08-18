//! destack.basics.social.notification

#![destack::partial(destack.basics.social.notification, file)]

#[destack::generated(NotificationStatus, -, block)]
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
