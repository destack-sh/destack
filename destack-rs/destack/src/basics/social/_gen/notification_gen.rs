//! destack.basics.social.notification@2025.08.15.1

#![destack::generated(destack.basics.social.notification, file)]

use crate::NotificationStatus;

#[destack::generated(NotificationStatus, Debug, block)]
impl std::fmt::Debug for NotificationStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NotificationStatus::Unread => write!(f, "UNREAD"),
            NotificationStatus::Read => write!(f, "READ"),
            NotificationStatus::Dismissed => write!(f, "DISMISSED"),
            NotificationStatus::Expired => write!(f, "EXPIRED"),
            NotificationStatus::Rescinded => write!(f, "RESCINDED"),
        }
    }
}
