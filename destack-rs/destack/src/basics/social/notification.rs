//! destack.basics.social.notification@2025.08.14.0

#![destack::partial(destack.basics.social.notification, file)]

#[destack::generated(NotificationStatus, enum, block)]
/// A Status of a Notification.
pub enum NotificationStatus {
    /// Pending
    UNREAD = 1,
    /// Read
    READ = 2,
    /// Dismissed
    DISMISSED = 3,
    /// Expired
    EXPIRED = 4,
    /// Rescinded
    RESCINDED = 5
}